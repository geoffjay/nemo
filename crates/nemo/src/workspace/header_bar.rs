use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    ActiveTheme, IconName, Sizable as _, ThemeMode, TitleBar,
};
use tracing::debug;

use crate::theme::change_color_mode;

pub struct HeaderBar {
    title: String,
}

impl HeaderBar {
    pub fn new(title: String, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { title }
    }

    pub fn change_mode(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        debug!("changing theme mode, current mode: {:?}", cx.theme().mode);
        let new_mode = if cx.theme().mode.is_dark() {
            ThemeMode::Light
        } else {
            ThemeMode::Dark
        };
        change_color_mode(new_mode, window, cx);
    }
}

impl Render for HeaderBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_dark = cx.theme().mode.is_dark();

        let theme_toggle = Button::new("theme-mode")
            .map(|this| {
                if is_dark {
                    this.icon(IconName::Sun)
                } else {
                    this.icon(IconName::Moon)
                }
            })
            .small()
            .ghost()
            .on_click(cx.listener(Self::change_mode));

        let github_button = Button::new("github")
            .icon(IconName::GitHub)
            .small()
            .ghost()
            .on_click(|_, _, cx| cx.open_url("https://github.com/geoffjay/nemo"));

        let title = self.title.clone();

        TitleBar::new().child(
            h_flex()
                .w_full()
                .h(px(32.))
                .pr_2()
                .justify_between()
                .child(Label::new(title).text_xs())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(theme_toggle)
                        .child(github_button),
                ),
        )
    }
}
