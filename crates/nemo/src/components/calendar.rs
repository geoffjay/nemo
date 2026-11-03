use chrono::NaiveDate;
use gpui::{Entity, IntoElement};
use gpui_kit::component::calendar::Calendar as GpuiCalendar;
use gpui_kit::component::calendar::{CalendarState, Date};
use gpui_kit::component::Sizable as _;
use gpui_kit::component::Size as ComponentSize;
use nemo_layout::BuiltComponent;

/// A month-view date picker (gpui-kit `Calendar`). The selection lives in a
/// shared `CalendarState` entity managed by the app and seeded from `value`;
/// clicking a day, navigating, and the built-in month/year pickers are all
/// kit-internal. When a new date is selected the app mirrors it back into the
/// component's `value` property (so scripts can read it) and fires
/// `on-change` with the ISO date string. Re-seed by changing the property.
///
/// ```xml
/// <calendar id="checkout" value="2026-01-15" first-day="mon"
///           min="2026-01-01" max="2026-12-31" on-change="handle_date" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `value` | string | Initially selected date, `yyyy-mm-dd`; mirrors back on selection |
/// | `months` | integer | Number of month grids side by side (default 1) |
/// | `size` | string | `small`, `medium` (default), or `large` |
/// | `first-day` | string | First day of the week: `sun` (default), `mon`, … `sat` |
/// | `min` | string | Disable dates before `yyyy-mm-dd` |
/// | `max` | string | Disable dates after `yyyy-mm-dd` |
/// | `disabled-days` | string | Comma-separated weekdays to disable, `0` = Sunday … `6` = Saturday |
/// | `on-change` | string | Handler invoked with the selected date as `yyyy-mm-dd` |
#[derive(IntoElement)]
pub struct Calendar {
    source: BuiltComponent,
    state: Entity<CalendarState>,
}

impl Calendar {
    pub fn new(source: BuiltComponent, state: Entity<CalendarState>) -> Self {
        Self { source, state }
    }
}

impl gpui::RenderOnce for Calendar {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let props = &self.source.properties;
        let size = match props.get("size").and_then(|v| v.as_str()) {
            Some("small") | Some("xsmall") => ComponentSize::Small,
            Some("large") => ComponentSize::Large,
            _ => ComponentSize::Medium,
        };
        let months = props
            .get("months")
            .and_then(|v| v.as_i64())
            .unwrap_or(1)
            .max(1) as usize;
        let first_day = match props
            .get("first_day")
            .and_then(|v| v.as_str())
            .and_then(parse_weekday)
        {
            Some(day) => day,
            None => chrono::Weekday::Sun,
        };

        GpuiCalendar::new(&self.state)
            .number_of_months(months)
            .first_day_of_week(first_day)
            .with_size(size)
    }
}

/// Parses a `yyyy-mm-dd` date string.
pub fn parse_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d").ok()
}

/// Maps a weekday name (`sun`…`sat`, or full names) to a `chrono::Weekday`.
pub fn parse_weekday(value: &str) -> Option<chrono::Weekday> {
    match value.trim().to_ascii_lowercase().as_str() {
        "sun" | "sunday" => Some(chrono::Weekday::Sun),
        "mon" | "monday" => Some(chrono::Weekday::Mon),
        "tue" | "tues" | "tuesday" => Some(chrono::Weekday::Tue),
        "wed" | "wednesday" => Some(chrono::Weekday::Wed),
        "thu" | "thurs" | "thursday" => Some(chrono::Weekday::Thu),
        "fri" | "friday" => Some(chrono::Weekday::Fri),
        "sat" | "saturday" => Some(chrono::Weekday::Sat),
        _ => None,
    }
}

/// Formats the single date a `CalendarEvent` carries as `yyyy-mm-dd`.
pub fn selected_date_string(date: &Date) -> Option<String> {
    match date {
        Date::Single(Some(day)) => Some(day.format("%Y-%m-%d").to_string()),
        _ => None,
    }
}
