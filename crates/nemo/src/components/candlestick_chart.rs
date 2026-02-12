use gpui::*;
use gpui_component::chart::CandlestickChart as GpuiCandlestickChart;
use nemo_config::Value;
use nemo_macros::NemoComponent;

use super::chart_utils::{
    empty_chart_placeholder, extract_data_array, get_f64_field, get_string_field,
};

#[derive(IntoElement, NemoComponent)]
pub struct CandlestickChart {
    #[property(default = "")]
    x_field: String,
    #[property(default = "")]
    open_field: String,
    #[property(default = "")]
    high_field: String,
    #[property(default = "")]
    low_field: String,
    #[property(default = "")]
    close_field: String,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for CandlestickChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let x_field = self.x_field.clone();
        let open_field = self.open_field.clone();
        let high_field = self.high_field.clone();
        let low_field = self.low_field.clone();
        let close_field = self.close_field.clone();

        let mut chart = GpuiCandlestickChart::new(data)
            .x(move |item: &Value| get_string_field(item, &x_field))
            .open(move |item: &Value| get_f64_field(item, &open_field))
            .high(move |item: &Value| get_f64_field(item, &high_field))
            .low(move |item: &Value| get_f64_field(item, &low_field))
            .close(move |item: &Value| get_f64_field(item, &close_field));

        if let Some(margin) = self.tick_margin {
            chart = chart.tick_margin(margin as usize);
        }

        chart.into_any_element()
    }
}
