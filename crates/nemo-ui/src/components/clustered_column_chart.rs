use gpui::*;
use gpui_component::plot::{
    scale::{Scale, ScaleBand, ScaleLinear},
    AxisText, Grid, PlotAxis, AXIS_GAP,
};
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_array,
    get_string_field,
};

/// A clustered (grouped) column chart: multiple series rendered side-by-side
/// within each category band.
///
/// # XML Configuration
///
/// ```xml
/// <clustered-column-chart id="comparison" x-field="month" y-fields='["2024","2025"]'
///   tick-margin="20" height="300">
///   <data bind="comparisonData" />
/// </clustered-column-chart>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `x-field` | string | Data field for the x-axis categories |
/// | `y-fields` | JSON array | Array of data field names for grouped series |
/// | `tick-margin` | int | Margin for axis tick labels |
/// | `height` | int | Chart height in pixels |
#[derive(IntoElement, NemoComponent)]
pub struct ClusteredColumnChart {
    #[property(default = "")]
    x_field: String,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for ClusteredColumnChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let y_fields = get_string_array(&self.source, "y_fields");
        if y_fields.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let x_field = self.x_field.clone();
        let tick_margin = self.tick_margin.unwrap_or(1) as usize;
        let colors: Vec<Hsla> = (0..y_fields.len()).map(|i| chart_color(i, cx)).collect();

        let x_labels: Vec<String> = data
            .iter()
            .map(|item| get_string_field(item, &x_field))
            .collect();

        let series_values: Vec<Vec<f64>> = y_fields
            .iter()
            .map(|field| data.iter().map(|item| get_f64_field(item, field)).collect())
            .collect();

        let max_y = series_values
            .iter()
            .flat_map(|s| s.iter().copied())
            .fold(0.0_f64, f64::max);

        let border_color = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;

        ClusteredColumnElement {
            x_labels,
            series_values,
            max_y,
            colors,
            tick_margin,
            border_color,
            muted_fg,
        }
        .into_any_element()
    }
}

struct ClusteredColumnElement {
    x_labels: Vec<String>,
    series_values: Vec<Vec<f64>>,
    max_y: f64,
    colors: Vec<Hsla>,
    tick_margin: usize,
    border_color: Hsla,
    muted_fg: Hsla,
}

impl IntoElement for ClusteredColumnElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ClusteredColumnElement {
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
        cx: &mut App,
    ) {
        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32() - AXIS_GAP;
        let n_series = self.series_values.len();
        if n_series == 0 {
            return;
        }

        let x_scale = ScaleBand::new(self.x_labels.clone(), vec![0., width])
            .padding_inner(0.3)
            .padding_outer(0.2);
        let band_width = x_scale.band_width();
        let sub_width = band_width / n_series as f32;

        let y_scale = ScaleLinear::new(vec![0.0_f64, self.max_y], vec![height, 10.]);

        // X axis
        let x_label = self.x_labels.iter().enumerate().filter_map(|(i, label)| {
            if (i + 1) % self.tick_margin == 0 {
                x_scale.tick(label).map(|x_tick| {
                    AxisText::new(label.clone(), x_tick + band_width / 2., self.muted_fg)
                        .align(TextAlign::Center)
                })
            } else {
                None
            }
        });
        PlotAxis::new()
            .x(height)
            .x_label(x_label)
            .stroke(self.border_color)
            .paint(&bounds, window, cx);

        // Grid
        Grid::new()
            .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
            .stroke(self.border_color)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        // Draw clustered bars
        let origin = bounds.origin;
        let y_zero = y_scale.tick(&0.0_f64).unwrap_or(height);
        for (si, series) in self.series_values.iter().enumerate() {
            let color = self.colors[si % self.colors.len()];
            for (idx, &val) in series.iter().enumerate() {
                if idx >= self.x_labels.len() {
                    continue;
                }
                if let Some(x_tick) = x_scale.tick(&self.x_labels[idx]) {
                    let sub_x = x_tick + sub_width * si as f32;
                    let py = y_scale.tick(&val).unwrap_or(height);

                    let p1 = point(px(sub_x) + origin.x, px(py) + origin.y);
                    let p2 = point(
                        px(sub_x + sub_width * 0.85) + origin.x,
                        px(y_zero) + origin.y,
                    );

                    window.paint_quad(fill(Bounds::from_corners(p1, p2), color));
                }
            }
        }
    }
}
