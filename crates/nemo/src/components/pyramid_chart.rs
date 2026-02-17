use gpui::*;
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_field,
};

/// A pyramid chart — horizontal bars of decreasing width stacked vertically
/// from bottom (widest) to top (narrowest), centred on the vertical axis.
/// Data items are sorted by value descending before rendering.
#[derive(IntoElement, NemoComponent)]
pub struct PyramidChart {
    #[property(default = "")]
    label_field: String,
    #[property(default = "")]
    value_field: String,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for PyramidChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let value_field = self.value_field.clone();
        let label_field = self.label_field.clone();

        // Sort descending by value so the widest bar is at the bottom
        data.sort_by(|a, b| {
            let va = get_f64_field(a, &value_field);
            let vb = get_f64_field(b, &value_field);
            vb.partial_cmp(&va).unwrap_or(std::cmp::Ordering::Equal)
        });

        let labels: Vec<String> = data
            .iter()
            .map(|item| get_string_field(item, &label_field))
            .collect();
        let values: Vec<f32> = data
            .iter()
            .map(|item| get_f64_field(item, &value_field) as f32)
            .collect();

        let max_val = values.iter().copied().fold(0.0_f32, f32::max);
        let colors: Vec<Hsla> = (0..values.len()).map(|i| chart_color(i, cx)).collect();
        let border_color = cx.theme().border;

        PyramidElement {
            _labels: labels,
            values,
            max_val,
            colors,
            _border_color: border_color,
        }
        .into_any_element()
    }
}

struct PyramidElement {
    _labels: Vec<String>,
    values: Vec<f32>,
    max_val: f32,
    colors: Vec<Hsla>,
    _border_color: Hsla,
}

impl IntoElement for PyramidElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for PyramidElement {
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
        let n = self.values.len();
        if n == 0 || self.max_val <= 0.0 {
            return;
        }

        let w = bounds.size.width.as_f32();
        let h = bounds.size.height.as_f32();
        let origin = bounds.origin;
        let gap = 2.0_f32;
        let row_height = (h - gap * (n as f32 - 1.0)) / n as f32;
        let center_x = w / 2.0;

        for (i, &val) in self.values.iter().enumerate() {
            let ratio = val / self.max_val;
            let bar_width = w * 0.9 * ratio;
            let y = (row_height + gap) * i as f32;

            let p1 = point(px(center_x - bar_width / 2.0) + origin.x, px(y) + origin.y);
            let p2 = point(
                px(center_x + bar_width / 2.0) + origin.x,
                px(y + row_height) + origin.y,
            );

            let color = self.colors[i % self.colors.len()];
            window.paint_quad(fill(Bounds::from_corners(p1, p2), color));
        }
    }
}
