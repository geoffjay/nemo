use gpui::*;
use gpui_component::chart::PieChart as GpuiPieChart;
use nemo_config::Value;
use nemo_macros::NemoComponent;

use super::chart_utils::{chart_color, empty_chart_placeholder, extract_data_array, get_f64_field};

#[derive(IntoElement, NemoComponent)]
pub struct PieChart {
    #[property(default = "")]
    value_field: String,
    #[property]
    outer_radius: Option<f64>,
    #[property]
    inner_radius: Option<f64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for PieChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let data_len = data.len();
        let colors: Vec<Hsla> = (0..data_len).map(|i| chart_color(i, cx)).collect();
        let counter = std::sync::atomic::AtomicUsize::new(0);
        let value_field = self.value_field.clone();

        // Workaround for gpui-component bug: PieChart.get_outer_radius() returns 0.0
        // when no explicit radius is set, which overrides the bounds-based auto-computed
        // value in Arc::paint. We must always provide an explicit outer_radius.
        let height = self
            .source
            .properties
            .get("height")
            .and_then(|v| v.as_i64())
            .unwrap_or(300) as f32;
        let outer_radius = self.outer_radius.map(|r| r as f32).unwrap_or(height * 0.4);
        let inner_radius = self.inner_radius.map(|r| r as f32).unwrap_or(0.0);

        let chart = GpuiPieChart::new(data)
            .value(move |item: &Value| get_f64_field(item, &value_field) as f32)
            .color(move |_item: &Value| {
                let idx = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                colors[idx % colors.len()]
            })
            .outer_radius(outer_radius)
            .inner_radius(inner_radius);

        chart.into_any_element()
    }
}
