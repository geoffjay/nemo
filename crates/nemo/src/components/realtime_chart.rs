use gpui::*;
use gpui_component::plot::{
    scale::{Scale, ScaleLinear, ScalePoint},
    shape::Line,
    AxisText, Grid, IntoPlot, Plot, PlotAxis, StrokeStyle, AXIS_GAP,
};
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_config::Value;
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_array,
    get_string_field,
};

#[derive(IntoElement, NemoComponent)]
pub struct RealtimeChart {
    #[property(default = "")]
    x_field: String,
    #[property]
    linear: Option<bool>,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for RealtimeChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let y_fields = get_string_array(&self.source, "y_fields");
        if y_fields.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let stroke_style = if self.linear == Some(true) {
            StrokeStyle::Linear
        } else {
            StrokeStyle::Natural
        };
        let tick_margin = self.tick_margin.unwrap_or(1) as usize;

        let series: Vec<(String, Hsla)> = y_fields
            .into_iter()
            .enumerate()
            .map(|(i, field)| (field, chart_color(i, cx)))
            .collect();

        MultiLineChartInner {
            data,
            x_field: self.x_field,
            series,
            stroke_style,
            tick_margin,
        }
        .into_any_element()
    }
}

/// Inner chart that implements Plot for multi-line rendering.
#[derive(IntoPlot)]
struct MultiLineChartInner {
    data: Vec<Value>,
    x_field: String,
    series: Vec<(String, Hsla)>,
    stroke_style: StrokeStyle,
    tick_margin: usize,
}

impl Plot for MultiLineChartInner {
    fn paint(&mut self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if self.series.is_empty() {
            return;
        }

        let width = bounds.size.width.as_f32();
        let height = bounds.size.height.as_f32() - AXIS_GAP;

        // X scale
        let x_field = self.x_field.clone();
        let x = ScalePoint::new(
            self.data
                .iter()
                .map(|v| get_string_field(v, &x_field))
                .collect(),
            vec![0., width],
        );

        // Y scale: compute domain from all series values
        let y_values: Vec<f64> = self
            .data
            .iter()
            .flat_map(|item| {
                self.series
                    .iter()
                    .map(move |(field, _)| get_f64_field(item, field))
            })
            .chain(Some(0.0))
            .collect();
        let y = ScaleLinear::new(y_values, vec![height, 10.]);

        // Draw X axis
        let data_len = self.data.len();
        let x_label = self.data.iter().enumerate().filter_map(|(i, d)| {
            if (i + 1) % self.tick_margin == 0 {
                let label = get_string_field(d, &self.x_field);
                x.tick(&label).map(|x_tick| {
                    let align = match i {
                        0 if data_len == 1 => TextAlign::Center,
                        0 => TextAlign::Left,
                        i if i == data_len - 1 => TextAlign::Right,
                        _ => TextAlign::Center,
                    };
                    AxisText::new(SharedString::from(label), x_tick, cx.theme().muted_foreground).align(align)
                })
            } else {
                None
            }
        });

        PlotAxis::new()
            .x(height)
            .x_label(x_label)
            .stroke(cx.theme().border)
            .paint(&bounds, window, cx);

        // Draw grid
        Grid::new()
            .y((0..=3).map(|i| height * i as f32 / 4.0).collect())
            .stroke(cx.theme().border)
            .dash_array(&[px(4.), px(2.)])
            .paint(&bounds, window);

        // Draw one Line per series
        for (field, color) in &self.series {
            let x = x.clone();
            let y = y.clone();
            let x_field = self.x_field.clone();
            let field = field.clone();

            Line::new()
                .data(&self.data)
                .x(move |d| x.tick(&get_string_field(d, &x_field)))
                .y(move |d| y.tick(&get_f64_field(d, &field)))
                .stroke(*color)
                .stroke_style(self.stroke_style)
                .stroke_width(2.)
                .paint(&bounds, window);
        }
    }
}
