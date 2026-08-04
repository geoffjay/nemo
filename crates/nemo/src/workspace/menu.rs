//! Native macOS application menu bar.
//!
//! Builds the `Vec<Menu>` handed to `cx.set_menus(...)` in `build_app_window`.
//! Menu items dispatch the existing `nemo` actions (see [`super::actions`]);
//! selection routes through the active window to the `Workspace` root element's
//! `.on_action` listeners — the same path the dev-panel header button uses. No
//! separate handler wiring is needed here.
//!
//! Item accelerators are auto-derived by the platform from the keymap bindings
//! registered for each action type (see `cx.bind_keys` in `main.rs`), so we do
//! not set shortcuts per item.

use gpui::{Menu, MenuItem, NoAction, OsAction, SystemMenuType};

use super::actions::{
    CloseProject, OpenProject, OpenSettings, QuitApp, ReloadConfig, ShowKeyboardShortcuts,
    ToggleDevPanel, ToggleTheme,
};

/// Builds the application menu bar.
///
/// `app_title` names the first (application) menu; `dev_mode` gates the
/// dev-panel toggle so it only appears when launched via `nemo dev`.
pub fn app_menus(app_title: String, dev_mode: bool) -> Vec<Menu> {
    let mut view_items = vec![MenuItem::action("Toggle Theme", ToggleTheme)];
    if dev_mode {
        view_items.push(MenuItem::action("Toggle Dev Panel", ToggleDevPanel));
    }

    vec![
        Menu::new(app_title).items([
            MenuItem::os_submenu("Services", SystemMenuType::Services),
            MenuItem::separator(),
            MenuItem::action("Settings…", OpenSettings),
            MenuItem::separator(),
            MenuItem::action("Reload Config", ReloadConfig),
            MenuItem::separator(),
            MenuItem::action("Quit", QuitApp),
        ]),
        Menu::new("File").items([
            MenuItem::action("Open Project…", OpenProject),
            MenuItem::action("Close Project", CloseProject),
        ]),
        // OS actions so text inputs get native Cut/Copy/Paste/Select All.
        // Undo/Redo are intentionally omitted — disabled in this gpui build.
        Menu::new("Edit").items([
            MenuItem::os_action("Cut", NoAction, OsAction::Cut),
            MenuItem::os_action("Copy", NoAction, OsAction::Copy),
            MenuItem::os_action("Paste", NoAction, OsAction::Paste),
            MenuItem::separator(),
            MenuItem::os_action("Select All", NoAction, OsAction::SelectAll),
        ]),
        Menu::new("View").items(view_items),
        Menu::new("Help").items([MenuItem::action(
            "Keyboard Shortcuts",
            ShowKeyboardShortcuts,
        )]),
    ]
}
