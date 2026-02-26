use gpui::*;
use gpui_component::ActiveTheme;
use nemo_config::Value;
use nemo_layout::BuiltComponent;

/// Extracts the `data` property from a component as a `Vec<Value>`.
pub(crate) fn extract_data_array(source: &BuiltComponent) -> Vec<Value> {
    source
        .properties
        .get("data")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

/// Extracts a string value from a `Value::Object` by key.
pub(crate) fn get_string_field(item: &Value, field: &str) -> String {
    item.get(field)
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

/// Extracts an f64 value from a `Value::Object` by key.
pub(crate) fn get_f64_field(item: &Value, field: &str) -> f64 {
    item.get(field).and_then(|v| v.as_f64()).unwrap_or(0.0)
}

/// Extracts a string array property from a component's properties.
pub(crate) fn get_string_array(source: &BuiltComponent, key: &str) -> Vec<String> {
    source
        .properties
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Returns a theme chart color by index (0-based, wraps around chart_1..chart_5).
pub(crate) fn chart_color(index: usize, cx: &App) -> Hsla {
    let colors = &cx.theme().colors;
    match index % 5 {
        0 => colors.chart_1,
        1 => colors.chart_2,
        2 => colors.chart_3,
        3 => colors.chart_4,
        _ => colors.chart_5,
    }
}

/// Returns a placeholder element for charts with no data.
pub(crate) fn empty_chart_placeholder(cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_center()
        .size_full()
        .min_h(px(100.0))
        .text_sm()
        .text_color(cx.theme().colors.muted_foreground)
        .child("No data")
        .into_any_element()
}
