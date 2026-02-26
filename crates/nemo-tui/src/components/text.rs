//! Text component -> ratatui Paragraph with word wrap.

use nemo_layout::BuiltComponent;
use ratatui::layout::Rect;
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, component: &BuiltComponent) {
    let content = component
        .properties
        .get("content")
        .or_else(|| component.properties.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let paragraph = Paragraph::new(content.to_string()).wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
