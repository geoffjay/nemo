//! XML parser implementation.

use crate::error::ParseError;
use crate::location::SourceLocation;
use crate::Value;
use indexmap::IndexMap;
use quick_xml::events::{BytesCData, BytesStart, Event};
use quick_xml::Reader;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// A parsed single-file component (`.nemo` SFC).
///
/// One `.nemo` file bundles a component's markup (`<template>`), scoped styling
/// (`<style>`), and scoped behavior (`<script>`). It is produced by
/// [`XmlParser::parse_sfc`] and compiled onto the existing template machinery by
/// the runtime: the [`template`](Self::template) becomes a `TemplateMap` entry
/// keyed by the SFC's tag, the [`script`](Self::script) is loaded under
/// `sfc:<tag>`, and instances of the tag are rewritten into template instances
/// before expansion.
#[derive(Debug, Clone, PartialEq)]
pub struct SfcDefinition {
    /// The `<template name>` attribute, if present. The final tag the SFC is
    /// used as is resolved by the importer (`as=` > this name > filename stem).
    pub name: Option<String>,
    /// The template body: a single-root component `Value` (the shape
    /// `process_component_element` produces), ready to merge into a `TemplateMap`.
    ///
    /// For an `app.nemo` SFC, this is the layout tree (what `<layout>` carries in
    /// `app.xml`); [`compile_app_sfc`](crate::compile_app_sfc) maps it to the
    /// `layout` key.
    pub template: Value,
    /// Raw `<style>` body, if present. Folded onto template nodes at compile time.
    pub style: Option<String>,
    /// Raw `<script>` body (Rhai), if present. Loaded under `sfc:<tag>`.
    pub script: Option<String>,
    /// Declared props from an optional `<props>` block. Empty when omitted (props
    /// are then stringly-typed and have no defaults).
    pub props: Vec<SfcProp>,
    /// Slots declared by `<slot [name] [required] [multiple]/>` in the template,
    /// in document order. Used for slot validation and schema export.
    pub slots: Vec<SfcSlot>,
    /// App-level blocks (an `app.nemo` SFC). `None` for a component `.nemo`.
    ///
    /// * `app` — the `<app>` block processed by `process_app` (window/theme/etc).
    /// * `data` — the accumulated `<data>` blocks processed by `process_data`.
    /// * `variables` — the accumulated `<variable>` blocks processed by
    ///   `process_variable`.
    /// * `sfc_imports` — the `sfc` sub-map built from `<imports>`/`<import>`
    ///   blocks (the same map `process_import` populates under the `sfc` key).
    /// * `scripts` — XML `<script src=… on-load=… />` elements processed by
    ///   `process_script` (the raw-text `<script>` body lives in
    ///   [`script`](Self::script) and is folded into `scripts.inline` by
    ///   `compile_app_sfc`).
    pub app_blocks: Option<AppBlocks>,
}

/// App-level blocks parsed from an `app.nemo` SFC — the SFC equivalent of the
/// top-level `<nemo>` children that `process_root` handles in `app.xml`.
///
/// Each field holds the same `Value` the corresponding `process_*` function
/// produces, so [`compile_app_sfc`](crate::compile_app_sfc) can assemble them
/// into the identical `Value` tree without re-running the processing logic.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AppBlocks {
    /// The `<app>` block → `config["app"]`.
    pub app: Option<Value>,
    /// Accumulated `<data>` blocks → `config["data"]`.
    pub data: Value,
    /// Accumulated `<variable>` blocks → `config["variable"]`.
    pub variables: Value,
    /// The `sfc` sub-map from `<imports>`/`<import>` → `config["sfc"]`.
    pub sfc_imports: Value,
    /// XML `<script src=… />` blocks → `config["scripts"]` (without the raw-text
    /// inline body, which is merged separately).
    pub scripts: Value,
    /// The `id` of the `<template>` root element, used as the key in the
    /// `layout.component` map (matching `process_layout`'s child-keying). Empty
    /// for a component `.nemo` (no app blocks).
    pub layout_root_id: String,
}

/// A slot declared in an SFC template via `<slot name="…" required multiple/>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SfcSlot {
    /// Slot name; `"default"` for an unnamed `<slot/>`.
    pub name: String,
    /// Whether at least one child must target this slot (`nemo validate` checks it).
    pub required: bool,
    /// Whether more than one child may target this slot (default `true`).
    pub multiple: bool,
}

/// A single declared SFC prop from `<props><prop name type default required/></props>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SfcProp {
    /// Prop name (matches the `${name}` placeholder and the instance attribute).
    pub name: String,
    /// Declared type: `string` (default), `int`, `float`, or `bool`.
    pub ty: String,
    /// Default value (already coerced to `ty`) used when an instance omits the prop.
    pub default: Option<Value>,
    /// Whether the prop must be supplied by the instance (checked by `nemo validate`).
    pub required: bool,
}

/// Coerces a raw string to the given SFC prop type. Type names mirror the scalar
/// `#[derive(NemoComponent)]` model: `string`/`int`/`float`/`bool` (with common
/// aliases). Returns `None` if the value doesn't parse as that type.
pub(crate) fn coerce_typed_value(ty: &str, raw: &str) -> Option<Value> {
    let raw = raw.trim();
    match ty {
        "int" | "integer" | "i64" => raw.parse::<i64>().ok().map(Value::Integer),
        "float" | "number" | "f64" => raw.parse::<f64>().ok().map(Value::Float),
        "bool" | "boolean" => raw.parse::<bool>().ok().map(Value::Bool),
        // "string" and anything unrecognized → string.
        _ => Some(Value::String(raw.to_string())),
    }
}

/// The pre-split result of a `.nemo` SFC: the `<template>` half (still XML) plus
/// the verbatim raw-text bodies of `<script>`/`<style>`, extracted *before* the
/// XML reader sees them so their contents never need `<![CDATA[…]]>`.
///
/// `script`/`style` are `None` only when the block is absent or empty after
/// trimming, matching the old `__cdata__` filter at `parse_sfc`.
struct SfcBlocks {
    /// The `.nemo` source with `<script>`/`<style>` blocks removed, leaving
    /// `<template>` (and `<props>`) for `quick-xml`. Whitespace where the
    /// blocks stood is collapsed so the XML reader sees clean siblings.
    template_xml: String,
    script: Option<String>,
    style: Option<String>,
}

/// Splits a `.nemo` SFC into its `<template>`/`<props>` half (kept as XML) and
/// the verbatim raw-text bodies of top-level `<script>`/`<style>` blocks,
/// treating the latter as HTML-style raw-text elements — their interior is
/// captured up to the matching close tag without XML-parsing, so `<`/`&` (Rhai
/// `&&`, generics, CSS `>` combinators) need no escaping or CDATA wrapper.
///
/// A captured body that *still* carries a `<![CDATA[ … ]]>` wrapper has the
/// leading/trailing markers stripped, so existing CDATA-wrapped files parse
/// unchanged. CDATA becomes optional, not forbidden.
///
/// Top-level here means a sibling of `<template>`/`<props>`, not a descendant:
/// a `<script>` nested inside `<template>` is left for the XML reader and is
/// *not* captured. A single depth-aware pass tracks element nesting (skipping
/// comments, CDATA sections, and self-closing tags) so only depth-0
/// `<script>`/`<style>` are treated as raw-text.
///
/// v1 limitations (pinned by tests): a literal `</script>`/`</style>` inside a
/// Rhai/CSS string closes the block — the same known limitation HTML has; at
/// most one `<script>` and one `<style>` are captured (a later occurrence
/// overwrites an earlier one).
fn split_sfc_blocks(content: &str) -> SfcBlocks {
    let bytes = content.as_bytes();
    let len = bytes.len();
    let mut script: Option<String> = None;
    let mut style: Option<String> = None;
    let mut removals: Vec<(usize, usize)> = Vec::new();
    let mut depth: usize = 0;
    let mut i = 0usize;

    while i < len {
        // Comment: `<!-- … -->` — skip, never affects depth or capture.
        if bytes[i..].starts_with(b"<!--") {
            i += 4;
            while i < len && !bytes[i..].starts_with(b"-->") {
                i += 1;
            }
            i = i.saturating_add(3).min(len);
            continue;
        }
        // CDATA section: `<![CDATA[ … ]]>` — skip verbatim, never affects depth.
        if bytes[i..].starts_with(b"<![CDATA[") {
            i += 9;
            while i < len && !bytes[i..].starts_with(b"]]>") {
                i += 1;
            }
            i = i.saturating_add(3).min(len);
            continue;
        }
        if bytes[i] == b'<' {
            // Closing tag: `</name …>` — decrement depth (bounded at 0).
            if i + 1 < len && bytes[i + 1] == b'/' {
                depth = depth.saturating_sub(1);
                i += 2;
                // Skip to the tag's `>`.
                while i < len && bytes[i] != b'>' {
                    i += 1;
                }
                i = i.saturating_add(1).min(len);
                continue;
            }
            // Open tag: parse the tag name.
            let tag_start = i + 1;
            let mut j = tag_start;
            while j < len {
                let c = bytes[j];
                if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == b'>' || c == b'/' {
                    break;
                }
                j += 1;
            }
            if j == tag_start {
                // Stray `<` with no name — skip the byte.
                i += 1;
                continue;
            }
            let name = &content[tag_start..j];
            // Skip attributes to the end of the open tag.
            let mut k = j;
            while k < len && bytes[k] != b'>' {
                k += 1;
            }
            let tag_end = k; // index of `>` (or `len` if unterminated)
                             // Self-closing `<name …/>`: no depth change, no capture.
            let self_closing = tag_end > 0 && bytes[tag_end - 1] == b'/';

            if !self_closing {
                // A raw-text element at the top level: capture its verbatim
                // body up to the first literal `</name>` close tag.
                if depth == 0 && (name == "script" || name == "style") {
                    let body_start = (tag_end + 1).min(len);
                    let close = format!("</{}>", name);
                    let close_b = close.as_bytes();
                    // Scan for the close tag starting at body_start.
                    let mut m = body_start;
                    let body_end = loop {
                        if m + close_b.len() > len {
                            // Unterminated raw-text block: capture to EOF.
                            break len;
                        }
                        if &bytes[m..m + close_b.len()] == close_b {
                            break m;
                        }
                        m += 1;
                    };
                    let block_end = (body_end + close_b.len()).min(len);
                    let body = &content[body_start..body_end];
                    let stripped = strip_cdata(body);
                    let captured = stripped.trim().to_string();
                    if !captured.is_empty() {
                        match name {
                            "script" => script = Some(captured),
                            "style" => style = Some(captured),
                            _ => {}
                        }
                    }
                    removals.push((i, block_end));
                    // Resume scanning after the captured block; depth stays 0.
                    i = block_end;
                    continue;
                }
                // Any other open tag: enter it (depth tracks nesting so a
                // `<script>` inside `<template>` is not captured).
                depth += 1;
            }
            i = tag_end.saturating_add(1).min(len);
            continue;
        }
        i += 1;
    }

    let template_xml = remove_ranges(content, removals);

    SfcBlocks {
        template_xml,
        script: script.filter(|s| !s.trim().is_empty()),
        style: style.filter(|s| !s.trim().is_empty()),
    }
}

/// Strips a single optional `<![CDATA[ … ]]>` wrapper from a raw-text body, if
/// present. Only trims one leading marker and one trailing marker so a body
/// that genuinely starts/ends with those literals (vanishingly rare in Rhai/CSS)
/// is still handled by the `trim()` in the caller.
fn strip_cdata(body: &str) -> String {
    let trimmed = body.trim();
    let s = trimmed.strip_prefix("<![CDATA[").unwrap_or(trimmed);
    let s = s.strip_suffix("]]>").unwrap_or(s);
    s.to_string()
}

/// Removes `ranges` (byte-offset spans, inclusive start, exclusive end) from
/// `content`, replacing each with a single space so siblings don't fuse.
/// Ranges are applied back-to-front to keep earlier indices valid.
fn remove_ranges(content: &str, mut ranges: Vec<(usize, usize)>) -> String {
    if ranges.is_empty() {
        return content.to_string();
    }
    ranges.sort_unstable_by_key(|r| std::cmp::Reverse(r.0));
    let mut out = content.to_string();
    for (start, end) in ranges {
        out.replace_range(start..end, " ");
    }
    out
}

/// Parser for XML configuration files.
pub struct XmlParser {
    source_name: String,
    base_dir: Option<PathBuf>,
    /// Monotonic counter for generating ids for id-less ("anonymous")
    /// components. It must be unique across the *whole document*, not per
    /// parent: components are ultimately stored in a flat id-keyed map, so two
    /// anonymous siblings in different parents that shared an id (e.g. every
    /// `__anon_1`) would collapse into one — the classic "all labels show the
    /// last one's text" bug. `Cell` gives interior mutability under `&self`.
    anon_counter: Cell<usize>,
    /// The `.nemo/packages` cache dir, set when the project root is known, so a
    /// remote module `<import src="github.com/…">` resolves against the cache.
    packages_dir: Option<PathBuf>,
    /// `module → version` from `nemo.lock`, used to pick the cached package dir.
    locked_versions: BTreeMap<String, String>,
}

impl XmlParser {
    /// Creates a new XML parser.
    pub fn new() -> Self {
        XmlParser {
            source_name: "<input>".to_string(),
            base_dir: None,
            anon_counter: Cell::new(0),
            packages_dir: None,
            locked_versions: BTreeMap::new(),
        }
    }

    /// Sets the remote-package cache dir and the locked `module → version` map,
    /// so module imports resolve against `.nemo/packages`.
    pub fn with_packages(
        mut self,
        packages_dir: impl Into<PathBuf>,
        locked_versions: BTreeMap<String, String>,
    ) -> Self {
        self.packages_dir = Some(packages_dir.into());
        self.locked_versions = locked_versions;
        self
    }

    /// Sets the source name for error messages.
    pub fn with_source_name(mut self, name: impl Into<String>) -> Self {
        self.source_name = name.into();
        self
    }

    /// Sets the base directory for resolving `<include>` paths.
    pub fn with_base_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.base_dir = Some(dir.into());
        self
    }

    /// Parses XML content into a Value.
    pub fn parse(&self, content: &str) -> Result<Value, ParseError> {
        // Reset the anonymous-component counter so ids are deterministic per
        // document even if this parser instance is reused.
        self.anon_counter.set(0);

        let mut reader = Reader::from_str(content);
        reader.config_mut().trim_text(true);

        let root = self
            .parse_element(&mut reader, None)
            .map_err(|e| ParseError::new(e, SourceLocation::new(&self.source_name, 1, 1)))?;

        // The document-level parse returns an object with __children__ containing the <nemo> element.
        // We need to unwrap <nemo> and process its children into top-level keys.
        if let Some(doc_children) = root.as_object().and_then(|m| m.get("__children__")) {
            if let Some(arr) = doc_children.as_array() {
                // Find the <nemo> root element
                for child in arr {
                    if let Some(child_obj) = child.as_object() {
                        if child_obj.get("__type__").and_then(|v| v.as_str()) == Some("nemo") {
                            // Process the nemo element's children
                            if let Some(nemo_children) = child_obj.get("__children__") {
                                return self.process_root(nemo_children);
                            } else {
                                return Ok(Value::Object(IndexMap::new()));
                            }
                        }
                    }
                }
            }
        }

        // Fallback: no <nemo> wrapper found, try processing directly
        if let Some(children) = root.as_object().and_then(|m| m.get("__children__")) {
            self.process_root(children)
        } else {
            Ok(Value::Object(IndexMap::new()))
        }
    }

    /// Processes the root <nemo> element's children into the expected top-level structure.
    fn process_root(&self, children: &Value) -> Result<Value, ParseError> {
        let children_arr = match children.as_array() {
            Some(arr) => arr,
            None => return Ok(Value::Object(IndexMap::new())),
        };

        let mut result = IndexMap::new();

        for child in children_arr {
            let obj = match child.as_object() {
                Some(o) => o,
                None => continue,
            };

            let element_type = match obj.get("__type__").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => continue,
            };

            match element_type.as_str() {
                "variable" => {
                    self.process_variable(obj, &mut result);
                }
                "app" => {
                    let app_val = self.process_app(obj);
                    result.insert("app".to_string(), app_val);
                }
                "script" => {
                    self.process_script(obj, &mut result);
                }
                "data" => {
                    self.process_data(obj, &mut result);
                }
                "template" => {
                    self.process_template(obj, &mut result);
                }
                "templates" => {
                    // <templates> wrapper element containing multiple <template> children
                    if let Some(tmpl_children) = obj.get("__children__").and_then(|v| v.as_array())
                    {
                        for tmpl_child in tmpl_children {
                            if let Some(tmpl_obj) = tmpl_child.as_object() {
                                if tmpl_obj.get("__type__").and_then(|v| v.as_str())
                                    == Some("template")
                                {
                                    self.process_template(tmpl_obj, &mut result);
                                }
                            }
                        }
                    }
                }
                "include" => {
                    self.process_include(obj, &mut result)?;
                }
                "imports" => {
                    // <imports> wrapper containing multiple <import> children.
                    if let Some(import_children) =
                        obj.get("__children__").and_then(|v| v.as_array())
                    {
                        for import_child in import_children {
                            if let Some(import_obj) = import_child.as_object() {
                                if import_obj.get("__type__").and_then(|v| v.as_str())
                                    == Some("import")
                                {
                                    self.process_import(import_obj, &mut result)?;
                                }
                            }
                        }
                    }
                }
                "import" => {
                    self.process_import(obj, &mut result)?;
                }
                "components" => {
                    self.process_components_dir(obj, &mut result)?;
                }
                "layout" => {
                    let layout_val = self.process_layout(obj);
                    result.insert("layout".to_string(), layout_val);
                }
                "themes" => {
                    let themes_val = self.process_themes_block(obj);
                    result.insert("themes".to_string(), themes_val);
                }
                _ => {
                    // Unknown top-level element, store as-is
                    let cleaned = self.clean_element(obj);
                    result.insert(element_type, cleaned);
                }
            }
        }

        Ok(Value::Object(result))
    }

    /// Processes a <variable> element into the `variable` map.
    fn process_variable(
        &self,
        obj: &IndexMap<String, Value>,
        result: &mut IndexMap<String, Value>,
    ) {
        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            return;
        }

        let mut var_config = IndexMap::new();
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "name" => continue,
                _ => {
                    var_config.insert(key.clone(), val.clone());
                }
            }
        }

        let variables = result
            .entry("variable".to_string())
            .or_insert_with(|| Value::Object(IndexMap::new()));
        if let Value::Object(vars) = variables {
            vars.insert(name.to_string(), Value::Object(var_config));
        }
    }

    /// Processes an <app> element into the `app` object.
    fn process_app(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut app = IndexMap::new();

        // Copy attributes
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" => continue,
                _ => {
                    app.insert(key.clone(), val.clone());
                }
            }
        }

        // Process children
        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    let child_type = child_obj
                        .get("__type__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    match child_type {
                        "window" => {
                            app.insert("window".to_string(), self.process_nested_block(child_obj));
                        }
                        "theme" => {
                            app.insert("theme".to_string(), self.process_theme_block(child_obj));
                        }
                        "plugins" => {
                            let plugins = self.process_plugins_block(child_obj);
                            app.insert("plugins".to_string(), plugins);
                        }
                        _ => {
                            let cleaned = self.clean_element(child_obj);
                            app.insert(child_type.to_string(), cleaned);
                        }
                    }
                }
            }
        }

        Value::Object(app)
    }

    /// Processes a `<plugins>` block into a `Value::Array` of plugin objects.
    ///
    /// Each `<plugin name="foo" />` child becomes `{ "name": "foo" }`.
    /// An optional `load="false"` attribute is preserved for the runtime to filter on.
    fn process_plugins_block(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut plugins = Vec::new();

        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    let child_type = child_obj
                        .get("__type__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if child_type == "plugin" {
                        plugins.push(self.clean_element(child_obj));
                    }
                }
            }
        }

        Value::Array(plugins)
    }

    /// Processes a top-level `<themes>` block into a `Value::Array` of theme-set
    /// references.
    ///
    /// Each `<theme-set src="themes/foo.json" />` child becomes `{ "src": "..." }`.
    /// The JSON files themselves are **not** read or parsed here — that requires
    /// the gpui-component `ThemeSet` type, which lives in the `nemo` crate. This
    /// parser only records the reference so the app layer can load it later
    /// (resolving `src` relative to the config directory).
    fn process_themes_block(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut sets = Vec::new();

        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    let child_type = child_obj
                        .get("__type__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    // `<theme-set>` is kebab→snake normalized to `theme_set`.
                    if child_type == "theme_set" {
                        sets.push(self.clean_element(child_obj));
                    }
                }
            }
        }

        Value::Array(sets)
    }

    /// Processes an `<app><theme>` element, preserving a `<extend>` override block.
    ///
    /// Attributes (`name`, `mode`, `font-family`) are copied through. An optional
    /// `<extend>` child, whose `<color key="..." value="..." />` children carry
    /// per-color overrides, is flattened into `theme.extend = { key: value, ... }`
    /// for `apply_theme_from_runtime` to merge over the resolved base theme.
    fn process_theme_block(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut theme = IndexMap::new();

        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "__cdata__" => continue,
                _ => {
                    theme.insert(key.clone(), val.clone());
                }
            }
        }

        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            for child in children {
                let Some(child_obj) = child.as_object() else {
                    continue;
                };
                if child_obj.get("__type__").and_then(|v| v.as_str()) != Some("extend") {
                    continue;
                }

                let mut colors = IndexMap::new();
                if let Some(color_children) =
                    child_obj.get("__children__").and_then(|v| v.as_array())
                {
                    for color in color_children {
                        let Some(color_obj) = color.as_object() else {
                            continue;
                        };
                        if color_obj.get("__type__").and_then(|v| v.as_str()) != Some("color") {
                            continue;
                        }
                        let key = color_obj.get("key").and_then(|v| v.as_str());
                        let value = color_obj.get("value").and_then(|v| v.as_str());
                        if let (Some(k), Some(v)) = (key, value) {
                            colors.insert(k.to_string(), Value::String(v.to_string()));
                        }
                    }
                }
                theme.insert("extend".to_string(), Value::Object(colors));
            }
        }

        Value::Object(theme)
    }

    /// Processes a nested block element (like <window>) that may have sub-elements.
    fn process_nested_block(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut block = IndexMap::new();

        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" => continue,
                _ => {
                    block.insert(key.clone(), val.clone());
                }
            }
        }

        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    let child_type = child_obj
                        .get("__type__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if child_type == "header_bar" {
                        // `<header-bar>` needs dedicated handling so its repeated
                        // `<menu-item>` children survive as a list rather than
                        // collapsing into a single key.
                        block.insert("header_bar".to_string(), self.process_header_bar(child_obj));
                    } else if !child_type.is_empty() {
                        block.insert(
                            kebab_to_snake(child_type),
                            self.process_nested_block(child_obj),
                        );
                    }
                }
            }
        }

        Value::Object(block)
    }

    /// Processes a `<header-bar>` element: copies its attributes and collects any
    /// `<menu-item>` children into a `menu_items` array. The generic
    /// `process_nested_block` would otherwise collapse repeated `menu-item`
    /// children into a single `menu_item` key, losing all but the last.
    fn process_header_bar(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut block = IndexMap::new();

        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" => continue,
                _ => {
                    block.insert(key.clone(), val.clone());
                }
            }
        }

        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            let mut menu_items = Vec::new();
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    let child_type = child_obj
                        .get("__type__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if child_type == "menu_item" {
                        // Attribute keys are already kebab→snake normalized at
                        // parse time (`on-click` → `on_click`).
                        let mut item = IndexMap::new();
                        for (k, v) in child_obj {
                            match k.as_str() {
                                "__type__" | "__children__" => continue,
                                _ => {
                                    item.insert(k.clone(), v.clone());
                                }
                            }
                        }
                        menu_items.push(Value::Object(item));
                    }
                }
            }
            if !menu_items.is_empty() {
                block.insert("menu_items".to_string(), Value::Array(menu_items));
            }
        }

        Value::Object(block)
    }

    /// Processes <script> elements into the `scripts` object.
    fn process_script(&self, obj: &IndexMap<String, Value>, result: &mut IndexMap<String, Value>) {
        let scripts = result
            .entry("scripts".to_string())
            .or_insert_with(|| Value::Object(IndexMap::new()));

        if let Value::Object(scripts_obj) = scripts {
            // src attribute → path key
            if let Some(src) = obj.get("src").and_then(|v| v.as_str()) {
                scripts_obj.insert("path".to_string(), Value::String(src.to_string()));
            }

            // features attribute → features key (comma-separated, e.g.
            // "file-io"). Used by the runtime to enable opt-in Rhai
            // packages like rhai-fs. Stored as an array of strings.
            if let Some(features) = obj.get("features").and_then(|v| v.as_str()) {
                let list: Vec<Value> = features
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .map(Value::String)
                    .collect();
                if !list.is_empty() {
                    scripts_obj.insert("features".to_string(), Value::Array(list));
                }
            }

            // on-load attribute → on_load key. Names a handler function that
            // the runtime calls exactly once, after the layout is built, so a
            // script can hydrate the UI from persisted state at startup (the
            // only "run once on load" hook nemo exposes). Attribute keys are
            // kebab→snake normalized at parse time, so `on-load` arrives as
            // `on_load`.
            if let Some(on_load) = obj.get("on_load").and_then(|v| v.as_str()) {
                let trimmed = on_load.trim();
                if !trimmed.is_empty() {
                    scripts_obj.insert("on_load".to_string(), Value::String(trimmed.to_string()));
                }
            }

            // CDATA content → inline key. Trimmed to match the raw-text
            // splitter's behavior (split_sfc_blocks trims script bodies), so
            // an app.nemo's raw-text <script> and an app.xml's CDATA <script>
            // produce identical scripts.inline values.
            if let Some(cdata) = obj.get("__cdata__").and_then(|v| v.as_str()) {
                let trimmed = cdata.trim();
                if !trimmed.is_empty() {
                    let inline = scripts_obj
                        .entry("inline".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = inline {
                        arr.push(Value::String(trimmed.to_string()));
                    }
                }
            }
        }
    }

    /// Processes <data> elements into the `data` object.
    fn process_data(&self, obj: &IndexMap<String, Value>, result: &mut IndexMap<String, Value>) {
        let data = result
            .entry("data".to_string())
            .or_insert_with(|| Value::Object(IndexMap::new()));

        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            if let Value::Object(data_obj) = data {
                for child in children {
                    if let Some(child_obj) = child.as_object() {
                        let child_type = child_obj
                            .get("__type__")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let name = child_obj.get("name").and_then(|v| v.as_str()).unwrap_or("");

                        if name.is_empty() {
                            continue;
                        }

                        match child_type {
                            "source" => {
                                let sources = data_obj
                                    .entry("source".to_string())
                                    .or_insert_with(|| Value::Object(IndexMap::new()));
                                if let Value::Object(sources_obj) = sources {
                                    sources_obj.insert(
                                        name.to_string(),
                                        self.clean_data_element(child_obj),
                                    );
                                }
                            }
                            "sink" => {
                                let sinks = data_obj
                                    .entry("sink".to_string())
                                    .or_insert_with(|| Value::Object(IndexMap::new()));
                                if let Value::Object(sinks_obj) = sinks {
                                    sinks_obj.insert(
                                        name.to_string(),
                                        self.clean_data_element(child_obj),
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// Cleans a data element (source/sink), removing internal keys and `name`.
    fn clean_data_element(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut cleaned = IndexMap::new();
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "name" => continue,
                _ => {
                    cleaned.insert(key.clone(), val.clone());
                }
            }
        }

        // Process array attributes (like topics, channels, subjects)
        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    let child_type = child_obj
                        .get("__type__")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if !child_type.is_empty() {
                        // Collect as array items
                        let arr = cleaned
                            .entry(child_type.to_string())
                            .or_insert_with(|| Value::Array(Vec::new()));
                        if let Value::Array(a) = arr {
                            // If the child has a "value" attr, use that; otherwise use text content
                            if let Some(val) = child_obj.get("value") {
                                a.push(val.clone());
                            }
                        }
                    }
                }
            }
        }

        Value::Object(cleaned)
    }

    /// Processes `<template>` elements into the `templates.template` map.
    ///
    /// In HCL, `template "nav_item" { type = "button" variant = "ghost" }` produces
    /// `{type: "button", variant: "ghost"}` — the template body IS the component
    /// definition directly. XML templates wrap content in child elements:
    ///
    /// ```xml
    /// <template name="nav_item">
    ///   <button variant="ghost" />
    /// </template>
    /// ```
    ///
    /// When a template has exactly one child element, we unwrap it so the child's
    /// processed value becomes the template definition (matching the expected shape).
    fn process_template(
        &self,
        obj: &IndexMap<String, Value>,
        result: &mut IndexMap<String, Value>,
    ) {
        let name = obj.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            return;
        }

        let templates = result
            .entry("templates".to_string())
            .or_insert_with(|| Value::Object(IndexMap::new()));

        if let Value::Object(templates_obj) = templates {
            let template_entries = templates_obj
                .entry("template".to_string())
                .or_insert_with(|| Value::Object(IndexMap::new()));

            if let Value::Object(entries) = template_entries {
                let template_val = self.build_template_value(obj);
                entries.insert(name.to_string(), template_val);
            }
        }
    }

    /// Builds the Value for a template definition.
    ///
    /// If the template has exactly one child element, unwraps it so the child
    /// becomes the template body directly (e.g., `<template name="x"><button .../></template>`
    /// produces `{type: "button", ...}` rather than `{component: {__anon: {type: "button", ...}}}`).
    ///
    /// If the template has attributes beyond `name` (and internal keys), those are
    /// included as properties of the template body, allowing inline definitions like
    /// `<template name="x" type="button" variant="ghost" />`.
    fn build_template_value(&self, obj: &IndexMap<String, Value>) -> Value {
        let children = obj
            .get("__children__")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        // Collect non-internal, non-name attributes from the template element itself
        let mut template_attrs = IndexMap::new();
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "__cdata__" | "name" => continue,
                _ => {
                    template_attrs.insert(key.clone(), val.clone());
                }
            }
        }

        // Filter children to only component-like elements (not bindings, slots, vars)
        let component_children: Vec<&Value> = children
            .iter()
            .filter(|c| {
                c.as_object()
                    .and_then(|o| o.get("__type__").and_then(|v| v.as_str()))
                    .map(|t| !["binding", "slot", "vars"].contains(&t))
                    .unwrap_or(false)
            })
            .collect();

        if component_children.len() == 1 && template_attrs.is_empty() {
            // Single child element: unwrap it as the template body
            let child_obj = component_children[0].as_object().unwrap();
            self.process_component_element(child_obj)
        } else if component_children.is_empty() && !template_attrs.is_empty() {
            // Inline template: attributes are the template body
            // (e.g., <template name="x" type="button" variant="ghost" />)
            Value::Object(template_attrs)
        } else {
            // Multiple children or mixed: use process_component_tree which wraps
            // children in a component map
            self.process_component_tree(obj)
        }
    }

    /// Processes an <include> element by loading and merging an external file.
    fn process_include(
        &self,
        obj: &IndexMap<String, Value>,
        result: &mut IndexMap<String, Value>,
    ) -> Result<(), ParseError> {
        let src = match obj
            .get("src")
            .or_else(|| obj.get("href"))
            .and_then(|v| v.as_str())
        {
            Some(s) => s,
            None => return Ok(()),
        };

        let include_path = self.resolve_path(src);

        if !include_path.exists() {
            return Err(ParseError::new(
                format!("Include file not found: {}", include_path.display()),
                SourceLocation::new(&self.source_name, 1, 1),
            ));
        }

        let content = std::fs::read_to_string(&include_path).map_err(|e| {
            ParseError::new(
                format!(
                    "Failed to read include file {}: {}",
                    include_path.display(),
                    e
                ),
                SourceLocation::new(&self.source_name, 1, 1),
            )
        })?;

        let include_parser = XmlParser::new()
            .with_source_name(include_path.display().to_string())
            .with_base_dir(
                include_path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .to_path_buf(),
            );

        let included = include_parser.parse(&content)?;

        // Merge included values into result
        if let Some(included_obj) = included.as_object() {
            for (key, val) in included_obj {
                merge_into(result, key, val);
            }
        }

        Ok(())
    }
    /// Parses a single-file component (`.nemo`) document into an
    /// [`SfcDefinition`].
    ///
    /// A `.nemo` file is *not* wrapped in `<nemo>`; its top-level children are
    /// `<template>` (required, exactly one element child), `<style>` (optional),
    /// and `<script>` (optional). The `<template>` body is flattened with the
    /// same `process_component_element` used for layout components, so an SFC is
    /// a namespaced, file-scoped superset of the existing `<template>` mechanism.
    ///
    /// `<script>`/`<style>` are parsed as HTML-style raw-text elements: a
    /// pre-pass ([`split_sfc_blocks`]) extracts their bodies verbatim *before*
    /// the XML reader sees them, so `<`/`&` (Rhai `&&`, generics, CSS `>`
    /// combinators) need no `<![CDATA[…]]>` wrapper. A body that still carries
    /// a CDATA wrapper has it stripped, so existing CDATA-wrapped files load
    /// unchanged. The multi-line/multi-run script body is captured whole.
    pub fn parse_sfc(&self, content: &str) -> Result<SfcDefinition, ParseError> {
        let blocks = split_sfc_blocks(content);

        let mut reader = Reader::from_str(&blocks.template_xml);
        reader.config_mut().trim_text(true);

        let root = self
            .parse_element(&mut reader, None)
            .map_err(|e| ParseError::new(e, SourceLocation::new(&self.source_name, 1, 1)))?;

        let children = root
            .as_object()
            .and_then(|m| m.get("__children__"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut name: Option<String> = None;
        let mut template: Option<Value> = None;
        let mut props: Vec<SfcProp> = Vec::new();
        let mut slots: Vec<SfcSlot> = Vec::new();
        // App-level block accumulators (app.nemo). Each is built with the same
        // process_* logic process_root uses, so compile_app_sfc can assemble
        // the identical Value tree.
        let mut app_result: IndexMap<String, Value> = IndexMap::new();
        let mut layout_root_id = String::new();

        // Pre-scan: detect whether this is an app.nemo (has app-level blocks)
        // before processing, so the <template> arm knows whether to capture the
        // root id for layout wrapping. App-level blocks may appear after
        // <template> in document order.
        let has_app_blocks = children.iter().any(|child| {
            child
                .as_object()
                .and_then(|o| o.get("__type__").and_then(|v| v.as_str()))
                .map(|t| {
                    matches!(
                        t,
                        "app"
                            | "data"
                            | "variable"
                            | "imports"
                            | "import"
                            | "components"
                            | "themes"
                    ) || (t == "script" && child.as_object().unwrap().get("__children__").is_none())
                })
                .unwrap_or(false)
        });

        for child in &children {
            let obj = match child.as_object() {
                Some(o) => o,
                None => continue,
            };
            match obj.get("__type__").and_then(|v| v.as_str()) {
                Some("props") => {
                    if let Some(prop_children) = obj.get("__children__").and_then(|v| v.as_array())
                    {
                        for prop_child in prop_children {
                            let po = match prop_child.as_object() {
                                Some(o) => o,
                                None => continue,
                            };
                            if po.get("__type__").and_then(|v| v.as_str()) != Some("prop") {
                                continue;
                            }
                            let pname = match po.get("name").and_then(|v| v.as_str()) {
                                Some(n) if !n.is_empty() => n.to_string(),
                                _ => {
                                    return Err(ParseError::new(
                                        "SFC <prop> requires a non-empty name".to_string(),
                                        SourceLocation::new(&self.source_name, 1, 1),
                                    ))
                                }
                            };
                            let ty = po
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("string")
                                .to_string();
                            // `default` is an attribute, so `coerce_value` may
                            // have already typed it (e.g. `"3"` → Integer). Render
                            // it back to a string and coerce to the declared type.
                            let default = po.get("default").and_then(|v| {
                                let raw = match v {
                                    Value::String(s) => s.clone(),
                                    Value::Integer(i) => i.to_string(),
                                    Value::Float(f) => f.to_string(),
                                    Value::Bool(b) => b.to_string(),
                                    _ => return None,
                                };
                                coerce_typed_value(&ty, &raw)
                            });
                            let required = po
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            props.push(SfcProp {
                                name: pname,
                                ty,
                                default,
                                required,
                            });
                        }
                    }
                }
                Some("template") => {
                    name = obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Require exactly one element child as the single template
                    // root (matches find_and_inject_slot's single-root model).
                    let element_children: Vec<&Value> = obj
                        .get("__children__")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter(|c| c.as_object().and_then(|o| o.get("__type__")).is_some())
                                .collect()
                        })
                        .unwrap_or_default();

                    if element_children.len() != 1 {
                        return Err(ParseError::new(
                            format!(
                                "SFC <template> must contain exactly one root element, found {}",
                                element_children.len()
                            ),
                            SourceLocation::new(&self.source_name, 1, 1),
                        ));
                    }
                    let root_child = element_children[0].as_object().unwrap();
                    // Capture the root id for app SFC layout wrapping (the id
                    // is stripped by process_component_element but needed as the
                    // key in layout.component, matching process_layout).
                    if has_app_blocks {
                        layout_root_id = root_child
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("root")
                            .to_string();
                    }
                    // Collect declared slots from the raw element tree (before
                    // flattening loses the `required`/`multiple` attributes).
                    Self::collect_slot_specs(root_child, &mut slots);
                    template = Some(self.process_component_element(root_child));
                }
                // App-level blocks (app.nemo). Each delegates to the same
                // process_* function process_root uses, accumulating into
                // app_result so compile_app_sfc can assemble the Value tree.
                Some("app") => {
                    let app = self.process_app(obj);
                    app_result.insert("app".to_string(), app);
                }
                Some("data") => {
                    self.process_data(obj, &mut app_result);
                }
                Some("variable") => {
                    self.process_variable(obj, &mut app_result);
                }
                Some("script") => {
                    // An XML <script src=… on-load=… /> (self-closing, so
                    // split_sfc_blocks left it for the reader). The raw-text
                    // <script> body is in blocks.script and folded in by
                    // compile_app_sfc. process_script writes to scripts.*.

                    self.process_script(obj, &mut app_result);
                }
                Some("imports") => {
                    if let Some(import_children) =
                        obj.get("__children__").and_then(|v| v.as_array())
                    {
                        for import_child in import_children {
                            if let Some(import_obj) = import_child.as_object() {
                                if import_obj.get("__type__").and_then(|v| v.as_str())
                                    == Some("import")
                                {
                                    self.process_import(import_obj, &mut app_result)?;
                                }
                            }
                        }
                    }
                }
                Some("import") => {
                    self.process_import(obj, &mut app_result)?;
                }
                Some("components") => {
                    self.process_components_dir(obj, &mut app_result)?;
                }
                Some("themes") => {
                    let themes_val = self.process_themes_block(obj);
                    app_result.insert("themes".to_string(), themes_val);
                }
                _ => {}
            }
        }

        let template = template.ok_or_else(|| {
            ParseError::new(
                "SFC file must contain a <template> element".to_string(),
                SourceLocation::new(&self.source_name, 1, 1),
            )
        })?;

        Ok(SfcDefinition {
            name,
            template,
            style: blocks.style,
            script: blocks.script,
            props,
            slots,
            app_blocks: if has_app_blocks {
                let app = app_result.shift_remove("app");
                let data = app_result
                    .shift_remove("data")
                    .unwrap_or(Value::Object(IndexMap::new()));
                let variables = app_result
                    .shift_remove("variable")
                    .unwrap_or(Value::Object(IndexMap::new()));
                let sfc_imports = app_result
                    .shift_remove("sfc")
                    .unwrap_or(Value::Object(IndexMap::new()));
                let scripts = app_result
                    .shift_remove("scripts")
                    .unwrap_or(Value::Object(IndexMap::new()));
                Some(AppBlocks {
                    app,
                    data,
                    variables,
                    sfc_imports,
                    scripts,
                    layout_root_id,
                })
            } else {
                None
            },
        })
    }

    /// Recursively walks a raw parsed element tree collecting `<slot>` elements
    /// and their `name`/`required`/`multiple` attributes.
    fn collect_slot_specs(elem: &IndexMap<String, Value>, out: &mut Vec<SfcSlot>) {
        let children = match elem.get("__children__").and_then(|v| v.as_array()) {
            Some(c) => c,
            None => return,
        };
        for child in children {
            let co = match child.as_object() {
                Some(o) => o,
                None => continue,
            };
            if co.get("__type__").and_then(|v| v.as_str()) == Some("slot") {
                let name = co
                    .get("name")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .unwrap_or("default")
                    .to_string();
                let required = co
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let multiple = co.get("multiple").and_then(|v| v.as_bool()).unwrap_or(true);
                out.push(SfcSlot {
                    name,
                    required,
                    multiple,
                });
            } else {
                Self::collect_slot_specs(co, out);
            }
        }
    }

    /// Processes an `<import src="…" [as="tag"]>` element.
    ///
    /// A **local** `src` (`./x.nemo`, or an existing file) parses that one `.nemo`
    /// and stores it under the top-level `sfc` map, keyed by tag (resolution
    /// order: `as=` > `<template name>` > filename stem). A **remote module**
    /// `src` (`github.com/owner/repo`) resolves against the `.nemo/packages` cache
    /// and brings in *all* exported components of the package (like
    /// `<components dir>`); `as=` becomes a tag-namespace prefix (`nf` → `nf_card`).
    fn process_import(
        &self,
        obj: &IndexMap<String, Value>,
        result: &mut IndexMap<String, Value>,
    ) -> Result<(), ParseError> {
        let src = match obj
            .get("src")
            .or_else(|| obj.get("href"))
            .and_then(|v| v.as_str())
        {
            Some(s) => s,
            None => return Ok(()),
        };

        // Remote module import: resolve against `.nemo/packages` and register the
        // whole package. A local file with the same spelling (rare) wins if it
        // exists, keeping local paths unambiguous.
        if crate::pkg::is_module_path(src) && !self.resolve_path(src).exists() {
            let dir = self.resolve_package_dir(src).ok_or_else(|| {
                ParseError::new(
                    format!("module '{src}' is not in the package cache — run `nemo get` first"),
                    SourceLocation::new(&self.source_name, 1, 1),
                )
            })?;
            let prefix = obj
                .get("as")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(kebab_to_snake);
            return self.register_components_from_dir(&dir, result, prefix.as_deref());
        }

        let import_path = self.resolve_path(src);

        if !import_path.exists() {
            return Err(ParseError::new(
                format!("Import file not found: {}", import_path.display()),
                SourceLocation::new(&self.source_name, 1, 1),
            ));
        }

        let content = std::fs::read_to_string(&import_path).map_err(|e| {
            ParseError::new(
                format!(
                    "Failed to read import file {}: {}",
                    import_path.display(),
                    e
                ),
                SourceLocation::new(&self.source_name, 1, 1),
            )
        })?;

        let sfc_parser = XmlParser::new()
            .with_source_name(import_path.display().to_string())
            .with_base_dir(import_path.parent().unwrap_or_else(|| Path::new(".")));
        let sfc = sfc_parser.parse_sfc(&content)?;

        // Resolve the tag: as= > <template name> > filename stem. The `as=`
        // override is kebab→snake normalized like the rest; the name/stem
        // fallback is the canonical `sfc_default_tag` derivation.
        let tag = match obj
            .get("as")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
        {
            Some(alias) => kebab_to_snake(alias),
            None => sfc_default_tag(sfc.name.as_deref(), &import_path).ok_or_else(|| {
                ParseError::new(
                    format!("Could not determine a tag name for import '{}'", src),
                    SourceLocation::new(&self.source_name, 1, 1),
                )
            })?,
        };

        let entry = Self::sfc_to_value(sfc, &import_path.display().to_string());
        Self::register_sfc(result, tag, entry);
        Ok(())
    }

    /// Flattens an [`SfcDefinition`] into the plain `Value` stored under the
    /// top-level `sfc` map, so the config stays a homogeneous `Value` tree that
    /// the runtime reads back.
    fn sfc_to_value(sfc: SfcDefinition, source_path: &str) -> Value {
        let mut entry = IndexMap::new();
        entry.insert("template".to_string(), sfc.template);
        if let Some(style) = sfc.style {
            entry.insert("style".to_string(), Value::String(style));
        }
        if let Some(script) = sfc.script {
            entry.insert("script".to_string(), Value::String(script));
        }
        if !sfc.props.is_empty() {
            let props: Vec<Value> = sfc
                .props
                .into_iter()
                .map(|p| {
                    let mut m = IndexMap::new();
                    m.insert("name".to_string(), Value::String(p.name));
                    m.insert("type".to_string(), Value::String(p.ty));
                    if let Some(default) = p.default {
                        m.insert("default".to_string(), default);
                    }
                    m.insert("required".to_string(), Value::Bool(p.required));
                    Value::Object(m)
                })
                .collect();
            entry.insert("props".to_string(), Value::Array(props));
        }
        if !sfc.slots.is_empty() {
            let slots: Vec<Value> = sfc
                .slots
                .into_iter()
                .map(|s| {
                    let mut m = IndexMap::new();
                    m.insert("name".to_string(), Value::String(s.name));
                    m.insert("required".to_string(), Value::Bool(s.required));
                    m.insert("multiple".to_string(), Value::Bool(s.multiple));
                    Value::Object(m)
                })
                .collect();
            entry.insert("slots".to_string(), Value::Array(slots));
        }
        entry.insert(
            "source_path".to_string(),
            Value::String(source_path.to_string()),
        );
        Value::Object(entry)
    }

    /// Inserts a compiled SFC entry into the top-level `sfc` map under `tag`.
    fn register_sfc(result: &mut IndexMap<String, Value>, tag: String, entry: Value) {
        let sfc_map = result
            .entry("sfc".to_string())
            .or_insert_with(|| Value::Object(IndexMap::new()));
        if let Value::Object(map) = sfc_map {
            map.insert(tag, entry);
        }
    }

    /// Processes a `<components dir="./components"/>` element: globs `*.nemo` in
    /// the directory and registers each as an SFC. The tag is the file's
    /// `<template name>` or its filename stem (kebab→snake normalized). Files
    /// that fail to parse are skipped with an error return.
    fn process_components_dir(
        &self,
        obj: &IndexMap<String, Value>,
        result: &mut IndexMap<String, Value>,
    ) -> Result<(), ParseError> {
        let dir_attr = match obj.get("dir").and_then(|v| v.as_str()) {
            Some(d) => d,
            None => return Ok(()),
        };
        let dir = self.resolve_path(dir_attr);
        if !dir.is_dir() {
            return Err(ParseError::new(
                format!("Components directory not found: {}", dir.display()),
                SourceLocation::new(&self.source_name, 1, 1),
            ));
        }
        self.register_components_from_dir(&dir, result, None)
    }

    /// Registers every top-level `*.nemo` in `dir` as an SFC (sorted for
    /// deterministic order). The tag is the file's `<template name>` or filename
    /// stem (kebab→snake); when `tag_prefix` is `Some(p)`, each tag becomes
    /// `<p>_<tag>` (the `as=` namespace for a module import). Each sub-parser gets
    /// `dir` as its base dir so package-internal relative paths resolve locally.
    fn register_components_from_dir(
        &self,
        dir: &Path,
        result: &mut IndexMap<String, Value>,
        tag_prefix: Option<&str>,
    ) -> Result<(), ParseError> {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| {
                ParseError::new(
                    format!("Failed to read components dir {}: {}", dir.display(), e),
                    SourceLocation::new(&self.source_name, 1, 1),
                )
            })?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().map(|e| e == "nemo").unwrap_or(false))
            .collect();
        paths.sort();

        for path in paths {
            let content = std::fs::read_to_string(&path).map_err(|e| {
                ParseError::new(
                    format!("Failed to read component {}: {}", path.display(), e),
                    SourceLocation::new(&self.source_name, 1, 1),
                )
            })?;
            let sfc_parser = XmlParser::new()
                .with_source_name(path.display().to_string())
                .with_base_dir(path.parent().unwrap_or(dir));
            let sfc = sfc_parser.parse_sfc(&content)?;
            if let Some(mut tag) = sfc_default_tag(sfc.name.as_deref(), &path) {
                if let Some(prefix) = tag_prefix {
                    tag = format!("{prefix}_{tag}");
                }
                let entry = Self::sfc_to_value(sfc, &path.display().to_string());
                Self::register_sfc(result, tag, entry);
            }
        }
        Ok(())
    }

    /// Resolves a remote module path to its cached package directory under
    /// `.nemo/packages`. Prefers the version pinned in `nemo.lock`; falls back to
    /// any single cached `<module>@*` version. Returns `None` when the package
    /// cache is unknown or the module is not cached.
    fn resolve_package_dir(&self, module: &str) -> Option<PathBuf> {
        let base = self.packages_dir.as_ref()?;
        if let Some(version) = self.locked_versions.get(module) {
            let dir = base.join(format!("{module}@{version}"));
            if dir.is_dir() {
                return Some(dir);
            }
        }
        // Fallback: any cached version of the module (`<last-segment>@*`).
        let (parent, last) = match module.rsplit_once('/') {
            Some((p, l)) => (base.join(p), l.to_string()),
            None => (base.clone(), module.to_string()),
        };
        let prefix = format!("{last}@");
        std::fs::read_dir(&parent)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.is_dir()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(&prefix))
                        .unwrap_or(false)
            })
    }

    /// Processes a <layout> element into the layout structure.
    fn process_layout(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut layout = IndexMap::new();

        // Copy attributes (like type)
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" => continue,
                _ => {
                    layout.insert(key.clone(), val.clone());
                }
            }
        }

        // Process children as components
        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            let components = self.children_to_component_map(children);
            if !components.is_empty() {
                layout.insert("component".to_string(), Value::Object(components));
            }
        }

        Value::Object(layout)
    }

    /// Converts a component tree element (for templates and layout components).
    fn process_component_tree(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut component = IndexMap::new();

        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "name" => continue,
                _ => {
                    component.insert(key.clone(), val.clone());
                }
            }
        }

        // If the element has a __type__ that's a known component type, add it
        if let Some(element_type) = obj.get("__type__").and_then(|v| v.as_str()) {
            if ![
                "template", "variable", "app", "script", "data", "include", "nemo", "layout",
            ]
            .contains(&element_type)
            {
                component.insert("type".to_string(), Value::String(element_type.to_string()));
            }
        }

        // Process children
        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            let child_components = self.children_to_component_map(children);
            if !child_components.is_empty() {
                component.insert("component".to_string(), Value::Object(child_components));
            }

            // Process binding children
            let bindings = self.extract_bindings(children);
            if !bindings.is_empty() {
                if bindings.len() == 1 {
                    component.insert("binding".to_string(), bindings.into_iter().next().unwrap());
                } else {
                    component.insert("binding".to_string(), Value::Array(bindings));
                }
            }
        }

        Value::Object(component)
    }

    /// Extracts binding elements from children.
    fn extract_bindings(&self, children: &[Value]) -> Vec<Value> {
        let mut bindings = Vec::new();
        for child in children {
            if let Some(child_obj) = child.as_object() {
                if child_obj.get("__type__").and_then(|v| v.as_str()) == Some("binding") {
                    bindings.push(self.clean_element(child_obj));
                }
            }
        }
        bindings
    }

    /// Converts child elements into a component map keyed by id.
    fn children_to_component_map(&self, children: &[Value]) -> IndexMap<String, Value> {
        let mut components = IndexMap::new();

        for child in children {
            if let Some(child_obj) = child.as_object() {
                let child_type = child_obj
                    .get("__type__")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Skip non-component elements
                if ["binding", "slot", "vars"].contains(&child_type) {
                    continue;
                }

                // Get the id attribute, or generate a document-unique one.
                // The counter is shared across the whole document (not reset
                // per parent) so anonymous components in different parents
                // never collide when flattened into the id-keyed map.
                let id = child_obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        let n = self.anon_counter.get() + 1;
                        self.anon_counter.set(n);
                        format!("__anon_{}", n)
                    });

                let component_val = self.process_component_element(child_obj);
                components.insert(id, component_val);
            }
        }

        components
    }

    /// Processes a single component element into the Value structure the runtime expects.
    fn process_component_element(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut component = IndexMap::new();

        // Element name becomes the type
        if let Some(element_type) = obj.get("__type__").and_then(|v| v.as_str()) {
            component.insert("type".to_string(), Value::String(element_type.to_string()));
        }

        // Copy attributes (excluding internal keys)
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "__cdata__" => continue,
                "id" => continue, // id is used as the map key, not a property
                _ => {
                    component.insert(key.clone(), val.clone());
                }
            }
        }

        // Process children recursively
        if let Some(children) = obj.get("__children__").and_then(|v| v.as_array()) {
            let child_components = self.children_to_component_map(children);
            if !child_components.is_empty() {
                component.insert("component".to_string(), Value::Object(child_components));
            }

            // Process binding children
            let bindings = self.extract_bindings(children);
            if !bindings.is_empty() {
                if bindings.len() == 1 {
                    component.insert("binding".to_string(), bindings.into_iter().next().unwrap());
                } else {
                    component.insert("binding".to_string(), Value::Array(bindings));
                }
            }

            // Check for <slot/> children. An unnamed `<slot/>` marks the default
            // slot (`slot = true`); a named `<slot name="header"/>` records the
            // name (`slot = "header"`) so consumer children targeting it via
            // `slot="header"` can be routed at expand time.
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    if child_obj.get("__type__").and_then(|v| v.as_str()) == Some("slot") {
                        match child_obj.get("name").and_then(|v| v.as_str()) {
                            Some(name) if !name.is_empty() => {
                                component
                                    .insert("slot".to_string(), Value::String(name.to_string()));
                            }
                            _ => {
                                component.insert("slot".to_string(), Value::Bool(true));
                            }
                        }
                    }
                }
            }

            // Check for <vars> children
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    if child_obj.get("__type__").and_then(|v| v.as_str()) == Some("vars") {
                        let vars = self.clean_element(child_obj);
                        component.insert("vars".to_string(), vars);
                    }
                }
            }
        }

        Value::Object(component)
    }

    /// Removes internal keys (__type__, __children__, __cdata__) from an element.
    fn clean_element(&self, obj: &IndexMap<String, Value>) -> Value {
        let mut cleaned = IndexMap::new();
        for (key, val) in obj {
            match key.as_str() {
                "__type__" | "__children__" | "__cdata__" => continue,
                _ => {
                    cleaned.insert(key.clone(), val.clone());
                }
            }
        }
        Value::Object(cleaned)
    }

    /// Resolves a path relative to the base directory.
    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            return p.to_path_buf();
        }
        match &self.base_dir {
            Some(base) => base.join(path),
            None => PathBuf::from(path),
        }
    }

    /// Parses a single XML element and its children into a Value.
    fn parse_element(
        &self,
        reader: &mut Reader<&[u8]>,
        start_tag: Option<&BytesStart>,
    ) -> Result<Value, String> {
        let mut obj = IndexMap::new();
        let mut children: Vec<Value> = Vec::new();
        let mut cdata_content: Option<String> = None;

        // If we have a start tag, extract its attributes and name
        let element_name = if let Some(tag) = start_tag {
            let name = std::str::from_utf8(tag.name().as_ref())
                .map_err(|e| format!("Invalid UTF-8 in element name: {}", e))?
                .to_string();

            obj.insert("__type__".to_string(), Value::String(kebab_to_snake(&name)));

            // Parse attributes
            for attr_result in tag.attributes() {
                let attr = attr_result.map_err(|e| format!("Invalid attribute: {}", e))?;
                let key = std::str::from_utf8(attr.key.as_ref())
                    .map_err(|e| format!("Invalid UTF-8 in attribute name: {}", e))?;
                let value = attr
                    .unescape_value()
                    .map_err(|e| format!("Invalid attribute value: {}", e))?
                    .to_string();

                let snake_key = kebab_to_snake(key);
                obj.insert(snake_key, coerce_value(&value));
            }

            Some(name)
        } else {
            None
        };

        // Embedded SVG: capture the element's raw markup verbatim instead of
        // recursing its children into the component tree. A standard SVG body
        // holds elements (`<path>`, `<circle>`) and hyphenated attributes
        // (`stroke-width`) that the generic component parser would either
        // reject as unknown component types or mangle via kebab->snake. We
        // re-serialize the subtree into a `content` string for the `svg`
        // component to rasterize, keeping only the nemo-relevant attributes.
        if element_name.as_deref() == Some("svg") {
            if let Some(tag) = start_tag {
                return self.capture_svg_element(reader, tag, obj);
            }
        }

        // Read events until we hit the closing tag or EOF
        loop {
            match reader.read_event() {
                Ok(Event::Start(ref tag)) => {
                    let child = self.parse_element(reader, Some(tag))?;
                    children.push(child);
                }
                Ok(Event::Empty(ref tag)) => {
                    let child = self.parse_empty_element(tag)?;
                    children.push(child);
                }
                Ok(Event::CData(ref cdata)) => {
                    let text = Self::decode_cdata(cdata);
                    cdata_content = Some(text);
                }
                Ok(Event::Text(ref text)) => {
                    let s = text
                        .unescape()
                        .map_err(|e| format!("Invalid text: {}", e))?;
                    if !s.trim().is_empty() {
                        // Store text content
                        if cdata_content.is_none() {
                            cdata_content = Some(s.to_string());
                        }
                    }
                }
                Ok(Event::End(_)) => {
                    break;
                }
                Ok(Event::Comment(_)) => continue,
                Ok(Event::Decl(_)) => continue,
                Ok(Event::PI(_)) => continue,
                Ok(Event::DocType(_)) => continue,
                Ok(Event::Eof) => {
                    if element_name.is_some() {
                        return Err("Unexpected end of file".to_string());
                    }
                    break;
                }
                Err(e) => return Err(format!("XML parse error: {}", e)),
            }
        }

        if !children.is_empty() {
            obj.insert("__children__".to_string(), Value::Array(children));
        }

        if let Some(cdata) = cdata_content {
            obj.insert("__cdata__".to_string(), Value::String(cdata));
        }

        Ok(Value::Object(obj))
    }

    /// Re-serializes an embedded `<svg>` element (start tag through its
    /// matching end tag) into a single verbatim markup string, stored under
    /// `content`. `attrs` carries the already-parsed start-tag attributes; only
    /// the nemo-relevant ones (`id`, `src`, `width`, `height`) are retained as
    /// component properties — the SVG's own presentation attributes stay inside
    /// `content`, so they neither pollute the component's property set nor trip
    /// the `unknown-attribute` lint.
    fn capture_svg_element(
        &self,
        reader: &mut Reader<&[u8]>,
        start_tag: &BytesStart,
        attrs: IndexMap<String, Value>,
    ) -> Result<Value, String> {
        use quick_xml::Writer;

        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Event::Start(start_tag.borrow()))
            .map_err(|e| format!("Failed to serialize <svg>: {}", e))?;

        let mut depth = 1usize;
        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) => {
                    depth += 1;
                    writer
                        .write_event(Event::Start(e))
                        .map_err(|e| format!("Failed to serialize <svg> child: {}", e))?;
                }
                Ok(Event::End(e)) => {
                    depth -= 1;
                    writer
                        .write_event(Event::End(e))
                        .map_err(|e| format!("Failed to serialize <svg> child: {}", e))?;
                    if depth == 0 {
                        break;
                    }
                }
                Ok(Event::Empty(e)) => {
                    writer
                        .write_event(Event::Empty(e))
                        .map_err(|e| format!("Failed to serialize <svg> child: {}", e))?;
                }
                Ok(Event::Text(e)) => {
                    writer
                        .write_event(Event::Text(e))
                        .map_err(|e| format!("Failed to serialize <svg> text: {}", e))?;
                }
                Ok(Event::CData(e)) => {
                    writer
                        .write_event(Event::CData(e))
                        .map_err(|e| format!("Failed to serialize <svg> cdata: {}", e))?;
                }
                Ok(Event::Comment(_)) => continue,
                Ok(Event::Eof) => return Err("Unexpected end of file inside <svg>".to_string()),
                Ok(_) => continue,
                Err(e) => return Err(format!("XML parse error: {}", e)),
            }
        }

        let raw = String::from_utf8(writer.into_inner())
            .map_err(|e| format!("Invalid UTF-8 in <svg> content: {}", e))?;

        let mut obj = IndexMap::new();
        obj.insert("__type__".to_string(), Value::String("svg".to_string()));
        // Retain nemo-relevant attributes: identity/source/size, plus event
        // handlers (`on_*`) and data bindings (`bind_*`). The SVG's own
        // presentation attributes (`viewBox`, `fill`, `stroke-width`, ...) stay
        // inside `content`, so they neither pollute the property set nor trip
        // the `unknown-attribute` lint.
        const KEEP: [&str; 4] = ["id", "src", "width", "height"];
        for (key, value) in &attrs {
            if key == "__type__" {
                continue;
            }
            if KEEP.contains(&key.as_str()) || key.starts_with("on_") || key.starts_with("bind_") {
                obj.insert(key.clone(), value.clone());
            }
        }
        obj.insert("content".to_string(), Value::String(raw));
        Ok(Value::Object(obj))
    }

    /// Parses a self-closing XML element.
    fn parse_empty_element(&self, tag: &BytesStart) -> Result<Value, String> {
        let mut obj = IndexMap::new();

        let qname = tag.name();
        let name = std::str::from_utf8(qname.as_ref())
            .map_err(|e| format!("Invalid UTF-8 in element name: {}", e))?;

        obj.insert("__type__".to_string(), Value::String(kebab_to_snake(name)));

        for attr_result in tag.attributes() {
            let attr = attr_result.map_err(|e| format!("Invalid attribute: {}", e))?;
            let key = std::str::from_utf8(attr.key.as_ref())
                .map_err(|e| format!("Invalid UTF-8 in attribute name: {}", e))?;
            let value = attr
                .unescape_value()
                .map_err(|e| format!("Invalid attribute value: {}", e))?
                .to_string();

            let snake_key = kebab_to_snake(key);
            obj.insert(snake_key, coerce_value(&value));
        }

        Ok(Value::Object(obj))
    }

    /// Decodes CDATA content, handling the raw bytes.
    fn decode_cdata(cdata: &BytesCData) -> String {
        String::from_utf8_lossy(cdata.as_ref()).to_string()
    }
    /// Compiles an `app.nemo` SFC into the resolved `Value` tree — the same
    /// shape [`process_root`](Self::process_root) produces for an equivalent
    /// `app.xml`.
    ///
    /// The SFC's `<template name="app">` becomes the `layout` key; the
    /// app-level blocks (`<app>`, `<data>`, `<imports>`, `<variable>`,
    /// `<script>`) map to the same keys their `process_root` arms produce. A
    /// raw-text `<script>` body (captured by [`parse_sfc`](Self::parse_sfc)
    /// before the XML reader sees it) is folded into `scripts.inline` so the
    /// output matches an `app.xml` that carries the same body in CDATA.
    ///
    /// Returns the raw parsed tree (before directive compilation or `${}`
    /// resolution) — the caller runs [`compile_directives`](crate::compile_directives)
    /// and the resolver as `load_xml_string` does.
    pub fn compile_app_sfc(&self, content: &str) -> Result<Value, ParseError> {
        let sfc = self.parse_sfc(content)?;
        Ok(Self::app_sfc_to_value(sfc))
    }

    /// Assembles the top-level `Value` tree from a parsed `app.nemo`
    /// [`SfcDefinition`] — the SFC counterpart of [`process_root`](Self::process_root).
    ///
    /// `sfc.template` → `layout`; the [`AppBlocks`] fields → their `process_root`
    /// keys. The raw-text `sfc.script` body is merged into `scripts.inline`
    /// (matching `process_script`'s CDATA path) so the output matches an
    /// equivalent `app.xml`.
    fn app_sfc_to_value(sfc: SfcDefinition) -> Value {
        let mut result = IndexMap::new();

        // The <template> body is the layout tree. In app.xml, <layout type=…>
        // wraps its children in a `component` map; the SFC <template>'s single
        // root child IS the layout root, so we wrap it the same way: the root's
        // `type` becomes the layout type, and the processed component value
        // goes under `component[<root_id>]` — matching process_layout exactly.
        let layout_type = sfc
            .template
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("stack")
            .to_string();
        let root_id = sfc
            .app_blocks
            .as_ref()
            .map(|b| b.layout_root_id.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("root")
            .to_string();
        let mut layout = IndexMap::new();
        layout.insert("type".to_string(), Value::String(layout_type));
        let mut component_map = IndexMap::new();
        component_map.insert(root_id, sfc.template);
        layout.insert("component".to_string(), Value::Object(component_map));
        result.insert("layout".to_string(), Value::Object(layout));
        if let Some(blocks) = sfc.app_blocks {
            if let Some(app) = blocks.app {
                result.insert("app".to_string(), app);
            }
            // data / variable / sfc are Object maps; only insert if non-empty
            // so the output matches process_root (which only creates them when
            // the blocks are present).
            if is_non_empty_map(&blocks.data) {
                result.insert("data".to_string(), blocks.data);
            }
            if is_non_empty_map(&blocks.variables) {
                result.insert("variable".to_string(), blocks.variables);
            }
            if is_non_empty_map(&blocks.sfc_imports) {
                result.insert("sfc".to_string(), blocks.sfc_imports);
            }
            if is_non_empty_map(&blocks.scripts) {
                result.insert("scripts".to_string(), blocks.scripts);
            }
        }

        // Fold the raw-text <script> body into scripts.inline, matching
        // process_script's CDATA path. If scripts already has an inline array
        // (from an XML <script> element), append; otherwise create it.
        if let Some(script_body) = sfc.script {
            let scripts = result
                .entry("scripts".to_string())
                .or_insert_with(|| Value::Object(IndexMap::new()));
            if let Value::Object(scripts_obj) = scripts {
                let inline = scripts_obj
                    .entry("inline".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = inline {
                    arr.push(Value::String(script_body));
                }
            }
        }

        Value::Object(result)
    }
}

impl Default for XmlParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts kebab-case to snake_case.
fn kebab_to_snake(s: &str) -> String {
    s.replace('-', "_")
}

/// Returns `true` if `v` is a non-empty `Value::Object` (the shape `process_root`
/// produces for `data`/`variable`/`sfc`/`scripts`). Used by `app_sfc_to_value` to
/// skip empty maps so the output matches `process_root` (which only inserts a key
/// when the corresponding block is present).
fn is_non_empty_map(v: &Value) -> bool {
    v.as_object().map(|m| !m.is_empty()).unwrap_or(false)
}

/// Flattens an [`SfcDefinition`] into the `Value` stored under `config["sfc"][tag]`
/// (`template`/`style`/`script`/`props`/`slots`/`source_path`). Public so
/// `nemo build` produces artifacts in the same shape the runtime reads back.
pub fn sfc_definition_to_value(sfc: SfcDefinition, source_path: &str) -> Value {
    XmlParser::sfc_to_value(sfc, source_path)
}

/// Derives the SFC registration tag from an optional `<template name>` and the
/// source file path (filename-stem fallback), kebab→snake normalized (a
/// `<labeled-button>` usage parses to type `labeled_button`, so tags match).
///
/// This is the canonical derivation shared by `<import>` (which layers its `as=`
/// override on top), `<components dir>`, and `nemo build`. Returns `None` when
/// neither a template name nor a usable filename stem is available.
pub fn sfc_default_tag(template_name: Option<&str>, source_path: &Path) -> Option<String> {
    template_name
        .map(|s| s.to_string())
        .or_else(|| {
            source_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
        })
        .filter(|s| !s.is_empty())
        .map(|s| kebab_to_snake(&s))
}

/// Coerces a string value to the appropriate Value type.
fn coerce_value(s: &str) -> Value {
    // Check for booleans
    if s == "true" {
        return Value::Bool(true);
    }
    if s == "false" {
        return Value::Bool(false);
    }

    // Check for integers
    if let Ok(i) = s.parse::<i64>() {
        return Value::Integer(i);
    }

    // Check for floats (but not expressions like ${...})
    if !s.contains("${") {
        if let Ok(f) = s.parse::<f64>() {
            return Value::Float(f);
        }
    }

    // Check for JSON arrays in attributes (e.g., columns='[{"key":"a"}]')
    if s.starts_with('[') && s.ends_with(']') {
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(s) {
            return Value::from(json_val);
        }
    }

    // Otherwise it's a string (preserving ${...} expressions)
    Value::String(s.to_string())
}

/// Merges a key-value pair into an existing IndexMap, handling collisions.
fn merge_into(target: &mut IndexMap<String, Value>, key: &str, val: &Value) {
    match target.get_mut(key) {
        Some(existing) => {
            // Merge objects together
            if let (Some(existing_obj), Some(new_obj)) =
                (existing.as_object().cloned(), val.as_object())
            {
                let mut merged = existing_obj;
                for (k, v) in new_obj {
                    match merged.get_mut(k) {
                        Some(existing_inner) => {
                            if let (Some(ei), Some(ni)) =
                                (existing_inner.as_object().cloned(), v.as_object())
                            {
                                let mut inner_merged = ei;
                                for (ik, iv) in ni {
                                    inner_merged.insert(ik.clone(), iv.clone());
                                }
                                *existing_inner = Value::Object(inner_merged);
                            } else {
                                *existing_inner = v.clone();
                            }
                        }
                        None => {
                            merged.insert(k.clone(), v.clone());
                        }
                    }
                }
                *existing = Value::Object(merged);
            }
        }
        None => {
            target.insert(key.to_string(), val.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sfc_template_style_script() {
        let sfc = r#"
        <template name="labeled-button">
          <button label="${label}" variant="primary" on-click="handleClick" />
        </template>
        <style>
          button { height: 32px; }
        </style>
        <script><![CDATA[
          fn handleClick(component_id, event_data) { log_info("clicked"); }
        ]]></script>
        "#;

        let parser = XmlParser::new();
        let def = parser.parse_sfc(sfc).unwrap();

        assert_eq!(def.name.as_deref(), Some("labeled-button"));

        // Template body is a flattened single-root component Value.
        assert_eq!(
            def.template.get("type"),
            Some(&Value::String("button".to_string()))
        );
        assert_eq!(
            def.template.get("label"),
            Some(&Value::String("${label}".to_string()))
        );
        // on-click is kebab→snake normalized like any attribute.
        assert_eq!(
            def.template.get("on_click"),
            Some(&Value::String("handleClick".to_string()))
        );

        assert!(def.style.as_deref().unwrap().contains("height: 32px"));
        assert!(def.script.as_deref().unwrap().contains("handleClick"));
    }

    #[test]
    fn test_parse_sfc_default_slot() {
        let sfc = r#"
        <template name="card">
          <panel padding="16">
            <stack id="inner" direction="vertical"><slot /></stack>
          </panel>
        </template>
        "#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        assert_eq!(
            def.template.get("type"),
            Some(&Value::String("panel".to_string()))
        );
        // The inner stack carries the slot marker for injection at expand time.
        let inner = def
            .template
            .get("component")
            .and_then(|c| c.get("inner"))
            .unwrap();
        assert_eq!(inner.get("slot"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_parse_sfc_named_slots() {
        let sfc = r#"
        <template name="panel-card">
          <panel>
            <stack id="head"><slot name="header" /></stack>
            <stack id="body"><slot /></stack>
          </panel>
        </template>
        "#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        let components = def.template.get("component").unwrap();
        // Named slot records its name; unnamed slot stays `true` (default).
        assert_eq!(
            components.get("head").and_then(|c| c.get("slot")),
            Some(&Value::String("header".to_string()))
        );
        assert_eq!(
            components.get("body").and_then(|c| c.get("slot")),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn test_parse_sfc_typed_props() {
        let sfc = r#"
        <props>
          <prop name="label" type="string" default="Button" />
          <prop name="count" type="int" default="3" />
          <prop name="title" type="string" required="true" />
        </props>
        <template name="widget">
          <button label="${label}" />
        </template>
        "#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        assert_eq!(def.props.len(), 3);

        let label = &def.props[0];
        assert_eq!(label.name, "label");
        assert_eq!(label.ty, "string");
        assert_eq!(label.default, Some(Value::String("Button".to_string())));
        assert!(!label.required);

        let count = &def.props[1];
        assert_eq!(count.ty, "int");
        // `default` is coerced to the declared type.
        assert_eq!(count.default, Some(Value::Integer(3)));

        let title = &def.props[2];
        assert!(title.required);
        assert_eq!(title.default, None);
    }

    #[test]
    fn test_parse_sfc_slot_specs() {
        let sfc = r#"
        <template name="card">
          <panel>
            <stack id="head"><slot name="header" required="true" multiple="false" /></stack>
            <stack id="body"><slot /></stack>
          </panel>
        </template>
        "#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        assert_eq!(def.slots.len(), 2);

        let header = def.slots.iter().find(|s| s.name == "header").unwrap();
        assert!(header.required);
        assert!(!header.multiple);

        // Unnamed slot → "default", not required, multiple by default.
        let default = def.slots.iter().find(|s| s.name == "default").unwrap();
        assert!(!default.required);
        assert!(default.multiple);
    }

    #[test]
    fn test_components_dir_auto_discovery() {
        let dir =
            std::env::temp_dir().join(format!("nemo_sfc_components_test_{}", std::process::id()));
        let comp_dir = dir.join("components");
        std::fs::create_dir_all(&comp_dir).unwrap();
        std::fs::write(
            comp_dir.join("card.nemo"),
            r#"<template name="card"><panel><slot /></panel></template>"#,
        )
        .unwrap();
        std::fs::write(
            comp_dir.join("labeled-button.nemo"),
            r#"<template name="labeled-button"><button label="${label}" /></template>"#,
        )
        .unwrap();

        let app = r#"
        <nemo>
          <components dir="./components" />
          <layout type="stack">
            <card><labeled-button label="Hi" /></card>
          </layout>
        </nemo>
        "#;
        let value = XmlParser::new()
            .with_source_name("app.xml")
            .with_base_dir(&dir)
            .parse(app)
            .unwrap();

        let sfc = value.get("sfc").unwrap();
        // Both files discovered; tags kebab→snake normalized.
        assert!(sfc.get("card").is_some());
        assert!(sfc.get("labeled_button").is_some());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_parse_sfc_requires_single_root() {
        let sfc = r#"
        <template name="bad">
          <button label="a" />
          <button label="b" />
        </template>
        "#;
        let err = XmlParser::new().parse_sfc(sfc).unwrap_err();
        assert!(err.to_string().contains("exactly one root element"));
    }

    #[test]
    fn test_parse_sfc_requires_template() {
        let sfc = r#"<style>button { height: 32px; }</style>"#;
        let err = XmlParser::new().parse_sfc(sfc).unwrap_err();
        assert!(err.to_string().contains("must contain a <template>"));
    }

    #[test]
    fn test_imports_resolves_and_aliases() {
        let dir = std::env::temp_dir().join(format!("nemo_sfc_import_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let comp = dir.join("btn.nemo");
        std::fs::write(
            &comp,
            r#"<template name="labeled-button"><button label="${label}" /></template>"#,
        )
        .unwrap();

        let app = r#"
        <nemo>
          <imports>
            <import src="./btn.nemo" as="my-button" />
          </imports>
          <layout type="stack">
            <my-button label="Save" />
          </layout>
        </nemo>
        "#;

        let parser = XmlParser::new()
            .with_source_name("app.xml")
            .with_base_dir(&dir);
        let value = parser.parse(app).unwrap();

        // `as=` overrides the tag; tags are kebab→snake normalized to match how
        // element usages parse (`<my-button>` → `my_button`).
        let sfc = value.get("sfc").unwrap();
        let entry = sfc.get("my_button").unwrap();
        assert_eq!(
            entry.get("template").and_then(|t| t.get("type")),
            Some(&Value::String("button".to_string()))
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_parse_basic_structure() {
        let xml = r#"
        <nemo>
            <app title="My App">
                <window title="Test" width="800" height="600">
                    <header-bar github-url="https://example.com" theme-toggle="true" />
                </window>
                <theme name="kanagawa" mode="dark" />
            </app>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let app = value.get("app").unwrap();
        assert_eq!(app.get("title"), Some(&Value::String("My App".to_string())));

        let window = app.get("window").unwrap();
        assert_eq!(
            window.get("title"),
            Some(&Value::String("Test".to_string()))
        );
        assert_eq!(window.get("width"), Some(&Value::Integer(800)));
        assert_eq!(window.get("height"), Some(&Value::Integer(600)));

        let header_bar = window.get("header_bar").unwrap();
        assert_eq!(
            header_bar.get("github_url"),
            Some(&Value::String("https://example.com".to_string()))
        );
        assert_eq!(header_bar.get("theme_toggle"), Some(&Value::Bool(true)));

        let theme = app.get("theme").unwrap();
        assert_eq!(
            theme.get("name"),
            Some(&Value::String("kanagawa".to_string()))
        );
    }

    #[test]
    fn test_parse_header_bar_menu_items() {
        let xml = r#"
        <nemo>
            <app title="My App">
                <window title="Test">
                    <header-bar theme-toggle="true">
                        <menu-item label="Preferences" icon="settings" on-click="open_prefs" />
                        <menu-item separator="true" />
                        <menu-item label="About" on-click="show_about" />
                    </header-bar>
                </window>
            </app>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let header_bar = value
            .get("app")
            .and_then(|a| a.get("window"))
            .and_then(|w| w.get("header_bar"))
            .unwrap();
        assert_eq!(header_bar.get("theme_toggle"), Some(&Value::Bool(true)));

        let items = header_bar
            .get("menu_items")
            .and_then(|v| v.as_array())
            .unwrap();
        assert_eq!(items.len(), 3);

        // First item: label + icon + on_click (kebab→snake normalized).
        assert_eq!(
            items[0].get("label"),
            Some(&Value::String("Preferences".to_string()))
        );
        assert_eq!(
            items[0].get("icon"),
            Some(&Value::String("settings".to_string()))
        );
        assert_eq!(
            items[0].get("on_click"),
            Some(&Value::String("open_prefs".to_string()))
        );

        // Second item: a separator.
        assert_eq!(items[1].get("separator"), Some(&Value::Bool(true)));

        // Third item preserved (repeated `menu-item` did not collapse).
        assert_eq!(
            items[2].get("label"),
            Some(&Value::String("About".to_string()))
        );
    }

    #[test]
    fn test_parse_variables() {
        let xml = r#"
        <nemo>
            <variable name="button_height" type="int" default="48" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let vars = value.get("variable").unwrap();
        let bh = vars.get("button_height").unwrap();
        assert_eq!(bh.get("type"), Some(&Value::String("int".to_string())));
        assert_eq!(bh.get("default"), Some(&Value::Integer(48)));
    }

    #[test]
    fn test_parse_scripts_with_path() {
        let xml = r#"
        <nemo>
            <script src="./scripts" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        assert_eq!(
            scripts.get("path"),
            Some(&Value::String("./scripts".to_string()))
        );
    }

    #[test]
    fn test_parse_scripts_with_features() {
        let xml = r#"
        <nemo>
            <script src="./scripts" features="file-io" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        let features = scripts.get("features").unwrap().as_array().unwrap();
        assert_eq!(features.len(), 1);
        assert_eq!(features[0].as_str().unwrap(), "file-io");
    }

    #[test]
    fn test_parse_scripts_with_on_load() {
        let xml = r#"
        <nemo>
            <script src="./scripts" on-load="on_load" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        assert_eq!(
            scripts.get("on_load"),
            Some(&Value::String("on_load".to_string()))
        );
    }

    #[test]
    fn test_parse_scripts_without_on_load_omits_key() {
        let xml = r#"
        <nemo>
            <script src="./scripts" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        assert!(scripts.get("on_load").is_none());
    }

    #[test]
    fn test_anonymous_components_get_document_unique_ids() {
        // Regression: id-less ("anonymous") components in different parents
        // must get distinct ids. Previously the counter reset per parent, so
        // the first id-less child of every parent became `__anon_1`; when the
        // runtime flattened components into an id-keyed map they collapsed and
        // every such label rendered the last one's text (the dev-dashboard
        // "all labels show Median:" bug).
        let xml = r#"
        <nemo>
            <layout type="stack">
                <stack id="row1">
                    <label text="Alpha" />
                    <label id="v1" text="one" />
                </stack>
                <stack id="row2">
                    <label text="Beta" />
                    <label id="v2" text="two" />
                </stack>
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let rows = value
            .get("layout")
            .and_then(|l| l.get("component"))
            .and_then(|c| c.as_object())
            .unwrap();

        // Collect the anonymous (prefix) label ids and their text from each row.
        let mut anon_ids = Vec::new();
        let mut anon_texts = Vec::new();
        for (_row_id, row) in rows {
            let children = row.get("component").and_then(|c| c.as_object()).unwrap();
            for (child_id, child) in children {
                if child_id.starts_with("__anon") {
                    anon_ids.push(child_id.clone());
                    anon_texts.push(
                        child
                            .get("text")
                            .and_then(|t| t.as_str())
                            .unwrap()
                            .to_string(),
                    );
                }
            }
        }

        assert_eq!(anon_ids.len(), 2, "expected two anonymous labels");
        assert_ne!(
            anon_ids[0], anon_ids[1],
            "anonymous components across parents must have distinct ids, got {anon_ids:?}"
        );
        // Both distinct texts survive (they don't collapse to one).
        assert!(anon_texts.contains(&"Alpha".to_string()));
        assert!(anon_texts.contains(&"Beta".to_string()));
    }

    #[test]
    fn test_parse_scripts_with_multiple_features() {
        let xml = r#"
        <nemo>
            <script src="./scripts" features="file-io, network, system" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        let features = scripts.get("features").unwrap().as_array().unwrap();
        assert_eq!(features.len(), 3);
        assert_eq!(features[0].as_str().unwrap(), "file-io");
        assert_eq!(features[1].as_str().unwrap(), "network");
        assert_eq!(features[2].as_str().unwrap(), "system");
    }

    #[test]
    fn test_parse_scripts_without_features_omits_key() {
        let xml = r#"
        <nemo>
            <script src="./scripts" />
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        assert!(scripts.get("features").is_none());
    }

    #[test]
    fn test_parse_scripts_with_cdata() {
        let xml = r#"
        <nemo>
            <script><![CDATA[
    fn on_click(id, data) { log_info("clicked"); }
            ]]></script>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let scripts = value.get("scripts").unwrap();
        let inline = scripts.get("inline").unwrap().as_array().unwrap();
        assert_eq!(inline.len(), 1);
        assert!(inline[0].as_str().unwrap().contains("on_click"));
    }

    #[test]
    fn test_parse_data_sources_and_sinks() {
        let xml = r#"
        <nemo>
            <data>
                <source name="ticker" type="timer" interval="1" />
                <source name="api" type="http" url="https://api.example.com" interval="30" />
                <sink name="commands" type="mqtt" host="localhost" port="1883" topic="commands" />
            </data>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let data = value.get("data").unwrap();
        let sources = data.get("source").unwrap();

        let ticker = sources.get("ticker").unwrap();
        assert_eq!(
            ticker.get("type"),
            Some(&Value::String("timer".to_string()))
        );
        assert_eq!(ticker.get("interval"), Some(&Value::Integer(1)));

        let api = sources.get("api").unwrap();
        assert_eq!(api.get("type"), Some(&Value::String("http".to_string())));
        assert_eq!(api.get("interval"), Some(&Value::Integer(30)));

        let sinks = data.get("sink").unwrap();
        let commands = sinks.get("commands").unwrap();
        assert_eq!(
            commands.get("type"),
            Some(&Value::String("mqtt".to_string()))
        );
        assert_eq!(commands.get("port"), Some(&Value::Integer(1883)));
    }

    #[test]
    fn test_parse_layout_with_components() {
        let xml = r#"
        <nemo>
            <layout type="stack">
                <label id="header" text="Welcome to Nemo" />
                <panel id="content">
                    <button id="btn" label="Click Me" on-click="on_button_click" />
                </panel>
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let layout = value.get("layout").unwrap();
        assert_eq!(
            layout.get("type"),
            Some(&Value::String("stack".to_string()))
        );

        let components = layout.get("component").unwrap();
        let header = components.get("header").unwrap();
        assert_eq!(
            header.get("type"),
            Some(&Value::String("label".to_string()))
        );
        assert_eq!(
            header.get("text"),
            Some(&Value::String("Welcome to Nemo".to_string()))
        );

        let content = components.get("content").unwrap();
        assert_eq!(
            content.get("type"),
            Some(&Value::String("panel".to_string()))
        );

        let inner_components = content.get("component").unwrap();
        let btn = inner_components.get("btn").unwrap();
        assert_eq!(btn.get("type"), Some(&Value::String("button".to_string())));
        assert_eq!(
            btn.get("on_click"),
            Some(&Value::String("on_button_click".to_string()))
        );
    }

    #[test]
    fn test_parse_embedded_svg_captures_raw_content() {
        let xml = r##"
        <nemo>
            <layout type="stack">
                <svg id="logo" width="120" height="120" viewBox="0 0 100 100" on-click="on_logo_click">
                    <circle cx="50" cy="50" r="40" fill="#ff0000" stroke-width="2" />
                </svg>
            </layout>
        </nemo>
        "##;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let components = value
            .get("layout")
            .and_then(|l| l.get("component"))
            .unwrap();
        let svg = components.get("logo").unwrap();

        assert_eq!(svg.get("type"), Some(&Value::String("svg".to_string())));

        // Nemo-relevant attributes are retained as component properties.
        assert_eq!(svg.get("width"), Some(&Value::Integer(120)));
        assert_eq!(svg.get("height"), Some(&Value::Integer(120)));

        // Event-handler attributes survive so the component stays interactive.
        assert_eq!(
            svg.get("on_click"),
            Some(&Value::String("on_logo_click".to_string()))
        );

        // Raw SVG markup is captured verbatim under `content`, including the
        // nested element and its hyphenated attribute — neither of which the
        // component parser could otherwise represent (unknown component /
        // kebab->snake mangling).
        let content = svg.get("content").and_then(|v| v.as_str()).unwrap();
        assert!(content.starts_with("<svg"));
        assert!(content.contains("<circle"));
        assert!(content.contains("stroke-width=\"2\""));
        assert!(content.contains("viewBox=\"0 0 100 100\""));
        assert!(content.contains("fill=\"#ff0000\""));

        // The nested <circle> must NOT leak into the component tree, and the
        // SVG's presentation attributes must NOT surface as component props.
        assert!(svg.get("component").is_none());
        assert!(svg.get("view_box").is_none());
        assert!(svg.get("fill").is_none());
    }

    #[test]
    fn test_parse_templates() {
        let xml = r#"
        <nemo>
            <template name="nav_item">
                <button variant="ghost" size="sm" full-width="true" on-click="on_nav" />
            </template>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let templates = value.get("templates").unwrap();
        let template = templates.get("template").unwrap();
        let nav_item = template.get("nav_item").unwrap();

        // Single-child template is unwrapped: the button IS the template body
        assert_eq!(
            nav_item.get("type"),
            Some(&Value::String("button".to_string()))
        );
        assert_eq!(
            nav_item.get("variant"),
            Some(&Value::String("ghost".to_string()))
        );
        assert_eq!(
            nav_item.get("on_click"),
            Some(&Value::String("on_nav".to_string()))
        );
    }

    #[test]
    fn test_type_coercion() {
        assert_eq!(coerce_value("true"), Value::Bool(true));
        assert_eq!(coerce_value("false"), Value::Bool(false));
        assert_eq!(coerce_value("42"), Value::Integer(42));
        assert_eq!(coerce_value("-7"), Value::Integer(-7));
        assert_eq!(coerce_value("3.125"), Value::Float(3.125));
        assert_eq!(coerce_value("hello"), Value::String("hello".to_string()));
        assert_eq!(
            coerce_value("${var.name}"),
            Value::String("${var.name}".to_string())
        );
    }

    #[test]
    fn test_kebab_to_snake() {
        assert_eq!(kebab_to_snake("on-click"), "on_click");
        assert_eq!(kebab_to_snake("min-height"), "min_height");
        assert_eq!(kebab_to_snake("simple"), "simple");
        assert_eq!(kebab_to_snake("border-color"), "border_color");
    }

    #[test]
    fn test_expression_passthrough() {
        let xml = r#"
        <nemo>
            <layout type="stack">
                <button id="btn" min-height="${var.button_height}" label="7" />
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let btn = components.get("btn").unwrap();
        assert_eq!(
            btn.get("min_height"),
            Some(&Value::String("${var.button_height}".to_string()))
        );
    }

    #[test]
    fn test_parse_binding_elements() {
        let xml = r#"
        <nemo>
            <layout type="stack">
                <label id="tick_count" text="Tick: waiting...">
                    <binding source="data.ticker" target="text" transform="tick" />
                </label>
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let label = components.get("tick_count").unwrap();

        let binding = label.get("binding").unwrap();
        assert_eq!(
            binding.get("source"),
            Some(&Value::String("data.ticker".to_string()))
        );
        assert_eq!(
            binding.get("target"),
            Some(&Value::String("text".to_string()))
        );
        assert_eq!(
            binding.get("transform"),
            Some(&Value::String("tick".to_string()))
        );
    }

    #[test]
    fn test_parse_template_with_slot() {
        let xml = r#"
        <nemo>
            <template name="content_page">
                <panel visible="false">
                    <stack id="inner" direction="vertical" spacing="12" padding="32">
                        <slot />
                    </stack>
                </panel>
            </template>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let templates = value.get("templates").unwrap();
        let template = templates.get("template").unwrap();
        let page = template.get("content_page").unwrap();

        // Should have component children
        let components = page.get("component").unwrap();
        let inner = components.as_object().unwrap().values().next().unwrap();

        // The panel should contain a component with slot=true
        // Navigate into the nested structure
        if let Some(panel_components) = inner.get("component") {
            let inner_stack = panel_components.as_object().unwrap().get("inner").unwrap();
            assert_eq!(inner_stack.get("slot"), Some(&Value::Bool(true)));
        }
    }

    #[test]
    fn test_parse_template_reference() {
        let xml = r#"
        <nemo>
            <layout type="stack">
                <panel id="motor1_pid" template="pid_control">
                    <vars ns="pid.motor1" />
                </panel>
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let motor = components.get("motor1_pid").unwrap();

        assert_eq!(
            motor.get("template"),
            Some(&Value::String("pid_control".to_string()))
        );

        let vars = motor.get("vars").unwrap();
        assert_eq!(
            vars.get("ns"),
            Some(&Value::String("pid.motor1".to_string()))
        );
    }

    #[test]
    fn test_parse_malformed_xml() {
        let xml = r#"<nemo><unclosed>"#;
        let parser = XmlParser::new();
        assert!(parser.parse(xml).is_err());
    }

    #[test]
    fn test_parse_empty_nemo() {
        let xml = r#"<nemo></nemo>"#;
        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();
        assert!(value.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_parse_app_with_plugins() {
        let xml = r#"
        <nemo>
            <app title="Test">
                <window title="Test" />
                <theme name="kanagawa" mode="dark" />
            </app>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let app = value.get("app").unwrap();
        assert_eq!(app.get("title"), Some(&Value::String("Test".to_string())));

        let theme = app.get("theme").unwrap();
        assert_eq!(
            theme.get("name"),
            Some(&Value::String("kanagawa".to_string()))
        );
    }

    #[test]
    fn test_parse_themes_block() {
        let xml = r#"
        <nemo>
            <themes>
                <theme-set src="themes/corporate.json" />
                <theme-set src="themes/solar.json" />
            </themes>
            <app title="Test">
                <theme name="corporate" mode="dark" />
            </app>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let themes = value.get("themes").and_then(|v| v.as_array()).unwrap();
        assert_eq!(themes.len(), 2);
        assert_eq!(
            themes[0].get("src"),
            Some(&Value::String("themes/corporate.json".to_string()))
        );
        assert_eq!(
            themes[1].get("src"),
            Some(&Value::String("themes/solar.json".to_string()))
        );
    }

    #[test]
    fn test_parse_theme_extend_overrides() {
        let xml = r##"
        <nemo>
            <app title="Test">
                <theme name="nord" mode="dark">
                    <extend>
                        <color key="primary.background" value="#ff6600" />
                        <color key="foreground" value="#ffffff" />
                    </extend>
                </theme>
            </app>
        </nemo>
        "##;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let theme = value.get("app").unwrap().get("theme").unwrap();
        assert_eq!(theme.get("name"), Some(&Value::String("nord".to_string())));
        assert_eq!(theme.get("mode"), Some(&Value::String("dark".to_string())));

        let extend = theme.get("extend").and_then(|v| v.as_object()).unwrap();
        assert_eq!(
            extend.get("primary.background"),
            Some(&Value::String("#ff6600".to_string()))
        );
        assert_eq!(
            extend.get("foreground"),
            Some(&Value::String("#ffffff".to_string()))
        );
    }

    #[test]
    fn test_basic_example_equivalent() {
        // This should produce the same Value structure as examples/basic/app.xml
        let xml = r#"
        <nemo>
            <app title="My Nemo App">
                <window title="Nemo Example">
                    <header-bar github-url="https://github.com/geoffjay/nemo/tree/main/examples/basic" theme-toggle="true" />
                </window>
                <theme name="kanagawa" mode="dark" />
            </app>

            <script src="./scripts" />

            <layout type="stack">
                <label id="header" text="Welcome to Nemo" />
                <panel id="content">
                    <button id="button" label="Click Me" on-click="on_button_click" />
                </panel>
            </layout>

            <data>
                <source name="api" type="http" url="https://api.example.com" refresh="30" />
            </data>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        // Verify app section
        let app = value.get("app").unwrap();
        assert_eq!(
            app.get("title"),
            Some(&Value::String("My Nemo App".to_string()))
        );

        // Verify scripts section
        let scripts = value.get("scripts").unwrap();
        assert_eq!(
            scripts.get("path"),
            Some(&Value::String("./scripts".to_string()))
        );

        // Verify layout section
        let layout = value.get("layout").unwrap();
        assert_eq!(
            layout.get("type"),
            Some(&Value::String("stack".to_string()))
        );
        let components = layout.get("component").unwrap();
        assert!(components.get("header").is_some());
        assert!(components.get("content").is_some());

        // Verify data section
        let data = value.get("data").unwrap();
        let sources = data.get("source").unwrap();
        let api = sources.get("api").unwrap();
        assert_eq!(api.get("type"), Some(&Value::String("http".to_string())));
    }

    #[test]
    fn test_multiple_bindings() {
        let xml = r#"
        <nemo>
            <layout type="stack">
                <label id="multi" text="test">
                    <binding source="data.a" target="text" transform="x" />
                    <binding source="data.b" target="color" />
                </label>
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let label = components.get("multi").unwrap();
        let bindings = label.get("binding").unwrap().as_array().unwrap();
        assert_eq!(bindings.len(), 2);
    }

    #[test]
    fn test_nested_component_tree() {
        let xml = r#"
        <nemo>
            <layout type="stack">
                <stack id="row1" direction="horizontal" spacing="6">
                    <button id="btn_1" label="1" on-click="on_digit" />
                    <button id="btn_2" label="2" on-click="on_digit" />
                </stack>
            </layout>
        </nemo>
        "#;

        let parser = XmlParser::new();
        let value = parser.parse(xml).unwrap();

        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let row = components.get("row1").unwrap();
        assert_eq!(row.get("type"), Some(&Value::String("stack".to_string())));
        assert_eq!(
            row.get("direction"),
            Some(&Value::String("horizontal".to_string()))
        );

        let inner = row.get("component").unwrap();
        assert!(inner.get("btn_1").is_some());
        assert!(inner.get("btn_2").is_some());
    }

    /// Helper to load and parse an example XML file from the workspace.
    fn parse_example(name: &str) -> Value {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join(name)
            .join("app.xml");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
        let parser = XmlParser::new()
            .with_source_name(path.display().to_string())
            .with_base_dir(path.parent().unwrap());
        parser
            .parse(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e))
    }

    #[test]
    fn test_parse_example_basic() {
        let value = parse_example("basic");
        assert!(value.get("app").is_some());
        assert!(value.get("scripts").is_some());
        assert!(value.get("layout").is_some());
        assert!(value.get("data").is_some());
    }

    #[test]
    fn test_parse_example_calculator() {
        let value = parse_example("calculator");
        assert!(value.get("variable").is_some());
        assert!(value.get("app").is_some());
        assert!(value.get("layout").is_some());
        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        assert!(components.get("display").is_some());
        assert!(components.get("buttons").is_some());
    }

    #[test]
    fn test_parse_example_data_binding() {
        let value = parse_example("data-binding");
        let data = value.get("data").unwrap();
        let sources = data.get("source").unwrap();
        assert!(sources.get("ticker").is_some());
        assert!(sources.get("api").is_some());
        assert!(sources.get("sensors").is_some());
        let sinks = data.get("sink").unwrap();
        assert!(sinks.get("commands").is_some());
    }

    #[test]
    fn test_parse_example_data_streaming() {
        let value = parse_example("data-streaming");
        assert!(value.get("app").is_some());
        assert!(value.get("layout").is_some());
        let data = value.get("data").unwrap();
        let sources = data.get("source").unwrap();
        assert!(sources.get("metrics").is_some());
    }

    #[test]
    fn test_parse_example_pid_control() {
        let value = parse_example("pid-control");
        assert!(value.get("layout").is_some());
        let layout = value.get("layout").unwrap();
        let components = layout.get("component").unwrap();
        let root = components.get("root_panel").unwrap();
        let main = root.get("component").unwrap().get("main_content").unwrap();
        let motor1 = main.get("component").unwrap().get("motor1_pid").unwrap();
        assert_eq!(
            motor1.get("template"),
            Some(&Value::String("pid_control".to_string()))
        );
    }

    #[test]
    fn test_parse_example_components() {
        let value = parse_example("components");
        assert!(value.get("app").is_some());
        assert!(value.get("templates").is_some());
        assert!(value.get("layout").is_some());
        let templates = value.get("templates").unwrap();
        let template = templates.get("template").unwrap();
        // The components gallery navigates via <router>/<route>/<nav-link> now;
        // `content_page` is the remaining template wrapper.
        assert!(template.get("content_page").is_some());
    }

    #[test]
    fn test_include_href_attribute() {
        let dir = tempfile::tempdir().unwrap();

        // Create an included file with templates
        let templates_dir = dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();
        std::fs::write(
            templates_dir.join("buttons.xml"),
            r#"<nemo>
  <templates>
    <template name="primary_btn">
      <button variant="primary" size="md" />
    </template>
  </templates>
</nemo>"#,
        )
        .unwrap();

        // Main file uses href instead of src
        let main_xml = r#"<nemo>
  <include href="templates/buttons.xml" />

  <layout type="stack">
    <button id="my_btn" template="primary_btn" label="Click" />
  </layout>
</nemo>"#;

        let parser = XmlParser::new()
            .with_source_name("test".to_string())
            .with_base_dir(dir.path());
        let value = parser.parse(main_xml).unwrap();

        // Verify the included templates were merged
        let templates = value.get("templates").unwrap();
        let template = templates.get("template").unwrap();
        assert!(template.get("primary_btn").is_some());
    }

    #[test]
    fn test_parse_example_complete() {
        let value = parse_example("complete");
        assert!(value.get("app").is_some());
        assert!(value.get("templates").is_some());
        assert!(value.get("scripts").is_some());
        assert!(value.get("layout").is_some());
        assert!(value.get("data").is_some());

        // Verify templates from included files were merged
        let templates = value.get("templates").unwrap();
        let template = templates.get("template").unwrap();
        assert!(template.get("nav_item").is_some());
        assert!(template.get("status_card").is_some());
        assert!(template.get("metric_display").is_some());
    }

    // ── Raw-text SFC: CDATA-free parsing ───────────────────────────────────

    /// A `.nemo` with un-escaped `&&`/`<`-bearing Rhai `<script>` and a
    /// `>`-combinator CSS `<style>`, **no CDATA**, parses to the *same*
    /// `SfcDefinition` as the CDATA-wrapped equivalent. This is the core
    /// round-trip guarantee: CDATA is now optional, and dropping it changes
    /// nothing downstream.
    #[test]
    fn test_parse_sfc_raw_text_round_trip_cdata_free_equals_wrapped() {
        let template = r#"<template name="rt">
  <button label="${label}" on-click="handle" />
</template>"#;

        // Script body uses `<` (generics-ish), `&&`, and `>` — all of which
        // would be markup/errors under the old XML-only path without CDATA.
        let script_body = r#"fn handle(id, ev) {
    let xs = [1, 2, 3];
    if xs.len() > 0 && id != "" { log_info("hit"); }
}"#;
        // Style body uses the `>` combinator.
        let style_body = "button > span { color: red; }";

        let cdata_free = format!(
            "{template}\n<script>\n{script_body}\n</script>\n<style>\n{style_body}\n</style>\n"
        );
        let cdata_wrapped = format!(
            "{template}\n<script><![CDATA[\n{script_body}\n]]></script>\n<style><![CDATA[\n{style_body}\n]]></style>\n"
        );

        let free = XmlParser::new().parse_sfc(&cdata_free).unwrap();
        let wrapped = XmlParser::new().parse_sfc(&cdata_wrapped).unwrap();

        // Round-trip equality: the whole SfcDefinition matches.
        assert_eq!(free, wrapped);
        // And the raw-text bodies survived verbatim (whitespace-trimmed by the
        // `trim()` in split_sfc_blocks, so compare trimmed content).
        assert_eq!(free.script.as_deref().unwrap().trim(), script_body.trim());
        assert_eq!(free.style.as_deref().unwrap().trim(), style_body.trim());
    }

    /// The old "first contiguous run" limit is gone: a multi-run script body
    /// (text + CDATA + text under the old model, or just a multi-line block
    /// now) is captured whole.
    #[test]
    fn test_parse_sfc_raw_text_multi_run_script_captured_whole() {
        let sfc = r#"<template name="m"><button /></template>
<script>
fn a() { log_info("first"); }
// a comment in the middle
fn b() { log_info("second"); }
</script>"#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        let script = def.script.unwrap();
        assert!(script.contains("fn a()"));
        assert!(script.contains("fn b()"));
        assert!(script.contains("a comment in the middle"));
    }

    /// `</script>` inside a Rhai string literal closes the raw-text block at
    /// v1 — the same known limitation HTML has. Pinned at the `split_sfc_blocks`
    /// level (the source of the behavior) because the leftover text after the
    /// early close is not valid XML, so `parse_sfc` would error on the
    /// remainder; the truncation itself is what matters and is tested here.
    /// A future literal-aware scan would update this test deliberately.
    #[test]
    fn test_parse_sfc_raw_text_close_tag_in_string_is_v1_limitation() {
        // The `</script>` inside the string truncates the body; the trailing
        // `let y = 1;` lands *outside* the captured block.
        let sfc = r#"<template name="s"><button /></template>
<script>
fn f() {
    let x = "</script>";
    let y = 1;
}
</script>"#;
        let blocks = split_sfc_blocks(sfc);
        let script = blocks.script.unwrap();
        assert!(script.contains("let x = "));
        // v1 closes on the first literal `</script>`, so `let y` is lost.
        assert!(!script.contains("let y = 1;"));
    }

    /// CRLF line endings in the captured body survive verbatim. The body must
    /// be multi-line so *interior* CRLFs (not just leading/trailing ones, which
    /// `trim()` removes) are exercised.
    #[test]
    fn test_parse_sfc_raw_text_crlf_survives() {
        let sfc = "<template name=\"c\"><button /></template>\r\n<script>\r\nfn f() {\r\n    log_info(\"crlf\");\r\n}\r\n</script>";
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        let script = def.script.unwrap();
        assert!(
            script.contains("\r\n"),
            "interior CRLF must survive verbatim in the captured body"
        );
        assert!(script.contains("log_info"));
    }

    /// Empty or missing `<script>`/`<style>` blocks yield `None`, matching
    /// the old `.filter(|s| !s.trim().is_empty())` behavior.
    #[test]
    fn test_parse_sfc_raw_text_empty_and_missing_blocks_yield_none() {
        // Empty bodies.
        let empty =
            "<template name=\"e\"><button /></template>\n<script>   </script>\n<style>\n\n</style>";
        let def = XmlParser::new().parse_sfc(empty).unwrap();
        assert!(
            def.script.is_none(),
            "whitespace-only script must yield None"
        );
        assert!(def.style.is_none(), "whitespace-only style must yield None");

        // Missing entirely.
        let missing = "<template name=\"e\"><button /></template>";
        let def = XmlParser::new().parse_sfc(missing).unwrap();
        assert!(def.script.is_none());
        assert!(def.style.is_none());
    }

    /// Template text that looks like a block — a `<script>` nested inside
    /// `<template>` — must *not* be captured as raw-text; only top-level
    /// blocks (siblings of `<template>`/`<props>`) are split. Verified at the
    /// `split_sfc_blocks` level so the test is independent of the XML reader's
    /// tolerance for the nested element.
    #[test]
    fn test_parse_sfc_raw_text_template_interior_script_not_captured() {
        // A `<script>` nested inside `<template>` is a descendant, not a
        // top-level sibling. The splitter must leave it in `template_xml` and
        // capture only the top-level `<script>`.
        let sfc = r#"<template name="t">
  <panel><script>fn nested() { }</script></panel>
</template>
<script>fn real() { log_info("captured"); }</script>"#;
        let blocks = split_sfc_blocks(sfc);
        // Top-level script captured.
        assert_eq!(
            blocks.script.as_deref().unwrap().trim(),
            r#"fn real() { log_info("captured"); }"#
        );
        // The nested `<script>` stayed in the template half (not removed).
        assert!(
            blocks.template_xml.contains("nested"),
            "nested <script> must remain in template_xml"
        );
        assert!(
            blocks.template_xml.contains("<template"),
            "template must remain in template_xml"
        );
    }

    /// A `<style>` body containing a CSS `>` combinator and `#id` selector
    /// parses without CDATA, and the body is captured verbatim.
    #[test]
    fn test_parse_sfc_raw_text_style_combinator_no_cdata() {
        let sfc = r#"<template name="st"><button id="go" /></template>
<style>
#go > span { color: red; }
button:hover { opacity: 0.8; }
</style>"#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        let style = def.style.unwrap();
        assert!(style.contains("#go > span"));
        assert!(style.contains("button:hover"));
    }

    /// Last-wins: a second top-level `<script>` overwrites the first (v1
    /// behavior, documented and pinned).
    #[test]
    fn test_parse_sfc_raw_text_second_script_last_wins() {
        let sfc = r#"<template name="lw"><button /></template>
<script>fn first() { }</script>
<script>fn second() { }</script>"#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        let script = def.script.unwrap();
        assert!(script.contains("fn second()"));
        assert!(!script.contains("fn first()"));
    }

    /// Regression: the `examples/sfc/*.nemo` files — the definitive CDATA-free
    /// SFC reference — parse under the raw-text splitter and exercise the
    /// features that previously required CDATA: Rhai `&&`/`>` in `<script>` and
    /// the CSS `>` combinator in `<style>`. Bodies are captured verbatim.
    #[test]
    fn test_parse_sfc_examples_are_cdata_free_raw_text_reference() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let sfc_dir = std::path::Path::new(manifest_dir)
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("examples")
            .join("sfc")
            .join("components");

        for entry in std::fs::read_dir(&sfc_dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) != Some("nemo") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap();
            let def = XmlParser::new()
                .with_source_name(path.display().to_string())
                .parse_sfc(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));
            // The examples are the CDATA-free reference: captured script/style
            // bodies must not carry a CDATA wrapper (the splitter strips one if
            // present, so a surviving wrapper means the body was *only* CDATA
            // with no real content — i.e. the file wasn't actually migrated).
            for body in def.script.iter().chain(def.style.iter()) {
                assert!(
                    !body.contains("<![CDATA["),
                    "{} script/style still carries a CDATA wrapper",
                    path.display()
                );
            }
            // Every example SFC has a template.
            assert!(
                def.template.get("type").is_some(),
                "missing template in {}",
                path.display()
            );
            // labeled-button: <script> exercises Rhai `&&` and `>` (raw-text).
            // card: <style> exercises the CSS `>` combinator (raw-text).
            let stem = path.file_stem().unwrap().to_str().unwrap();
            match stem {
                "labeled-button" => {
                    let script = def
                        .script
                        .as_deref()
                        .unwrap_or_else(|| panic!("missing script in {}", stem));
                    assert!(script.contains("&&"), "script must exercise && (raw-text)");
                    assert!(script.contains("> 0"), "script must exercise > (raw-text)");
                }
                "card" => {
                    let style = def
                        .style
                        .as_deref()
                        .unwrap_or_else(|| panic!("missing style in {}", stem));
                    // The style body is captured verbatim (raw-text, no CDATA).
                    assert!(style.contains("panel {"), "style must be captured verbatim");
                    assert!(style.contains("#head"), "id selector must survive");
                }
                _ => {}
            }
        }
    }

    /// Round-trip equality: an `app.nemo` SFC compiles to the same `Value` tree
    /// as the equivalent `app.xml` — the core Phase 1 verification from the
    /// `app-nemo-sfc-entry` plan. Covers `<app>`, `<data>`, `<variable>`,
    /// `<template>` (→ `layout`), and both `<script>` forms (attribute-based
    /// `src`/`on-load` and raw-text inline body).
    #[test]
    fn test_app_sfc_round_trip_equals_app_xml() {
        let app_xml = r#"<nemo>
  <app title="My Dashboard">
    <window title="My Dashboard" width="1200" height="800">
      <header-bar github-url="https://example.com" theme-toggle="true" />
    </window>
    <theme name="nord" mode="dark" />
  </app>
  <variable name="refresh_interval" type="string" default="30" />
  <data>
    <source name="api" type="http" url="https://api.example.com" interval="30" />
  </data>
  <script src="./scripts" on-load="on_load" />
  <layout type="stack">
    <stack id="root" direction="vertical" spacing="20" padding="32">
      <label id="title" text="Dashboard" size="xl" />
    </stack>
  </layout>
</nemo>"#;

        let app_nemo = r#"<app title="My Dashboard">
  <window title="My Dashboard" width="1200" height="800">
    <header-bar github-url="https://example.com" theme-toggle="true" />
  </window>
  <theme name="nord" mode="dark" />
</app>
<variable name="refresh_interval" type="string" default="30" />
<data>
  <source name="api" type="http" url="https://api.example.com" interval="30" />
</data>
<script src="./scripts" on-load="on_load" />
<template name="app">
  <stack id="root" direction="vertical" spacing="20" padding="32">
    <label id="title" text="Dashboard" size="xl" />
  </stack>
</template>"#;

        let xml_value = XmlParser::new().parse(app_xml).unwrap();
        let sfc_value = XmlParser::new().compile_app_sfc(app_nemo).unwrap();

        assert_eq!(
            xml_value, sfc_value,
            "app.nemo SFC must compile to the same Value tree as the equivalent app.xml\n\
             --- app.xml ---\n{xml_value:#?}\n\
             --- app.nemo ---\n{sfc_value:#?}"
        );
    }

    /// Round-trip with a raw-text inline `<script>` body: the SFC's raw-text
    /// script must land in `scripts.inline` the same way `app.xml`'s CDATA
    /// `<script>` does.
    #[test]
    fn test_app_sfc_inline_script_equals_cdata() {
        let app_xml = r#"<nemo>
  <layout type="stack">
    <stack id="root"><label id="hi" text="Hi" /></stack>
  </layout>
  <script><![CDATA[
    fn init(component_id, event_data) { log_info("loaded"); }
  ]]></script>
</nemo>"#;

        let app_nemo = r#"<template name="app">
  <stack id="root"><label id="hi" text="Hi" /></stack>
</template>
<script>
    fn init(component_id, event_data) { log_info("loaded"); }
</script>"#;

        let xml_value = XmlParser::new().parse(app_xml).unwrap();
        let sfc_value = XmlParser::new().compile_app_sfc(app_nemo).unwrap();

        // The raw-text and CDATA bodies are trimmed by their respective
        // capture paths, so the inline strings match exactly.
        assert_eq!(
            xml_value.get("scripts"),
            sfc_value.get("scripts"),
            "raw-text <script> must produce the same scripts.inline as CDATA\n\
             xml: {:?}\nsfc: {:?}",
            xml_value.get("scripts"),
            sfc_value.get("scripts")
        );
        // The full tree matches too.
        assert_eq!(xml_value, sfc_value);
    }

    /// Round-trip with `<imports>`: an `<import>` in app.nemo must register the
    /// SFC under the `sfc` key the same way `<imports>` does in app.xml.
    #[test]
    fn test_app_sfc_imports_equals_app_xml() {
        let dir =
            std::env::temp_dir().join(format!("nemo_app_sfc_import_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let comp = dir.join("card.nemo");
        std::fs::write(
            &comp,
            r#"<template name="card"><panel><slot /></panel></template>"#,
        )
        .unwrap();

        let app_xml = r#"<nemo>
  <imports>
    <import src="./card.nemo" />
  </imports>
  <layout type="stack">
    <stack id="root"><card /></stack>
  </layout>
</nemo>"#
            .to_string();

        let app_nemo = r#"<imports>
  <import src="./card.nemo" />
</imports>
<template name="app">
  <stack id="root"><card /></stack>
</template>"#
            .to_string();

        let xml_value = XmlParser::new()
            .with_source_name("app.xml")
            .with_base_dir(&dir)
            .parse(&app_xml)
            .unwrap();
        let sfc_value = XmlParser::new()
            .with_source_name("app.nemo")
            .with_base_dir(&dir)
            .compile_app_sfc(&app_nemo)
            .unwrap();

        assert_eq!(
            xml_value.get("sfc"),
            sfc_value.get("sfc"),
            "<imports> must register the same sfc map in both formats"
        );
        assert_eq!(xml_value, sfc_value);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// A component `.nemo` (no app-level blocks) must still parse unchanged —
    /// `app_blocks` is `None` and `compile_app_sfc` is not used for components.
    #[test]
    fn test_component_sfc_app_blocks_none() {
        let sfc = r#"<template name="card">
  <panel><slot /></panel>
</template>"#;
        let def = XmlParser::new().parse_sfc(sfc).unwrap();
        assert!(
            def.app_blocks.is_none(),
            "component SFC must have no app_blocks"
        );
    }
}
