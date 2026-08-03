use gpui::*;
use nemo_macros::NemoComponent;
use std::path::PathBuf;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

/// An SVG display component.
///
/// Renders vector graphics either loaded from a file (or URL) via `src`, or
/// embedded directly in the configuration as standard SVG markup. Embedded SVG
/// is written as a normal `<svg>` element whose body is captured verbatim by
/// the XML parser (see `capture_svg_element`) and rasterized here through
/// gpui's SVG renderer.
///
/// # XML Configuration
///
/// ```xml
/// <!-- Load from a file (relative paths resolve against the config file's
///      directory, so the app launches correctly from any working dir) -->
/// <svg id="logo" src="assets/logo.svg" width="120" height="120" />
///
/// <!-- Embed standard SVG markup inline -->
/// <svg id="badge" width="64" height="64" viewBox="0 0 100 100">
///   <circle cx="50" cy="50" r="40" fill="#4c566a" stroke="#eceff4" stroke-width="4" />
/// </svg>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `src` | string | File path or `http(s)` URL of an SVG. Takes precedence over embedded markup. |
/// | `content` | string | Embedded SVG markup (populated automatically from an inline `<svg>` body). |
/// | `width` / `height` | int | Optional render size in px; otherwise the SVG's intrinsic size is used. |
/// | `on-click` | string | Handler invoked when the SVG is clicked (`event_data` = `"click"`). |
/// | `on-hover` | string | Handler invoked on hover enter/leave (`event_data` = `"hover"` / `"hover_end"`). |
///
/// # Interactivity
///
/// The SVG rasterizes to a single image, so its child elements (`<path>`,
/// `<circle>`, ...) are not individually addressable or hit-testable. An
/// `on-click`/`on-hover` handlers fire for the SVG as a whole; a handler can then recolor
/// or transform the graphic by rewriting the whole markup via
/// `set_component_property(id, "content", "<svg…>")` (or swapping `src`), which
/// re-rasterizes on the next render.
///
#[derive(IntoElement, NemoComponent)]
pub struct Svg {
    #[property(default = "")]
    src: String,
    #[property(default = "")]
    content: String,
    runtime: Option<Arc<NemoRuntime>>,
    entity_id: Option<EntityId>,
    #[source]
    source: nemo_layout::BuiltComponent,
}

impl Svg {
    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }
}

/// usvg only recognizes elements in the SVG namespace, so inline snippets that
/// omit `xmlns` would otherwise fail to parse. Inject the default namespace on
/// the root `<svg>` tag when it is absent.
fn ensure_xmlns(svg: &str) -> String {
    if svg.contains("xmlns") {
        return svg.to_string();
    }
    match svg.find("<svg") {
        Some(pos) => {
            let insert_at = pos + "<svg".len();
            let mut out = String::with_capacity(svg.len() + 40);
            out.push_str(&svg[..insert_at]);
            out.push_str(r#" xmlns="http://www.w3.org/2000/svg""#);
            out.push_str(&svg[insert_at..]);
            out
        }
        None => svg.to_string(),
    }
}

fn is_url(src: &str) -> bool {
    src.starts_with("http://") || src.starts_with("https://")
}

/// Resolves an SVG `src` filesystem path. Absolute paths are used as-is; a
/// relative path resolves against the config file's directory (available via
/// the runtime) so launching from any working directory finds the asset. When
/// the runtime/config path is unavailable (e.g. tests), falls back to the path
/// as given (resolved against the process cwd).
fn resolve_src_path(src: &str, runtime: Option<&NemoRuntime>) -> PathBuf {
    let path = PathBuf::from(src);
    if path.is_absolute() {
        return path;
    }
    if let Some(rt) = runtime {
        if let Some(dir) = rt.config_path().parent() {
            return dir.join(src);
        }
    }
    path
}

impl RenderOnce for Svg {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let width = self.source.properties.get("width").and_then(|v| v.as_i64());
        let height = self
            .source
            .properties
            .get("height")
            .and_then(|v| v.as_i64());

        let src = self.src.trim();
        let content = self.content.trim();

        // A `src` file/URL wins over embedded markup: the inline form never sets
        // `src`, so the two modes are unambiguous in practice.
        let source: Option<ImageSource> = if !src.is_empty() {
            if is_url(src) {
                // Network SVGs load via gpui's image pipeline (SVG fallback).
                Some(src.to_string().into())
            } else {
                // Local files are read from disk (`Resource::Path`), unlike the
                // bare-string form which resolves against the asset source. A
                // relative `src` resolves against the config file's directory
                // (matching `<script src>`/`<theme src>`), not the process cwd —
                // otherwise the file silently fails to load when the app is
                // launched from a different working directory.
                let path = resolve_src_path(src, self.runtime.as_deref());
                Some(path.into())
            }
        } else if !content.is_empty() && content.contains("<svg") {
            let bytes = ensure_xmlns(content).into_bytes();
            Some(ImageSource::Image(Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                bytes,
            ))))
        } else {
            None
        };

        let Some(source) = source else {
            return div().child("No SVG").into_any_element();
        };

        let mut element = img(source);
        if let Some(w) = width {
            element = element.w(px(w as f32));
        }
        if let Some(h) = height {
            element = element.h(px(h as f32));
        }

        // Wrap in an interactive div when a click and/or hover handler is
        // wired, so the whole graphic is hit-testable. `on-click` fires on press;
        // `on-hover` fires on enter (event_data "hover") and leave ("hover_end").
        let click_handler = self.source.handlers.get("click").cloned();
        let hover_handler = self.source.handlers.get("hover").cloned();
        let component_id = self.source.id.clone();
        if click_handler.is_some() || hover_handler.is_some() {
            if let (Some(runtime), Some(entity_id)) = (self.runtime, self.entity_id) {
                let mut wrapper = div().id(SharedString::from(component_id.clone()));
                if click_handler.is_some() {
                    wrapper = wrapper.cursor_pointer();
                }
                wrapper = wrapper.child(element);
                if let Some(handler) = click_handler {
                    let rt = Arc::clone(&runtime);
                    let id = component_id.clone();
                    wrapper = wrapper.on_click(move |_event, _window, cx| {
                        rt.call_handler(&handler, &id, "click");
                        cx.notify(entity_id);
                    });
                }
                if let Some(handler) = hover_handler {
                    let rt = Arc::clone(&runtime);
                    let id = component_id.clone();
                    wrapper = wrapper.on_hover(move |hovered, _window, cx| {
                        rt.call_handler(
                            &handler,
                            &id,
                            if *hovered { "hover" } else { "hover_end" },
                        );
                        cx.notify(entity_id);
                    });
                }
                return wrapper.into_any_element();
            }
        }

        element.into_any_element()
    }
}
