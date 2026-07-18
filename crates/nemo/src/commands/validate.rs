//! `nemo validate` — validate a configuration file and exit.
//!
//! Runs the same parse + resolve path the app uses at load time
//! (`ConfigurationLoader::load`), reporting located diagnostics. `--strict`
//! adds component-level lints. Exits non-zero if any error-severity diagnostic
//! is produced.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use nemo_config::{
    ConfigError, ConfigurationLoader, SchemaRegistry, SourceLocation, ValidationRule, Value,
};
use nemo_registry::{register_all_builtins, ComponentRegistry};
use serde::Serialize;

use crate::args::{ValidateArgs, ValidateFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
enum Severity {
    Error,
    Warning,
}

impl Severity {
    fn label(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// A single validation finding.
#[derive(Debug, Serialize)]
struct Diagnostic {
    severity: Severity,
    /// Short kebab-case category, e.g. `parse`, `resolve`, `unknown-attribute`.
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    column: Option<usize>,
}

impl Diagnostic {
    fn error(code: &str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code: code.to_string(),
            message: message.into(),
            file: None,
            line: None,
            column: None,
        }
    }

    fn warning(code: &str, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            code: code.to_string(),
            message: message.into(),
            file: None,
            line: None,
            column: None,
        }
    }

    /// Attach a source location (file always; line/column only if known).
    fn at(mut self, loc: &SourceLocation) -> Self {
        self.file = Some(loc.file.clone());
        if !loc.is_unknown() {
            self.line = Some(loc.line);
            self.column = Some(loc.column);
        }
        self
    }
}

pub fn run(args: ValidateArgs) -> Result<()> {
    let path = args.app_config.as_path();
    let source = std::fs::read_to_string(path).unwrap_or_default();

    let mut diagnostics = Vec::new();
    let loader = ConfigurationLoader::new(Arc::new(SchemaRegistry::new()));
    match loader.load(path) {
        Ok(value) => {
            if args.strict {
                diagnostics.extend(strict_lints(&value));
            }
        }
        Err(err) => diagnostics.extend(config_error_to_diagnostics(err)),
    }

    let error_count = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warning_count = diagnostics.len() - error_count;

    match args.format {
        ValidateFormat::Human => {
            render_human(path, &source, &diagnostics, error_count, warning_count)
        }
        ValidateFormat::Json => render_json(path, &diagnostics, error_count, warning_count),
    }

    if error_count > 0 {
        // Diagnostics already rendered; exit non-zero without an anyhow message.
        std::process::exit(1);
    }
    Ok(())
}

/// Convert a structured `ConfigError` into one or more diagnostics.
fn config_error_to_diagnostics(err: ConfigError) -> Vec<Diagnostic> {
    match err {
        ConfigError::Parse(pe) => {
            let mut message = pe.message.clone();
            if !pe.suggestions.is_empty() {
                message.push_str(&format!(" (hint: {})", pe.suggestions.join("; ")));
            }
            vec![Diagnostic::error("parse", message).at(&pe.location)]
        }
        ConfigError::Validation { errors } => errors
            .into_iter()
            .map(|ve| {
                let d = Diagnostic::error("validation", ve.message.clone());
                match &ve.location {
                    Some(loc) => d.at(loc),
                    None => d,
                }
            })
            .collect(),
        ConfigError::Resolve(re) => vec![Diagnostic::error("resolve", re.to_string())],
        ConfigError::Io { path, message } => {
            vec![Diagnostic::error("io", format!("{}: {}", path, message))]
        }
        ConfigError::SchemaNotFound { name } => {
            vec![Diagnostic::error(
                "schema",
                format!("Schema not found: {}", name),
            )]
        }
    }
}

/// Structural keys on a component node that are not user-facing attributes.
fn is_structural_key(key: &str) -> bool {
    matches!(
        key,
        "type" | "component" | "binding" | "slot" | "vars" | "template"
    )
}

/// Universal styling attributes applied by `apply_layout_styles` to every
/// component wrapper, regardless of component type. These are not enumerated
/// in individual builtin schemas, so the `unknown-attribute` lint must skip
/// them to avoid false positives.
///
/// The canonical list lives in `nemo_registry::schema_surface` so the linter and
/// the `nemo schema` exporter share one source and cannot drift.
fn is_universal_style(key: &str) -> bool {
    nemo_registry::universal_style_attributes()
        .iter()
        .any(|a| a.name == key)
}

/// Component-level lints, only run under `--strict`.
///
/// Walks the parsed config the way the runtime does (`layout.component` is an
/// `{id -> node}` map; a node's type is its `type` field; children live under a
/// nested `component` map) and cross-checks each component against the registry.
///
/// Note: the parsed `Value` tree carries no source locations, so these
/// diagnostics are unlocated (identified by component id / type instead).
fn strict_lints(root: &Value) -> Vec<Diagnostic> {
    let registry = ComponentRegistry::new();
    register_all_builtins(&registry);
    lint_config(root, &registry)
}

/// Core of [`strict_lints`], parameterized over the registry for testing.
fn lint_config(root: &Value, registry: &ComponentRegistry) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut template_refs = std::collections::HashSet::new();

    // Lint the live layout tree.
    if let Some(layout) = root.get("layout") {
        lint_component_children(layout, registry, &mut diagnostics, &mut template_refs);
    }

    // Lint template bodies too, and collect any template-to-template references.
    let templates = root
        .get("templates")
        .and_then(|t| t.get("template"))
        .and_then(|t| t.as_object());
    if let Some(templates) = templates {
        for (name, body) in templates {
            lint_component(name, body, registry, &mut diagnostics, &mut template_refs);
        }
        // Flag templates that are defined but never referenced.
        for name in templates.keys() {
            if !template_refs.contains(name) {
                diagnostics.push(Diagnostic::warning(
                    "unused-template",
                    format!("Template '{name}' is defined but never referenced"),
                ));
            }
        }
    }

    diagnostics
}

/// Lint every component in a node's `component` child map.
fn lint_component_children(
    node: &Value,
    registry: &ComponentRegistry,
    diagnostics: &mut Vec<Diagnostic>,
    template_refs: &mut std::collections::HashSet<String>,
) {
    if let Some(children) = node.get("component").and_then(|c| c.as_object()) {
        for (id, child) in children {
            lint_component(id, child, registry, diagnostics, template_refs);
        }
    }
}

/// Lint a single component node and recurse into its children.
fn lint_component(
    id: &str,
    component: &Value,
    registry: &ComponentRegistry,
    diagnostics: &mut Vec<Diagnostic>,
    template_refs: &mut std::collections::HashSet<String>,
) {
    let Some(obj) = component.as_object() else {
        return;
    };

    let ctype = obj.get("type").and_then(|v| v.as_str()).unwrap_or_default();
    let uses_template = obj.contains_key("template");
    if let Some(name) = obj.get("template").and_then(|v| v.as_str()) {
        template_refs.insert(name.to_string());
    }

    // Component/attribute checks only apply to non-templated components (a
    // templated node's shape is provided by the template at runtime).
    if !ctype.is_empty() && !uses_template {
        if !registry.has_component(ctype) {
            diagnostics.push(Diagnostic::error(
                "unknown-component",
                format!("Unknown component type '{ctype}' (id '{id}')"),
            ));
        } else if let Some(descriptor) = registry.get_component(ctype) {
            let schema = &descriptor.schema;

            // `missing-required` is only reliable when a schema declares itself
            // exhaustive (`additional_properties == false`). Permissive builtin
            // schemas don't declare requireds reliably (props can arrive via
            // bindings, child elements, or template expansion), so enforcing
            // required against them produces false positives. Strict/plugin
            // schemas that opt in get the full check.
            if !schema.additional_properties {
                for required in &schema.required {
                    if !obj.contains_key(required) {
                        diagnostics.push(Diagnostic::error(
                            "missing-required",
                            format!(
                                "Component '{id}' (type '{ctype}') is missing required property '{required}'"
                            ),
                        ));
                    }
                }
            }

            // `unknown-attribute` runs on all schemas. Universal styling
            // attributes (padding, border, width, ...) are applied by
            // `apply_layout_styles` to every component wrapper and are not
            // enumerated in individual schemas — they are allowlisted via
            // `is_universal_style`. Structural keys, handler prefixes (`on_`),
            // and binding prefixes (`bind_`) are also skipped.
            for key in obj.keys() {
                if is_structural_key(key)
                    || is_universal_style(key)
                    || key.starts_with("on_")
                    || key.starts_with("bind_")
                {
                    continue;
                }
                if !schema.properties.contains_key(key) {
                    diagnostics.push(Diagnostic::warning(
                        "unknown-attribute",
                        format!("Component '{id}' (type '{ctype}') has unknown attribute '{key}'"),
                    ));
                }
            }

            // `invalid-value`: a property with an enum (`one_of`) constraint
            // whose literal value isn't allowed (e.g. `variant="bogus"`). Skips
            // unresolved `${...}` expressions and non-string values, which are
            // only knowable after resolution/binding.
            for (key, prop_schema) in &schema.properties {
                let Some(value) = obj.get(key).and_then(|v| v.as_str()) else {
                    continue;
                };
                if value.contains("${") {
                    continue;
                }
                for rule in &prop_schema.rules {
                    let ValidationRule::OneOf(allowed) = rule else {
                        continue;
                    };
                    if !allowed.iter().any(|a| a.as_str() == Some(value)) {
                        let allowed_list: Vec<&str> =
                            allowed.iter().filter_map(|a| a.as_str()).collect();
                        diagnostics.push(Diagnostic::warning(
                            "invalid-value",
                            format!(
                                "Component '{id}' (type '{ctype}') property '{key}'=\"{value}\" is not one of: {}",
                                allowed_list.join(", ")
                            ),
                        ));
                    }
                }
            }
        }
    }

    // A component wired to a handler or binding but left anonymous can't be
    // targeted reliably; flag the missing id.
    let is_anonymous = id.starts_with("__anon");
    let has_handler = obj.keys().any(|k| k.starts_with("on_"));
    let has_binding = obj.contains_key("binding") || obj.keys().any(|k| k.starts_with("bind_"));
    if is_anonymous && (has_handler || has_binding) {
        diagnostics.push(Diagnostic::warning(
            "missing-id",
            format!("Component of type '{ctype}' has a handler or binding but no id"),
        ));
    }

    lint_component_children(component, registry, diagnostics, template_refs);
}

fn render_human(
    path: &Path,
    source: &str,
    diagnostics: &[Diagnostic],
    error_count: usize,
    warning_count: usize,
) {
    for d in diagnostics {
        let location = match (d.line, d.column) {
            (Some(line), Some(col)) => {
                format!(
                    "{}:{}:{}",
                    d.file.as_deref().unwrap_or("<unknown>"),
                    line,
                    col
                )
            }
            _ => d.file.clone().unwrap_or_default(),
        };

        eprintln!("{}: [{}] {}", d.severity.label(), d.code, d.message);
        if !location.is_empty() {
            eprintln!("  --> {location}");
        }
        if let Some(line) = d.line {
            let loc = SourceLocation::new(
                d.file.clone().unwrap_or_default(),
                line,
                d.column.unwrap_or(0),
            );
            let context = loc.display_context(source, 1);
            if !context.is_empty() {
                eprint!("{context}");
            }
        }
    }

    if error_count == 0 && warning_count == 0 {
        println!("{}: configuration is valid", path.display());
    } else {
        eprintln!("\n{error_count} error(s), {warning_count} warning(s)");
    }
}

fn render_json(path: &Path, diagnostics: &[Diagnostic], error_count: usize, warning_count: usize) {
    let report = serde_json::json!({
        "file": path.display().to_string(),
        "valid": error_count == 0,
        "errorCount": error_count,
        "warningCount": warning_count,
        "diagnostics": diagnostics,
    });
    match serde_json::to_string_pretty(&report) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("failed to serialize diagnostics: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nemo_config::{ConfigSchema, ConfigurationLoader, PropertySchema, SchemaRegistry};
    use nemo_registry::{ComponentCategory, ComponentDescriptor, ComponentRegistry};

    fn parse(xml: &str) -> Value {
        ConfigurationLoader::new(std::sync::Arc::new(SchemaRegistry::new()))
            .load_xml_string(xml, "test.xml", None)
            .expect("test config should parse")
    }

    fn builtins() -> ComponentRegistry {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        registry
    }

    fn codes(diags: &[Diagnostic]) -> Vec<&str> {
        diags.iter().map(|d| d.code.as_str()).collect()
    }

    #[test]
    fn flags_unknown_component_type() {
        let value = parse(r#"<nemo><layout type="stack"><notacomponent id="x" /></layout></nemo>"#);
        let diags = lint_config(&value, &builtins());
        assert!(codes(&diags).contains(&"unknown-component"), "{diags:?}");
    }

    #[test]
    fn flags_unused_template_but_not_referenced_one() {
        let value = parse(
            r#"<nemo>
                <templates>
                    <template name="used"><label text="a" /></template>
                    <template name="orphan"><label text="b" /></template>
                </templates>
                <layout type="stack"><label id="l" template="used" /></layout>
            </nemo>"#,
        );
        let diags = lint_config(&value, &builtins());
        let unused: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "unused-template")
            .collect();
        assert_eq!(unused.len(), 1, "{diags:?}");
        assert!(unused[0].message.contains("orphan"));
    }

    #[test]
    fn flags_anonymous_component_with_handler() {
        let value = parse(
            r#"<nemo><layout type="stack"><button label="Go" on-click="h" /></layout></nemo>"#,
        );
        let diags = lint_config(&value, &builtins());
        assert!(codes(&diags).contains(&"missing-id"), "{diags:?}");
    }

    #[test]
    fn flags_invalid_enum_value_but_not_valid() {
        // A `variant` outside the component's `one_of` set is flagged; a valid
        // one is not.
        let value = parse(
            r#"<nemo><layout type="stack">
                <button id="bad" label="a" variant="bogus" />
                <button id="ok" label="b" variant="primary" />
            </layout></nemo>"#,
        );
        let diags = lint_config(&value, &builtins());
        let invalid: Vec<_> = diags.iter().filter(|d| d.code == "invalid-value").collect();
        assert_eq!(invalid.len(), 1, "{diags:?}");
        assert!(invalid[0].message.contains("bogus"));
        assert!(invalid[0].message.contains("primary")); // lists allowed values
    }

    #[test]
    fn universal_style_attributes_not_flagged() {
        // Universal styling attributes (padding, border, width, ...) are applied
        // by apply_layout_styles to every component wrapper and are not
        // enumerated in individual schemas — they must not be flagged.
        let value = parse(
            r#"<nemo><layout type="stack"><label id="l" text="hi" padding="4" border="1" width="200" margin="8" rounded="sm" background="red.500" /></layout></nemo>"#,
        );
        let diags = lint_config(&value, &builtins());
        assert!(
            !codes(&diags).contains(&"unknown-attribute"),
            "universal style attributes should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn universal_style_max_and_scroll_not_flagged() {
        // Regression: is_universal_style previously omitted max-width/max-height/
        // scroll (drift from apply_layout_styles), so --strict falsely flagged
        // them. They are now single-sourced from schema_surface.
        let value = parse(
            r#"<nemo><layout type="stack"><stack id="s" max-width="400" max-height="300" scroll="true" /></layout></nemo>"#,
        );
        let diags = lint_config(&value, &builtins());
        assert!(
            !codes(&diags).contains(&"unknown-attribute"),
            "max-width/max-height/scroll should not be flagged: {diags:?}"
        );
    }

    #[test]
    fn unknown_attribute_flagged_on_permissive_schema() {
        // A genuinely unknown attribute (typo, non-existent property) should be
        // flagged even on a permissive builtin schema — the lint is no longer
        // gated on additional_properties.
        let value = parse(
            r#"<nemo><layout type="stack"><label id="l" text="hi" typo_attribute="oops" /></layout></nemo>"#,
        );
        let diags = lint_config(&value, &builtins());
        assert!(
            codes(&diags).contains(&"unknown-attribute"),
            "unknown attribute should be flagged: {diags:?}"
        );
    }

    #[test]
    fn strict_schema_flags_unknown_and_missing_properties() {
        // A component whose schema opts into strict validation gets full checks.
        let registry = ComponentRegistry::new();
        registry
            .register_component(
                ComponentDescriptor::new("widget", ComponentCategory::Display).schema(
                    ConfigSchema::new("widget")
                        .property("label", PropertySchema::string())
                        .require("label")
                        .strict(),
                ),
            )
            .expect("register widget");

        let value =
            parse(r#"<nemo><layout type="stack"><widget id="w" title="oops" /></layout></nemo>"#);
        let diags = lint_config(&value, &registry);
        let found = codes(&diags);
        assert!(found.contains(&"missing-required"), "{diags:?}");
        assert!(found.contains(&"unknown-attribute"), "{diags:?}");
    }
}
