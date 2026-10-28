use gpui::{prelude::FluentBuilder as _, *};
use gpui_kit::component::table::{Column, TableDelegate, TableState};
use gpui_kit::component::ActiveTheme;
use nemo_config::Value;
use nemo_layout::BuiltComponent;

/// A column of a declared table: header label, pixel width, alignment.
#[derive(Clone, Debug)]
pub(crate) struct DeclaredColumn {
    /// Data field name (maps row objects in a data-driven table).
    pub field: String,
    pub label: String,
    pub width: Option<f32>,
    pub align: gpui::TextAlign,
}

/// One cell of a declared table row. Cells are positional (like HTML).
pub(crate) struct DeclaredCell {
    pub col_span: usize,
    pub row_span: usize,
    pub align: gpui::TextAlign,
    pub content: Vec<AnyElement>,
}

/// One declared row: its cells plus the row's own col_span (a full-width row).
pub(crate) struct DeclaredRow {
    pub col_span: usize,
    pub cells: Vec<DeclaredCell>,
}

/// Declared-mode table data extracted from `<table-header>`/`<table-row>`
/// children by the dispatcher.
pub(crate) struct DeclaredTableData {
    pub columns: Vec<DeclaredColumn>,
    pub rows: Vec<DeclaredRow>,
}

impl Table {
    /// Renders the declared-mode grid: fixed-width columns, uniform rows
    /// (grown by row-spans), absolutely positioned cells.
    ///
    /// Painting is two-pass: every row's background/border strip first, then
    /// every cell on top — a row-spanned cell covers rows below it, and GPUI
    /// paints later siblings over earlier ones, so a strip added after the
    /// spanned cell would paint over its lower half.
    fn render_declared(source: BuiltComponent, data: DeclaredTableData, cx: &App) -> AnyElement {
        let row_height = 32.0f32;

        let total_width: f32 = data.columns.iter().map(|c| c.width.unwrap_or(0.0)).sum();

        // Per-row heights: a spanning cell's extra height distributes across
        // the rows it covers, so the cell's height is the sum of its rows.
        let row_heights = {
            let mut heights = vec![row_height; data.rows.len()];
            for (row_ix, row) in data.rows.iter().enumerate() {
                for cell in &row.cells {
                    if cell.row_span > 1 {
                        let extra = row_height * (cell.row_span - 1) as f32;
                        let share = extra / cell.row_span as f32;
                        for h in heights.iter_mut().take(row_ix + cell.row_span).skip(row_ix) {
                            *h += share;
                        }
                    }
                }
            }
            heights
        };

        let header = Self::render_declared_header(&data, cx);
        let rows_count = data.rows.len();

        let mut body = div()
            .relative()
            .w_full()
            .h(px(row_heights.iter().sum::<f32>()));

        // Pass one: stripe/last-row strips.
        let stripe_color = cx.theme().tokens.table_even;
        let mut y = 0.0f32;
        for (row_ix, _) in data.rows.iter().enumerate() {
            let row_h = row_heights[row_ix];
            if row_ix % 2 == 1 || row_ix + 1 == rows_count {
                body = body.child(
                    div()
                        .absolute()
                        .left(px(0.))
                        .top(px(y))
                        .w(px(total_width))
                        .h(px(row_h))
                        .when(row_ix % 2 == 1, |el| el.bg(stripe_color))
                        .when(row_ix + 1 == rows_count, |el| {
                            el.border_b_1().border_color(cx.theme().table_row_border)
                        }),
                );
            }
            y += row_h;
        }

        // Pass two: cells on top of the strips.
        let mut y = 0.0f32;
        for (row_ix, row) in data.rows.into_iter().enumerate() {
            let mut start_col = 0usize;

            // A row-level col-span turns the row into a single full-width
            // cell spanning that many columns.
            let row_span_override = row.col_span.max(1);
            for cell in row.cells {
                let span = if row_span_override > 1 {
                    row_span_override
                } else {
                    cell.col_span
                };
                if let Some((x, w)) = cell_x_width(&data.columns, start_col, span) {
                    let cell_h: f32 = (0..cell.row_span)
                        .map(|i| row_heights.get(row_ix + i).copied().unwrap_or(0.0))
                        .sum();

                    body = body.child(
                        div()
                            .absolute()
                            .left(px(x))
                            .top(px(y))
                            .w(px(w))
                            .h(px(cell_h))
                            .flex()
                            .flex_none()
                            .when(cell.align == gpui::TextAlign::Center, |el| {
                                el.justify_center()
                            })
                            .when(cell.align == gpui::TextAlign::Right, |el| el.justify_end())
                            .px(px(8.))
                            .py(px(4.))
                            .items_center()
                            .overflow_hidden()
                            .children(cell.content),
                    );
                }
                start_col += span;
            }

            y += row_heights[row_ix];
        }

        let bordered = source
            .properties
            .get("bordered")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut table = div()
            .relative()
            .overflow_hidden()
            .when(bordered, |el| {
                el.border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(6.))
            })
            .child(header)
            .child(body);

        // The grid is exactly the declared columns' total width; it does not
        // stretch to the container (columns are pixel-definite, so a wider
        // parent would leave a dangling fifth track).
        table = table.w(px(total_width.max(1.0)));

        table.into_any_element()
    }

    /// The declared header: a background strip plus one positioned cell per
    /// column, matching the kit's `table_head` token.
    fn render_declared_header(data: &DeclaredTableData, cx: &App) -> Div {
        let head_bg = cx.theme().tokens.table_head;
        let head_fg = cx.theme().table_head_foreground;

        let mut header = div()
            .relative()
            .w_full()
            .h(px(32.))
            .flex_none()
            .bg(head_bg)
            .text_color(head_fg)
            .border_b_1()
            .border_color(cx.theme().table_row_border);

        for (ix, col) in data.columns.iter().enumerate() {
            if let Some((x, w)) = cell_x_width(&data.columns, ix, 1) {
                header = header.child(
                    div()
                        .absolute()
                        .left(px(x))
                        .top(px(0.))
                        .w(px(w))
                        .h_full()
                        .flex()
                        .items_center()
                        .when(col.align == gpui::TextAlign::Center, |el| {
                            el.justify_center()
                        })
                        .when(col.align == gpui::TextAlign::Right, |el| el.justify_end())
                        .px(px(8.))
                        .child(col.label.clone()),
                );
            }
        }

        header
    }
}

/// Layout math shared by the header and the body: computes each cell's x/width
/// from the grid of column widths, honoring `col_span`. Returns
/// `(x, width)` in logical pixels; `None` for cells pushed past the grid.
pub(crate) fn cell_x_width(
    columns: &[DeclaredColumn],
    start_col: usize,
    col_span: usize,
) -> Option<(f32, f32)> {
    let span = col_span.max(1).min(columns.len().saturating_sub(start_col));
    if span == 0 {
        return None;
    }
    let mut x = 0.0;
    for col in columns.iter().take(start_col) {
        x += col.width.unwrap_or(0.0);
    }
    let mut w = 0.0;
    for col in columns.iter().skip(start_col).take(span) {
        w += col.width.unwrap_or(0.0);
    }
    Some((x, w))
}

/// Table delegate that renders rows from `Vec<Value>` objects.
pub struct NemoTableDelegate {
    columns: Vec<Column>,
    column_keys: Vec<String>,
    rows: Vec<Value>,
}

impl NemoTableDelegate {
    /// Build a delegate from explicit column definitions and rows.
    pub fn from_parts(columns: Vec<Column>, column_keys: Vec<String>, rows: Vec<Value>) -> Self {
        if columns.is_empty() {
            // Auto-detect columns from the first row.
            let mut columns = Vec::new();
            let mut column_keys = Vec::new();
            if let Some(first) = rows.first() {
                if let Some(obj) = first.as_object() {
                    for key in obj.keys() {
                        columns.push(Column::new(key.clone(), key.clone()));
                        column_keys.push(key.clone());
                    }
                }
            }
            return Self {
                columns,
                column_keys,
                rows,
            };
        }
        Self {
            columns,
            column_keys,
            rows,
        }
    }

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

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
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

/// A data table component.
///
/// Two authoring modes, picked automatically:
///
/// * **Data mode** — a `data` array property (usually via `<binding>`), rendered
///   through gpui-kit's virtualized `DataTable`. Columns come from the
///   `columns` JSON property or are auto-detected from the first row.
/// * **Declared mode** — content authored as child components:
///
/// ```xml
/// <table id="users">
///   <table-header>
///     <table-column key="name" width="150">Name</table-column>
///     <table-column key="email" width="220">Email</table-column>
///   </table-header>
///   <table-row>
///     <table-cell>Alice</table-cell>
///     <table-cell>alice@example.com</table-cell>
///   </table-row>
/// </table>
/// ```
///
/// Declared cells may span: `<table-cell col-span="2">` merges columns,
/// `row-span` merges rows, and a `<table-row col-span="N">` with a single
/// cell renders a full-width row across `N` columns. Cells hold arbitrary
/// child components, so a declared row can contain live buttons or
/// checkboxes — impossible in data mode.
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `data` | JSON array | Rows as objects keyed by column key (data mode) |
/// | `columns` | JSON array | Column definitions with `key`/`label`/`width` (data mode) |
/// | `stripe` | bool | Alternate row background colors |
/// | `bordered` | bool | Show cell borders |
/// | `height` | int | Fixed table height in pixels (default 300) |
#[derive(IntoElement)]
pub struct Table {
    source: BuiltComponent,
    table_state: Option<Entity<TableState<NemoTableDelegate>>>,
    /// Declared-mode data; `None` renders the data-mode table.
    declared: Option<DeclaredTableData>,
}

impl Table {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            table_state: None,
            declared: None,
        }
    }

    pub fn table_state(mut self, state: Entity<TableState<NemoTableDelegate>>) -> Self {
        self.table_state = Some(state);
        self
    }

    /// Switch to declared mode with the extracted header/row structure.
    pub fn declared(mut self, data: DeclaredTableData) -> Self {
        self.declared = Some(data);
        self
    }
}

impl RenderOnce for Table {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.declared.is_some() {
            let declared = self.declared.unwrap();
            return Self::render_declared(self.source, declared, cx);
        }

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

        let table = gpui_kit::component::table::DataTable::new(&state)
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

#[cfg(test)]
mod tests {
    use crate::components::table::{cell_x_width, DeclaredColumn};

    fn columns() -> Vec<DeclaredColumn> {
        vec![
            DeclaredColumn {
                field: "a".to_string(),
                label: "A".to_string(),
                width: Some(100.0),
                align: gpui::TextAlign::Left,
            },
            DeclaredColumn {
                field: "b".to_string(),
                label: "B".to_string(),
                width: Some(50.0),
                align: gpui::TextAlign::Left,
            },
            DeclaredColumn {
                field: "c".to_string(),
                label: "C".to_string(),
                width: Some(150.0),
                align: gpui::TextAlign::Left,
            },
        ]
    }

    #[test]
    fn cell_x_width_positions_single_cell() {
        let cols = columns();
        let (x, w) = cell_x_width(&cols, 1, 1).unwrap();
        assert_eq!(x, 100.0);
        assert_eq!(w, 50.0);
    }

    #[test]
    fn cell_x_width_sums_span() {
        let cols = columns();
        let (x, w) = cell_x_width(&cols, 1, 2).unwrap();
        assert_eq!(x, 100.0);
        assert_eq!(w, 200.0); // B (50) + C (150)
    }

    #[test]
    fn cell_x_width_clamps_past_grid() {
        let cols = columns();
        // Spanning past the last column clamps instead of dropping the cell.
        let (x, w) = cell_x_width(&cols, 2, 5).unwrap();
        assert_eq!(x, 150.0);
        assert_eq!(w, 150.0);
        // A start past the grid returns None.
        assert!(cell_x_width(&cols, 3, 1).is_none());
    }
}
