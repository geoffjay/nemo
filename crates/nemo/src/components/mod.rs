mod accordion;
mod alert;
mod area_chart;
mod avatar;
mod badge;
mod bar_chart;
mod button;
mod candlestick_chart;
pub(crate) mod chart_utils;
mod checkbox;
mod collapsible;
mod dropdown_button;
pub(crate) mod icon;
mod image;
mod input;
mod label;
mod line_chart;
mod list;
mod modal;
mod notification;
mod panel;
mod pie_chart;
mod progress;
mod radio;
mod select;
pub(crate) mod slider;
mod spinner;
mod stack;
pub(crate) mod state;
mod switch;
pub(crate) mod table;
mod tabs;
mod tag;
mod text;
mod toggle;
mod tooltip;
pub(crate) mod tree;

use gpui::*;
use gpui_component::ActiveTheme;

/// Resolves a color string to an Hsla value.
///
/// Supports two formats:
/// - Theme reference: `"theme.border"`, `"theme.accent"`, `"theme.danger"`, etc.
/// - Hex color: `"#4c566a"`, `"4c566aff"`, `"#FF0000"`
pub(crate) fn resolve_color(value: &str, cx: &App) -> Option<Hsla> {
    if let Some(name) = value.strip_prefix("theme.") {
        resolve_theme_color(name, cx)
    } else {
        parse_hex_color(value)
    }
}

fn parse_hex_color(hex: &str) -> Option<Hsla> {
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    let len = hex.len();
    if len != 6 && len != 8 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let a = if len == 8 {
        u8::from_str_radix(&hex[6..8], 16).ok()?
    } else {
        255
    };
    Some(rgba(((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)).into())
}

fn resolve_theme_color(name: &str, cx: &App) -> Option<Hsla> {
    let c = &cx.theme().colors;
    let color = match name {
        "accent" => c.accent,
        "accent_foreground" => c.accent_foreground,
        "background" => c.background,
        "border" => c.border,
        "foreground" => c.foreground,
        "muted" => c.muted,
        "muted_foreground" => c.muted_foreground,
        "primary" => c.primary,
        "primary_active" => c.primary_active,
        "primary_foreground" => c.primary_foreground,
        "primary_hover" => c.primary_hover,
        "secondary" => c.secondary,
        "secondary_active" => c.secondary_active,
        "secondary_foreground" => c.secondary_foreground,
        "secondary_hover" => c.secondary_hover,
        "danger" => c.danger,
        "danger_active" => c.danger_active,
        "danger_foreground" => c.danger_foreground,
        "danger_hover" => c.danger_hover,
        "warning" => c.warning,
        "warning_active" => c.warning_active,
        "warning_foreground" => c.warning_foreground,
        "warning_hover" => c.warning_hover,
        "success" => c.success,
        "success_active" => c.success_active,
        "success_foreground" => c.success_foreground,
        "success_hover" => c.success_hover,
        "info" => c.info,
        "info_active" => c.info_active,
        "info_foreground" => c.info_foreground,
        "info_hover" => c.info_hover,
        "input" => c.input,
        "ring" => c.ring,
        "selection" => c.selection,
        "overlay" => c.overlay,
        "popover" => c.popover,
        "popover_foreground" => c.popover_foreground,
        "link" => c.link,
        "link_hover" => c.link_hover,
        "link_active" => c.link_active,
        "list" => c.list,
        "list_active" => c.list_active,
        "list_active_border" => c.list_active_border,
        "list_hover" => c.list_hover,
        "table" => c.table,
        "table_active" => c.table_active,
        "table_active_border" => c.table_active_border,
        "table_hover" => c.table_hover,
        "table_head" => c.table_head,
        "table_head_foreground" => c.table_head_foreground,
        "tab" => c.tab,
        "tab_active" => c.tab_active,
        "tab_active_foreground" => c.tab_active_foreground,
        "tab_foreground" => c.tab_foreground,
        "sidebar" => c.sidebar,
        "sidebar_border" => c.sidebar_border,
        "sidebar_foreground" => c.sidebar_foreground,
        "title_bar" => c.title_bar,
        "title_bar_border" => c.title_bar_border,
        "red" => c.red,
        "red_light" => c.red_light,
        "green" => c.green,
        "green_light" => c.green_light,
        "blue" => c.blue,
        "blue_light" => c.blue_light,
        "yellow" => c.yellow,
        "yellow_light" => c.yellow_light,
        "magenta" => c.magenta,
        "magenta_light" => c.magenta_light,
        "cyan" => c.cyan,
        "cyan_light" => c.cyan_light,
        _ => return None,
    };
    Some(color)
}

/// Applies a shadow preset to a div element.
///
/// Supported sizes: "sm", "md", "lg", "xl", "2xl"
pub(crate) fn apply_shadow(base: Div, shadow: Option<&str>) -> Div {
    match shadow {
        Some("sm") => base.shadow_sm(),
        Some("md") => base.shadow_md(),
        Some("lg") => base.shadow_lg(),
        Some("xl") => base.shadow_xl(),
        Some("2xl") => base.shadow_2xl(),
        _ => base,
    }
}

pub use accordion::Accordion;
pub use alert::Alert;
pub use area_chart::AreaChart;
pub use avatar::Avatar;
pub use badge::Badge;
pub use bar_chart::BarChart;
pub use button::Button;
pub use candlestick_chart::CandlestickChart;
pub use checkbox::Checkbox;
pub use collapsible::Collapsible;
pub use dropdown_button::DropdownButton;
pub use icon::Icon;
pub use image::Image;
pub use input::Input;
pub use label::Label;
pub use line_chart::LineChart;
pub use list::List;
pub use modal::Modal;
pub use notification::Notification;
pub use panel::Panel;
pub use pie_chart::PieChart;
pub use progress::Progress;
pub use radio::Radio;
pub use select::Select;
pub use slider::Slider;
pub use spinner::Spinner;
pub use stack::Stack;
pub use switch::Switch;
pub use table::Table;
pub use tabs::Tabs;
pub use tag::Tag;
pub use text::Text;
pub use toggle::Toggle;
pub use tooltip::Tooltip;
pub use tree::Tree;

#[cfg(test)]
mod tests {
    use super::parse_hex_color;

    // ── parse_hex_color ───────────────────────────────────────────────

    #[test]
    fn test_hex_color_6_char_with_hash() {
        let color = parse_hex_color("#FF0000").unwrap();
        // Red: should have high hue saturation
        assert!(color.a > 0.99); // fully opaque
    }

    #[test]
    fn test_hex_color_6_char_without_hash() {
        let color = parse_hex_color("00FF00").unwrap();
        assert!(color.a > 0.99);
    }

    #[test]
    fn test_hex_color_8_char_with_alpha() {
        let color = parse_hex_color("#FF000080").unwrap();
        // Alpha 0x80 = 128/255 ≈ 0.502
        assert!(color.a > 0.49 && color.a < 0.51);
    }

    #[test]
    fn test_hex_color_8_char_full_alpha() {
        let color = parse_hex_color("4c566aff").unwrap();
        assert!(color.a > 0.99);
    }

    #[test]
    fn test_hex_color_black() {
        let color = parse_hex_color("#000000").unwrap();
        assert!(color.l < 0.01); // lightness near 0 = black
    }

    #[test]
    fn test_hex_color_white() {
        let color = parse_hex_color("#FFFFFF").unwrap();
        assert!(color.l > 0.99); // lightness near 1 = white
    }

    #[test]
    fn test_hex_color_invalid_length() {
        assert!(parse_hex_color("#FFF").is_none()); // 3 chars
        assert!(parse_hex_color("#FFFF").is_none()); // 4 chars
        assert!(parse_hex_color("#FFFFFFFFF").is_none()); // 9 chars
    }

    #[test]
    fn test_hex_color_invalid_chars() {
        assert!(parse_hex_color("#GGHHII").is_none());
        assert!(parse_hex_color("zzzzzz").is_none());
    }

    #[test]
    fn test_hex_color_empty() {
        assert!(parse_hex_color("").is_none());
        assert!(parse_hex_color("#").is_none());
    }

    // ── resolve_color routing ─────────────────────────────────────────
    // resolve_color and resolve_theme_color need a GPUI App context,
    // but we can test the routing logic for hex colors since
    // parse_hex_color is called for non-"theme." prefixes.

    #[test]
    fn test_parse_hex_color_consistency() {
        // Same color with/without hash should produce same result
        let a = parse_hex_color("#4c566a").unwrap();
        let b = parse_hex_color("4c566a").unwrap();
        assert!((a.h - b.h).abs() < 0.001);
        assert!((a.s - b.s).abs() < 0.001);
        assert!((a.l - b.l).abs() < 0.001);
        assert!((a.a - b.a).abs() < 0.001);
    }
}
