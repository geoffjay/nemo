use gpui::*;

use crate::app;
use crate::runtime;
use crate::workspace;
use crate::workspace::{FooterBar, HeaderBar};
use std::sync::Arc;

/// GPUI global that holds the active project state.
/// Set when a project is loaded, cleared when closed.
impl Global for ActiveProject {}

pub struct ActiveProject {
    pub runtime: Arc<runtime::NemoRuntime>,
    pub app_entity: Entity<app::App>,
    pub header_bar: Entity<HeaderBar>,
    pub footer_bar: Option<Entity<FooterBar>>,
    pub settings_view: Option<Entity<workspace::settings::SettingsView>>,
}
