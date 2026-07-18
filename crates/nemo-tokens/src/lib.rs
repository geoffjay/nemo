//! Nemo design tokens — the **gpui-free** source of truth for the spacing,
//! radius, and typography scales plus the semantic color *roles* layered over a
//! theme.
//!
//! This crate holds only pure data (scale values, names, role→field mappings) so
//! it can be shared by two consumers without pulling in gpui:
//!
//! * the `nemo` binary, whose `theme::tokens` re-exports these and adds the
//!   gpui-coupled render helpers (`space()`, `font_size()`, `TokenStyled`), and
//! * the `xtask` design-system exporter, which serializes these values into the
//!   `.pen`-friendly design-system JSON.
//!
//! Keeping the data here means the live UI and the exported design system can
//! never drift.

/// Spacing scale, in logical pixels. A small, consistent 4px-based step set.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
}

/// Named spacing steps — used by helpers and enumerated by the design-system
/// export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Space {
    Xs,
    Sm,
    Md,
    Lg,
    Xl,
    Xxl,
}

impl Space {
    /// The step's value in logical pixels.
    pub const fn value(self) -> f32 {
        match self {
            Space::Xs => space::XS,
            Space::Sm => space::SM,
            Space::Md => space::MD,
            Space::Lg => space::LG,
            Space::Xl => space::XL,
            Space::Xxl => space::XXL,
        }
    }

    /// Stable token name (used by the design-system export).
    pub const fn name(self) -> &'static str {
        match self {
            Space::Xs => "xs",
            Space::Sm => "sm",
            Space::Md => "md",
            Space::Lg => "lg",
            Space::Xl => "xl",
            Space::Xxl => "xxl",
        }
    }

    /// All steps, in ascending order (for enumeration/export).
    pub const ALL: [Space; 6] = [
        Space::Xs,
        Space::Sm,
        Space::Md,
        Space::Lg,
        Space::Xl,
        Space::Xxl,
    ];
}

/// Corner-radius scale, in logical pixels. Values match the gpui-component
/// rounded presets (`sm`/`md`/`lg`/`xl`, Tailwind rems × 16px) so adopting the
/// tokens is visually neutral; `full` is a pill.
pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 6.0;
    pub const LG: f32 = 8.0;
    pub const XL: f32 = 12.0;
    pub const FULL: f32 = 9999.0;
}

/// Maps an XML rounded-preset name to a radius token value in pixels.
/// `None` for unknown names (callers fall back to their own default).
pub fn radius_px(preset: &str) -> Option<f32> {
    match preset {
        "sm" => Some(radius::SM),
        "md" => Some(radius::MD),
        "lg" => Some(radius::LG),
        "xl" => Some(radius::XL),
        "full" => Some(radius::FULL),
        _ => None,
    }
}

/// Radius preset names, in ascending order (for enumeration/export).
pub const RADIUS_NAMES: [&str; 5] = ["sm", "md", "lg", "xl", "full"];

/// Typography scale: font size + line height, in logical pixels. Sizes match the
/// gpui text helpers exactly (`text_xs`=12, `text_sm`=14, base=16, `text_lg`=18,
/// `text_xl`=20, `text_2xl`=24) so adopting the tokens is visually neutral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSize {
    Xs,
    Sm,
    Base,
    Lg,
    Xl,
    Xxl,
}

impl FontSize {
    /// Font size in logical pixels.
    pub const fn size(self) -> f32 {
        match self {
            FontSize::Xs => 12.0,   // text_xs
            FontSize::Sm => 14.0,   // text_sm
            FontSize::Base => 16.0, // text_base
            FontSize::Lg => 18.0,   // text_lg
            FontSize::Xl => 20.0,   // text_xl
            FontSize::Xxl => 24.0,  // text_2xl
        }
    }

    /// Line height in logical pixels (matches the gpui/Tailwind defaults).
    pub const fn line_height(self) -> f32 {
        match self {
            FontSize::Xs => 16.0,
            FontSize::Sm => 20.0,
            FontSize::Base => 24.0,
            FontSize::Lg => 28.0,
            FontSize::Xl => 28.0,
            FontSize::Xxl => 32.0,
        }
    }

    /// Stable token name (used by the design-system export).
    pub const fn name(self) -> &'static str {
        match self {
            FontSize::Xs => "xs",
            FontSize::Sm => "sm",
            FontSize::Base => "base",
            FontSize::Lg => "lg",
            FontSize::Xl => "xl",
            FontSize::Xxl => "xxl",
        }
    }

    /// All sizes, in ascending order (for enumeration/export).
    pub const ALL: [FontSize; 6] = [
        FontSize::Xs,
        FontSize::Sm,
        FontSize::Base,
        FontSize::Lg,
        FontSize::Xl,
        FontSize::Xxl,
    ];
}

/// Semantic color *roles* → the underlying gpui-component theme color field.
///
/// The design system speaks in roles ("surface", "text-muted", …); each resolves
/// to a concrete theme color so a role stays correct in light and dark. These
/// are registered in the app's `resolve_theme_color` (via [`resolve_role_alias`])
/// so XML can reference `theme.<role>`, and are enumerated by the export.
///
/// Roles that already have a direct theme name (`accent`, `primary`, `danger`,
/// `success`, `warning`, `info`, `link`) are intentionally omitted — they need
/// no alias.
pub const SEMANTIC_COLOR_ROLES: &[(&str, &str)] = &[
    ("surface", "background"),
    ("surface_raised", "secondary"),
    ("surface_overlay", "popover"),
    ("text", "foreground"),
    ("text_muted", "muted_foreground"),
    ("border_subtle", "border"),
    ("focus_ring", "ring"),
];

/// Translates a semantic color role (e.g. `"surface_raised"`) to its underlying
/// theme color field name (e.g. `"secondary"`). Returns `None` for names that
/// are not role aliases, so direct theme names pass through unchanged.
pub fn resolve_role_alias(name: &str) -> Option<&'static str> {
    SEMANTIC_COLOR_ROLES
        .iter()
        .find(|(role, _)| *role == name)
        .map(|(_, field)| *field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spacing_scale_is_ascending() {
        let vals: Vec<f32> = Space::ALL.iter().map(|s| s.value()).collect();
        assert!(vals.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn font_sizes_ascending_with_line_heights() {
        for s in FontSize::ALL {
            assert!(s.line_height() >= s.size());
        }
        let sizes: Vec<f32> = FontSize::ALL.iter().map(|s| s.size()).collect();
        assert!(sizes.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn radius_presets_match_tokens() {
        assert_eq!(radius_px("md"), Some(radius::MD));
        assert_eq!(radius_px("full"), Some(radius::FULL));
        assert_eq!(radius_px("bogus"), None);
    }

    #[test]
    fn role_aliases_translate_and_pass_through() {
        assert_eq!(resolve_role_alias("surface_raised"), Some("secondary"));
        assert_eq!(resolve_role_alias("text_muted"), Some("muted_foreground"));
        // Direct theme names are not aliases.
        assert_eq!(resolve_role_alias("secondary"), None);
        assert_eq!(resolve_role_alias("accent"), None);
    }
}
