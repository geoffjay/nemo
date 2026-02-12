use gpui::*;
use gpui_component::chart::AreaChart as GpuiAreaChart;
use nemo_config::Value;
use nemo_macros::NemoComponent;

use super::chart_utils::{
    chart_color, empty_chart_placeholder, extract_data_array, get_f64_field, get_string_array,
    get_string_field,
};

#[derive(IntoElement, NemoComponent)]
pub struct AreaChart {
    #[property(default = "")]
    x_field: String,
    #[property]
    fill_opacity: Option<f64>,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for AreaChart {
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
        let opacity = self.fill_opacity.unwrap_or(0.4) as f32;

        let mut chart =
            GpuiAreaChart::new(data).x(move |item: &Value| get_string_field(item, &x_field));

        for (i, field) in y_fields.into_iter().enumerate() {
            let color = chart_color(i, cx);
            chart = chart
                .y(move |item: &Value| get_f64_field(item, &field))
                .stroke(color)
                .fill(color.opacity(opacity));
        }

        if let Some(margin) = self.tick_margin {
            chart = chart.tick_margin(margin as usize);
        }

        chart.into_any_element()
    }
}
