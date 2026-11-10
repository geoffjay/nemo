//! Application layout — shared header bar + router outlet.
//!
//! This layout wraps the `/app` child routes (`/app` index = main, `/app/settings` = settings).
//! The project loader route `/` renders standalone without this layout.

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_kit::component::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_router::{IntoLayout, Outlet};

use crate::workspace::{FooterBar, HeaderBar};

/// The shared application layout with a header bar and outlet for child routes.
///
/// The header bar is `None` in kiosk mode; the footer bar is opt-in via
/// `app.window.footer_bar.enabled` (also suppressed in kiosk mode).
#[derive(IntoElement, IntoLayout)]
pub struct AppLayout {
    header_bar: Option<Entity<HeaderBar>>,
    footer_bar: Option<Entity<FooterBar>>,
    outlet: Outlet,
}

impl AppLayout {
    pub fn new(
        header_bar: Option<Entity<HeaderBar>>,
        footer_bar: Option<Entity<FooterBar>>,
    ) -> Self {
        Self {
            header_bar,
            footer_bar,
            outlet: Outlet::new(),
        }
    }
}

impl RenderOnce for AppLayout {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bg_color = cx.theme().colors.background;
        let text_color = cx.theme().colors.foreground;

        v_flex()
            .size_full()
            .overflow_hidden()
            .bg(bg_color)
            .text_color(text_color)
            .when_some(self.header_bar, |this, header: Entity<HeaderBar>| {
                this.child(header)
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.))
                    .child(self.outlet),
            )
            .when_some(self.footer_bar, |this, footer: Entity<FooterBar>| {
                this.child(footer)
            })
    }
}
