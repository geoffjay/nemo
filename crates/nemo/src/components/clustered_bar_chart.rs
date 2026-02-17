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

/// A clustered (grouped) bar chart with horizontal bars.
#[derive(IntoElement, NemoComponent)]
pub struct ClusteredBarChart {
    #[property(default = "")]
    y_field: String,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for ClusteredBarChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let x_fields = get_string_array(&self.source, "x_fields");
        if x_fields.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let y_field = self.y_field.clone();
        let tick_margin = self.tick_margin.unwrap_or(1) as usize;
        let colors: Vec<Hsla> = (0..x_fields.len()).map(|i| chart_color(i, cx)).collect();

        let y_labels: Vec<String> = data
            .iter()
            .map(|item| get_string_field(item, &y_field))
            .collect();

        let series_values: Vec<Vec<f64>> = x_fields
            .iter()
            .map(|field| data.iter().map(|item| get_f64_field(item, field)).collect())
            .collect();

        let max_x = series_values
            .iter()
            .flat_map(|s| s.iter().copied())
            .fold(0.0_f64, f64::max);

        let border_color = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;

        ClusteredBarElement {
            y_labels,
            series_values,
            max_x,
            colors,
            tick_margin,
            border_color,
            muted_fg,
        }
        .into_any_element()
    }
}

struct ClusteredBarElement {
    y_labels: Vec<String>,
    series_values: Vec<Vec<f64>>,
    max_x: f64,
    colors: Vec<Hsla>,
    tick_margin: usize,
    border_color: Hsla,
    muted_fg: Hsla,
}

impl IntoElement for ClusteredBarElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ClusteredBarElement {
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
        let width = bounds.size.width.as_f32() - AXIS_GAP;
        let height = bounds.size.height.as_f32() - AXIS_GAP;
        let left_margin = AXIS_GAP;
        let n_series = self.series_values.len();
        if n_series == 0 {
            return;
        }

        let y_scale = ScaleBand::new(self.y_labels.clone(), vec![0., height])
            .padding_inner(0.3)
            .padding_outer(0.2);
        let band_height = y_scale.band_width();
        let sub_height = band_height / n_series as f32;

        let x_scale = ScaleLinear::new(vec![0.0_f64, self.max_x], vec![0., width]);

        // Y axis labels
        let y_label = self.y_labels.iter().enumerate().filter_map(|(i, label)| {
            if (i + 1) % self.tick_margin == 0 {
                y_scale.tick(label).map(|y_tick| {
                    AxisText::new(label.clone(), y_tick + band_height / 2., self.muted_fg)
                })
            } else {
                None
            }
        });
        PlotAxis::new()
            .x(height)
            .x_label(y_label)
            .stroke(self.border_color)
            .paint(&bounds, window, cx);

        // Grid (vertical)
        Grid::new()
            .x((0..=4)
                .map(|i| left_margin + width * i as f32 / 4.0)
                .collect())
            .stroke(self.border_color)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        // Draw clustered horizontal bars
        let origin = bounds.origin;
        for (si, series) in self.series_values.iter().enumerate() {
            let color = self.colors[si % self.colors.len()];
            for (idx, &val) in series.iter().enumerate() {
                if idx >= self.y_labels.len() {
                    continue;
                }
                if let Some(y_tick) = y_scale.tick(&self.y_labels[idx]) {
                    let sub_y = y_tick + sub_height * si as f32;
                    let px_val = x_scale.tick(&val).unwrap_or(0.0);

                    let p1 = point(px(left_margin) + origin.x, px(sub_y) + origin.y);
                    let p2 = point(
                        px(left_margin + px_val) + origin.x,
                        px(sub_y + sub_height * 0.85) + origin.y,
                    );

                    window.paint_quad(fill(Bounds::from_corners(p1, p2), color));
                }
            }
        }
    }
}
