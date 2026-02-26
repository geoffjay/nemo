//! Progress component -> ratatui Gauge.

use nemo_layout::BuiltComponent;
use ratatui::layout::Rect;
use ratatui::widgets::Gauge;
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, component: &BuiltComponent) {
    let value = component
        .properties
        .get("value")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(0.0);

    let max = component
        .properties
        .get("max")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        .unwrap_or(100.0);

    let ratio = if max > 0.0 {
        (value / max).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let label = component
        .properties
        .get("label")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{:.0}%", ratio * 100.0));

    let gauge = Gauge::default()
        .label(label)
        .ratio(ratio);

    frame.render_widget(gauge, area);
}
