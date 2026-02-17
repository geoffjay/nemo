use gpui::*;
use gpui_component::plot::{
    scale::{Scale, ScaleBand, ScaleLinear},
    shape::Stack,
    AxisText, Grid, PlotAxis, AXIS_GAP,
};
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_config::Value;
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_array,
    get_string_field,
};

/// A stacked column chart: multiple series stacked vertically within each
/// category band, using the low-level Plot primitives.
#[derive(IntoElement, NemoComponent)]
pub struct StackedColumnChart {
    #[property(default = "")]
    x_field: String,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for StackedColumnChart {
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

        let series = Stack::new()
            .data(data.clone())
            .keys(y_fields.clone())
            .value(move |item: &Value, key: &str| Some(get_f64_field(item, key) as f32))
            .series();

        let max_y = series
            .iter()
            .flat_map(|s| s.points.iter().map(|p| p.y1 as f64))
            .fold(0.0_f64, f64::max);

        let border_color = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;

        let x_labels: Vec<String> = data
            .iter()
            .map(|item| get_string_field(item, &x_field))
            .collect();

        StackedColumnElement {
            series,
            x_labels,
            max_y,
            colors,
            tick_margin,
            border_color,
            muted_fg,
        }
        .into_any_element()
    }
}

struct StackedColumnElement {
    series: Vec<gpui_component::plot::shape::StackSeries<Value>>,
    x_labels: Vec<String>,
    max_y: f64,
    colors: Vec<Hsla>,
    tick_margin: usize,
    border_color: Hsla,
    muted_fg: Hsla,
}

impl IntoElement for StackedColumnElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for StackedColumnElement {
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

        let x_scale = ScaleBand::new(self.x_labels.clone(), vec![0., width])
            .padding_inner(0.4)
            .padding_outer(0.2);
        let band_width = x_scale.band_width();
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

        // Draw stacked bars
        let origin = bounds.origin;
        for (si, s) in self.series.iter().enumerate() {
            let color = self.colors[si % self.colors.len()];
            for (idx, pt) in s.points.iter().enumerate() {
                if idx >= self.x_labels.len() {
                    continue;
                }
                if let Some(x_tick) = x_scale.tick(&self.x_labels[idx]) {
                    let py0 = y_scale.tick(&(pt.y0 as f64)).unwrap_or(height);
                    let py1 = y_scale.tick(&(pt.y1 as f64)).unwrap_or(height);

                    let p1 = point(px(x_tick) + origin.x, px(py1) + origin.y);
                    let p2 = point(px(x_tick + band_width) + origin.x, px(py0) + origin.y);

                    window.paint_quad(fill(Bounds::from_corners(p1, p2), color));
                }
            }
        }
    }
}
