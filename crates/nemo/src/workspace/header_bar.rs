use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    label::Label,
    menu::{DropdownMenu as _, PopupMenuItem},
    ActiveTheme, Icon as GpuiIcon, IconName, Sizable as _, ThemeMode, TitleBar,
};
use tracing::debug;

use super::actions::ToggleDevPanel;
use crate::components::icon::map_icon_name;
use crate::runtime::NemoRuntime;
use crate::theme::change_color_mode;
use crate::theme::tokens::{FontSize, Space, TokenStyled};

/// A single application-provided entry in the header-bar menu.
#[derive(Clone)]
pub struct MenuItemConfig {
    pub label: String,
    pub icon: Option<String>,
    pub handler: Option<String>,
    pub separator: bool,
}

/// Reads the app's `<header-bar>` `<menu-item>` declarations from config into a
/// list of [`MenuItemConfig`]. Returns empty when no menu is declared, which is
/// what keeps the header menu opt-in.
pub fn menu_items_from_config(runtime: &Arc<NemoRuntime>) -> Vec<MenuItemConfig> {
    let Some(items) = runtime
        .get_config("app.window.header_bar.menu_items")
        .and_then(|v| v.as_array().cloned())
    else {
        return Vec::new();
    };

    items
        .iter()
        .filter_map(|item| {
            let obj = item.as_object()?;
            let separator = obj
                .get("separator")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let label = obj
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            // A non-separator entry must have a label to be useful.
            if !separator && label.is_empty() {
                return None;
            }
            let icon = obj
                .get("icon")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let handler = obj
                .get("on_click")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            Some(MenuItemConfig {
                label,
                icon,
                handler,
                separator,
            })
        })
        .collect()
}

pub struct HeaderBar {
    title: String,
    github_url: Option<String>,
    theme_toggle: bool,
    menu_items: Vec<MenuItemConfig>,
    runtime: Arc<NemoRuntime>,
    /// Whether the app was launched via `nemo dev` (shows the dev-panel button).
    dev_mode: bool,
}

impl HeaderBar {
    pub fn new(
        title: String,
        github_url: Option<String>,
        theme_toggle: bool,
        menu_items: Vec<MenuItemConfig>,
        runtime: Arc<NemoRuntime>,
        dev_mode: bool,
    ) -> Self {
        Self {
            title,
            github_url,
            theme_toggle,
            menu_items,
            runtime,
            dev_mode,
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
        let show_dev_button = self.dev_mode;
        let show_theme_toggle = self.theme_toggle;

        // Far-left hamburger menu, shown only when the app declared entries.
        let menu = if self.menu_items.is_empty() {
            None
        } else {
            let items = self.menu_items.clone();
            let runtime = Arc::clone(&self.runtime);
            // A single hamburger icon that opens the menu directly (the
            // `DropdownMenu` trait attaches a popup to the button itself, so
            // there's no separate chevron toggle).
            let dropdown = Button::new("header-menu")
                .icon(IconName::Menu)
                .small()
                .ghost()
                .dropdown_menu(move |mut menu, _window, _cx| {
                    for item in &items {
                        if item.separator {
                            menu = menu.separator();
                            continue;
                        }
                        let mut entry = PopupMenuItem::new(SharedString::from(item.label.clone()));
                        if let Some(icon) = &item.icon {
                            entry = entry.icon(GpuiIcon::new(map_icon_name(icon)));
                        }
                        if let Some(handler) = item.handler.clone() {
                            let runtime = Arc::clone(&runtime);
                            entry = entry.on_click(move |_ev, _window, _cx| {
                                runtime.call_handler(&handler, "header-bar", "click");
                            });
                        }
                        menu = menu.item(entry);
                    }
                    menu
                });
            Some(dropdown)
        };

        let mut actions = div().flex().items_center().gap_t(Space::Xs);

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

        if show_dev_button {
            let dev_button = Button::new("dev-panel")
                .icon(IconName::SquareTerminal)
                .small()
                .ghost()
                .on_click(move |_, window, cx| {
                    window.dispatch_action(Box::new(ToggleDevPanel), cx);
                });
            actions = actions.child(dev_button);
        }

        if let Some(url) = github_url {
            let github_button = Button::new("github")
                .icon(IconName::ExternalLink)
                .small()
                .ghost()
                .on_click(move |_, _, cx| cx.open_url(&url));
            actions = actions.child(github_button);
        }

        TitleBar::new().child(
            h_flex()
                .w_full()
                .h(px(32.))
                .pl_t(Space::Sm)
                .pr_t(Space::Sm)
                .justify_between()
                .child(
                    h_flex()
                        .items_center()
                        .gap_t(Space::Xs)
                        .when_some(menu, |this, menu| this.child(menu))
                        .child(Label::new(title).text_t(FontSize::Xs)),
                )
                .child(actions),
        )
    }
}
