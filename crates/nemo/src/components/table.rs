use gpui::*;
use gpui_component::table::{Column, TableDelegate, TableState};
use nemo_config::Value;
use nemo_layout::BuiltComponent;

/// Table delegate that renders rows from `Vec<Value>` objects.
pub struct NemoTableDelegate {
    columns: Vec<Column>,
    column_keys: Vec<String>,
    rows: Vec<Value>,
}

impl NemoTableDelegate {
    /// Build a delegate from the component's properties.
    pub fn from_properties(properties: &std::collections::HashMap<String, Value>) -> Self {
        let mut columns = Vec::new();
        let mut column_keys = Vec::new();

        // Parse explicit columns if provided
        if let Some(Value::Array(cols)) = properties.get("columns") {
            for col_val in cols {
                if let Some(obj) = col_val.as_object() {
                    let key = obj
                        .get("key")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .to_string();
                    let label = obj
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&key)
                        .to_string();
                    let mut col = Column::new(key.clone(), label);
                    if let Some(w) = obj.get("width").and_then(|v| v.as_i64()) {
                        col = col.width(px(w as f32));
                    }
                    columns.push(col);
                    column_keys.push(key);
                }
            }
        }

        // Parse data rows
        let rows = match properties.get("data") {
            Some(Value::Array(arr)) => arr.clone(),
            _ => Vec::new(),
        };

        // Auto-detect columns from first row if none specified
        if columns.is_empty() {
            if let Some(first) = rows.first() {
                if let Some(obj) = first.as_object() {
                    for key in obj.keys() {
                        columns.push(Column::new(key.clone(), key.clone()));
                        column_keys.push(key.clone());
                    }
                }
            }
        }

        Self {
            columns,
            column_keys,
            rows,
        }
    }

    /// Replace the row data.
    pub fn set_rows(&mut self, rows: Vec<Value>) {
        self.rows = rows;
    }
}

impl TableDelegate for NemoTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let text = self
            .rows
            .get(row_ix)
            .and_then(|row| self.column_keys.get(col_ix).and_then(|key| row.get(key)))
            .map(|v| v.to_string())
            .unwrap_or_default();

        div().child(text)
    }
}

/// Nemo Table wrapper component.
#[derive(IntoElement)]
pub struct Table {
    source: BuiltComponent,
    table_state: Option<Entity<TableState<NemoTableDelegate>>>,
}

impl Table {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            table_state: None,
        }
    }

    pub fn table_state(mut self, state: Entity<TableState<NemoTableDelegate>>) -> Self {
        self.table_state = Some(state);
        self
    }
}

impl RenderOnce for Table {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let Some(state) = self.table_state else {
            return div().child("Table: missing state").into_any_element();
        };

        let stripe = self
            .source
            .properties
            .get("stripe")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let bordered = self
            .source
            .properties
            .get("bordered")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let table = gpui_component::table::Table::new(&state)
            .stripe(stripe)
            .bordered(bordered);

        // Table internally uses size_full() with uniform_list in Auto sizing mode,
        // so it needs a parent with a definite height to render rows.
        let height = self
            .source
            .properties
            .get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(300) as f32;

        div().w_full().h(px(height)).child(table).into_any_element()
    }
}
