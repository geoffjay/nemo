use gpui::*;
use gpui_kit::component::kbd::Kbd as GpuiKbd;
use nemo_layout::BuiltComponent;

/// A tag for displaying keyboard keybindings, rendered with the same
/// platform-specific formatting as gpui-kit's Kbd (macOS symbols, e.g. ⌘⇧P;
/// text labels elsewhere, e.g. Ctrl+Shift+P).
///
/// # XML Configuration
///
/// ```xml
/// <kbd id="shortcut" keys="cmd-shift-p" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `keys` | string | Keystroke in gpui keymap syntax (e.g. `cmd-shift-p`) |
/// | `outline` | bool | Render in outline style instead of the muted filled tag |
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Kbd {
    source: BuiltComponent,
}

impl Kbd {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
    }
}

impl RenderOnce for Kbd {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let keys = props
            .get("keys")
            .and_then(|v| v.as_str())
            .unwrap_or("cmd-k")
            .to_string();
        let outline = props
            .get("outline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // `Keystroke::parse` demands a `&'static str`, so use `parse_with_os`
        // with the runtime's OS info (same path as the shortcuts dialog rows;
        // it also resolves the `secondary-` modifier alias). An unparsable
        // `keys` value renders the raw string unstyled rather than panicking.
        let stroke = Keystroke::parse_with_os(&keys, cx.platform_info().os);
        let mut kbd = match stroke {
            Ok(stroke) => GpuiKbd::new(stroke),
            Err(_) => GpuiKbd::new(Keystroke::parse("escape").unwrap()).appearance(false),
        };
        if outline {
            kbd = kbd.outline();
        }
        div().flex_none().child(kbd).into_any_element()
    }
}
