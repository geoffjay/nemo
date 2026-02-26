//! Table component -> ratatui Table.

use nemo_config::Value;
use nemo_layout::BuiltComponent;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Cell, Row, Table as RatatuiTable};
use ratatui::Frame;

pub fn render(frame: &mut Frame, area: Rect, component: &BuiltComponent) {
    let title = component
        .properties
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Extract column definitions
    let columns: Vec<String> = component
        .properties
        .get("columns")
        .and_then(|v| match v {
            Value::Array(arr) => Some(
                arr.iter()
                    .filter_map(|col| {
                        col.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| {
                                col.as_object()
                                    .and_then(|obj| obj.get("label"))
                                    .and_then(|l| l.as_str())
                                    .map(|s| s.to_string())
                            })
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();

    // Extract column keys (for looking up row data)
    let column_keys: Vec<String> = component
        .properties
        .get("columns")
        .and_then(|v| match v {
            Value::Array(arr) => Some(
                arr.iter()
                    .filter_map(|col| {
                        col.as_str()
                            .map(|s| s.to_string())
                            .or_else(|| {
                                col.as_object()
                                    .and_then(|obj| obj.get("key"))
                                    .and_then(|k| k.as_str())
                                    .map(|s| s.to_string())
                            })
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();

    // Extract row data
    let rows: Vec<Row> = component
        .properties
        .get("data")
        .and_then(|v| match v {
            Value::Array(arr) => Some(
                arr.iter()
                    .map(|row_val| {
                        let cells: Vec<Cell> = column_keys
                            .iter()
                            .map(|key| {
                                let cell_text = row_val
                                    .as_object()
                                    .and_then(|obj| obj.get(key))
                                    .map(|v| value_to_string(v))
                                    .unwrap_or_default();
                                Cell::from(cell_text)
                            })
                            .collect();
                        Row::new(cells)
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default();

    let header = Row::new(columns.iter().map(|c| Cell::from(c.as_str())));

    let widths: Vec<ratatui::layout::Constraint> = column_keys
        .iter()
        .map(|_| ratatui::layout::Constraint::Min(8))
        .collect();

    let mut block = Block::default().borders(Borders::ALL);
    if !title.is_empty() {
        block = block.title(title.to_string());
    }

    let table = RatatuiTable::new(rows, widths)
        .header(header)
        .block(block);

    frame.render_widget(table, area);
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Integer(n) => n.to_string(),
        Value::Float(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => format!("{:?}", other),
    }
}
