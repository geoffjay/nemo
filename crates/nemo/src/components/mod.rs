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

/// Generates the theme color match from a list of `name => field` pairs.
/// Adding a new theme color only requires adding one line here.
macro_rules! theme_color_match {
    ($colors:expr, $name:expr, $( $key:literal => $field:ident ),+ $(,)?) => {
        match $name {
            $( $key => Some($colors.$field), )+
            _ => None,
        }
    };
}

fn resolve_theme_color(name: &str, cx: &App) -> Option<Hsla> {
    let c = &cx.theme().colors;
    theme_color_match!(c, name,
        "accent" => accent, "accent_foreground" => accent_foreground,
        "background" => background, "border" => border, "foreground" => foreground,
        "muted" => muted, "muted_foreground" => muted_foreground,
        "primary" => primary, "primary_active" => primary_active,
        "primary_foreground" => primary_foreground, "primary_hover" => primary_hover,
        "secondary" => secondary, "secondary_active" => secondary_active,
        "secondary_foreground" => secondary_foreground, "secondary_hover" => secondary_hover,
        "danger" => danger, "danger_active" => danger_active,
        "danger_foreground" => danger_foreground, "danger_hover" => danger_hover,
        "warning" => warning, "warning_active" => warning_active,
        "warning_foreground" => warning_foreground, "warning_hover" => warning_hover,
        "success" => success, "success_active" => success_active,
        "success_foreground" => success_foreground, "success_hover" => success_hover,
        "info" => info, "info_active" => info_active,
        "info_foreground" => info_foreground, "info_hover" => info_hover,
        "input" => input, "ring" => ring, "selection" => selection, "overlay" => overlay,
        "popover" => popover, "popover_foreground" => popover_foreground,
        "link" => link, "link_hover" => link_hover, "link_active" => link_active,
        "list" => list, "list_active" => list_active,
        "list_active_border" => list_active_border, "list_hover" => list_hover,
        "table" => table, "table_active" => table_active,
        "table_active_border" => table_active_border, "table_hover" => table_hover,
        "table_head" => table_head, "table_head_foreground" => table_head_foreground,
        "tab" => tab, "tab_active" => tab_active,
        "tab_active_foreground" => tab_active_foreground, "tab_foreground" => tab_foreground,
        "sidebar" => sidebar, "sidebar_border" => sidebar_border,
        "sidebar_foreground" => sidebar_foreground,
        "title_bar" => title_bar, "title_bar_border" => title_bar_border,
        "red" => red, "red_light" => red_light,
        "green" => green, "green_light" => green_light,
        "blue" => blue, "blue_light" => blue_light,
        "yellow" => yellow, "yellow_light" => yellow_light,
        "magenta" => magenta, "magenta_light" => magenta_light,
        "cyan" => cyan, "cyan_light" => cyan_light,
    )
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
