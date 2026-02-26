//! Label component -> ratatui Paragraph (single line).

use nemo_layout::BuiltComponent;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, component: &BuiltComponent) {
    let text = component
        .properties
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let paragraph = Paragraph::new(text.to_string());
    frame.render_widget(paragraph, area);
}
