use gpui::*;
use gpui_component::{ActiveTheme, PixelsExt};
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_field,
};

/// A funnel chart — successive trapezoid segments narrowing from top to
/// bottom, representing stages in a process. Data items should be provided
/// in funnel-stage order (widest first).
#[derive(IntoElement, NemoComponent)]
pub struct FunnelChart {
    #[property(default = "")]
    label_field: String,
    #[property(default = "")]
    value_field: String,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for FunnelChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let value_field = self.value_field.clone();
        let label_field = self.label_field.clone();

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

        FunnelElement {
            _labels: labels,
            values,
            max_val,
            colors,
            _border_color: border_color,
        }
        .into_any_element()
    }
}

struct FunnelElement {
    _labels: Vec<String>,
    values: Vec<f32>,
    max_val: f32,
    colors: Vec<Hsla>,
    _border_color: Hsla,
}

impl IntoElement for FunnelElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for FunnelElement {
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

        // Compute widths for each level
        let widths: Vec<f32> = self
            .values
            .iter()
            .map(|&v| (v / self.max_val) * w * 0.9)
            .collect();

        for i in 0..n {
            let y_top = (row_height + gap) * i as f32;
            let y_bot = y_top + row_height;
            let w_top = widths[i];
            let w_bot = if i + 1 < n {
                widths[i + 1]
            } else {
                w_top * 0.6
            };

            let color = self.colors[i % self.colors.len()];

            // Draw trapezoid as a filled path
            let tl = point(px(center_x - w_top / 2.0) + origin.x, px(y_top) + origin.y);
            let tr = point(px(center_x + w_top / 2.0) + origin.x, px(y_top) + origin.y);
            let br = point(px(center_x + w_bot / 2.0) + origin.x, px(y_bot) + origin.y);
            let bl = point(px(center_x - w_bot / 2.0) + origin.x, px(y_bot) + origin.y);

            let mut builder = PathBuilder::fill();
            builder.add_polygon(&[tl, tr, br, bl, tl], false);
            if let Ok(path) = builder.build() {
                window.paint_path(path, color);
            }
        }
    }
}
