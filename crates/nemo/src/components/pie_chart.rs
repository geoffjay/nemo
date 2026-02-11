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

        let mut chart = GpuiPieChart::new(data)
            .value(move |item: &Value| get_f64_field(item, &value_field) as f32)
            .color(move |_item: &Value| {
                let idx = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                colors[idx % colors.len()]
            });

        if let Some(r) = self.outer_radius {
            chart = chart.outer_radius(r as f32);
        }
        if let Some(r) = self.inner_radius {
            chart = chart.inner_radius(r as f32);
        }

        chart.into_any_element()
    }
}
