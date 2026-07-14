use gpui::*;
use gpui_component::ActiveTheme;
use nemo_macros::NemoComponent;

/// A generic container panel component.
///
/// # XML Configuration
///
/// ```xml
/// <panel id="info" visible="true" padding="16" border="1">
///   <!-- child elements -->
/// </panel>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `visible` | bool | Whether the panel is visible |
/// | `padding` | int | Inner padding in pixels |
/// | `border` | int | Border width in pixels |
/// | `border-color` | string | Border color (theme ref or hex; default `theme.border`) |
/// | `rounded` | string | Corner rounding: `sm`/`md`/`lg`/`xl`/`full`/`none` (default `md`) |
/// | `shadow` | string | Drop shadow: `sm`/`md`/`lg`/`xl`/`2xl` |
/// | `flex` | bool/int | Grow to fill the parent (e.g. to bound an inner scroll stack) |
#[derive(IntoElement, NemoComponent)]
pub struct Panel {
    #[source]
    source: nemo_layout::BuiltComponent,
    #[property]
    visible: Option<bool>,
    #[property]
    padding: Option<i64>,
    #[property]
    border: Option<i64>,
    #[children]
    children: Vec<AnyElement>,
}

impl RenderOnce for Panel {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        if self.visible == Some(false) {
            return div().into_any_element();
        }

        let props = &self.source.properties;

        let mut el = div().flex().flex_col().bg(cx.theme().colors.secondary);

        // The panel is the single owner of its own decoration (padding, border,
        // rounding, shadow, background) — `apply_layout_styles` skips these props
        // for panels to avoid double-decorating (a stray outer border/box).
        el = match props.get("rounded").and_then(|v| v.as_str()) {
            Some("sm") => el.rounded_sm(),
            Some("lg") => el.rounded_lg(),
            Some("xl") => el.rounded_xl(),
            Some("full") => el.rounded(px(9999.)),
            Some("none") => el,
            _ => el.rounded_md(),
        };

        el = crate::components::apply_shadow(el, props.get("shadow").and_then(|v| v.as_str()));

        if let Some(p) = self.padding {
            el = el.p(px(p as f32));
        }

        if let Some(b) = self.border {
            if b > 0 {
                let border_color = props
                    .get("border_color")
                    .and_then(|v| v.as_str())
                    .and_then(|c| crate::components::resolve_color(c, cx))
                    .unwrap_or(cx.theme().colors.border);
                el = el.border(px(b as f32)).border_color(border_color);
            }
        }

        // Flexbox-native: grow to fill the parent only when opted in with a
        // truthy `flex` (e.g. a panel that must bound an inner scroll stack).
        if props
            .get("flex")
            .map(crate::components::flex_is_truthy)
            .unwrap_or(false)
        {
            el = el.flex_1().min_h(px(0.));
        }

        el.children(self.children).into_any_element()
    }
}
