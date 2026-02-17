//! Built-in component registrations.

use crate::descriptor::{
    ActionDescriptor, ActionMetadata, ComponentCategory, ComponentDescriptor, ComponentMetadata,
    DataSourceDescriptor, DataSourceMetadata, TransformDescriptor, TransformMetadata,
};
use crate::registry::ComponentRegistry;
use nemo_config::{ConfigSchema, PropertySchema};

/// Helper: register a component with display name, description, and schema.
fn reg(
    registry: &ComponentRegistry,
    name: &str,
    category: ComponentCategory,
    display: &str,
    desc: &str,
    schema: ConfigSchema,
) {
    let mut d = ComponentDescriptor::new(name, category);
    d.metadata = ComponentMetadata {
        display_name: display.to_string(),
        description: desc.to_string(),
        ..Default::default()
    };
    d.schema = schema;
    let _ = registry.register_component(d);
}

/// Registers all built-in components.
pub fn register_builtin_components(registry: &ComponentRegistry) {
    register_layout_components(registry);
    register_basic_components(registry);
    register_input_components(registry);
    register_display_components(registry);
    register_data_components(registry);
    register_feedback_components(registry);
    register_navigation_components(registry);
    register_chart_components(registry);
}

fn register_layout_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "dock",
        ComponentCategory::Layout,
        "Dock Area",
        "A dockable layout container with panels",
        ConfigSchema::new("dock")
            .property("position", PropertySchema::string().with_default("center")),
    );

    reg(
        registry,
        "stack",
        ComponentCategory::Layout,
        "Stack",
        "Vertical or horizontal stack layout",
        ConfigSchema::new("stack")
            .property(
                "direction",
                PropertySchema::string().with_default("vertical"),
            )
            .property("spacing", PropertySchema::integer().with_default(0i64))
            .property("width", PropertySchema::integer())
            .property("height", PropertySchema::integer())
            .property("min_width", PropertySchema::integer())
            .property("min_height", PropertySchema::integer())
            .property("flex", PropertySchema::float())
            .property("padding", PropertySchema::integer())
            .property("margin", PropertySchema::integer())
            .property("scroll", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "panel",
        ComponentCategory::Layout,
        "Panel",
        "A generic container panel",
        ConfigSchema::new("panel")
            .property("title", PropertySchema::string())
            .property("width", PropertySchema::integer())
            .property("height", PropertySchema::integer())
            .property("min_width", PropertySchema::integer())
            .property("min_height", PropertySchema::integer())
            .property("flex", PropertySchema::float())
            .property("padding", PropertySchema::integer())
            .property("margin", PropertySchema::integer()),
    );

    reg(
        registry,
        "tabs",
        ComponentCategory::Layout,
        "Tabs",
        "Tabbed container",
        ConfigSchema::new("tabs"),
    );
}

fn register_basic_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "accordion",
        ComponentCategory::Display,
        "Accordion",
        "A collapsible accordion with multiple items",
        ConfigSchema::new("accordion")
            .property("items", PropertySchema::any())
            .property("multiple", PropertySchema::boolean().with_default(false))
            .property("bordered", PropertySchema::boolean().with_default(true)),
    );

    reg(
        registry,
        "alert",
        ComponentCategory::Display,
        "Alert",
        "A status alert message",
        ConfigSchema::new("alert")
            .property("message", PropertySchema::string())
            .property("title", PropertySchema::string())
            .property("variant", PropertySchema::string().with_default("info"))
            .require("message"),
    );

    reg(
        registry,
        "avatar",
        ComponentCategory::Display,
        "Avatar",
        "A user avatar showing initials",
        ConfigSchema::new("avatar").property("name", PropertySchema::string()),
    );

    reg(
        registry,
        "badge",
        ComponentCategory::Display,
        "Badge",
        "A badge indicator with count or dot",
        ConfigSchema::new("badge")
            .property("count", PropertySchema::integer())
            .property("dot", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "collapsible",
        ComponentCategory::Display,
        "Collapsible",
        "A collapsible content section",
        ConfigSchema::new("collapsible")
            .property("title", PropertySchema::string().with_default("Details"))
            .property("open", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "dropdown_button",
        ComponentCategory::Display,
        "Dropdown Button",
        "A button with a dropdown menu indicator",
        ConfigSchema::new("dropdown_button")
            .property("label", PropertySchema::string().with_default("Action"))
            .property("variant", PropertySchema::string()),
    );

    reg(
        registry,
        "spinner",
        ComponentCategory::Display,
        "Spinner",
        "A loading spinner indicator",
        ConfigSchema::new("spinner").property("size", PropertySchema::string().with_default("md")),
    );

    reg(
        registry,
        "tag",
        ComponentCategory::Display,
        "Tag",
        "A small label tag",
        ConfigSchema::new("tag")
            .property("label", PropertySchema::string().with_default("Tag"))
            .property(
                "variant",
                PropertySchema::string().with_default("secondary"),
            )
            .property("outline", PropertySchema::boolean().with_default(false)),
    );
}

fn register_input_components(registry: &ComponentRegistry) {
    // Button has custom tags, so it uses the manual pattern.
    let mut button = ComponentDescriptor::new("button", ComponentCategory::Input);
    button.metadata = ComponentMetadata {
        display_name: "Button".to_string(),
        description: "A clickable button that triggers actions".to_string(),
        ..Default::default()
    };
    button.tags = vec!["interactive".to_string(), "clickable".to_string()];
    button.schema = ConfigSchema::new("button")
        .property("label", PropertySchema::string())
        .property("variant", PropertySchema::string().with_default("primary"))
        .property("disabled", PropertySchema::boolean().with_default(false))
        .property("width", PropertySchema::integer())
        .property("height", PropertySchema::integer())
        .property("min_width", PropertySchema::integer())
        .property("min_height", PropertySchema::integer())
        .property("flex", PropertySchema::float())
        .property("padding", PropertySchema::integer())
        .property("margin", PropertySchema::integer())
        .require("label");
    let _ = registry.register_component(button);

    reg(
        registry,
        "input",
        ComponentCategory::Input,
        "Text Input",
        "A text input field",
        ConfigSchema::new("input")
            .property("placeholder", PropertySchema::string())
            .property("value", PropertySchema::string())
            .property("disabled", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "textarea",
        ComponentCategory::Input,
        "Textarea",
        "A multi-line text input area",
        ConfigSchema::new("textarea")
            .property("placeholder", PropertySchema::string())
            .property("default_value", PropertySchema::string())
            .property("rows", PropertySchema::integer())
            .property("auto_grow_min", PropertySchema::integer())
            .property("auto_grow_max", PropertySchema::integer())
            .property("disabled", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "code_editor",
        ComponentCategory::Input,
        "Code Editor",
        "A code editor with syntax highlighting and line numbers",
        ConfigSchema::new("code_editor")
            .property("language", PropertySchema::string().with_default("plain"))
            .property("line_number", PropertySchema::boolean().with_default(true))
            .property("searchable", PropertySchema::boolean().with_default(true))
            .property("default_value", PropertySchema::string())
            .property("multi_line", PropertySchema::boolean().with_default(true))
            .property("tab_size", PropertySchema::integer().with_default(4i64))
            .property("hard_tabs", PropertySchema::boolean().with_default(false))
            .property("disabled", PropertySchema::boolean().with_default(false))
            .property("rows", PropertySchema::integer()),
    );

    reg(
        registry,
        "text_editor",
        ComponentCategory::Input,
        "Text Editor",
        "A rich text editor with formatting toolbar",
        ConfigSchema::new("text_editor")
            .property("placeholder", PropertySchema::string())
            .property("default_value", PropertySchema::string())
            .property("rows", PropertySchema::integer())
            .property("disabled", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "checkbox",
        ComponentCategory::Input,
        "Checkbox",
        "A checkbox input",
        ConfigSchema::new("checkbox")
            .property("label", PropertySchema::string())
            .property("checked", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "select",
        ComponentCategory::Input,
        "Select",
        "A dropdown select input",
        ConfigSchema::new("select")
            .property("options", PropertySchema::array(PropertySchema::string()))
            .property("value", PropertySchema::string()),
    );

    reg(
        registry,
        "radio",
        ComponentCategory::Input,
        "Radio",
        "A radio button group",
        ConfigSchema::new("radio")
            .property("options", PropertySchema::array(PropertySchema::string()))
            .property("value", PropertySchema::string())
            .property(
                "direction",
                PropertySchema::string().with_default("vertical"),
            ),
    );

    reg(
        registry,
        "slider",
        ComponentCategory::Input,
        "Slider",
        "A range slider input",
        ConfigSchema::new("slider")
            .property("min", PropertySchema::float().with_default(0.0))
            .property("max", PropertySchema::float().with_default(100.0))
            .property("step", PropertySchema::float().with_default(1.0))
            .property("value", PropertySchema::float().with_default(0.0)),
    );

    reg(
        registry,
        "switch",
        ComponentCategory::Input,
        "Switch",
        "A toggle switch",
        ConfigSchema::new("switch")
            .property("checked", PropertySchema::boolean().with_default(false))
            .property("label", PropertySchema::string())
            .property("disabled", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "toggle",
        ComponentCategory::Input,
        "Toggle",
        "A toggle button",
        ConfigSchema::new("toggle")
            .property("checked", PropertySchema::boolean().with_default(false))
            .property("label", PropertySchema::string())
            .property("icon", PropertySchema::string())
            .property("disabled", PropertySchema::boolean().with_default(false)),
    );
}

fn register_display_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "label",
        ComponentCategory::Display,
        "Label",
        "A text label",
        ConfigSchema::new("label")
            .property("text", PropertySchema::string())
            .property("size", PropertySchema::string().with_default("md"))
            .require("text"),
    );

    reg(
        registry,
        "icon",
        ComponentCategory::Display,
        "Icon",
        "An icon display",
        ConfigSchema::new("icon")
            .property("name", PropertySchema::string())
            .property("size", PropertySchema::integer().with_default(16i64))
            .require("name"),
    );

    reg(
        registry,
        "image",
        ComponentCategory::Display,
        "Image",
        "An image display",
        ConfigSchema::new("image")
            .property("src", PropertySchema::string())
            .property("alt", PropertySchema::string()),
    );

    reg(
        registry,
        "text",
        ComponentCategory::Display,
        "Text",
        "A text content block",
        ConfigSchema::new("text")
            .property("content", PropertySchema::string())
            .require("content"),
    );

    reg(
        registry,
        "progress",
        ComponentCategory::Display,
        "Progress",
        "A progress indicator",
        ConfigSchema::new("progress")
            .property("value", PropertySchema::float())
            .property("max", PropertySchema::float().with_default(100.0)),
    );
}

fn register_data_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "table",
        ComponentCategory::Data,
        "Table",
        "A data table",
        ConfigSchema::new("table")
            .property("data", PropertySchema::any())
            .property(
                "columns",
                PropertySchema::array(PropertySchema::object(
                    ConfigSchema::new("column")
                        .property("key", PropertySchema::string())
                        .property("label", PropertySchema::string())
                        .property("width", PropertySchema::integer()),
                )),
            )
            .property("stripe", PropertySchema::boolean().with_default(false))
            .property("bordered", PropertySchema::boolean().with_default(true)),
    );

    reg(
        registry,
        "list",
        ComponentCategory::Data,
        "List",
        "A data list",
        ConfigSchema::new("list").property("items", PropertySchema::array(PropertySchema::any())),
    );

    reg(
        registry,
        "tree",
        ComponentCategory::Data,
        "Tree",
        "A tree view",
        ConfigSchema::new("tree").property("items", PropertySchema::array(PropertySchema::any())),
    );
}

fn register_feedback_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "modal",
        ComponentCategory::Feedback,
        "Modal",
        "A modal dialog",
        ConfigSchema::new("modal")
            .property("title", PropertySchema::string())
            .property("open", PropertySchema::boolean().with_default(false)),
    );

    reg(
        registry,
        "notification",
        ComponentCategory::Feedback,
        "Notification",
        "A notification toast",
        ConfigSchema::new("notification")
            .property("message", PropertySchema::string())
            .property("kind", PropertySchema::string().with_default("info"))
            .require("message"),
    );

    reg(
        registry,
        "tooltip",
        ComponentCategory::Feedback,
        "Tooltip",
        "A tooltip popup",
        ConfigSchema::new("tooltip")
            .property("content", PropertySchema::string())
            .require("content"),
    );
}

fn register_navigation_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "sidenav_bar",
        ComponentCategory::Navigation,
        "Sidenav Bar",
        "A vertical navigation sidebar with collapsible icon+label items",
        ConfigSchema::new("sidenav_bar")
            .property("collapsed", PropertySchema::boolean().with_default(false))
            .property("width", PropertySchema::integer().with_default(200i64)),
    );

    reg(
        registry,
        "sidenav_bar_item",
        ComponentCategory::Navigation,
        "Sidenav Bar Item",
        "A navigation item with an icon and label for use inside a SidenavBar",
        ConfigSchema::new("sidenav_bar_item")
            .property("icon", PropertySchema::string().with_default("info"))
            .property("label", PropertySchema::string()),
    );
}

fn register_chart_components(registry: &ComponentRegistry) {
    reg(
        registry,
        "line_chart",
        ComponentCategory::Charts,
        "Line Chart",
        "A line chart visualization",
        ConfigSchema::new("line_chart")
            .property("x_field", PropertySchema::string())
            .property("y_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("dot", PropertySchema::boolean())
            .property("linear", PropertySchema::boolean())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_field"),
    );

    reg(
        registry,
        "bar_chart",
        ComponentCategory::Charts,
        "Bar Chart",
        "A bar chart visualization",
        ConfigSchema::new("bar_chart")
            .property("x_field", PropertySchema::string())
            .property("y_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("show_label", PropertySchema::boolean())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_field"),
    );

    reg(
        registry,
        "area_chart",
        ComponentCategory::Charts,
        "Area Chart",
        "A stacked area chart visualization",
        ConfigSchema::new("area_chart")
            .property("x_field", PropertySchema::string())
            .property("y_fields", PropertySchema::array(PropertySchema::string()))
            .property("data", PropertySchema::any())
            .property("fill_opacity", PropertySchema::float())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_fields"),
    );

    reg(
        registry,
        "pie_chart",
        ComponentCategory::Charts,
        "Pie Chart",
        "A pie or donut chart visualization",
        ConfigSchema::new("pie_chart")
            .property("value_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("outer_radius", PropertySchema::float())
            .property("inner_radius", PropertySchema::float())
            .require("value_field"),
    );

    reg(
        registry,
        "candlestick_chart",
        ComponentCategory::Charts,
        "Candlestick Chart",
        "An OHLC candlestick chart for financial data",
        ConfigSchema::new("candlestick_chart")
            .property("x_field", PropertySchema::string())
            .property("open_field", PropertySchema::string())
            .property("high_field", PropertySchema::string())
            .property("low_field", PropertySchema::string())
            .property("close_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("open_field")
            .require("high_field")
            .require("low_field")
            .require("close_field"),
    );

    reg(
        registry,
        "column_chart",
        ComponentCategory::Charts,
        "Column Chart",
        "A vertical column chart (alias for bar chart)",
        ConfigSchema::new("column_chart")
            .property("x_field", PropertySchema::string())
            .property("y_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("show_label", PropertySchema::boolean())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_field"),
    );

    reg(
        registry,
        "stacked_column_chart",
        ComponentCategory::Charts,
        "Stacked Column Chart",
        "A stacked vertical column chart with multiple series",
        ConfigSchema::new("stacked_column_chart")
            .property("x_field", PropertySchema::string())
            .property("y_fields", PropertySchema::array(PropertySchema::string()))
            .property("data", PropertySchema::any())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_fields"),
    );

    reg(
        registry,
        "clustered_column_chart",
        ComponentCategory::Charts,
        "Clustered Column Chart",
        "A grouped vertical column chart with side-by-side series",
        ConfigSchema::new("clustered_column_chart")
            .property("x_field", PropertySchema::string())
            .property("y_fields", PropertySchema::array(PropertySchema::string()))
            .property("data", PropertySchema::any())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_fields"),
    );

    reg(
        registry,
        "stacked_bar_chart",
        ComponentCategory::Charts,
        "Stacked Bar Chart",
        "A stacked horizontal bar chart with multiple series",
        ConfigSchema::new("stacked_bar_chart")
            .property("y_field", PropertySchema::string())
            .property("x_fields", PropertySchema::array(PropertySchema::string()))
            .property("data", PropertySchema::any())
            .property("tick_margin", PropertySchema::integer())
            .require("y_field")
            .require("x_fields"),
    );

    reg(
        registry,
        "clustered_bar_chart",
        ComponentCategory::Charts,
        "Clustered Bar Chart",
        "A grouped horizontal bar chart with side-by-side series",
        ConfigSchema::new("clustered_bar_chart")
            .property("y_field", PropertySchema::string())
            .property("x_fields", PropertySchema::array(PropertySchema::string()))
            .property("data", PropertySchema::any())
            .property("tick_margin", PropertySchema::integer())
            .require("y_field")
            .require("x_fields"),
    );

    reg(
        registry,
        "scatter_chart",
        ComponentCategory::Charts,
        "Scatter Chart",
        "A scatter plot with data points on a numeric plane",
        ConfigSchema::new("scatter_chart")
            .property("x_field", PropertySchema::string())
            .property("y_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("dot_size", PropertySchema::float())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_field"),
    );

    reg(
        registry,
        "bubble_chart",
        ComponentCategory::Charts,
        "Bubble Chart",
        "A scatter chart with variable-size bubbles",
        ConfigSchema::new("bubble_chart")
            .property("x_field", PropertySchema::string())
            .property("y_field", PropertySchema::string())
            .property("size_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("min_radius", PropertySchema::float())
            .property("max_radius", PropertySchema::float())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_field")
            .require("size_field"),
    );

    reg(
        registry,
        "heatmap_chart",
        ComponentCategory::Charts,
        "Heatmap Chart",
        "A grid of coloured cells representing values across two categories",
        ConfigSchema::new("heatmap_chart")
            .property("x_field", PropertySchema::string())
            .property("y_field", PropertySchema::string())
            .property("value_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .property("tick_margin", PropertySchema::integer())
            .require("x_field")
            .require("y_field")
            .require("value_field"),
    );

    reg(
        registry,
        "radar_chart",
        ComponentCategory::Charts,
        "Radar Chart",
        "A radar (spider/web) chart with polygonal data series",
        ConfigSchema::new("radar_chart")
            .property(
                "categories",
                PropertySchema::array(PropertySchema::string()),
            )
            .property("y_fields", PropertySchema::array(PropertySchema::string()))
            .property("data", PropertySchema::any())
            .property("max_value", PropertySchema::float())
            .require("categories")
            .require("y_fields"),
    );

    reg(
        registry,
        "pyramid_chart",
        ComponentCategory::Charts,
        "Pyramid Chart",
        "A pyramid of centred horizontal bars sorted by value",
        ConfigSchema::new("pyramid_chart")
            .property("label_field", PropertySchema::string())
            .property("value_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .require("label_field")
            .require("value_field"),
    );

    reg(
        registry,
        "funnel_chart",
        ComponentCategory::Charts,
        "Funnel Chart",
        "A funnel chart with narrowing trapezoid segments",
        ConfigSchema::new("funnel_chart")
            .property("label_field", PropertySchema::string())
            .property("value_field", PropertySchema::string())
            .property("data", PropertySchema::any())
            .require("label_field")
            .require("value_field"),
    );
}

/// Registers all built-in data sources.
pub fn register_builtin_data_sources(registry: &ComponentRegistry) {
    let mut http = DataSourceDescriptor::new("http");
    http.metadata = DataSourceMetadata {
        display_name: "HTTP".into(),
        description: "Fetch data from HTTP endpoints".into(),
        supports_polling: true,
        supports_manual_refresh: true,
        ..Default::default()
    };
    http.schema = ConfigSchema::new("http")
        .property("url", PropertySchema::string())
        .property("method", PropertySchema::string().with_default("GET"))
        .property("interval", PropertySchema::integer().with_default(0i64))
        .require("url");
    let _ = registry.register_data_source(http);

    let mut ws = DataSourceDescriptor::new("websocket");
    ws.metadata = DataSourceMetadata {
        display_name: "WebSocket".into(),
        description: "Stream data from WebSocket connections".into(),
        supports_streaming: true,
        supports_manual_refresh: true,
        ..Default::default()
    };
    ws.schema = ConfigSchema::new("websocket")
        .property("url", PropertySchema::string())
        .require("url");
    let _ = registry.register_data_source(ws);

    let mut timer = DataSourceDescriptor::new("timer");
    timer.metadata = DataSourceMetadata {
        display_name: "Timer".into(),
        description: "Generate events at intervals".into(),
        supports_polling: true,
        supports_manual_refresh: true,
        ..Default::default()
    };
    timer.schema = ConfigSchema::new("timer")
        .property("interval", PropertySchema::integer())
        .require("interval");
    let _ = registry.register_data_source(timer);

    let mut file = DataSourceDescriptor::new("file");
    file.metadata = DataSourceMetadata {
        display_name: "File".into(),
        description: "Read data from files".into(),
        supports_polling: true,
        supports_manual_refresh: true,
        ..Default::default()
    };
    file.schema = ConfigSchema::new("file")
        .property("path", PropertySchema::string())
        .property("watch", PropertySchema::boolean().with_default(false))
        .require("path");
    let _ = registry.register_data_source(file);
}

/// Registers all built-in transforms.
pub fn register_builtin_transforms(registry: &ComponentRegistry) {
    let transforms: &[(&str, &str, &str, TransformMetadata, ConfigSchema)] = &[
        (
            "map",
            "Map",
            "Transform each item",
            TransformMetadata {
                display_name: "Map".into(),
                description: "Transform each item".into(),
                preserves_order: true,
                ..Default::default()
            },
            ConfigSchema::new("map")
                .property("expression", PropertySchema::string())
                .require("expression"),
        ),
        (
            "filter",
            "Filter",
            "Filter items by condition",
            TransformMetadata {
                display_name: "Filter".into(),
                description: "Filter items by condition".into(),
                preserves_order: true,
                may_filter: true,
                ..Default::default()
            },
            ConfigSchema::new("filter")
                .property("condition", PropertySchema::string())
                .require("condition"),
        ),
        (
            "select",
            "Select",
            "Select specific fields",
            TransformMetadata {
                display_name: "Select".into(),
                description: "Select specific fields".into(),
                preserves_order: true,
                ..Default::default()
            },
            ConfigSchema::new("select")
                .property("fields", PropertySchema::array(PropertySchema::string()))
                .require("fields"),
        ),
        (
            "sort",
            "Sort",
            "Sort items",
            TransformMetadata {
                display_name: "Sort".into(),
                description: "Sort items".into(),
                ..Default::default()
            },
            ConfigSchema::new("sort")
                .property("by", PropertySchema::string())
                .property("direction", PropertySchema::string().with_default("asc"))
                .require("by"),
        ),
        (
            "aggregate",
            "Aggregate",
            "Aggregate items",
            TransformMetadata {
                display_name: "Aggregate".into(),
                description: "Aggregate items".into(),
                may_filter: true,
                stateful: true,
                ..Default::default()
            },
            ConfigSchema::new("aggregate")
                .property("group_by", PropertySchema::string())
                .property("operation", PropertySchema::string())
                .require("operation"),
        ),
    ];

    for (name, _display, _desc, metadata, schema) in transforms {
        let mut d = TransformDescriptor::new(*name);
        d.metadata = metadata.clone();
        d.schema = schema.clone();
        let _ = registry.register_transform(d);
    }
}

/// Registers all built-in actions.
pub fn register_builtin_actions(registry: &ComponentRegistry) {
    let actions: &[(&str, ActionMetadata, ConfigSchema)] = &[
        (
            "notification",
            ActionMetadata {
                display_name: "Show Notification".into(),
                description: "Display a notification to the user".into(),
                idempotent: true,
                ..Default::default()
            },
            ConfigSchema::new("notification")
                .property("message", PropertySchema::string())
                .property("type", PropertySchema::string().with_default("info"))
                .require("message"),
        ),
        (
            "navigate",
            ActionMetadata {
                display_name: "Navigate".into(),
                description: "Navigate to a different view".into(),
                idempotent: true,
                ..Default::default()
            },
            ConfigSchema::new("navigate")
                .property("target", PropertySchema::string())
                .require("target"),
        ),
        (
            "refresh",
            ActionMetadata {
                display_name: "Refresh".into(),
                description: "Refresh data or UI".into(),
                async_execution: true,
                idempotent: true,
                ..Default::default()
            },
            ConfigSchema::new("refresh").property("target", PropertySchema::string()),
        ),
        (
            "http_request",
            ActionMetadata {
                display_name: "HTTP Request".into(),
                description: "Make an HTTP request".into(),
                async_execution: true,
                may_fail: true,
                ..Default::default()
            },
            ConfigSchema::new("http_request")
                .property("url", PropertySchema::string())
                .property("method", PropertySchema::string().with_default("POST"))
                .property("body", PropertySchema::any())
                .require("url"),
        ),
        (
            "set_data",
            ActionMetadata {
                display_name: "Set Data".into(),
                description: "Set a data value".into(),
                idempotent: true,
                ..Default::default()
            },
            ConfigSchema::new("set_data")
                .property("target", PropertySchema::string())
                .property("value", PropertySchema::any())
                .require("target")
                .require("value"),
        ),
    ];

    for (name, metadata, schema) in actions {
        let mut d = ActionDescriptor::new(*name);
        d.metadata = metadata.clone();
        d.schema = schema.clone();
        let _ = registry.register_action(d);
    }
}

/// Registers all built-ins.
pub fn register_all_builtins(registry: &ComponentRegistry) {
    register_builtin_components(registry);
    register_builtin_data_sources(registry);
    register_builtin_transforms(registry);
    register_builtin_actions(registry);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_all_builtins() {
        let registry = ComponentRegistry::new();
        register_all_builtins(&registry);

        // Verify components
        assert!(registry.has_component("button"));
        assert!(registry.has_component("label"));
        assert!(registry.has_component("table"));
        assert!(registry.has_component("accordion"));
        assert!(registry.has_component("alert"));
        assert!(registry.has_component("avatar"));
        assert!(registry.has_component("badge"));
        assert!(registry.has_component("collapsible"));
        assert!(registry.has_component("dropdown_button"));
        assert!(registry.has_component("spinner"));
        assert!(registry.has_component("tag"));
        assert!(registry.has_component("radio"));
        assert!(registry.has_component("slider"));
        assert!(registry.has_component("switch"));
        assert!(registry.has_component("toggle"));
        assert!(registry.has_component("line_chart"));
        assert!(registry.has_component("bar_chart"));
        assert!(registry.has_component("area_chart"));
        assert!(registry.has_component("pie_chart"));
        assert!(registry.has_component("candlestick_chart"));
        assert!(registry.has_component("column_chart"));
        assert!(registry.has_component("stacked_column_chart"));
        assert!(registry.has_component("clustered_column_chart"));
        assert!(registry.has_component("stacked_bar_chart"));
        assert!(registry.has_component("clustered_bar_chart"));
        assert!(registry.has_component("scatter_chart"));
        assert!(registry.has_component("bubble_chart"));
        assert!(registry.has_component("heatmap_chart"));
        assert!(registry.has_component("radar_chart"));
        assert!(registry.has_component("pyramid_chart"));
        assert!(registry.has_component("funnel_chart"));
        assert!(registry.has_component("textarea"));
        assert!(registry.has_component("code_editor"));
        assert!(registry.has_component("text_editor"));
        assert!(registry.has_component("sidenav_bar"));
        assert!(registry.has_component("sidenav_bar_item"));

        // Verify data sources
        assert!(registry.has_data_source("http"));
        assert!(registry.has_data_source("websocket"));

        // Verify transforms
        assert!(registry.has_transform("map"));
        assert!(registry.has_transform("filter"));

        // Verify actions
        assert!(registry.has_action("notification"));
        assert!(registry.has_action("navigate"));
    }
}
