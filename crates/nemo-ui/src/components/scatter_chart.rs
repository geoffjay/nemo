use gpui::*;
use gpui_component::plot::{
    scale::{Scale, ScaleLinear},
    Grid, AXIS_GAP,
};
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_macros::NemoComponent;

use super::chart_utils::{chart_color, empty_chart_placeholder, extract_data_array, get_f64_field};

/// A scatter chart plotting individual data points on a two-dimensional
/// numeric plane. Each point is rendered as a filled circle.
///
/// # XML Configuration
///
/// ```xml
/// <scatter-chart id="correlation" x-field="height" y-field="weight" dot-size="4" height="300">
///   <data bind="measurements" />
/// </scatter-chart>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `x-field` | string | Data field for x-axis values |
/// | `y-field` | string | Data field for y-axis values |
/// | `dot-size` | float | Radius of each data point |
/// | `height` | int | Chart height in pixels |
#[derive(IntoElement, NemoComponent)]
pub struct ScatterChart {
    #[property(default = "")]
    x_field: String,
    #[property(default = "")]
    y_field: String,
    #[property]
    dot_size: Option<f64>,
    #[property]
    _tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for ScatterChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let x_field = self.x_field.clone();
        let y_field = self.y_field.clone();
        let dot_size = self.dot_size.unwrap_or(4.0) as f32;

        let xs: Vec<f64> = data
            .iter()
            .map(|item| get_f64_field(item, &x_field))
            .collect();
        let ys: Vec<f64> = data
            .iter()
            .map(|item| get_f64_field(item, &y_field))
            .collect();

        let x_min = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let x_max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let y_min = ys.iter().copied().fold(f64::INFINITY, f64::min);
        let y_max = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        let border_color = cx.theme().border;
        let dot_color = chart_color(0, cx);

        ScatterElement {
            xs,
            ys,
            x_min,
            x_max,
            y_min,
            y_max,
            dot_size,
            border_color,
            dot_color,
        }
        .into_any_element()
    }
}

struct ScatterElement {
    xs: Vec<f64>,
    ys: Vec<f64>,
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    dot_size: f32,
    border_color: Hsla,
    dot_color: Hsla,
}

impl IntoElement for ScatterElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ScatterElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            ..Default::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        let width = bounds.size.width.as_f32() - AXIS_GAP;
        let height = bounds.size.height.as_f32() - AXIS_GAP;
        let left_margin = AXIS_GAP;
        let top_margin = 10.0_f32;

        let x_pad = (self.x_max - self.x_min).max(1.0) * 0.05;
        let y_pad = (self.y_max - self.y_min).max(1.0) * 0.05;

        let x_scale = ScaleLinear::new(
            vec![self.x_min - x_pad, self.x_max + x_pad],
            vec![0., width],
        );
        let y_scale = ScaleLinear::new(
            vec![self.y_min - y_pad, self.y_max + y_pad],
            vec![height, top_margin],
        );

        // Grid
        Grid::new()
            .x((0..=4)
                .map(|i| left_margin + width * i as f32 / 4.0)
                .collect())
            .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
            .stroke(self.border_color)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        // Draw dots
        let origin = bounds.origin;
        let r = self.dot_size;
        for i in 0..self.xs.len() {
            let px_x = x_scale.tick(&self.xs[i]).unwrap_or(0.0);
            let px_y = y_scale.tick(&self.ys[i]).unwrap_or(0.0);

            let center = point(px(left_margin + px_x) + origin.x, px(px_y) + origin.y);

            let dot_bounds = Bounds::from_corners(
                point(center.x - px(r), center.y - px(r)),
                point(center.x + px(r), center.y + px(r)),
            );
            window.paint_quad(PaintQuad {
                bounds: dot_bounds,
                corner_radii: Corners::all(px(r)),
                background: self.dot_color.into(),
                border_widths: Edges::default(),
                border_color: transparent_black(),
                border_style: Default::default(),
            });
        }
    }
}
