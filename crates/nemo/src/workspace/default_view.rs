use gpui::*;
use gpui_component::v_flex;
use std::sync::Arc;

use crate::runtime::NemoRuntime;

pub struct DefaultView {
    runtime: Arc<NemoRuntime>,
}

impl DefaultView {
    pub fn new(runtime: Arc<NemoRuntime>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { runtime }
    }
}

impl Render for DefaultView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let title = self
            .runtime
            .get_config("app.title")
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "Welcome to Nemo".to_string());

        v_flex()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(div().text_3xl().font_weight(FontWeight::BOLD).child(title))
            .child(
                div()
                    .text_lg()
                    .text_color(rgb(0x6c7086))
                    .child("Configure your application in app.hcl"),
            )
    }
}
