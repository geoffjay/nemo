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
    github_url: Option<String>,
    theme_toggle: bool,
}

impl HeaderBar {
    pub fn new(
        title: String,
        github_url: Option<String>,
        theme_toggle: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            title,
            github_url,
            theme_toggle,
        }
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
        let title = self.title.clone();
        let github_url = self.github_url.clone();
        let show_theme_toggle = self.theme_toggle;

        let mut actions = div().flex().items_center().gap_1();

        if show_theme_toggle {
            let is_dark = cx.theme().mode.is_dark();
            let toggle = Button::new("theme-mode")
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
            actions = actions.child(toggle);
        }

        if let Some(url) = github_url {
            let github_button = Button::new("github")
                .icon(IconName::GitHub)
                .small()
                .ghost()
                .on_click(move |_, _, cx| cx.open_url(&url));
            actions = actions.child(github_button);
        }

        TitleBar::new().child(
            h_flex()
                .w_full()
                .h(px(32.))
                .pr_2()
                .justify_between()
                .child(Label::new(title).text_xs())
                .child(actions),
        )
    }
}
