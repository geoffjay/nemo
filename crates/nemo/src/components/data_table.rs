use std::cmp::Ordering;

use gpui::{
    div, px, App, Context, Entity, IntoElement, ParentElement as _, RenderOnce, SharedString,
    Styled as _, Window,
};
use gpui_kit::component::table::{
    Column, ColumnFixed, ColumnSort, DataTable as GpuiDataTable, TableDelegate, TableState,
};
use gpui_kit::component::{ActiveTheme as _, Sizable as _, Size as ComponentSize};
use nemo_config::Value;
use nemo_layout::BuiltComponent;

/// One declarative `<data-table-column>`: a column definition with its
/// behavior flags.
#[derive(Clone, Debug)]
pub struct DataTableColumnData {
    pub key: String,
    pub label: String,
    /// Pixel width; columns without one split the leftover space.
    pub width: Option<i64>,
    /// `left` (default), `center`, or `right`.
    pub align: String,
    /// Whether the column participates in sorting.
    pub sortable: bool,
    /// Initial sort direction; only meaningful with `sortable`, and the
    /// rows are pre-sorted accordingly when the state is created.
    pub sort: Option<ColumnSort>,
    /// `none` (default) or `left`.
    pub fixed: Option<ColumnFixed>,
    pub resizable: bool,
    pub movable: bool,
}

/// The declared schema's identity, used by the app to detect XML edits that
/// require rebuilding the table state (columns are baked into the state's
/// header layout; behavior flags live there too). Live `data` updates never
/// rebuild — they replace the delegate's rows in place.
pub fn data_table_schema_signature(
    columns: &[DataTableColumnData],
    properties: &std::collections::HashMap<String, Value>,
) -> String {
    let mut signature = columns
        .iter()
        .map(|column| {
            format!(
                "{}/{}/{}/{}/{:?}/{:?}/{}/{}/{}",
                column.key,
                column.label,
                column.width.unwrap_or(0),
                column.align,
                column.sortable,
                column.sort,
                matches!(column.fixed, Some(ColumnFixed::Left)),
                column.resizable,
                column.movable
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    // Behavior flags ride in the schema: changing one rebuilds the state
    // (they are plain state fields set once at creation).
    for (key, default) in [
        ("sortable", true),
        ("col_movable", true),
        ("col_resizable", true),
        ("row_selectable", true),
        ("cell_selectable", false),
    ] {
        let value = properties
            .get(key)
            .and_then(|v| v.as_bool())
            .unwrap_or(default);
        signature.push_str(&format!("|{key}={value}"));
    }
    signature
}

/// Builds column definitions when no `<data-table-column>` children are
/// declared: the first row object's fields, in order, become plain columns.
pub fn data_table_autodetect_columns(data: &[Value]) -> Vec<DataTableColumnData> {
    let Some(Value::Object(fields)) = data.iter().find(|row| matches!(row, Value::Object(_)))
    else {
        return Vec::new();
    };
    fields
        .keys()
        .filter(|key| !key.starts_with("__"))
        .map(|key| DataTableColumnData {
            key: key.clone(),
            label: key.clone(),
            width: None,
            align: "left".to_string(),
            sortable: false,
            sort: None,
            fixed: None,
            resizable: true,
            movable: true,
        })
        .collect()
}

impl DataTableColumnData {
    /// Parses a `<data-table-column>` element's properties into a column
    /// definition. `label` defaults to `field`; unknown
    /// `align`/`sort`/`fixed` spellings fall back to the plain behavior.
    /// (The attribute is `field` because `key` is reserved by the `n:for`
    /// directive pass, same as `<table-column>`.)
    pub fn from_properties(properties: &std::collections::HashMap<String, Value>) -> Self {
        let key = properties
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let label = properties
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| key.clone());
        let sort = match properties
            .get("sort")
            .and_then(|v| v.as_str())
            .map(str::trim)
        {
            Some("ascending") | Some("asc") => Some(ColumnSort::Ascending),
            Some("descending") | Some("desc") => Some(ColumnSort::Descending),
            _ => None,
        };
        let fixed = match properties
            .get("fixed")
            .and_then(|v| v.as_str())
            .map(str::trim)
        {
            Some("left") => Some(ColumnFixed::Left),
            _ => None,
        };
        Self {
            key,
            label,
            width: properties.get("width").and_then(|v| v.as_i64()),
            align: properties
                .get("align")
                .and_then(|v| v.as_str())
                .unwrap_or("left")
                .trim()
                .to_string(),
            sortable: data_table_flag(properties, "sortable", false),
            sort,
            fixed,
            resizable: data_table_flag(properties, "resizable", true),
            movable: data_table_flag(properties, "movable", true),
        }
    }

    /// Builds the kit's `Column` from the declared definition: width, text
    /// alignment, fixed side, and the sort/resize/move flags. A plain
    /// `sortable="true"` marks the column sort-participating without an
    /// initial direction.
    pub fn to_kit_column(&self) -> Column {
        let mut column = Column::new(self.key.clone(), self.label.clone());
        if let Some(width) = self.width {
            column = column.width(px(width as f32));
        }
        if let Some(fixed) = self.fixed {
            column = column.fixed(fixed);
        }
        if self.sortable {
            column = column.sortable();
        }
        if let Some(sort) = self.sort {
            column = column.sort(sort);
        }
        match self.align.as_str() {
            "center" => column = column.text_center(),
            "right" => column = column.text_right(),
            _ => {}
        }
        if !self.resizable {
            column = column.resizable(false);
        }
        if !self.movable {
            column = column.movable(false);
        }
        column
    }
}

/// Tolerantly reads a boolean-typed attribute value (various rasterized
/// spellings), with a per-attribute default.
pub(crate) fn data_table_flag(
    properties: &std::collections::HashMap<String, Value>,
    key: &str,
    default: bool,
) -> bool {
    properties
        .get(key)
        .and_then(|v| {
            v.as_bool().or_else(|| {
                v.as_str()
                    .map(|text| matches!(text.trim(), "true" | "1" | "on" | "yes"))
            })
        })
        .unwrap_or(default)
}

/// Ordering used by `perform_sort`: null, then booleans, then numbers
/// (Integers and Floats compare as one class), then strings; anything else
/// compares by its debug representation. Within a class it is a plain
/// lexicographic/natural order, case-sensitive.
pub fn data_table_cmp_values(left: &Value, right: &Value) -> Ordering {
    let rank = |value: &Value| match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Integer(_) | Value::Float(_) => 2,
        Value::String(_) => 3,
        _ => 4,
    };
    match rank(left).cmp(&rank(right)) {
        Ordering::Equal => match rank(left) {
            1 => match (left, right) {
                (Value::Bool(l), Value::Bool(r)) => l.cmp(r),
                _ => Ordering::Equal,
            },
            2 => {
                let l = left.as_f64().unwrap_or(0.);
                let r = right.as_f64().unwrap_or(0.);
                l.partial_cmp(&r).unwrap_or(Ordering::Equal)
            }
            3 => left
                .as_str()
                .unwrap_or("")
                .cmp(right.as_str().unwrap_or("")),
            _ => format!("{left:?}").cmp(&format!("{right:?}")),
        },
        other => other,
    }
}

/// Sorts rows by a column's key when the rows are objects with that field.
/// Rows without the field always sink to the tail, regardless of direction.
fn data_table_sort_rows(rows: &mut [Value], key: &str, direction: Ordering) {
    let field = |row: &Value| row.as_object().and_then(|obj| obj.get(key)).cloned();
    let mut sortable: Vec<(usize, Option<Value>)> = rows
        .iter()
        .enumerate()
        .map(|(ix, row)| (ix, field(row)))
        .collect();
    let unkeyed = |entry: &Option<Value>| entry.is_none();
    sortable.sort_by(|a, b| match (unkeyed(&a.1), unkeyed(&b.1)) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => {
            let l = a.1.as_ref().unwrap();
            let r = b.1.as_ref().unwrap();
            match direction {
                Ordering::Greater => data_table_cmp_values(r, l),
                _ => data_table_cmp_values(l, r),
            }
        }
    });
    let ordered: Vec<Value> = sortable
        .into_iter()
        .map(|(ix, _)| rows[ix].clone())
        .collect();
    rows.iter_mut()
        .zip(ordered)
        .for_each(|(slot, value)| *slot = value);
}

/// The delegate behind a declarative `<data-table>`: declared columns
/// (richer than `<table>`'s: per-column sort/fixed/resize/move flags) and
/// row objects sorted in place by the kit's sort protocol.
pub struct DataTableDelegate {
    columns: Vec<Column>,
    column_keys: Vec<String>,
    /// Live row order (rearranged by perform_sort, replaced by set_rows).
    rows: Vec<Value>,
    /// The declared order, restored when the kit cycles a column back to
    /// unsorted (`perform_sort` with `ColumnSort::Default`).
    source_rows: Vec<Value>,
}

impl DataTableDelegate {
    /// Builds a delegate from declared columns and a live `data` array.
    /// Rows missing a column field render as empty text.
    pub fn new(columns: Vec<Column>, column_keys: Vec<String>, rows: Vec<Value>) -> Self {
        let source_rows = rows.clone();
        let mut delegate = Self {
            columns,
            column_keys,
            rows,
            source_rows,
        };
        delegate.apply_declared_sorts();
        delegate
    }

    /// Pre-sorts the rows for columns declared with `sort="ascending"` or
    /// `"descending"`, so the header's initial chevron matches the data.
    fn apply_declared_sorts(&mut self) {
        // One sorted column at a time (the same contract the kit enforces
        // while running: exactly one column can carry a sort).
        let initial = self
            .columns
            .iter()
            .enumerate()
            .find_map(|(ix, column)| match column.sort {
                Some(ColumnSort::Default) | None => None,
                Some(other) => Some((ix, other)),
            });
        let Some((ix, sort)) = initial else {
            return;
        };
        let key = self.column_keys.get(ix).cloned().unwrap_or_default();
        let direction = match sort {
            ColumnSort::Descending => Ordering::Greater,
            _ => Ordering::Less,
        };
        data_table_sort_rows(&mut self.rows, &key, direction);
    }

    /// Replacing columns happens wholesale (the schema-signature rebuild in
    /// `get_or_create_data_table_state`), so no incremental column update is
    /// needed.
    /// Replace the row data. The declared order wins until the user sorts
    /// again — the active sort state lives in the table's column state, so
    /// a fresh `data` snapshot shows in declaration order.
    pub fn replace_rows(&mut self, rows: Vec<Value>) {
        self.source_rows = rows.clone();
        self.rows = rows;
    }

    fn key_of(&self, col_ix: usize) -> String {
        self.column_keys.get(col_ix).cloned().unwrap_or_default()
    }

    fn cell_text_of(&self, row_ix: usize, col_ix: usize) -> String {
        let key = self.key_of(col_ix);
        self.rows
            .get(row_ix)
            .and_then(|row| row.as_object().and_then(|obj| obj.get(&key)))
            .map(|value| match value {
                Value::Null => String::new(),
                Value::String(text) => text.clone(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    }
}

impl TableDelegate for DataTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    /// The kit drives the sort: it cycles the column's state
    /// (Default → Descending → Ascending → Default) and hands us the
    /// requested direction. A `Default` request restores the declared row
    /// order; the others sort by the row's field under that key.
    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) {
        let key = self.key_of(col_ix);
        match sort {
            ColumnSort::Default => {
                self.rows = self.source_rows.clone();
            }
            ColumnSort::Ascending => data_table_sort_rows(&mut self.rows, &key, Ordering::Less),
            ColumnSort::Descending => data_table_sort_rows(&mut self.rows, &key, Ordering::Greater),
        }
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) {
        if col_ix >= self.columns.len() || to_ix > self.columns.len() {
            return;
        }
        let column = self.columns.remove(col_ix);
        self.columns.insert(to_ix, column);
        let key = self.column_keys.remove(col_ix);
        self.column_keys.insert(to_ix, key);
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div().child(self.cell_text_of(row_ix, col_ix))
    }

    fn cell_label(&self, row_ix: usize, col_ix: usize, _cx: &App) -> Option<SharedString> {
        let text = self.cell_text_of(row_ix, col_ix);
        (!text.is_empty()).then_some(SharedString::from(text))
    }
}

/// A virtualized data table: row, column, and cell selection with keyboard
/// navigation, drag-resizable and drag-movable columns, fixed columns, and
/// kit-driven sorting with the declared rows.
///
/// # XML Configuration
///
/// ```xml
/// <data-table id="users" data="{{ rows }}"
///   sortable="true" row-selectable="true" cell-selectable="false"
///   col-movable="true" col-resizable="true"
///   on-select-row="handle_pick">
///   <data-table-column field="name" label="Name" width="140" sortable="true" />
///   <data-table-column field="role" label="Role" width="120" align="center" />
/// </data-table>
/// ```
///
/// Rows come from the `data` array property (usually through a `<binding>`);
/// each object's values render under the column whose `field` matches the
/// row's field. Without declared children the columns are auto-detected
/// from the first row object's fields; declared children carry the behavior
/// flags (sortable, fixed, …), so they are the recommended authoring face.
///
/// # Properties
///
/// | Property | Type | Default | Description |
/// |----------|------|---------|-------------|
/// | `data` | JSON array | `[]` | Rows as objects keyed by column field |
/// | `height` | int | `340` | Table height in px (virtual scrolling needs a definite height) |
/// | `size` | string | `medium` | xsmall, small, medium, or large |
/// | `stripe` | bool | `false` | Alternate row background colors |
/// | `bordered` | bool | `true` | Border and rounding around the table |
/// | `scrollbars` | string | `both` | `both`, `vertical`, `horizontal`, or `none` |
/// | `header-row-height` | int | kit default | Header row height in px |
/// | `sortable` | bool | `true` | Whether column headers can sort at all |
/// | `col-movable` | bool | `true` | Whether column headers can be dragged to reorder |
/// | `col-resizable` | bool | `true` | Whether column edges can be dragged to resize |
/// | `row-selectable` | bool | `true` | Whether row clicking selects the row |
/// | `cell-selectable` | bool | `false` | Cell-level selection with a row-header column and keyboard nav |
/// | `fixed-cols-border` | bool | `true` | Whether fixed columns draw a right edge |
/// | `selected-row` | int | — | Read-back: the index of the last selected row |
/// | `selected-col` | int | — | Read-back: the index of the last selected column |
/// | `selected-cell` | string | — | Read-back: the last selected cell as `row,col` |
///
/// `<data-table-column>`: `field` (the row's field; required; `key` is
/// reserved by the `n:for` directive pass), `label` (default `field`),
/// `width` (px), `align` (`left`/`center`/`right`), `sortable`, `sort`
/// (initial direction: `ascending`/`descending`; the rows are pre-sorted),
/// `fixed` (`left`), `resizable`, `movable`.
///
/// # Out of scope
///
/// Kit capabilities the declarative system cannot express, kept visible
/// here rather than silently dropped:
///
/// * Infinite scroll (`has_more`/`load_more`/threshold) — `data` is a
///   snapshot the binding replaces wholesale.
/// * Fit-to-content column measurement (`fit_column`) and per-column
///   `max_width`.
/// * Row context menus (the delegate's `context_menu` seam) and multi-level
///   group headers.
/// * Custom row/cell decoration (`render_tr`/`render_td` beyond plain text);
///   declared-mode `<table>` is the path for live elements in cells.
/// * `scroll_bounce` (platform default), a `col-selectable` toggle, and
///   programmatic selection seeding.
#[derive(IntoElement)]
pub struct DataTable {
    source: BuiltComponent,
    state: Option<Entity<TableState<DataTableDelegate>>>,
}

impl DataTable {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            state: None,
        }
    }

    /// The table state entity prepared by the app's render pass.
    pub fn table_state(mut self, state: Entity<TableState<DataTableDelegate>>) -> Self {
        self.state = Some(state);
        self
    }

    fn size_of(value: Option<&str>) -> ComponentSize {
        match value {
            Some("xsmall") => ComponentSize::XSmall,
            Some("small") => ComponentSize::Small,
            Some("large") => ComponentSize::Large,
            _ => ComponentSize::Medium,
        }
    }
}

impl RenderOnce for DataTable {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(state) = self.state else {
            return div()
                .text_color(cx.theme().colors.muted_foreground)
                .child("Data table: no state")
                .into_any_element();
        };

        let props = &self.source.properties;
        let stripe = data_table_flag(props, "stripe", false);
        let bordered = data_table_flag(props, "bordered", true);
        let fixed_cols_border = data_table_flag(props, "fixed_cols_border", true);
        let header_row_height = props
            .get("header_row_height")
            .and_then(|v| v.as_i64())
            .map(|h| px(h as f32));

        // Virtual scrolling needs a definite height, like every other list
        // widget in the kit.
        let height = props.get("height").and_then(|v| v.as_f64()).unwrap_or(340.) as f32;

        let mut table = GpuiDataTable::new(&state)
            .stripe(stripe)
            .bordered(bordered)
            .fixed_cols_border(fixed_cols_border)
            .with_size(Self::size_of(props.get("size").and_then(|v| v.as_str())));
        if let Some(h) = header_row_height {
            table = table.header_row_height(h);
        }
        match props
            .get("scrollbars")
            .and_then(|v| v.as_str())
            .unwrap_or("both")
        {
            "none" => table = table.scrollbar_visible(false, false),
            "vertical" => table = table.scrollbar_visible(true, false),
            "horizontal" => table = table.scrollbar_visible(false, true),
            _ => {}
        }

        div().w_full().h(px(height)).child(table).into_any_element()
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    use std::collections::HashMap;

    fn column(
        key: &str,
        width: Option<i64>,
        sortable: bool,
        sort: Option<ColumnSort>,
    ) -> DataTableColumnData {
        DataTableColumnData {
            key: key.to_string(),
            label: key.to_string(),
            width,
            align: "left".to_string(),
            sortable,
            sort,
            fixed: None,
            resizable: true,
            movable: true,
        }
    }

    fn row(name: &str, age: i64) -> Value {
        Value::Object(
            [
                ("name".to_string(), Value::String(name.to_string())),
                ("age".to_string(), Value::Integer(age)),
            ]
            .into_iter()
            .collect(),
        )
    }

    fn names(rows: &[Value]) -> Vec<String> {
        rows.iter()
            .filter_map(|r| {
                r.as_object()
                    .and_then(|o| o.get("name"))
                    .and_then(|v| v.as_str().map(String::from))
            })
            .collect()
    }

    #[test]
    fn value_ordering_puts_nulls_then_booleans_then_numbers_then_strings() {
        use Value::*;
        assert_eq!(data_table_cmp_values(&Null, &Bool(false)), Ordering::Less);
        assert_eq!(
            data_table_cmp_values(&Bool(false), &Integer(0)),
            Ordering::Less
        );
        assert_eq!(
            data_table_cmp_values(&Integer(2), &Float(2.5)),
            Ordering::Less
        );
        assert_eq!(
            data_table_cmp_values(&Integer(7), &String("a".into())),
            Ordering::Less
        );
        assert_eq!(
            data_table_cmp_values(&String("a".into()), &String("b".into())),
            Ordering::Less
        );
        assert_eq!(
            data_table_cmp_values(&String("b".into()), &String("a".into())),
            Ordering::Greater
        );
    }

    #[test]
    fn rows_missing_the_field_sink_to_the_tail() {
        let key = "age";
        let mut rows = vec![row("a", 30), Value::String("no field".into()), row("b", 20)];
        data_table_sort_rows(&mut rows, key, Ordering::Less);
        let ages: Vec<i64> = rows
            .iter()
            .filter_map(|r| {
                r.as_object()
                    .and_then(|o| o.get("age"))
                    .and_then(|v| v.as_i64())
            })
            .collect();
        assert_eq!(ages, vec![20, 30], "numeric ascending first, unkeyed last");
    }

    #[test]
    fn sort_directions_reorder_rows() {
        let key = "name";
        let mut rows = vec![row("carol", 1), row("alice", 2), row("bob", 3)];
        data_table_sort_rows(&mut rows, key, Ordering::Less);
        assert_eq!(names(&rows), vec!["alice", "bob", "carol"]);

        data_table_sort_rows(&mut rows, key, Ordering::Greater);
        assert_eq!(names(&rows), vec!["carol", "bob", "alice"]);
    }

    #[test]
    fn signature_tracks_columns_and_behavior_flags() {
        let props = HashMap::new();
        let cols = vec![
            column("a", Some(120), true, None),
            column("b", None, false, None),
        ];
        let base = data_table_schema_signature(&cols, &props);

        let mut retitled = vec![column("a", Some(120), true, None)];
        retitled.push({
            let mut c = column("b", None, false, None);
            c.label = "Bee".into();
            c
        });
        let reordered = data_table_schema_signature(&retitled, &props);
        assert_ne!(base, reordered, "label changes ride in the schema");

        let same = data_table_schema_signature(&cols, &props);
        assert_eq!(base, same, "re-reading the same declaration is stable");

        let mut flagged = props.clone();
        flagged.insert("cell_selectable".into(), Value::Bool(true));
        assert_ne!(
            base,
            data_table_schema_signature(&cols, &flagged),
            "behavior flags ride in the schema"
        );
    }

    #[test]
    fn declared_initial_sort_pre_sorts_rows() {
        let cols = [
            {
                let mut c = column("name", None, true, None);
                c.sort = Some(ColumnSort::Ascending);
                c
            },
            column("age", None, false, None),
        ];
        let keys = vec!["name".to_string(), "age".to_string()];
        let delegate = DataTableDelegate::new(
            cols.iter()
                .map(|c| {
                    let mut col = Column::new(c.key.clone(), c.label.clone());
                    if let Some(ColumnSort::Ascending) = c.sort {
                        col = col.sortable().ascending();
                    } else {
                        col = col.width(px(100.));
                    }
                    col
                })
                .collect(),
            keys,
            vec![row("carol", 1), row("alice", 2), row("bob", 3)],
        );

        assert_eq!(names(&delegate.rows), vec!["alice", "bob", "carol"]);
    }
}
