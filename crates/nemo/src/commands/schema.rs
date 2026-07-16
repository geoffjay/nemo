//! `nemo schema` — export the configuration schema for this build and exit.
//!
//! The schema is generated from the in-memory registries (`register_all_builtins`)
//! plus the canonical XML surface in `nemo_registry::schema_surface`, so it is
//! always current with the compiled binary — never hand-maintained. Output is a
//! nemo-native JSON document (see the DTOs below) intended to feed tooling: the
//! planned `nemo-lsp`, the schema-driven gallery, the LLM `nemo generate` prompt,
//! and docs.
//!
//! # Phase 1 caveat
//!
//! `events`, `bindableProperties`, `slots`, `allowedChildren`, and most property
//! `enum`/`min`/`max` constraints are empty because the builtins don't populate
//! that metadata yet. The JSON *shape* is stable; Phase 2 (macro-derived schema +
//! enum annotations + containment) fills the content.

use std::io::Write as _;

use anyhow::{Context, Result};
use nemo_config::{PropertySchema, ValidationRule, Value};
use nemo_registry::{
    attribute_families, register_all_builtins, structural_elements, universal_style_attributes,
    ActionDescriptor, ComponentCategory, ComponentDescriptor, ComponentRegistry,
    DataSourceDescriptor, TransformDescriptor,
};
use serde::Serialize;

use crate::args::{SchemaArgs, SchemaFormat};

pub fn run(args: SchemaArgs) -> Result<()> {
    let registry = ComponentRegistry::new();
    register_all_builtins(&registry);

    let export = build_export(&registry);

    let json = match args.format {
        SchemaFormat::Json => if args.compact {
            serde_json::to_string(&export)
        } else {
            serde_json::to_string_pretty(&export)
        }
        .context("serializing schema")?,
    };

    match args.output {
        Some(path) => {
            std::fs::write(&path, format!("{json}\n"))
                .with_context(|| format!("writing schema to {}", path.display()))?;
            eprintln!("Wrote schema to {}", path.display());
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{json}").context("writing schema to stdout")?;
        }
    }
    Ok(())
}

/// Builds the full schema export from a populated registry. Factored out so tests
/// can assert on the structured value without going through serialization/IO.
fn build_export(registry: &ComponentRegistry) -> SchemaExport {
    let mut components: Vec<ComponentDto> = registry
        .list_components()
        .iter()
        .map(map_component)
        .collect();
    components.sort_by(|a, b| a.name.cmp(&b.name));

    let mut data_sources: Vec<DataSourceDto> = registry
        .list_data_sources()
        .iter()
        .map(map_data_source)
        .collect();
    data_sources.sort_by(|a, b| a.name.cmp(&b.name));

    let mut transforms: Vec<EntityDto> = registry
        .list_transforms()
        .iter()
        .map(map_transform)
        .collect();
    transforms.sort_by(|a, b| a.name.cmp(&b.name));

    let mut actions: Vec<EntityDto> = registry.list_actions().iter().map(map_action).collect();
    actions.sort_by(|a, b| a.name.cmp(&b.name));

    SchemaExport {
        nemo_version: env!("CARGO_PKG_VERSION").to_string(),
        universal_attributes: universal_style_attributes()
            .iter()
            .map(|a| PropertyDto {
                name: a.name.to_string(),
                value_type: a.value_type.to_string(),
                description: non_empty(a.description),
                default: None,
                enum_values: None,
                min: None,
                max: None,
                required: false,
            })
            .collect(),
        attribute_families: attribute_families()
            .iter()
            .map(|f| FamilyDto {
                prefix: f.prefix.to_string(),
                description: f.description.to_string(),
            })
            .collect(),
        structural: structural_elements()
            .iter()
            .map(|s| StructuralDto {
                element: s.element.to_string(),
                description: non_empty(s.description),
                attributes: s
                    .attributes
                    .iter()
                    .map(|a| PropertyDto {
                        name: a.name.to_string(),
                        value_type: a.value_type.to_string(),
                        description: non_empty(a.description),
                        default: None,
                        enum_values: None,
                        min: None,
                        max: None,
                        required: false,
                    })
                    .collect(),
                child_elements: s.child_elements.iter().map(|c| c.to_string()).collect(),
            })
            .collect(),
        components,
        data_sources,
        transforms,
        actions,
    }
}

/// Maps a `ConfigSchema`'s properties into ordered `PropertyDto`s, translating
/// `ValidationRule`s into `enum`/`min`/`max` where present.
fn map_properties(schema: &nemo_config::ConfigSchema) -> Vec<PropertyDto> {
    schema
        .properties
        .iter()
        .map(|(name, ps)| map_property(name, ps, schema.required.iter().any(|r| r == name)))
        .collect()
}

fn map_property(name: &str, ps: &PropertySchema, required: bool) -> PropertyDto {
    let mut enum_values = None;
    let mut min = None;
    let mut max = None;
    for rule in &ps.rules {
        match rule {
            ValidationRule::OneOf(values) => enum_values = Some(values.clone()),
            ValidationRule::Min(v) => min = Some(*v),
            ValidationRule::Max(v) => max = Some(*v),
            _ => {}
        }
    }
    PropertyDto {
        name: name.to_string(),
        value_type: ps.value_type.to_string(),
        description: ps.description.clone(),
        default: ps.default.clone(),
        enum_values,
        min,
        max,
        required,
    }
}

fn map_component(d: &ComponentDescriptor) -> ComponentDto {
    ComponentDto {
        name: d.name.clone(),
        category: category_str(&d.category).to_string(),
        display_name: d.metadata.display_name.clone(),
        description: d.metadata.description.clone(),
        properties: map_properties(&d.schema),
        // Phase-2 metadata surfaces (empty until the builtins populate them).
        events: d.metadata.events.iter().map(|e| e.name.clone()).collect(),
        bindable_properties: d
            .metadata
            .bindable_properties
            .iter()
            .map(|b| b.name.clone())
            .collect(),
        slots: d.metadata.slots.iter().map(|s| s.name.clone()).collect(),
        allowed_children: Vec::new(),
    }
}

fn map_data_source(d: &DataSourceDescriptor) -> DataSourceDto {
    DataSourceDto {
        name: d.name.clone(),
        display_name: d.metadata.display_name.clone(),
        description: d.metadata.description.clone(),
        properties: map_properties(&d.schema),
        capabilities: DataSourceCapabilities {
            polling: d.metadata.supports_polling,
            streaming: d.metadata.supports_streaming,
            manual_refresh: d.metadata.supports_manual_refresh,
        },
    }
}

fn map_transform(d: &TransformDescriptor) -> EntityDto {
    EntityDto {
        name: d.name.clone(),
        display_name: d.metadata.display_name.clone(),
        description: d.metadata.description.clone(),
        properties: map_properties(&d.schema),
    }
}

fn map_action(d: &ActionDescriptor) -> EntityDto {
    EntityDto {
        name: d.name.clone(),
        display_name: d.metadata.display_name.clone(),
        description: d.metadata.description.clone(),
        properties: map_properties(&d.schema),
    }
}

fn category_str(c: &ComponentCategory) -> &'static str {
    match c {
        ComponentCategory::Layout => "layout",
        ComponentCategory::Display => "display",
        ComponentCategory::Input => "input",
        ComponentCategory::Data => "data",
        ComponentCategory::Feedback => "feedback",
        ComponentCategory::Navigation => "navigation",
        ComponentCategory::Charts => "charts",
        ComponentCategory::Custom => "custom",
    }
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

// ── Serialized DTOs (the published JSON shape) ──────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SchemaExport {
    nemo_version: String,
    universal_attributes: Vec<PropertyDto>,
    attribute_families: Vec<FamilyDto>,
    structural: Vec<StructuralDto>,
    components: Vec<ComponentDto>,
    data_sources: Vec<DataSourceDto>,
    transforms: Vec<EntityDto>,
    actions: Vec<EntityDto>,
}

#[derive(Serialize)]
struct PropertyDto {
    name: String,
    #[serde(rename = "type")]
    value_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<Value>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    enum_values: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max: Option<i64>,
    #[serde(skip_serializing_if = "is_false")]
    required: bool,
}

#[derive(Serialize)]
struct FamilyDto {
    prefix: String,
    description: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StructuralDto {
    element: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    attributes: Vec<PropertyDto>,
    child_elements: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentDto {
    name: String,
    category: String,
    display_name: String,
    description: String,
    properties: Vec<PropertyDto>,
    events: Vec<String>,
    bindable_properties: Vec<String>,
    slots: Vec<String>,
    allowed_children: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DataSourceDto {
    name: String,
    display_name: String,
    description: String,
    properties: Vec<PropertyDto>,
    capabilities: DataSourceCapabilities,
}

#[derive(Serialize)]
struct DataSourceCapabilities {
    polling: bool,
    streaming: bool,
    manual_refresh: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EntityDto {
    name: String,
    display_name: String,
    description: String,
    properties: Vec<PropertyDto>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn export() -> SchemaExport {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        build_export(&registry)
    }

    #[test]
    fn version_matches_crate() {
        assert_eq!(export().nemo_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn includes_known_components() {
        let e = export();
        assert!(e.components.iter().any(|c| c.name == "button"));
        assert!(e.components.iter().any(|c| c.name == "app_shell"));
    }

    #[test]
    fn components_are_sorted() {
        let e = export();
        let names: Vec<&str> = e.components.iter().map(|c| c.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn no_component_is_dropped() {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        let expected = registry.list_components().len();
        assert_eq!(build_export(&registry).components.len(), expected);
    }

    #[test]
    fn universal_attributes_include_previously_missing_ones() {
        let e = export();
        let names: Vec<&str> = e
            .universal_attributes
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        for n in ["max_width", "max_height", "scroll"] {
            assert!(names.contains(&n), "universal attr {n} missing");
        }
    }

    #[test]
    fn output_is_valid_and_deterministic_json() {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        let a = serde_json::to_string_pretty(&build_export(&registry)).unwrap();
        let b = serde_json::to_string_pretty(&build_export(&registry)).unwrap();
        assert_eq!(a, b, "schema export must be byte-deterministic");
        let parsed: serde_json::Value = serde_json::from_str(&a).unwrap();
        assert!(parsed.get("components").unwrap().is_array());
        assert!(parsed.get("universalAttributes").unwrap().is_array());
        assert!(parsed.get("nemoVersion").unwrap().is_string());
    }

    #[test]
    fn includes_data_sources_transforms_actions() {
        let e = export();
        assert!(e.data_sources.iter().any(|d| d.name == "http"));
        assert!(e.transforms.iter().any(|t| t.name == "filter"));
        assert!(!e.actions.is_empty());
    }
}
