use gpui::*;
use gpui_component::plot::{
    scale::{Scale, ScaleBand},
    AxisText, PlotAxis, AXIS_GAP,
};
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_macros::NemoComponent;

use super::chart_utils::{
    empty_chart_placeholder, extract_data_array, get_f64_field, get_string_field,
};

/// A heatmap chart rendering a grid of coloured cells. Each data item
/// specifies an x category, y category, and a numeric value that is mapped
/// to a colour intensity.
#[derive(IntoElement, NemoComponent)]
pub struct HeatmapChart {
    #[property(default = "")]
    x_field: String,
    #[property(default = "")]
    y_field: String,
    #[property(default = "")]
    value_field: String,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for HeatmapChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let x_field = self.x_field.clone();
        let y_field = self.y_field.clone();
        let value_field = self.value_field.clone();
        let tick_margin = self.tick_margin.unwrap_or(1) as usize;

        // Collect unique categories preserving order
        let mut x_cats: Vec<String> = Vec::new();
        let mut y_cats: Vec<String> = Vec::new();
        let mut vals: Vec<f32> = Vec::new();
        let mut cell_map: Vec<(usize, usize, f32)> = Vec::new();

        for item in &data {
            let xc = get_string_field(item, &x_field);
            let yc = get_string_field(item, &y_field);
            let v = get_f64_field(item, &value_field) as f32;

            let xi = if let Some(pos) = x_cats.iter().position(|c| c == &xc) {
                pos
            } else {
                x_cats.push(xc);
                x_cats.len() - 1
            };
            let yi = if let Some(pos) = y_cats.iter().position(|c| c == &yc) {
                pos
            } else {
                y_cats.push(yc);
                y_cats.len() - 1
            };

            vals.push(v);
            cell_map.push((xi, yi, v));
        }

        let v_min = vals.iter().copied().fold(f32::INFINITY, f32::min);
        let v_max = vals.iter().copied().fold(f32::NEG_INFINITY, f32::max);

        let border_color = cx.theme().border;
        let muted_fg = cx.theme().muted_foreground;
        let low_color = cx.theme().colors.chart_5;
        let high_color = cx.theme().colors.chart_1;

        HeatmapElement {
            x_cats,
            y_cats,
            cell_map,
            v_min,
            v_max,
            tick_margin,
            border_color,
            muted_fg,
            low_color,
            high_color,
        }
        .into_any_element()
    }
}

struct HeatmapElement {
    x_cats: Vec<String>,
    y_cats: Vec<String>,
    cell_map: Vec<(usize, usize, f32)>,
    v_min: f32,
    v_max: f32,
    tick_margin: usize,
    border_color: Hsla,
    muted_fg: Hsla,
    low_color: Hsla,
    high_color: Hsla,
}

impl IntoElement for HeatmapElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for HeatmapElement {
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

        let x_scale = ScaleBand::new(self.x_cats.clone(), vec![0., width])
            .padding_inner(0.05)
            .padding_outer(0.05);
        let y_scale = ScaleBand::new(self.y_cats.clone(), vec![0., height])
            .padding_inner(0.05)
            .padding_outer(0.05);

        let x_band = x_scale.band_width();
        let y_band = y_scale.band_width();

        // X axis labels
        let x_label = self.x_cats.iter().enumerate().filter_map(|(i, label)| {
            if (i + 1) % self.tick_margin == 0 {
                x_scale.tick(label).map(|x_tick| {
                    AxisText::new(
                        label.clone(),
                        left_margin + x_tick + x_band / 2.,
                        self.muted_fg,
                    )
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

        // Draw cells
        let origin = bounds.origin;
        let v_range = (self.v_max - self.v_min).max(1.0);
        for &(xi, yi, v) in &self.cell_map {
            if let (Some(x_tick), Some(y_tick)) = (
                x_scale.tick(&self.x_cats[xi]),
                y_scale.tick(&self.y_cats[yi]),
            ) {
                let t = (v - self.v_min) / v_range;
                let color = lerp_color(self.low_color, self.high_color, t);

                let p1 = point(px(left_margin + x_tick) + origin.x, px(y_tick) + origin.y);
                let p2 = point(
                    px(left_margin + x_tick + x_band) + origin.x,
                    px(y_tick + y_band) + origin.y,
                );

                window.paint_quad(fill(Bounds::from_corners(p1, p2), color));
            }
        }
    }
}

/// Linearly interpolate between two HSLA colours.
fn lerp_color(a: Hsla, b: Hsla, t: f32) -> Hsla {
    Hsla {
        h: a.h + (b.h - a.h) * t,
        s: a.s + (b.s - a.s) * t,
        l: a.l + (b.l - a.l) * t,
        a: a.a + (b.a - a.a) * t,
    }
}
