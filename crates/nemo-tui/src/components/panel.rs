//! Panel component -> ratatui Block with borders and title.

use nemo_layout::BuiltComponent;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;
use std::collections::HashMap;

use crate::renderer;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    component: &BuiltComponent,
    components: &HashMap<String, BuiltComponent>,
) {
    let title = component
        .properties
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let show_border = component
        .properties
        .get("border")
        .and_then(|v| v.as_bool().or_else(|| v.as_i64().map(|i| i != 0)))
        .unwrap_or(true);

    let mut block = Block::default();
    if show_border {
        block = block.borders(Borders::ALL);
    }
    if !title.is_empty() {
        block = block.title(title.to_string());
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Render children inside the block's inner area
    renderer::render_children_vertical(frame, inner, component, components);
}
