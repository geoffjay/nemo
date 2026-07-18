//! `cargo xtask design-export` — emit the nemo design system as a JSON
//! intermediate shaped to map onto pencil.dev `.pen` concepts (design tokens +
//! reusable components with variants/states/slots).
//!
//! This is a **faithful intermediate**, not a `.pen` file: the actual `.pen`
//! conversion is a later Pencil MCP/skill step. The export is fully gpui-free —
//! tokens come from `nemo-tokens`, component structure from `nemo-registry`, and
//! theme palettes are parsed straight from the theme JSON files.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nemo_config::{ConfigSchema, PropertySchema, ValidationRule, Value};
use nemo_registry::{
    register_all_builtins, ComponentCategory, ComponentDescriptor, ComponentRegistry,
};
use nemo_tokens::{FontSize, Space, RADIUS_NAMES, SEMANTIC_COLOR_ROLES};
use serde::{Deserialize, Serialize};

use crate::DesignExportArgs;

pub fn run(args: DesignExportArgs) -> Result<()> {
    let registry = ComponentRegistry::new();
    register_all_builtins(&registry);

    let export = build_export(&registry, &theme_dir())?;

    let json = if args.compact {
        serde_json::to_string(&export)
    } else {
        serde_json::to_string_pretty(&export)
    }
    .context("serializing design export")?;

    match args.output {
        Some(path) => {
            std::fs::write(&path, format!("{json}\n"))
                .with_context(|| format!("writing design export to {}", path.display()))?;
            eprintln!("Wrote design export to {}", path.display());
        }
        None => {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{json}").context("writing design export to stdout")?;
        }
    }
    Ok(())
}

/// The theme JSON directory, resolved relative to this crate so it works from any
/// cwd: `<workspace>/crates/nemo/src/theme`.
fn theme_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent (workspace root)")
        .join("crates/nemo/src/theme")
}

fn build_export(registry: &ComponentRegistry, theme_dir: &Path) -> Result<DesignExport> {
    let mut components: Vec<ComponentDto> = registry
        .list_components()
        .iter()
        .map(map_component)
        .collect();
    components.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(DesignExport {
        nemo_version: env!("CARGO_PKG_VERSION").to_string(),
        note: "Faithful design-system intermediate for pencil.dev; not a .pen file. \
               Tokens from nemo-tokens, components from nemo-registry, theme palettes \
               parsed verbatim from the theme JSON (absent fields use gpui-component \
               defaults and are omitted here)."
            .to_string(),
        tokens: build_tokens(),
        themes: load_themes(theme_dir)?,
        components,
    })
}

fn build_tokens() -> Tokens {
    Tokens {
        spacing: Space::ALL
            .iter()
            .map(|s| (s.name().to_string(), s.value()))
            .collect(),
        radius: RADIUS_NAMES
            .iter()
            .map(|n| {
                (
                    n.to_string(),
                    nemo_tokens::radius_px(n).expect("known radius name"),
                )
            })
            .collect(),
        typography: FontSize::ALL
            .iter()
            .map(|f| {
                (
                    f.name().to_string(),
                    TypeStep {
                        size: f.size(),
                        line_height: f.line_height(),
                    },
                )
            })
            .collect(),
        color_roles: SEMANTIC_COLOR_ROLES
            .iter()
            .map(|(role, field)| ColorRole {
                role: role.to_string(),
                field: field.to_string(),
            })
            .collect(),
    }
}

/// Loads every `*.json` theme set in `theme_dir`, sorted for deterministic output.
fn load_themes(theme_dir: &Path) -> Result<Vec<ThemeSetDto>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(theme_dir)
        .with_context(|| format!("reading theme dir {}", theme_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    files.sort();

    let mut sets = Vec::new();
    for path in files {
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let file: ThemeFile =
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        let id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        sets.push(ThemeSetDto {
            id,
            name: file.name,
            variants: file
                .themes
                .into_iter()
                .map(|v| ThemeVariantDto {
                    name: v.name,
                    mode: v.mode,
                    // Normalize dotted color keys (`muted.foreground`) to the
                    // snake_case field names the semantic roles reference
                    // (`muted_foreground`), keeping only string (hex) values.
                    colors: v
                        .colors
                        .into_iter()
                        .filter_map(|(k, val)| {
                            val.as_str().map(|s| (normalize_key(&k), s.to_string()))
                        })
                        .collect(),
                })
                .collect(),
        });
    }
    Ok(sets)
}

fn normalize_key(key: &str) -> String {
    key.replace('.', "_")
}

fn map_component(d: &ComponentDescriptor) -> ComponentDto {
    ComponentDto {
        name: d.name.clone(),
        category: category_str(&d.category).to_string(),
        display_name: d.metadata.display_name.clone(),
        description: d.metadata.description.clone(),
        // Variants/sizes are derived from the component's own enum properties
        // where present (populated as the registry gains enum annotations).
        variants: enum_values_of(&d.schema, "variant"),
        sizes: enum_values_of(&d.schema, "size"),
        states: states_for(&d.category),
        slots: d.metadata.slots.iter().map(|s| s.name.clone()).collect(),
        properties: map_properties(&d.schema),
    }
}

/// String values of a property's `OneOf` rule, if the property exists and is an
/// enum. Empty otherwise.
fn enum_values_of(schema: &ConfigSchema, prop: &str) -> Vec<String> {
    let Some(ps) = schema.properties.get(prop) else {
        return Vec::new();
    };
    ps.rules
        .iter()
        .find_map(|r| match r {
            ValidationRule::OneOf(vals) => Some(vals),
            _ => None,
        })
        .map(|vals| {
            vals.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// A small, static interaction-state vocabulary per component category — the
/// states a design tool would model for that kind of component.
fn states_for(category: &ComponentCategory) -> Vec<String> {
    let states: &[&str] = match category {
        ComponentCategory::Input | ComponentCategory::Navigation => {
            &["default", "hover", "focus", "active", "disabled"]
        }
        ComponentCategory::Feedback => &["default"],
        _ => &["default", "hover"],
    };
    states.iter().map(|s| s.to_string()).collect()
}

fn map_properties(schema: &ConfigSchema) -> Vec<PropertyDto> {
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

fn is_false(b: &bool) -> bool {
    !*b
}

// ── Theme JSON (minimal, gpui-free deserialization) ─────────────────────────

#[derive(Deserialize)]
struct ThemeFile {
    name: String,
    themes: Vec<ThemeFileVariant>,
}

#[derive(Deserialize)]
struct ThemeFileVariant {
    name: String,
    mode: String,
    #[serde(default)]
    colors: BTreeMap<String, serde_json::Value>,
}

// ── Serialized DTOs (the exported JSON shape) ───────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DesignExport {
    nemo_version: String,
    note: String,
    tokens: Tokens,
    themes: Vec<ThemeSetDto>,
    components: Vec<ComponentDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Tokens {
    /// name → pixels
    spacing: BTreeMap<String, f32>,
    /// name → pixels
    radius: BTreeMap<String, f32>,
    /// name → {size, lineHeight}
    typography: BTreeMap<String, TypeStep>,
    color_roles: Vec<ColorRole>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TypeStep {
    size: f32,
    line_height: f32,
}

#[derive(Serialize)]
struct ColorRole {
    role: String,
    /// The theme color field this role resolves to (look up in a theme's `colors`).
    field: String,
}

#[derive(Serialize)]
struct ThemeSetDto {
    id: String,
    name: String,
    variants: Vec<ThemeVariantDto>,
}

#[derive(Serialize)]
struct ThemeVariantDto {
    name: String,
    mode: String,
    /// Normalized color field name → hex.
    colors: BTreeMap<String, String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ComponentDto {
    name: String,
    category: String,
    display_name: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    variants: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sizes: Vec<String>,
    states: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    slots: Vec<String>,
    properties: Vec<PropertyDto>,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn export() -> DesignExport {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        build_export(&registry, &theme_dir()).expect("build export")
    }

    #[test]
    fn tokens_have_all_scales() {
        let e = export();
        assert_eq!(e.tokens.spacing.len(), Space::ALL.len());
        assert_eq!(e.tokens.typography.len(), FontSize::ALL.len());
        assert!(e.tokens.radius.contains_key("md"));
        assert!(e.tokens.color_roles.iter().any(|r| r.role == "surface"));
    }

    #[test]
    fn themes_load_with_normalized_colors() {
        let e = export();
        assert!(!e.themes.is_empty(), "expected theme sets");
        let nord = e.themes.iter().find(|t| t.id == "nord").expect("nord set");
        let dark = nord
            .variants
            .iter()
            .find(|v| v.mode == "dark")
            .expect("nord dark");
        // dotted `muted.foreground` normalized to `muted_foreground`
        assert!(dark.colors.contains_key("background"));
        assert!(dark.colors.contains_key("muted_foreground"));
    }

    #[test]
    fn components_present_and_sorted() {
        let e = export();
        assert!(e.components.iter().any(|c| c.name == "button"));
        let names: Vec<&str> = e.components.iter().map(|c| c.name.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }

    #[test]
    fn output_is_deterministic_valid_json() {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);
        let a =
            serde_json::to_string_pretty(&build_export(&registry, &theme_dir()).unwrap()).unwrap();
        let b =
            serde_json::to_string_pretty(&build_export(&registry, &theme_dir()).unwrap()).unwrap();
        assert_eq!(a, b, "design export must be byte-deterministic");
        let parsed: serde_json::Value = serde_json::from_str(&a).unwrap();
        assert!(parsed.get("tokens").unwrap().is_object());
        assert!(parsed.get("themes").unwrap().is_array());
        assert!(parsed.get("components").unwrap().is_array());
    }
}
