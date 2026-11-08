use gpui::{
    div, px, App, Entity, IntoElement, ParentElement as _, RenderOnce, Styled as _, Window,
};
use gpui_kit::component::diff::{
    Diff as KitDiff, DiffChangeIndicator, DiffHoverHighlight, DiffHunkSeparator, DiffInlineUnit,
    DiffMode, DiffState,
};
use gpui_kit::component::ActiveTheme as _;
use nemo_config::Value;
use nemo_layout::BuiltComponent;

use crate::components::data_table::data_table_flag;

/// How a hunk's start is marked, from the `hunk-separator` attribute:
/// `metadata` (the `@@` header, default), `line-info` (count of unsupplied
/// lines), or `simple` (a thin divider).
pub fn diff_hunk_separator_of(value: Option<&str>) -> DiffHunkSeparator {
    match value.map(str::trim) {
        Some("line-info") => DiffHunkSeparator::LineInfo,
        Some("simple") => DiffHunkSeparator::Simple,
        _ => DiffHunkSeparator::Metadata,
    }
}

/// How changed lines are marked, from `change-indicator`:
/// `signs` (`+`/`−`, default), `bars` (a colored bar), or `none`.
pub fn diff_change_indicator_of(value: Option<&str>) -> DiffChangeIndicator {
    match value.map(str::trim) {
        Some("bars") => DiffChangeIndicator::Bars,
        Some("none") => DiffChangeIndicator::None,
        _ => DiffChangeIndicator::Signs,
    }
}

/// What a row emphasizes under the pointer, from `hover-highlight`:
/// `none` (default), `line`, `line-number`, or `both`.
pub fn diff_hover_highlight_of(value: Option<&str>) -> DiffHoverHighlight {
    match value.map(str::trim) {
        Some("line") => DiffHoverHighlight::Line,
        Some("line-number") => DiffHoverHighlight::LineNumber,
        Some("both") => DiffHoverHighlight::Both,
        _ => DiffHoverHighlight::None,
    }
}

/// Splits the two sides of a paired diff: `unified` (one column, default) or
/// `split` (side by side), from `mode`.
pub fn diff_mode_of(value: Option<&str>) -> DiffMode {
    match value.map(str::trim) {
        Some("split") => DiffMode::Split,
        _ => DiffMode::Unified,
    }
}

/// The inline-change emphasis unit, from `inline-emphasis`: `word` (words and
/// punctuation, default), `character`/`char` (graphemes), or `none` (whole
/// lines only).
pub fn diff_inline_unit_of(value: Option<&str>) -> Option<DiffInlineUnit> {
    match value.map(str::trim) {
        Some("character") | Some("char") => Some(DiffInlineUnit::Character),
        Some("none") => None,
        _ => Some(DiffInlineUnit::Word),
    }
}

/// Reads the `context-lines` attribute: 3 by default; a negative value means
/// "no collapse — show every line the patch supplies".
pub fn diff_context_lines_of(
    properties: &std::collections::HashMap<String, Value>,
) -> Option<usize> {
    match properties
        .get("context_lines")
        .and_then(|v| v.as_i64())
        .unwrap_or(3)
    {
        negative if negative < 0 => None,
        lines => Some(lines.max(0) as usize),
    }
}

/// The declared state configuration's identity. The kit bakes the mode,
/// context, expansion, and inline-emphasis into its row model at creation
/// (layout-ready), so a change to any of them rebuilds the state; changing
/// the patch text alone does not.
pub fn diff_schema_signature(properties: &std::collections::HashMap<String, Value>) -> String {
    let mode = properties
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("unified");
    let context = match diff_context_lines_of(properties) {
        Some(lines) => lines.to_string(),
        None => "all".to_string(),
    };
    let expansion = properties
        .get("expansion_lines")
        .and_then(|v| v.as_i64())
        .unwrap_or(20);
    let min_collapsed = properties
        .get("min_collapsed_lines")
        .and_then(|v| v.as_i64())
        .unwrap_or(2);
    let inline = properties
        .get("inline_emphasis")
        .and_then(|v| v.as_str())
        .unwrap_or("word");
    format!("mode={mode}|context={context}|expansion={expansion}|min_collapsed={min_collapsed}|inline={inline}")
}

/// Displays a selected line range for the `selected-lines` read-back
/// property: `path:start-end`.
pub fn diff_selection_display(range: Option<&gpui_kit::component::diff::DiffLineRange>) -> String {
    match range {
        Some(range) => format!("{}:{}-{}", range.path(), range.start(), range.end()),
        None => String::new(),
    }
}

/// A readonly source-diff viewer: parses a unified (or Git) diff patch into
/// files, hunks, and paired lines with a virtualized row list, syntax
/// emphasis for the detected language, file headers with change counts, and
/// gutter-driven line selection (compare sides, then act on
/// `on-selection-changed`). Modes: one unified column or split side by side.
///
/// # XML Configuration
///
/// ```xml
/// <diff id="review" diff="{{ patch }}" height="420" mode="split"
///   hover-highlight="line" on-selection-changed="handle_select" />
/// ```
///
/// The patch text comes from the `diff` property — put it in a `<script>`
/// variable (the example pages do) rather than a literal attribute, since
/// XML attribute normalization folds newlines to spaces and a patch is one
/// line per line. The patch is reloaded in place when the property changes
/// (selection resets).
///
/// # Properties
///
/// | Property | Type | Default | Description |
/// |----------|------|---------|-------------|
/// | `diff` | string | — | The unified-diff patch text |
/// | `height` | int | `480` | View height in px (the row list needs a definite height) |
/// | `mode` | string | `unified` | `unified` (one column) or `split` (side by side) |
/// | `context-lines` | int | `3` | Unchanged context around changes; negative shows every supplied line |
/// | `expansion-lines` | int | `20` | Rows added per expand click in collapsed context |
/// | `min-collapsed-lines` | int | `2` | Hunk gap collapses only when it exceeds this |
/// | `inline-emphasis` | string | `word` | Paired-line change emphasis: `word`, `character`, or `none` |
/// | `line-number` | bool | `true` | Original/modified line-number lanes |
/// | `syntax-highlight` | bool | `true` | Syntax emphasis (language detected from the path) |
/// | `header-visible` | bool | `true` | Header above each file — path, status, change counts |
/// | `hover-highlight` | string | `none` | Pointer emphasis: `none`, `line`, `line-number`, `both` |
/// | `hunk-separator` | string | `metadata` | Hunk start: `metadata`, `line-info`, or `simple` |
/// | `change-indicator` | string | `signs` | Changed-line marker: `signs`, `bars`, or `none` |
/// | `change-background` | bool | `true` | Tints changed lines (inline changes stay emphasized) |
/// | `soft-wrap` | bool | `false` | Wraps long lines instead of horizontal scrolling |
/// | `selected-lines` | string | — | Read-back: the selected range as `path:start-end` |
///
/// # Events
///
/// `on-selection-changed` fires as a gutter/gesture selection moves and when
/// it ends, with the range as `path:start-end` (empty string on clear).
///
/// # Out of scope
///
/// Kit capabilities the declarative system cannot express, kept visible
/// here rather than silently dropped:
///
/// * Annotations (`DiffAnnotation`): application-owned content attached to
///   lines or files, with its customization renderers.
/// * Custom header/row rendering (`header`, `header_prefix`, … builder
///   seams) — headers are the kit's.
/// * Per-line `on_line_click` / `on_line_hover` handlers and annotation
///   click-to-add.
/// * Merge-conflict resolution presentation (`resolve_conflict`) needs an
///   interactive widget seam the config system does not author.
/// * Imperative navigation (`scroll_to_line`, `scroll_to_file`,
///   `next_change` / `previous_change`) and `selected_text` extraction.
/// * `inline_max_line_length` / `syntax_max_line_length` tuning (kit
///   defaults stand) and forcing a syntax language
///   (`DiffFile::with_language` — detection is from the path).
/// * File expansion/collapse, patch truncation relaxations, and binary-file
///   notices are kit-side view state.
#[derive(IntoElement)]
pub struct Diff {
    source: BuiltComponent,
    state: Option<Entity<DiffState>>,
    parse_error: Option<String>,
}

impl Diff {
    pub fn new(source: BuiltComponent) -> Self {
        Self {
            source,
            state: None,
            parse_error: None,
        }
    }

    /// The diff state entity prepared by the app's render pass.
    pub fn diff_state(mut self, state: Entity<DiffState>) -> Self {
        self.state = Some(state);
        self
    }

    /// Sets the message shown in place of the view when the patch could not
    /// be parsed.
    pub fn parse_error(mut self, error: Option<String>) -> Self {
        self.parse_error = error;
        self
    }
}

impl RenderOnce for Diff {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(state) = self.state else {
            return div()
                .text_color(cx.theme().colors.muted_foreground)
                .child("Diff: no state")
                .into_any_element();
        };

        if let Some(error) = &self.parse_error {
            return div()
                .text_color(cx.theme().colors.muted_foreground)
                .child(format!("Diff: {error}"))
                .into_any_element();
        }

        let props = &self.source.properties;
        let view = KitDiff::new(&state)
            .line_number(data_table_flag(props, "line_number", true))
            .syntax_highlight(data_table_flag(props, "syntax_highlight", true))
            .header_visible(data_table_flag(props, "header_visible", true))
            .change_background(data_table_flag(props, "change_background", true))
            .soft_wrap(data_table_flag(props, "soft_wrap", false))
            .hover_highlight(diff_hover_highlight_of(
                props.get("hover_highlight").and_then(|v| v.as_str()),
            ))
            .hunk_separator(diff_hunk_separator_of(
                props.get("hunk_separator").and_then(|v| v.as_str()),
            ))
            .change_indicator(diff_change_indicator_of(
                props.get("change_indicator").and_then(|v| v.as_str()),
            ));

        // The row list is virtualized; a definite height is required.
        let height = props.get("height").and_then(|v| v.as_i64()).unwrap_or(480) as f32;

        div().w_full().h(px(height)).child(view).into_any_element()
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    use gpui_kit::component::diff::DiffFile;
    use std::collections::HashMap;

    #[test]
    fn attribute_mappers_cover_every_variant() {
        // Mode.
        assert!(matches!(diff_mode_of(None), DiffMode::Unified));
        assert!(matches!(diff_mode_of(Some("split")), DiffMode::Split));
        assert!(matches!(diff_mode_of(Some("unified")), DiffMode::Unified));

        // Hunk separators and change indicators.
        assert!(matches!(
            diff_hunk_separator_of(Some("line-info")),
            DiffHunkSeparator::LineInfo
        ));
        assert!(matches!(
            diff_hunk_separator_of(Some("simple")),
            DiffHunkSeparator::Simple
        ));
        assert!(matches!(
            diff_hunk_separator_of(None),
            DiffHunkSeparator::Metadata
        ));
        assert!(matches!(
            diff_change_indicator_of(Some("bars")),
            DiffChangeIndicator::Bars
        ));
        assert!(matches!(
            diff_change_indicator_of(Some("none")),
            DiffChangeIndicator::None
        ));
        assert!(matches!(
            diff_change_indicator_of(None),
            DiffChangeIndicator::Signs
        ));

        // Hover highlights.
        assert!(matches!(
            diff_hover_highlight_of(Some("both")),
            DiffHoverHighlight::Both
        ));
        assert!(matches!(
            diff_hover_highlight_of(Some("line-number")),
            DiffHoverHighlight::LineNumber
        ));
        assert!(matches!(
            diff_hover_highlight_of(None),
            DiffHoverHighlight::None
        ));

        // Inline emphasis.
        assert!(matches!(
            diff_inline_unit_of(Some("character")),
            Some(DiffInlineUnit::Character)
        ));
        assert!(matches!(
            diff_inline_unit_of(None),
            Some(DiffInlineUnit::Word)
        ));
        assert!(diff_inline_unit_of(Some("none")).is_none());
    }

    #[test]
    fn context_lines_read_negative_as_unbounded() {
        let mut props = HashMap::new();
        assert_eq!(diff_context_lines_of(&props), Some(3), "default is 3");

        props.insert("context_lines".into(), Value::Integer(0));
        assert_eq!(diff_context_lines_of(&props), Some(0));

        props.insert("context_lines".into(), Value::Integer(-1));
        assert_eq!(diff_context_lines_of(&props), None);
    }

    #[test]
    fn signature_tracks_state_configuration_only() {
        let properties = HashMap::new();
        let base = diff_schema_signature(&properties);

        // Presentation-only changes must not rebuild the state.
        let mut presentation = properties.clone();
        presentation.insert("line_number".into(), Value::Bool(false));
        presentation.insert("soft_wrap".into(), Value::Bool(true));
        presentation.insert("mode_is_absent".into(), Value::Bool(true));
        assert_eq!(base, diff_schema_signature(&presentation));

        // State configuration changes ride in the signature.
        let mut mode = properties.clone();
        mode.insert("mode".into(), Value::String("split".into()));
        assert_ne!(base, diff_schema_signature(&mode));

        let mut context = properties.clone();
        context.insert("context_lines".into(), Value::Integer(-1));
        assert_ne!(base, diff_schema_signature(&context));
    }

    #[test]
    fn selection_display_matches_the_documented_shape() {
        let range = gpui_kit::component::diff::DiffLineRange::new(
            "src/main.rs",
            gpui_kit::component::diff::DiffSide::Modified,
            3,
            9,
        );
        assert_eq!(diff_selection_display(Some(&range)), "src/main.rs:3-9");
        assert_eq!(diff_selection_display(None), "");
    }

    #[test]
    fn patch_text_parses_and_reports_errors_located() {
        let patch = concat!(
            "--- a/src/main.rs\n",
            "+++ b/src/main.rs\n",
            "@@ -1,3 +1,3 @@\n",
            " fn main() {\n",
            "-    a();\n",
            "+    b();\n",
            " }\n",
        );
        let files = DiffFile::parse(patch).expect("a well-formed patch parses");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path(), "src/main.rs");

        // A malformed hunk reports its one-based patch line.
        let broken = "--- a/x\n+++ b/x\n@@ -1,1 +1,1 @@\n";
        let error = match DiffFile::parse(broken) {
            Ok(_) => panic!("a hunk without body lines is malformed"),
            Err(error) => error,
        };
        assert!(error.line() >= 1, "the error carries its patch line");
        assert!(error.to_string().contains("patch line"));
    }
}
