use gpui::*;
use nemo_macros::NemoComponent;

/// A flex container component for arranging children horizontally or vertically.
///
/// # XML Configuration
///
/// ```xml
/// <stack id="main" direction="horizontal" spacing="8" scroll="true">
///   <!-- child elements -->
/// </stack>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `direction` | string | Layout direction: `"horizontal"` or `"vertical"` (default) |
/// | `spacing` | int | Gap between children in pixels (default: 4) |
/// | `scroll` | bool | Enable scrolling along the layout axis (also grows) |
/// | `flex` | bool/int | Grow to fill the main axis (`"1"`/`"true"`); content-sized otherwise |
/// | `align` | string | Cross-axis alignment: `start`/`center`/`end`/`stretch` (default: `center` for horizontal, `stretch` for vertical) |
/// | `justify` | string | Main-axis alignment: `start`/`center`/`end`/`between`/`around` |
/// | `wrap` | bool | Wrap items onto new rows/columns instead of one line |
///
/// A stack is **content-sized** by default and grows only when it has a truthy
/// `flex`, has `scroll="true"`, or is the layout root.
#[derive(IntoElement, NemoComponent)]
pub struct Stack {
    #[property(default = "vertical")]
    direction: String,
    #[property(default = 4)]
    spacing: i64,
    #[property]
    scroll: Option<bool>,
    /// Wrap items onto new rows/columns instead of a single line (icon grids,
    /// tag clouds). Combines with `direction`: `horizontal` wraps to new rows.
    #[property]
    wrap: Option<bool>,
    /// CSS-grid mode: lay children out in exactly N equal columns (`.grid()`
    /// + `grid_cols(n)`). Column count is exact at any container width.
    #[property]
    columns: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Stack {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let gap = px(self.spacing as f32);
        let is_horizontal = self.direction == "horizontal";
        let scroll = self.scroll == Some(true);
        let props = &self.source.properties;
        let columns = self.columns.filter(|c| *c > 0).map(|c| c as u16);

        // CSS-grid mode: exactly N equal columns at any container width;
        // `spacing` becomes the column gap and `scroll` still scrolls.
        if let Some(cols) = columns {
            let mut grid = div()
                .id(SharedString::from(self.source.id.clone()))
                .grid()
                .grid_cols(cols)
                .gap(gap);
            if crate::components::container_grows(props) {
                grid = grid.flex_1().min_h(px(0.));
            }
            if scroll {
                grid = grid.overflow_y_scroll();
            }
            return grid.children(self.children).into_any_element();
        }

        // Flexbox-native sizing: a stack is content-sized by default and only
        // grows along its main axis when it opts in (truthy `flex`) or scrolls.
        let grows = crate::components::container_grows(props);
        let align = props.get("align").and_then(|v| v.as_str());
        let justify = props.get("justify").and_then(|v| v.as_str());

        let mut base = div()
            .id(SharedString::from(self.source.id.clone()))
            .flex()
            .gap(gap);

        if grows {
            base = base.flex_1().min_h(px(0.));
        }

        base = if is_horizontal {
            base.flex_row()
        } else {
            base.flex_col()
        };

        // Cross-axis alignment. Default: center for rows, stretch for columns.
        base = match align {
            Some("start") => base.items_start(),
            Some("center") => base.items_center(),
            Some("end") => base.items_end(),
            Some("stretch") => base.items_stretch(),
            _ if is_horizontal => base.items_center(),
            _ => base.items_stretch(),
        };

        // Main-axis justification (opt-in).
        base = match justify {
            Some("start") => base.justify_start(),
            Some("center") => base.justify_center(),
            Some("end") => base.justify_end(),
            Some("between") => base.justify_between(),
            Some("around") => base.justify_around(),
            _ => base,
        };

        if scroll {
            base = if is_horizontal {
                base.overflow_x_scroll()
            } else {
                base.overflow_y_scroll()
            };
        }

        if self.wrap == Some(true) {
            base = base.flex_wrap();
        }

        base.children(self.children).into_any_element()
    }
}
