//! Main application page — renders the HCL layout content.
//!
//! This is the content shown at the `/app` route.
//! It creates and manages an `app::App` entity that handles HCL layout rendering.

use gpui::*;
use std::sync::Arc;

use crate::app;
use crate::runtime::NemoRuntime;

#[allow(dead_code)]
pub struct MainView {
    app_entity: Entity<app::App>,
}

#[allow(dead_code)]
impl MainView {
    pub fn new(runtime: Arc<NemoRuntime>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let app_entity = cx.new(|cx| app::App::new(runtime, window, cx));
        Self { app_entity }
    }

    pub fn shutdown(&self, cx: &mut Context<Self>) {
        self.app_entity.update(cx, |a, cx| {
            a.shutdown(cx);
        });
    }
}

impl Render for MainView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.app_entity.clone()
    }
}
