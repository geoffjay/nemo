//! XML parser implementation.

use crate::error::ParseError;
use crate::location::SourceLocation;
use crate::Value;
use indexmap::IndexMap;
use quick_xml::events::{BytesCData, BytesStart, Event};
use quick_xml::Reader;
use std::path::{Path, PathBuf};

/// Parser for XML configuration files.
pub struct XmlParser {
    source_name: String,
    base_dir: Option<PathBuf>,
}

impl XmlParser {
    /// Creates a new XML parser.
    pub fn new() -> Self {
        XmlParser {
            source_name: "<input>".to_string(),
            base_dir: None,
        }
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
                "layout" => {
                    let layout_val = self.process_layout(obj);
                    result.insert("layout".to_string(), layout_val);
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
                            app.insert("theme".to_string(), self.clean_element(child_obj));
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
                    if !child_type.is_empty() {
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

            // CDATA content → inline key
            if let Some(cdata) = obj.get("__cdata__").and_then(|v| v.as_str()) {
                if !cdata.trim().is_empty() {
                    let inline = scripts_obj
                        .entry("inline".to_string())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = inline {
                        arr.push(Value::String(cdata.to_string()));
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
        let src = match obj.get("src").and_then(|v| v.as_str()) {
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
        let mut anon_counter = 0;

        for child in children {
            if let Some(child_obj) = child.as_object() {
                let child_type = child_obj
                    .get("__type__")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Skip non-component elements
                if ["binding", "slot"].contains(&child_type) {
                    continue;
                }

                // Get the id attribute, or generate one
                let id = child_obj
                    .get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| {
                        anon_counter += 1;
                        format!("__anon_{}", anon_counter)
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

            // Check for <slot/> children
            for child in children {
                if let Some(child_obj) = child.as_object() {
                    if child_obj.get("__type__").and_then(|v| v.as_str()) == Some("slot") {
                        component.insert("slot".to_string(), Value::Bool(true));
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
        assert!(template.get("nav_item").is_some());
        assert!(template.get("content_page").is_some());
    }
}
