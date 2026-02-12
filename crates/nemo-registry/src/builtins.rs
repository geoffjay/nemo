//! Built-in component registrations.

use crate::descriptor::{
    ActionDescriptor, ActionMetadata, ComponentCategory, ComponentDescriptor, ComponentMetadata,
    DataSourceDescriptor, DataSourceMetadata, TransformDescriptor, TransformMetadata,
};
use crate::registry::ComponentRegistry;
use nemo_config::{ConfigSchema, PropertySchema};

/// Registers all built-in components.
pub fn register_builtin_components(registry: &ComponentRegistry) {
    // Layout components
    register_layout_components(registry);

    // Basic components
    register_basic_components(registry);

    // Input components
    register_input_components(registry);

    // Display components
    register_display_components(registry);

    // Data components
    register_data_components(registry);

    // Feedback components
    register_feedback_components(registry);

    // Chart components
    register_chart_components(registry);
}

fn register_layout_components(registry: &ComponentRegistry) {
    // Dock Area
    let mut dock = ComponentDescriptor::new("dock", ComponentCategory::Layout);
    dock.metadata = ComponentMetadata {
        display_name: "Dock Area".to_string(),
        description: "A dockable layout container with panels".to_string(),
        ..Default::default()
    };
    dock.schema = ConfigSchema::new("dock")
        .property("position", PropertySchema::string().with_default("center"));
    let _ = registry.register_component(dock);

    // Stack
    let mut stack = ComponentDescriptor::new("stack", ComponentCategory::Layout);
    stack.metadata = ComponentMetadata {
        display_name: "Stack".to_string(),
        description: "Vertical or horizontal stack layout".to_string(),
        ..Default::default()
    };
    stack.schema = ConfigSchema::new("stack")
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
        .property("scroll", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(stack);

    // Panel - a generic container
    let mut panel = ComponentDescriptor::new("panel", ComponentCategory::Layout);
    panel.metadata = ComponentMetadata {
        display_name: "Panel".to_string(),
        description: "A generic container panel".to_string(),
        ..Default::default()
    };
    panel.schema = ConfigSchema::new("panel")
        .property("title", PropertySchema::string())
        .property("width", PropertySchema::integer())
        .property("height", PropertySchema::integer())
        .property("min_width", PropertySchema::integer())
        .property("min_height", PropertySchema::integer())
        .property("flex", PropertySchema::float())
        .property("padding", PropertySchema::integer())
        .property("margin", PropertySchema::integer());
    let _ = registry.register_component(panel);

    // Tabs
    let mut tabs = ComponentDescriptor::new("tabs", ComponentCategory::Layout);
    tabs.metadata = ComponentMetadata {
        display_name: "Tabs".to_string(),
        description: "Tabbed container".to_string(),
        ..Default::default()
    };
    let _ = registry.register_component(tabs);
}

fn register_basic_components(registry: &ComponentRegistry) {
    // Accordion
    let mut accordion = ComponentDescriptor::new("accordion", ComponentCategory::Display);
    accordion.metadata = ComponentMetadata {
        display_name: "Accordion".to_string(),
        description: "A collapsible accordion with multiple items".to_string(),
        ..Default::default()
    };
    accordion.schema = ConfigSchema::new("accordion")
        .property("items", PropertySchema::any())
        .property("multiple", PropertySchema::boolean().with_default(false))
        .property("bordered", PropertySchema::boolean().with_default(true));
    let _ = registry.register_component(accordion);

    // Alert
    let mut alert = ComponentDescriptor::new("alert", ComponentCategory::Display);
    alert.metadata = ComponentMetadata {
        display_name: "Alert".to_string(),
        description: "A status alert message".to_string(),
        ..Default::default()
    };
    alert.schema = ConfigSchema::new("alert")
        .property("message", PropertySchema::string())
        .property("title", PropertySchema::string())
        .property(
            "variant",
            PropertySchema::string().with_default("info"),
        )
        .require("message");
    let _ = registry.register_component(alert);

    // Avatar
    let mut avatar = ComponentDescriptor::new("avatar", ComponentCategory::Display);
    avatar.metadata = ComponentMetadata {
        display_name: "Avatar".to_string(),
        description: "A user avatar showing initials".to_string(),
        ..Default::default()
    };
    avatar.schema = ConfigSchema::new("avatar")
        .property("name", PropertySchema::string());
    let _ = registry.register_component(avatar);

    // Badge
    let mut badge = ComponentDescriptor::new("badge", ComponentCategory::Display);
    badge.metadata = ComponentMetadata {
        display_name: "Badge".to_string(),
        description: "A badge indicator with count or dot".to_string(),
        ..Default::default()
    };
    badge.schema = ConfigSchema::new("badge")
        .property("count", PropertySchema::integer())
        .property("dot", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(badge);

    // Collapsible
    let mut collapsible = ComponentDescriptor::new("collapsible", ComponentCategory::Display);
    collapsible.metadata = ComponentMetadata {
        display_name: "Collapsible".to_string(),
        description: "A collapsible content section".to_string(),
        ..Default::default()
    };
    collapsible.schema = ConfigSchema::new("collapsible")
        .property("title", PropertySchema::string().with_default("Details"))
        .property("open", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(collapsible);

    // Dropdown Button
    let mut dropdown_button = ComponentDescriptor::new("dropdown_button", ComponentCategory::Display);
    dropdown_button.metadata = ComponentMetadata {
        display_name: "Dropdown Button".to_string(),
        description: "A button with a dropdown menu indicator".to_string(),
        ..Default::default()
    };
    dropdown_button.schema = ConfigSchema::new("dropdown_button")
        .property("label", PropertySchema::string().with_default("Action"))
        .property("variant", PropertySchema::string());
    let _ = registry.register_component(dropdown_button);

    // Spinner
    let mut spinner = ComponentDescriptor::new("spinner", ComponentCategory::Display);
    spinner.metadata = ComponentMetadata {
        display_name: "Spinner".to_string(),
        description: "A loading spinner indicator".to_string(),
        ..Default::default()
    };
    spinner.schema = ConfigSchema::new("spinner")
        .property("size", PropertySchema::string().with_default("md"));
    let _ = registry.register_component(spinner);

    // Tag
    let mut tag = ComponentDescriptor::new("tag", ComponentCategory::Display);
    tag.metadata = ComponentMetadata {
        display_name: "Tag".to_string(),
        description: "A small label tag".to_string(),
        ..Default::default()
    };
    tag.schema = ConfigSchema::new("tag")
        .property("label", PropertySchema::string().with_default("Tag"))
        .property("variant", PropertySchema::string().with_default("secondary"))
        .property("outline", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(tag);
}

fn register_input_components(registry: &ComponentRegistry) {
    // Button
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

    // Text Input
    let mut input = ComponentDescriptor::new("input", ComponentCategory::Input);
    input.metadata = ComponentMetadata {
        display_name: "Text Input".to_string(),
        description: "A text input field".to_string(),
        ..Default::default()
    };
    input.schema = ConfigSchema::new("input")
        .property("placeholder", PropertySchema::string())
        .property("value", PropertySchema::string())
        .property("disabled", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(input);

    // Checkbox
    let mut checkbox = ComponentDescriptor::new("checkbox", ComponentCategory::Input);
    checkbox.metadata = ComponentMetadata {
        display_name: "Checkbox".to_string(),
        description: "A checkbox input".to_string(),
        ..Default::default()
    };
    checkbox.schema = ConfigSchema::new("checkbox")
        .property("label", PropertySchema::string())
        .property("checked", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(checkbox);

    // Select
    let mut select = ComponentDescriptor::new("select", ComponentCategory::Input);
    select.metadata = ComponentMetadata {
        display_name: "Select".to_string(),
        description: "A dropdown select input".to_string(),
        ..Default::default()
    };
    select.schema = ConfigSchema::new("select")
        .property("options", PropertySchema::array(PropertySchema::string()))
        .property("value", PropertySchema::string());
    let _ = registry.register_component(select);

    // Radio
    let mut radio = ComponentDescriptor::new("radio", ComponentCategory::Input);
    radio.metadata = ComponentMetadata {
        display_name: "Radio".to_string(),
        description: "A radio button group".to_string(),
        ..Default::default()
    };
    radio.schema = ConfigSchema::new("radio")
        .property("options", PropertySchema::array(PropertySchema::string()))
        .property("value", PropertySchema::string())
        .property("direction", PropertySchema::string().with_default("vertical"));
    let _ = registry.register_component(radio);

    // Slider
    let mut slider = ComponentDescriptor::new("slider", ComponentCategory::Input);
    slider.metadata = ComponentMetadata {
        display_name: "Slider".to_string(),
        description: "A range slider input".to_string(),
        ..Default::default()
    };
    slider.schema = ConfigSchema::new("slider")
        .property("min", PropertySchema::float().with_default(0.0))
        .property("max", PropertySchema::float().with_default(100.0))
        .property("step", PropertySchema::float().with_default(1.0))
        .property("value", PropertySchema::float().with_default(0.0));
    let _ = registry.register_component(slider);

    // Switch
    let mut switch = ComponentDescriptor::new("switch", ComponentCategory::Input);
    switch.metadata = ComponentMetadata {
        display_name: "Switch".to_string(),
        description: "A toggle switch".to_string(),
        ..Default::default()
    };
    switch.schema = ConfigSchema::new("switch")
        .property("checked", PropertySchema::boolean().with_default(false))
        .property("label", PropertySchema::string())
        .property("disabled", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(switch);

    // Toggle
    let mut toggle = ComponentDescriptor::new("toggle", ComponentCategory::Input);
    toggle.metadata = ComponentMetadata {
        display_name: "Toggle".to_string(),
        description: "A toggle button".to_string(),
        ..Default::default()
    };
    toggle.schema = ConfigSchema::new("toggle")
        .property("checked", PropertySchema::boolean().with_default(false))
        .property("label", PropertySchema::string())
        .property("icon", PropertySchema::string())
        .property("disabled", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(toggle);
}

fn register_display_components(registry: &ComponentRegistry) {
    // Label
    let mut label = ComponentDescriptor::new("label", ComponentCategory::Display);
    label.metadata = ComponentMetadata {
        display_name: "Label".to_string(),
        description: "A text label".to_string(),
        ..Default::default()
    };
    label.schema = ConfigSchema::new("label")
        .property("text", PropertySchema::string())
        .property("size", PropertySchema::string().with_default("md"))
        .require("text");
    let _ = registry.register_component(label);

    // Icon
    let mut icon = ComponentDescriptor::new("icon", ComponentCategory::Display);
    icon.metadata = ComponentMetadata {
        display_name: "Icon".to_string(),
        description: "An icon display".to_string(),
        ..Default::default()
    };
    icon.schema = ConfigSchema::new("icon")
        .property("name", PropertySchema::string())
        .property("size", PropertySchema::integer().with_default(16i64))
        .require("name");
    let _ = registry.register_component(icon);

    // Image
    let mut image = ComponentDescriptor::new("image", ComponentCategory::Display);
    image.metadata = ComponentMetadata {
        display_name: "Image".to_string(),
        description: "An image display".to_string(),
        ..Default::default()
    };
    image.schema = ConfigSchema::new("image")
        .property("src", PropertySchema::string())
        .property("alt", PropertySchema::string());
    let _ = registry.register_component(image);

    // Text
    let mut text = ComponentDescriptor::new("text", ComponentCategory::Display);
    text.metadata = ComponentMetadata {
        display_name: "Text".to_string(),
        description: "A text content block".to_string(),
        ..Default::default()
    };
    text.schema = ConfigSchema::new("text")
        .property("content", PropertySchema::string())
        .require("content");
    let _ = registry.register_component(text);

    // Progress
    let mut progress = ComponentDescriptor::new("progress", ComponentCategory::Display);
    progress.metadata = ComponentMetadata {
        display_name: "Progress".to_string(),
        description: "A progress indicator".to_string(),
        ..Default::default()
    };
    progress.schema = ConfigSchema::new("progress")
        .property("value", PropertySchema::float())
        .property("max", PropertySchema::float().with_default(100.0));
    let _ = registry.register_component(progress);
}

fn register_data_components(registry: &ComponentRegistry) {
    // Table
    let mut table = ComponentDescriptor::new("table", ComponentCategory::Data);
    table.metadata = ComponentMetadata {
        display_name: "Table".to_string(),
        description: "A data table".to_string(),
        ..Default::default()
    };
    table.schema = ConfigSchema::new("table")
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
        .property("bordered", PropertySchema::boolean().with_default(true));
    let _ = registry.register_component(table);

    // List
    let mut list = ComponentDescriptor::new("list", ComponentCategory::Data);
    list.metadata = ComponentMetadata {
        display_name: "List".to_string(),
        description: "A data list".to_string(),
        ..Default::default()
    };
    list.schema =
        ConfigSchema::new("list").property("items", PropertySchema::array(PropertySchema::any()));
    let _ = registry.register_component(list);

    // Tree
    let mut tree = ComponentDescriptor::new("tree", ComponentCategory::Data);
    tree.metadata = ComponentMetadata {
        display_name: "Tree".to_string(),
        description: "A tree view".to_string(),
        ..Default::default()
    };
    tree.schema = ConfigSchema::new("tree").property("items", PropertySchema::array(PropertySchema::any()));
    let _ = registry.register_component(tree);
}

fn register_feedback_components(registry: &ComponentRegistry) {
    // Modal
    let mut modal = ComponentDescriptor::new("modal", ComponentCategory::Feedback);
    modal.metadata = ComponentMetadata {
        display_name: "Modal".to_string(),
        description: "A modal dialog".to_string(),
        ..Default::default()
    };
    modal.schema = ConfigSchema::new("modal")
        .property("title", PropertySchema::string())
        .property("open", PropertySchema::boolean().with_default(false));
    let _ = registry.register_component(modal);

    // Notification
    let mut notification = ComponentDescriptor::new("notification", ComponentCategory::Feedback);
    notification.metadata = ComponentMetadata {
        display_name: "Notification".to_string(),
        description: "A notification toast".to_string(),
        ..Default::default()
    };
    notification.schema = ConfigSchema::new("notification")
        .property("message", PropertySchema::string())
        .property("kind", PropertySchema::string().with_default("info"))
        .require("message");
    let _ = registry.register_component(notification);

    // Tooltip
    let mut tooltip = ComponentDescriptor::new("tooltip", ComponentCategory::Feedback);
    tooltip.metadata = ComponentMetadata {
        display_name: "Tooltip".to_string(),
        description: "A tooltip popup".to_string(),
        ..Default::default()
    };
    tooltip.schema = ConfigSchema::new("tooltip")
        .property("content", PropertySchema::string())
        .require("content");
    let _ = registry.register_component(tooltip);
}

fn register_chart_components(registry: &ComponentRegistry) {
    // Line Chart
    let mut line_chart = ComponentDescriptor::new("line_chart", ComponentCategory::Charts);
    line_chart.metadata = ComponentMetadata {
        display_name: "Line Chart".to_string(),
        description: "A line chart visualization".to_string(),
        ..Default::default()
    };
    line_chart.schema = ConfigSchema::new("line_chart")
        .property("x_field", PropertySchema::string())
        .property("y_field", PropertySchema::string())
        .property("data", PropertySchema::any())
        .property("dot", PropertySchema::boolean())
        .property("linear", PropertySchema::boolean())
        .property("tick_margin", PropertySchema::integer())
        .require("x_field")
        .require("y_field");
    let _ = registry.register_component(line_chart);

    // Bar Chart
    let mut bar_chart = ComponentDescriptor::new("bar_chart", ComponentCategory::Charts);
    bar_chart.metadata = ComponentMetadata {
        display_name: "Bar Chart".to_string(),
        description: "A bar chart visualization".to_string(),
        ..Default::default()
    };
    bar_chart.schema = ConfigSchema::new("bar_chart")
        .property("x_field", PropertySchema::string())
        .property("y_field", PropertySchema::string())
        .property("data", PropertySchema::any())
        .property("show_label", PropertySchema::boolean())
        .property("tick_margin", PropertySchema::integer())
        .require("x_field")
        .require("y_field");
    let _ = registry.register_component(bar_chart);

    // Area Chart
    let mut area_chart = ComponentDescriptor::new("area_chart", ComponentCategory::Charts);
    area_chart.metadata = ComponentMetadata {
        display_name: "Area Chart".to_string(),
        description: "A stacked area chart visualization".to_string(),
        ..Default::default()
    };
    area_chart.schema = ConfigSchema::new("area_chart")
        .property("x_field", PropertySchema::string())
        .property("y_fields", PropertySchema::array(PropertySchema::string()))
        .property("data", PropertySchema::any())
        .property("fill_opacity", PropertySchema::float())
        .property("tick_margin", PropertySchema::integer())
        .require("x_field")
        .require("y_fields");
    let _ = registry.register_component(area_chart);

    // Pie Chart
    let mut pie_chart = ComponentDescriptor::new("pie_chart", ComponentCategory::Charts);
    pie_chart.metadata = ComponentMetadata {
        display_name: "Pie Chart".to_string(),
        description: "A pie or donut chart visualization".to_string(),
        ..Default::default()
    };
    pie_chart.schema = ConfigSchema::new("pie_chart")
        .property("value_field", PropertySchema::string())
        .property("data", PropertySchema::any())
        .property("outer_radius", PropertySchema::float())
        .property("inner_radius", PropertySchema::float())
        .require("value_field");
    let _ = registry.register_component(pie_chart);

    // Candlestick Chart
    let mut candlestick =
        ComponentDescriptor::new("candlestick_chart", ComponentCategory::Charts);
    candlestick.metadata = ComponentMetadata {
        display_name: "Candlestick Chart".to_string(),
        description: "An OHLC candlestick chart for financial data".to_string(),
        ..Default::default()
    };
    candlestick.schema = ConfigSchema::new("candlestick_chart")
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
        .require("close_field");
    let _ = registry.register_component(candlestick);
}

/// Registers all built-in data sources.
pub fn register_builtin_data_sources(registry: &ComponentRegistry) {
    // HTTP Source
    let mut http = DataSourceDescriptor::new("http");
    http.metadata = DataSourceMetadata {
        display_name: "HTTP".to_string(),
        description: "Fetch data from HTTP endpoints".to_string(),
        supports_polling: true,
        supports_streaming: false,
        supports_manual_refresh: true,
        ..Default::default()
    };
    http.schema = ConfigSchema::new("http")
        .property("url", PropertySchema::string())
        .property("method", PropertySchema::string().with_default("GET"))
        .property("interval", PropertySchema::integer().with_default(0i64))
        .require("url");
    let _ = registry.register_data_source(http);

    // WebSocket Source
    let mut ws = DataSourceDescriptor::new("websocket");
    ws.metadata = DataSourceMetadata {
        display_name: "WebSocket".to_string(),
        description: "Stream data from WebSocket connections".to_string(),
        supports_polling: false,
        supports_streaming: true,
        supports_manual_refresh: true,
        ..Default::default()
    };
    ws.schema = ConfigSchema::new("websocket")
        .property("url", PropertySchema::string())
        .require("url");
    let _ = registry.register_data_source(ws);

    // Timer Source
    let mut timer = DataSourceDescriptor::new("timer");
    timer.metadata = DataSourceMetadata {
        display_name: "Timer".to_string(),
        description: "Generate events at intervals".to_string(),
        supports_polling: true,
        supports_streaming: false,
        supports_manual_refresh: true,
        ..Default::default()
    };
    timer.schema = ConfigSchema::new("timer")
        .property("interval", PropertySchema::integer())
        .require("interval");
    let _ = registry.register_data_source(timer);

    // File Source
    let mut file = DataSourceDescriptor::new("file");
    file.metadata = DataSourceMetadata {
        display_name: "File".to_string(),
        description: "Read data from files".to_string(),
        supports_polling: true,
        supports_streaming: false,
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
    // Map Transform
    let mut map = TransformDescriptor::new("map");
    map.metadata = TransformMetadata {
        display_name: "Map".to_string(),
        description: "Transform each item".to_string(),
        preserves_order: true,
        may_filter: false,
        stateful: false,
        ..Default::default()
    };
    map.schema = ConfigSchema::new("map")
        .property("expression", PropertySchema::string())
        .require("expression");
    let _ = registry.register_transform(map);

    // Filter Transform
    let mut filter = TransformDescriptor::new("filter");
    filter.metadata = TransformMetadata {
        display_name: "Filter".to_string(),
        description: "Filter items by condition".to_string(),
        preserves_order: true,
        may_filter: true,
        stateful: false,
        ..Default::default()
    };
    filter.schema = ConfigSchema::new("filter")
        .property("condition", PropertySchema::string())
        .require("condition");
    let _ = registry.register_transform(filter);

    // Select Transform
    let mut select = TransformDescriptor::new("select");
    select.metadata = TransformMetadata {
        display_name: "Select".to_string(),
        description: "Select specific fields".to_string(),
        preserves_order: true,
        may_filter: false,
        stateful: false,
        ..Default::default()
    };
    select.schema = ConfigSchema::new("select")
        .property("fields", PropertySchema::array(PropertySchema::string()))
        .require("fields");
    let _ = registry.register_transform(select);

    // Sort Transform
    let mut sort = TransformDescriptor::new("sort");
    sort.metadata = TransformMetadata {
        display_name: "Sort".to_string(),
        description: "Sort items".to_string(),
        preserves_order: false,
        may_filter: false,
        stateful: false,
        ..Default::default()
    };
    sort.schema = ConfigSchema::new("sort")
        .property("by", PropertySchema::string())
        .property("direction", PropertySchema::string().with_default("asc"))
        .require("by");
    let _ = registry.register_transform(sort);

    // Aggregate Transform
    let mut aggregate = TransformDescriptor::new("aggregate");
    aggregate.metadata = TransformMetadata {
        display_name: "Aggregate".to_string(),
        description: "Aggregate items".to_string(),
        preserves_order: false,
        may_filter: true,
        stateful: true,
        ..Default::default()
    };
    aggregate.schema = ConfigSchema::new("aggregate")
        .property("group_by", PropertySchema::string())
        .property("operation", PropertySchema::string())
        .require("operation");
    let _ = registry.register_transform(aggregate);
}

/// Registers all built-in actions.
pub fn register_builtin_actions(registry: &ComponentRegistry) {
    // Notification Action
    let mut notify = ActionDescriptor::new("notification");
    notify.metadata = ActionMetadata {
        display_name: "Show Notification".to_string(),
        description: "Display a notification to the user".to_string(),
        async_execution: false,
        may_fail: false,
        idempotent: true,
        ..Default::default()
    };
    notify.schema = ConfigSchema::new("notification")
        .property("message", PropertySchema::string())
        .property("type", PropertySchema::string().with_default("info"))
        .require("message");
    let _ = registry.register_action(notify);

    // Navigate Action
    let mut navigate = ActionDescriptor::new("navigate");
    navigate.metadata = ActionMetadata {
        display_name: "Navigate".to_string(),
        description: "Navigate to a different view".to_string(),
        async_execution: false,
        may_fail: false,
        idempotent: true,
        ..Default::default()
    };
    navigate.schema = ConfigSchema::new("navigate")
        .property("target", PropertySchema::string())
        .require("target");
    let _ = registry.register_action(navigate);

    // Refresh Action
    let mut refresh = ActionDescriptor::new("refresh");
    refresh.metadata = ActionMetadata {
        display_name: "Refresh".to_string(),
        description: "Refresh data or UI".to_string(),
        async_execution: true,
        may_fail: false,
        idempotent: true,
        ..Default::default()
    };
    refresh.schema = ConfigSchema::new("refresh").property("target", PropertySchema::string());
    let _ = registry.register_action(refresh);

    // HTTP Request Action
    let mut http = ActionDescriptor::new("http_request");
    http.metadata = ActionMetadata {
        display_name: "HTTP Request".to_string(),
        description: "Make an HTTP request".to_string(),
        async_execution: true,
        may_fail: true,
        idempotent: false,
        ..Default::default()
    };
    http.schema = ConfigSchema::new("http_request")
        .property("url", PropertySchema::string())
        .property("method", PropertySchema::string().with_default("POST"))
        .property("body", PropertySchema::any())
        .require("url");
    let _ = registry.register_action(http);

    // Set Data Action
    let mut set_data = ActionDescriptor::new("set_data");
    set_data.metadata = ActionMetadata {
        display_name: "Set Data".to_string(),
        description: "Set a data value".to_string(),
        async_execution: false,
        may_fail: false,
        idempotent: true,
        ..Default::default()
    };
    set_data.schema = ConfigSchema::new("set_data")
        .property("target", PropertySchema::string())
        .property("value", PropertySchema::any())
        .require("target")
        .require("value");
    let _ = registry.register_action(set_data);
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

        // Verify some components were registered
        assert!(registry.has_component("button"));
        assert!(registry.has_component("label"));
        assert!(registry.has_component("table"));

        // Verify basic components
        assert!(registry.has_component("accordion"));
        assert!(registry.has_component("alert"));
        assert!(registry.has_component("avatar"));
        assert!(registry.has_component("badge"));
        assert!(registry.has_component("collapsible"));
        assert!(registry.has_component("dropdown_button"));
        assert!(registry.has_component("spinner"));
        assert!(registry.has_component("tag"));

        // Verify input components
        assert!(registry.has_component("radio"));
        assert!(registry.has_component("slider"));
        assert!(registry.has_component("switch"));
        assert!(registry.has_component("toggle"));

        // Verify chart components
        assert!(registry.has_component("line_chart"));
        assert!(registry.has_component("bar_chart"));
        assert!(registry.has_component("area_chart"));
        assert!(registry.has_component("pie_chart"));
        assert!(registry.has_component("candlestick_chart"));

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
