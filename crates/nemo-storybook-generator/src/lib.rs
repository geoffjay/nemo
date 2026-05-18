//! Nemo Storybook Generator
//!
//! Generates a complete Nemo XML config for the component storybook by
//! introspecting the built-in component registry.

use nemo_config::ValueType;
use nemo_registry::{
    register_builtin_components, ComponentCategory, ComponentDescriptor, ComponentRegistry,
};

/// The ordered list of categories displayed in the sidebar.
const CATEGORIES: &[ComponentCategory] = &[
    ComponentCategory::Layout,
    ComponentCategory::Display,
    ComponentCategory::Input,
    ComponentCategory::Data,
    ComponentCategory::Feedback,
    ComponentCategory::Navigation,
    ComponentCategory::Charts,
];

fn category_label(cat: &ComponentCategory) -> &'static str {
    match cat {
        ComponentCategory::Layout => "Layout",
        ComponentCategory::Display => "Display",
        ComponentCategory::Input => "Input",
        ComponentCategory::Data => "Data",
        ComponentCategory::Feedback => "Feedback",
        ComponentCategory::Navigation => "Navigation",
        ComponentCategory::Charts => "Charts",
        ComponentCategory::Custom => "Custom",
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Generate an actual XML element for the live preview panel, with required
/// properties filled in so the layout builder's `MissingProperty` check passes.
fn generate_preview_element(comp: &ComponentDescriptor) -> String {
    let name = &comp.name;
    let schema = &comp.schema;

    let mut attrs = String::new();
    for prop_name in &schema.required {
        let value = if let Some(prop) = schema.properties.get(prop_name) {
            match &prop.value_type {
                ValueType::String => "placeholder",
                ValueType::Integer => "0",
                ValueType::Float => "0.0",
                ValueType::Boolean => "false",
                ValueType::Array => "[]",
                ValueType::Object => "{}",
                ValueType::Any => "",
            }
        } else {
            "placeholder"
        };
        attrs.push_str(&format!(" {}=\"{}\"", prop_name, xml_escape(value)));
    }

    format!("<{}{} />", name, attrs)
}


fn generate_sidebar(registry: &ComponentRegistry) -> String {
    let mut out = String::new();
    out.push_str("      <panel id=\"sidebar\" width=\"240\">\n");
    out.push_str("        <stack id=\"sidebar_inner\" direction=\"vertical\" spacing=\"0\">\n");
    out.push_str("          <label id=\"sidebar_title\" text=\"Nemo Storybook\" size=\"lg\" padding=\"16\" />\n");
    out.push_str("          <input id=\"sidebar_search\" placeholder=\"Search components...\" padding-x=\"8\" />\n");

    for cat in CATEGORIES {
        let cat_label = category_label(cat);
        out.push_str(&format!(
            "          <label text=\"{}\" size=\"sm\" padding=\"8\" />\n",
            cat_label
        ));
        let mut comps = registry.list_by_category(cat.clone());
        comps.sort_by(|a, b| a.name.cmp(&b.name));
        for comp in &comps {
            let display_name = if comp.metadata.display_name.is_empty() {
                &comp.name
            } else {
                &comp.metadata.display_name
            };
            out.push_str(&format!(
                "          <button id=\"nav_{}\" label=\"{}\" variant=\"ghost\" />\n",
                comp.name,
                xml_escape(display_name)
            ));
        }
    }

    out.push_str("        </stack>\n");
    out.push_str("      </panel>\n");
    out
}

fn generate_component_page(comp: &ComponentDescriptor) -> String {
    let name = &comp.name;
    let display_name = if comp.metadata.display_name.is_empty() {
        name.clone()
    } else {
        comp.metadata.display_name.clone()
    };
    let description = &comp.metadata.description;
    let schema = &comp.schema;
    let required: std::collections::HashSet<&str> =
        schema.required.iter().map(|s| s.as_str()).collect();

    let mut out = String::new();
    out.push_str(&format!("        <panel id=\"page_{}\">\n", name));
    out.push_str(&format!(
        "          <stack id=\"{}_inner\" direction=\"vertical\" spacing=\"12\" padding=\"24\">\n",
        name
    ));

    // Title and description
    out.push_str(&format!(
        "            <label id=\"{}_title\" text=\"{}\" size=\"xl\" />\n",
        name,
        xml_escape(&display_name)
    ));
    if !description.is_empty() {
        out.push_str(&format!(
            "            <text id=\"{}_desc\" content=\"{}\" />\n",
            name,
            xml_escape(description)
        ));
    }

    // Properties — one label per property, no table/code-editor
    if !schema.properties.is_empty() {
        out.push_str(&format!(
            "            <label id=\"{}_props_label\" text=\"Properties\" size=\"md\" />\n",
            name
        ));
        for (i, (prop_name, prop)) in schema.properties.iter().enumerate() {
            let req = if required.contains(prop_name.as_str()) {
                " *"
            } else {
                ""
            };
            let type_str = match &prop.value_type {
                ValueType::String => "string",
                ValueType::Integer => "integer",
                ValueType::Float => "float",
                ValueType::Boolean => "boolean",
                ValueType::Array => "array",
                ValueType::Object => "object",
                ValueType::Any => "any",
            };
            let default_str = prop
                .default
                .as_ref()
                .map(|v| match v {
                    nemo_config::Value::String(s) => format!(" = \"{}\"", s),
                    nemo_config::Value::Integer(n) => format!(" = {}", n),
                    nemo_config::Value::Float(f) => format!(" = {}", f),
                    nemo_config::Value::Bool(b) => format!(" = {}", b),
                    _ => String::new(),
                })
                .unwrap_or_default();
            let row = format!("{}{}: {}{}", prop_name, req, type_str, default_str);
            out.push_str(&format!(
                "            <label id=\"{}_prop_{}\" text=\"{}\" size=\"sm\" />\n",
                name,
                i,
                xml_escape(&row)
            ));
        }
    }

    // Live preview — component instance with required props filled in
    out.push_str(&format!(
        "            <label id=\"{}_preview_label\" text=\"Preview\" size=\"md\" />\n",
        name
    ));
    out.push_str(&format!(
        "            {}\n",
        generate_preview_element(comp)
    ));

    out.push_str("          </stack>\n");
    out.push_str("        </panel>\n");
    out
}

fn generate_home_page(registry: &ComponentRegistry) -> String {
    let total: usize = CATEGORIES
        .iter()
        .map(|cat| registry.list_by_category(cat.clone()).len())
        .sum();
    let mut out = String::new();
    out.push_str("        <panel id=\"page_home\">\n");
    out.push_str("          <stack id=\"home_inner\" direction=\"vertical\" spacing=\"16\" padding=\"32\">\n");
    out.push_str(
        "            <label id=\"home_title\" text=\"Nemo Component Storybook\" size=\"xl\" />\n",
    );
    out.push_str("            <text id=\"home_desc\" content=\"Select a component from the sidebar to view its documentation, properties, and live examples.\" />\n");
    out.push_str(&format!(
        "            <label id=\"home_count\" text=\"{} components available\" size=\"sm\" />\n",
        total
    ));
    out.push_str("          </stack>\n");
    out.push_str("        </panel>\n");
    out
}

/// Generate a complete Nemo XML storybook config.
pub fn generate_storybook_xml() -> String {
    let registry = ComponentRegistry::new();
    register_builtin_components(&registry);

    let mut out = String::new();
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str("<!-- Generated by nemo-storybook-generator. Do not edit by hand. -->\n");
    out.push_str("<nemo>\n");

    // App config
    out.push_str("  <app title=\"Nemo Storybook\">\n");
    out.push_str("    <window title=\"Nemo Storybook\" width=\"1200\" height=\"800\" min-width=\"900\" min-height=\"600\">\n");
    out.push_str("      <header-bar theme-toggle=\"true\" />\n");
    out.push_str("      <footer-bar enabled=\"false\" />\n");
    out.push_str("    </window>\n");
    out.push_str("    <theme name=\"kanagawa\" mode=\"dark\" />\n");
    out.push_str("  </app>\n\n");

    // Layout
    out.push_str("  <layout type=\"stack\">\n");
    out.push_str(
        "    <stack id=\"root_row\" direction=\"horizontal\" spacing=\"0\" width=\"1200\">\n\n",
    );

    // Sidebar
    out.push_str(&generate_sidebar(&registry));
    out.push('\n');

    // Content area
    out.push_str("      <stack id=\"content_area\" direction=\"vertical\" spacing=\"0\" flex=\"1.0\" scroll=\"true\">\n\n");

    // Home page
    out.push_str(&generate_home_page(&registry));
    out.push('\n');

    // Component pages
    for cat in CATEGORIES {
        let mut comps = registry.list_by_category(cat.clone());
        comps.sort_by(|a, b| a.name.cmp(&b.name));
        for comp in &comps {
            out.push_str(&generate_component_page(comp));
            out.push('\n');
        }
    }

    out.push_str("      </stack>\n");
    out.push_str("    </stack>\n");
    out.push_str("  </layout>\n\n");

    // Collect all component names for the script section
    let all_names: Vec<String> = {
        let mut names = Vec::new();
        for cat in CATEGORIES {
            let mut comps = registry.list_by_category(cat.clone());
            comps.sort_by(|a, b| a.name.cmp(&b.name));
            for comp in comps {
                names.push(comp.name.clone());
            }
        }
        names
    };

    let page_ids_json = all_names
        .iter()
        .map(|n| format!("\"page_{}\"", n))
        .collect::<Vec<_>>()
        .join(", ");
    let comp_names_json = all_names
        .iter()
        .map(|n| format!("\"{}\"", n))
        .collect::<Vec<_>>()
        .join(", ");

    out.push_str("  <script lang=\"rhai\"><![CDATA[\n");
    out.push_str(&format!("    let PAGE_IDS = [{}];\n", page_ids_json));
    out.push_str(&format!(
        "    let COMPONENT_NAMES = [{}];\n\n",
        comp_names_json
    ));
    out.push_str("    fn navigate_to(component_name) {\n");
    out.push_str("        for id in PAGE_IDS {\n");
    out.push_str(
        "            set_component_property(id, \"visible\", id == \"page_\" + component_name);\n",
    );
    out.push_str("        }\n");
    out.push_str("    }\n\n");
    out.push_str("    fn on_search_change(value) {\n");
    out.push_str("        for name in COMPONENT_NAMES {\n");
    out.push_str("            let btn_id = \"nav_\" + name;\n");
    out.push_str("            let visible = value == \"\" || name.contains(value);\n");
    out.push_str("            set_component_property(btn_id, \"visible\", visible);\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("  ]]></script>\n");

    out.push_str("</nemo>\n");
    out
}

/// Write the generated storybook XML to a file.
pub fn generate_storybook_xml_to_file(path: &std::path::Path) -> anyhow::Result<()> {
    let xml = generate_storybook_xml();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, xml)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nemo_registry::{register_builtin_components, ComponentCategory, ComponentRegistry};

    #[test]
    fn test_generate_storybook_xml_is_valid_xml() {
        let xml = generate_storybook_xml();
        assert!(!xml.is_empty());
        // Must start with XML declaration
        assert!(xml.starts_with("<?xml"));
        // Must contain the nemo root element
        assert!(xml.contains("<nemo>"));
        assert!(xml.contains("</nemo>"));
    }

    #[test]
    fn test_generated_xml_contains_all_categories() {
        let xml = generate_storybook_xml();
        assert!(xml.contains("Layout"));
        assert!(xml.contains("Display"));
        assert!(xml.contains("Input"));
        assert!(xml.contains("Data"));
        assert!(xml.contains("Feedback"));
        assert!(xml.contains("Navigation"));
        assert!(xml.contains("Charts"));
    }

    #[test]
    fn test_generated_xml_has_page_for_every_component() {
        let xml = generate_storybook_xml();
        let registry = ComponentRegistry::new();
        register_builtin_components(&registry);
        let components = registry.list_components();
        assert!(!components.is_empty());
        for comp in &components {
            let page_id = format!("id=\"page_{}\"", comp.name);
            assert!(
                xml.contains(&page_id),
                "Generated XML missing page for component: {}",
                comp.name
            );
        }
    }

    #[test]
    fn test_generated_xml_has_nav_button_for_every_component() {
        let xml = generate_storybook_xml();
        let registry = ComponentRegistry::new();
        register_builtin_components(&registry);
        let components = registry.list_components();
        for comp in &components {
            let nav_id = format!("id=\"nav_{}\"", comp.name);
            assert!(
                xml.contains(&nav_id),
                "Generated XML missing nav button for component: {}",
                comp.name
            );
        }
    }

    #[test]
    fn test_generated_xml_contains_sidebar_search() {
        let xml = generate_storybook_xml();
        assert!(
            xml.contains("sidebar_search"),
            "Generated XML missing sidebar search input"
        );
    }

    #[test]
    fn test_generated_xml_has_property_rows_for_button() {
        let xml = generate_storybook_xml();
        assert!(xml.contains("page_button"), "Missing button page");
        // Button has a required "label" property — should appear as a label row
        assert!(xml.contains("label *: string"), "Missing label property row for button");
    }

    #[test]
    fn test_registry_search_components() {
        let registry = ComponentRegistry::new();
        register_builtin_components(&registry);
        let results = registry.search_components("button");
        assert!(!results.is_empty());
        assert!(results.iter().any(|c| c.name == "button"));
    }

    #[test]
    fn test_registry_list_by_category_input() {
        let registry = ComponentRegistry::new();
        register_builtin_components(&registry);
        let inputs = registry.list_by_category(ComponentCategory::Input);
        assert!(!inputs.is_empty());
        assert!(inputs.iter().any(|c| c.name == "button"));
    }

    #[test]
    fn test_generate_to_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("test_storybook.xml");
        generate_storybook_xml_to_file(&path).unwrap();
        assert!(path.exists());
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("<nemo>"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_generated_xml_has_script_section() {
        let xml = generate_storybook_xml();
        assert!(
            xml.contains("<script lang=\"rhai\">"),
            "Generated XML missing Rhai script section"
        );
        assert!(
            xml.contains("navigate_to"),
            "Script section missing navigate_to function"
        );
        assert!(
            xml.contains("on_search_change"),
            "Script section missing on_search_change function"
        );
    }

    #[test]
    fn test_generated_xml_has_app_config() {
        let xml = generate_storybook_xml();
        assert!(xml.contains("<app"), "Missing <app> element");
        assert!(xml.contains("<window"), "Missing <window> element");
        assert!(xml.contains("Nemo Storybook"), "Missing app title");
    }

    #[test]
    fn test_generated_xml_has_layout() {
        let xml = generate_storybook_xml();
        assert!(xml.contains("<layout"), "Missing <layout> element");
        assert!(xml.contains("content_area"), "Missing content_area");
        assert!(xml.contains("root_row"), "Missing root_row stack");
    }

    #[test]
    fn test_preview_element_has_required_props() {
        let xml = generate_storybook_xml();
        // button requires "label" — the preview element must include label="placeholder"
        assert!(xml.contains("page_button"), "Missing button page");
        assert!(
            xml.contains("label=\"placeholder\""),
            "Preview element missing required label prop"
        );
    }
}
