use gpui::*;
use gpui_kit::component::skeleton::Skeleton as GpuiSkeleton;
use nemo_layout::BuiltComponent;

use crate::theme::tokens::radius_for;

/// A skeleton loading placeholder (gpui-kit `Skeleton`): a pulsing bar sized
/// through the universal `width`/`height`/`rounded` wrapper attributes.
/// Defaults to full width and 16 px tall in the theme's `skeleton` color, with
/// a built-in 2 s pulse animation (100% ↔ 50% opacity).
///
/// # XML Configuration
///
/// ```xml
/// <skeleton id="loading_bar" />
/// <skeleton id="avatar_loading" width="48" height="48" rounded="full" />
/// <skeleton id="subtle_bar" width="200" secondary="true" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `secondary` | bool | Use 50%-opacity `skeleton` color for a subtler placeholder |
/// | `width` | integer | Width in px; defaults to full width |
/// | `height` | integer | Height in px; defaults to 16 |
/// | `rounded` | string | Corner preset: sm/md/lg/xl/full |
#[derive(IntoElement)]
#[allow(dead_code)]
pub struct Skeleton {
    source: BuiltComponent,
}

impl Skeleton {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source }
    }
}

impl RenderOnce for Skeleton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let props = &self.source.properties;
        let secondary = props
            .get("secondary")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let width = props.get("width").and_then(|v| v.as_i64());
        let height = props.get("height").and_then(|v| v.as_i64());
        let rounded = props.get("rounded").and_then(|v| v.as_str());

        // The upstream element defaults to `w_full().h_4()` and carries the
        // pulse animation; only apply what the config overrides. Sizing must
        // land on the skeleton itself — the generic wrapper's `width`/`height`
        // can't reach the element's own background otherwise.
        let mut skeleton = GpuiSkeleton::new();
        if secondary {
            skeleton = skeleton.secondary();
        }
        if let Some(w) = width {
            skeleton = skeleton.w(px(w as f32));
        } else {
            skeleton = skeleton.w_full();
        }
        if let Some(h) = height {
            skeleton = skeleton.h(px(h as f32));
        }
        if let Some(r) = rounded.and_then(|preset| radius_for(preset, cx)) {
            skeleton = skeleton.rounded(r);
        }
        skeleton.into_any_element()
    }
}
