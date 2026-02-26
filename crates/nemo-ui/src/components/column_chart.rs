use gpui::*;
use gpui_component::chart::BarChart as GpuiBarChart;
use nemo_config::Value;
use nemo_macros::NemoComponent;

use super::chart_utils::{
    empty_chart_placeholder, extract_data_array, get_f64_field, get_string_field,
};

/// A column chart (vertical bars). Functionally identical to BarChart but
/// provided as a distinct component name for chart-taxonomy clarity.
///
/// # XML Configuration
///
/// ```xml
/// <column-chart id="revenue" x-field="month" y-field="revenue" show-label="true"
///   tick-margin="20" height="300">
///   <data bind="revenueData" />
/// </column-chart>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `x-field` | string | Data field for the x-axis categories |
/// | `y-field` | string | Data field for the y-axis values |
/// | `show-label` | bool | Display value labels on columns |
/// | `tick-margin` | int | Margin for axis tick labels |
/// | `height` | int | Chart height in pixels |
#[derive(IntoElement, NemoComponent)]
pub struct ColumnChart {
    #[property(default = "")]
    x_field: String,
    #[property(default = "")]
    y_field: String,
    #[property]
    show_label: Option<bool>,
    #[property]
    tick_margin: Option<i64>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl RenderOnce for ColumnChart {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let data = extract_data_array(&self.source);
        if data.is_empty() {
            return empty_chart_placeholder(cx);
        }

        let x_field = self.x_field.clone();
        let y_field = self.y_field.clone();

        let mut chart = GpuiBarChart::new(data)
            .x(move |item: &Value| get_string_field(item, &x_field))
            .y(move |item: &Value| get_f64_field(item, &y_field));

        if self.show_label == Some(true) {
            let label_y_field = self.y_field.clone();
            chart = chart.label(move |item: &Value| {
                let val = get_f64_field(item, &label_y_field);
                format!("{val}")
            });
        }
        if let Some(margin) = self.tick_margin {
            chart = chart.tick_margin(margin as usize);
        }

        chart.into_any_element()
    }
}
