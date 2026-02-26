//! Stack component -> ratatui Layout with Direction.

use nemo_layout::BuiltComponent;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::HashMap;

use crate::renderer;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    component: &BuiltComponent,
    components: &HashMap<String, BuiltComponent>,
) {
    let children: Vec<&BuiltComponent> = component
        .children
        .iter()
        .filter_map(|id| components.get(id))
        .collect();

    if children.is_empty() {
        return;
    }

    let direction = match component
        .properties
        .get("direction")
        .and_then(|v| v.as_str())
    {
        Some("horizontal") => Direction::Horizontal,
        _ => Direction::Vertical,
    };

    // Build constraints: use flex property if available, otherwise equal distribution
    let constraints: Vec<Constraint> = children
        .iter()
        .map(|child| {
            if let Some(flex) = child
                .properties
                .get("flex")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            {
                Constraint::Ratio(flex as u32, total_flex(&children) as u32)
            } else if let Some(height) = child.properties.get("height").and_then(|v| v.as_i64()) {
                if direction == Direction::Vertical {
                    Constraint::Length(height as u16)
                } else {
                    Constraint::Min(1)
                }
            } else if let Some(width) = child.properties.get("width").and_then(|v| v.as_i64()) {
                if direction == Direction::Horizontal {
                    Constraint::Length(width as u16)
                } else {
                    Constraint::Min(1)
                }
            } else {
                Constraint::Min(1)
            }
        })
        .collect();

    let chunks = Layout::default()
        .direction(direction)
        .constraints(constraints)
        .split(area);

    for (child, chunk) in children.iter().zip(chunks.iter()) {
        renderer::render_component(frame, *chunk, child, components);
    }
}

fn total_flex(children: &[&BuiltComponent]) -> f64 {
    let sum: f64 = children
        .iter()
        .filter_map(|c| {
            c.properties
                .get("flex")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        })
        .sum();
    if sum > 0.0 {
        sum
    } else {
        children.len() as f64
    }
}
