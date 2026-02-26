//! Component rendering dispatch for ratatui.
//!
//! Maps BuiltComponent types to ratatui widgets.

use nemo_layout::BuiltComponent;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::HashMap;

use crate::components::{label, panel, progress, stack, table, text};

/// Render a component tree into the given area.
pub fn render_component(
    frame: &mut Frame,
    area: Rect,
    component: &BuiltComponent,
    components: &HashMap<String, BuiltComponent>,
) {
    match component.component_type.as_str() {
        "stack" => stack::render(frame, area, component, components),
        "panel" => panel::render(frame, area, component, components),
        "label" => label::render(frame, area, component),
        "text" => text::render(frame, area, component),
        "progress" => progress::render(frame, area, component),
        "table" => table::render(frame, area, component),
        _ => {
            // Graceful degradation: render children in a vertical stack
            render_children_vertical(frame, area, component, components);
        }
    }
}

/// Render children of a component in a vertical stack (fallback for unsupported types).
pub fn render_children_vertical(
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

    let chunks = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Vertical)
        .constraints(
            children
                .iter()
                .map(|_| ratatui::layout::Constraint::Min(1))
                .collect::<Vec<_>>(),
        )
        .split(area);

    for (child, chunk) in children.iter().zip(chunks.iter()) {
        render_component(frame, *chunk, child, components);
    }
}
