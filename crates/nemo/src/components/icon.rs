use gpui::*;
use gpui_component::{Icon as GpuiIcon, IconName};
use nemo_macros::NemoComponent;

#[derive(IntoElement, NemoComponent)]
pub struct Icon {
    #[property(default = "info")]
    name: String,
}

pub(crate) fn map_icon_name(name: &str) -> IconName {
    match name {
        "arrow-down" => IconName::ArrowDown,
        "arrow-left" => IconName::ArrowLeft,
        "arrow-right" => IconName::ArrowRight,
        "arrow-up" => IconName::ArrowUp,
        "bell" => IconName::Bell,
        "book" | "book-open" => IconName::BookOpen,
        "calendar" => IconName::Calendar,
        "check" => IconName::Check,
        "chevron-down" => IconName::ChevronDown,
        "chevron-left" => IconName::ChevronLeft,
        "chevron-right" => IconName::ChevronRight,
        "chevron-up" => IconName::ChevronUp,
        "close" | "x" => IconName::Close,
        "copy" => IconName::Copy,
        "delete" | "trash" => IconName::Delete,
        "ellipsis" | "dots" => IconName::Ellipsis,
        "external-link" => IconName::ExternalLink,
        "eye" => IconName::Eye,
        "eye-off" => IconName::EyeOff,
        "file" => IconName::File,
        "folder" => IconName::Folder,
        "folder-open" => IconName::FolderOpen,
        "github" => IconName::GitHub,
        "globe" => IconName::Globe,
        "heart" => IconName::Heart,
        "inbox" => IconName::Inbox,
        "info" => IconName::Info,
        "loader" => IconName::Loader,
        "menu" => IconName::Menu,
        "minus" => IconName::Minus,
        "moon" => IconName::Moon,
        "plus" => IconName::Plus,
        "search" => IconName::Search,
        "settings" => IconName::Settings,
        "star" => IconName::Star,
        "sun" => IconName::Sun,
        "user" => IconName::User,
        _ => IconName::Info,
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon_name = map_icon_name(&self.name);
        GpuiIcon::new(icon_name)
    }
}
