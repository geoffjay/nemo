use gpui::*;
use gpui_kit::component::ActiveTheme;
use nemo_config::Value;
use nemo_macros::NemoComponent;

/// A Phosphor icon component.
///
/// Renders any of the 1,512 [Phosphor](https://phosphoricons.com) icons in any
/// of the six weights, at any pixel size and any hex (or `theme.*`) color:
///
/// ```xml
/// <icon-phosphor name="acorn" size="64" color="#f0f0f0" weight="light" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `name` | string | The Phosphor icon name, e.g. `acorn` (default `acorn`). Unknown names render nothing, matching Phosphor's own behavior of serving only known paths. |
/// | `size` | number | Render size in px (default `16`). Overrides a `width`/`height` style. |
/// | `color` | string | Hex color (`#f0f0f0`, `f0f0f0`, `#rrggbbaa`) or a `theme.<color>` token name. Default `currentColor` resolves against the theme's foreground, like inline SVG. |
/// | `weight` | string | One of `thin`, `light`, `regular`, `bold`, `fill`, `duotone` (default `regular`). |
///
/// The SVG comes from the `nemo-phosphor` asset source (registered ahead of GPUI
/// Kit's `Assets` in `main.rs`), at the path `phosphor/{weight}/{name}.svg`.
#[derive(IntoElement, NemoComponent)]
pub struct IconPhosphor {
    #[property(default = "acorn")]
    name: String,
    #[property(default = 16i64)]
    size: i64,
    #[property(default = "currentColor")]
    color: String,
    #[property(default = "regular")]
    weight: String,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl IconPhosphor {
    /// The phosphor asset path for this component, or `None` when the name or
    /// weight is unknown.
    pub(crate) fn asset_path(name: &str, weight: &str) -> Option<String> {
        let icon = nemo_phosphor::PhosphorIcon::from_name(name)?;
        let weight = nemo_phosphor::Weight::from_name(&weight.trim().to_lowercase())?;
        Some(phosphor_path(icon, weight))
    }
}

/// Formats the `phosphor/{weight}/{name}.svg` asset path.
fn phosphor_path(icon: nemo_phosphor::PhosphorIcon, weight: nemo_phosphor::Weight) -> String {
    use nemo_phosphor::Phosphor;
    Phosphor::new(icon, weight).path().to_string()
}

impl RenderOnce for IconPhosphor {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(path) = Self::asset_path(&self.name, &self.weight) else {
            return div().into_any_element();
        };

        // A `size` property is the exact pixel diameter; fall back to the
        // universal `width`/`height` styles like `<svg>` does. When neither is
        // set the icon scales to the surrounding text style. Numeric strings
        // ("32", from `${item}` template substitution) coerce like integers.
        let size = self
            .source
            .properties
            .get("size")
            .map(|v| match v {
                Value::Integer(i) => Some(*i),
                Value::String(s) => s.trim().parse::<i64>().ok(),
                _ => None,
            })
            .unwrap_or_else(|| Some(self.size))
            .filter(|v| *v > 0)
            .map(|v| px(v as f32));
        let size = size.or_else(|| {
            let width = self.source.properties.get("width").and_then(|v| v.as_i64());
            let height = self
                .source
                .properties
                .get("height")
                .and_then(|v| v.as_i64());
            width.or(height).filter(|v| *v > 0).map(|v| px(v as f32))
        });

        // `currentColor` (the default) tracks the theme foreground; a hex value
        // or `theme.<token>` name resolves through the shared helper.
        let text_color = if self.color.trim() == "currentColor" {
            cx.theme().colors.foreground
        } else {
            crate::components::resolve_color(self.color.trim(), cx)
                .unwrap_or_else(|| cx.theme().colors.foreground)
        };

        let mut svg = svg()
            .path(SharedString::from(path))
            .flex_shrink_0()
            .text_color(text_color);
        if let Some(size) = size {
            svg = svg.size(size);
        }
        svg.into_any_element()
    }
}
