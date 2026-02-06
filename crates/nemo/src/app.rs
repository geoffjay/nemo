//! GPUI application wrapper.

use crate::runtime::NemoRuntime;
use anyhow::Result;
use gpui::*;
use nemo_layout::BuiltComponent;
use std::sync::Arc;

/// The main Nemo GPUI application.
pub struct NemoApp {
    runtime: Arc<NemoRuntime>,
}

impl NemoApp {
    /// Creates a new Nemo application.
    pub fn new(runtime: NemoRuntime) -> Self {
        Self {
            runtime: Arc::new(runtime),
        }
    }

    /// Runs the GPUI application.
    pub fn run(runtime: NemoRuntime) -> Result<()> {
        let app = Self::new(runtime);
        app.start()
    }

    /// Starts the GPUI application.
    fn start(self) -> Result<()> {
        Application::new().run(move |cx: &mut App| {
            // Get window configuration from config
            let title = self
                .runtime
                .get_config("app.window.title")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "Nemo Application".to_string());

            let width = self
                .runtime
                .get_config("app.window.width")
                .and_then(|v| v.as_i64())
                .unwrap_or(1200) as u32;

            let height = self
                .runtime
                .get_config("app.window.height")
                .and_then(|v| v.as_i64())
                .unwrap_or(800) as u32;

            // Create the main window
            let window_options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(width as f32), px(height as f32)),
                    cx,
                ))),
                ..Default::default()
            };

            let runtime = Arc::clone(&self.runtime);
            cx.open_window(window_options, |window, cx| {
                window.set_window_title(&title);
                cx.new(|_cx| NemoRootView::new(runtime))
            })
            .ok();
        });

        Ok(())
    }
}

/// The root view for the Nemo application.
pub struct NemoRootView {
    runtime: Arc<NemoRuntime>,
}

impl NemoRootView {
    /// Creates a new root view.
    pub fn new(runtime: Arc<NemoRuntime>) -> Self {
        Self { runtime }
    }
}

impl Render for NemoRootView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // Get theme colors from config or use defaults
        let bg_color = self
            .runtime
            .get_config("app.theme.background")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|s| parse_hex_color(&s))
            .unwrap_or_else(|| rgb(0x1e1e2e).into());

        let text_color = self
            .runtime
            .get_config("app.theme.foreground")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .and_then(|s| parse_hex_color(&s))
            .unwrap_or_else(|| rgb(0xcdd6f4).into());

        // Try to render from layout manager
        let layout_content = self.render_layout();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg_color)
            .text_color(text_color)
            .child(layout_content)
    }
}

impl NemoRootView {
    /// Renders the layout from the layout manager.
    fn render_layout(&self) -> Div {
        // Get the layout manager and render the component tree
        let layout_manager = self.runtime.tokio_runtime.block_on(async {
            self.runtime.layout_manager.read().await
        });

        // Get the root component
        if let Some(root_id) = layout_manager.root_id() {
            if let Some(root) = layout_manager.get_component(&root_id) {
                return self.render_component(root, &layout_manager);
            }
        }

        // Fall back to default view if no layout
        self.render_default_view()
    }

    /// Renders a component and its children recursively.
    fn render_component(
        &self,
        component: &BuiltComponent,
        layout_manager: &tokio::sync::RwLockReadGuard<'_, nemo_layout::LayoutManager>,
    ) -> Div {
        let mut container = match component.component_type.as_str() {
            "stack" => div().flex().flex_col().gap_4(),
            "panel" => div().flex().flex_col().p_4().rounded_md().bg(rgb(0x313244)),
            "label" => {
                let text = component
                    .properties
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return div().child(text.to_string());
            }
            "button" => {
                let label = component
                    .properties
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Button");

                let mut button = div()
                    .px_4()
                    .py_2()
                    .bg(rgb(0x89b4fa))
                    .text_color(rgb(0x1e1e2e))
                    .rounded_md()
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(0xb4befe)))
                    .child(label.to_string());

                // Wire up click handler if present
                if let Some(handler) = component.handlers.get("click") {
                    let runtime = Arc::clone(&self.runtime);
                    let component_id = component.id.clone();
                    let handler = handler.clone();

                    button = button.on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                        runtime.call_handler(&handler, &component_id, "click");
                    });
                }

                return button;
            }
            "text" => {
                let content = component
                    .properties
                    .get("content")
                    .or_else(|| component.properties.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                return div().child(content.to_string());
            }
            _ => div().flex().flex_col(),
        };

        // Render children
        for child_id in &component.children {
            if let Some(child) = layout_manager.get_component(child_id) {
                container = container.child(self.render_component(child, layout_manager));
            }
        }

        container
    }

    /// Renders the default view when no layout is configured.
    fn render_default_view(&self) -> Div {
        let title = self
            .runtime
            .get_config("app.title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Welcome to Nemo".to_string());

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                div()
                    .text_3xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_lg()
                    .text_color(rgb(0x6c7086))
                    .child("Configure your application in app.hcl"),
            )
    }
}

/// Parses a hex color string to a Hsla color.
fn parse_hex_color(s: &str) -> Option<Hsla> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }

    let hex = u32::from_str_radix(s, 16).ok()?;
    Some(rgb(hex).into())
}
