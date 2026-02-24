use gpui::*;
use gpui_component::{Icon as GpuiIcon, IconName};
use nemo_macros::NemoComponent;

/// An icon display component.
///
/// # XML Configuration
///
/// ```xml
/// <icon id="status" name="check" />
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `name` | string | Icon name (e.g. `"check"`, `"bell"`, `"arrow-right"`) |
#[derive(IntoElement, NemoComponent)]
pub struct Icon {
    #[property(default = "info")]
    name: String,
}

pub(crate) fn map_icon_name(name: &str) -> IconName {
    match name {
        "a-large-small" => IconName::ALargeSmall,
        "arrow-down" => IconName::ArrowDown,
        "arrow-left" => IconName::ArrowLeft,
        "arrow-right" => IconName::ArrowRight,
        "arrow-up" => IconName::ArrowUp,
        "asterisk" => IconName::Asterisk,
        "bell" => IconName::Bell,
        "book" | "book-open" => IconName::BookOpen,
        "bot" => IconName::Bot,
        "building-2" => IconName::Building2,
        "calendar" => IconName::Calendar,
        "case-sensitive" => IconName::CaseSensitive,
        "chart-pie" => IconName::ChartPie,
        "check" => IconName::Check,
        "chevron-down" => IconName::ChevronDown,
        "chevron-left" => IconName::ChevronLeft,
        "chevron-right" => IconName::ChevronRight,
        "chevrons-up-down" => IconName::ChevronsUpDown,
        "chevron-up" => IconName::ChevronUp,
        "circle-check" => IconName::CircleCheck,
        "circle-user" => IconName::CircleUser,
        "circle-x" => IconName::CircleX,
        "close" | "x" => IconName::Close,
        "copy" => IconName::Copy,
        "dash" => IconName::Dash,
        "delete" | "trash" => IconName::Delete,
        "ellipsis" | "dots" => IconName::Ellipsis,
        "ellipsis-vertical" => IconName::EllipsisVertical,
        "external-link" => IconName::ExternalLink,
        "eye" => IconName::Eye,
        "eye-off" => IconName::EyeOff,
        "file" => IconName::File,
        "folder" => IconName::Folder,
        "folder-closed" => IconName::FolderClosed,
        "folder-open" => IconName::FolderOpen,
        "frame" => IconName::Frame,
        "gallery-vertical-end" => IconName::GalleryVerticalEnd,
        "github" => IconName::GitHub,
        "globe" => IconName::Globe,
        "heart" => IconName::Heart,
        "heart-off" => IconName::HeartOff,
        "inbox" => IconName::Inbox,
        "info" => IconName::Info,
        "inspector" => IconName::Inspector,
        "layout-dashboard" => IconName::LayoutDashboard,
        "loader" => IconName::Loader,
        "loader-circle" => IconName::LoaderCircle,
        "map" => IconName::Map,
        "maximize" => IconName::Maximize,
        "menu" => IconName::Menu,
        "minimize" => IconName::Minimize,
        "minus" => IconName::Minus,
        "moon" => IconName::Moon,
        "palette" => IconName::Palette,
        "panel-bottom" => IconName::PanelBottom,
        "panel-bottom-open" => IconName::PanelBottomOpen,
        "panel-left" => IconName::PanelLeft,
        "panel-left-close" => IconName::PanelLeftClose,
        "panel-left-open" => IconName::PanelLeftOpen,
        "panel-right" => IconName::PanelRight,
        "panel-right-close" => IconName::PanelRightClose,
        "panel-right-open" => IconName::PanelRightOpen,
        "plus" => IconName::Plus,
        "redo" => IconName::Redo,
        "redo-2" => IconName::Redo2,
        "replace" => IconName::Replace,
        "search" => IconName::Search,
        "settings" => IconName::Settings,
        "settings-2" => IconName::Settings2,
        "sort-ascending" => IconName::SortAscending,
        "sort-descending" => IconName::SortDescending,
        "square-terminal" => IconName::SquareTerminal,
        "star" => IconName::Star,
        "star-off" => IconName::StarOff,
        "sun" => IconName::Sun,
        "thumbs-down" => IconName::ThumbsDown,
        "thumbs-up" => IconName::ThumbsUp,
        "triangle-alert" => IconName::TriangleAlert,
        "undo" => IconName::Undo,
        "undo-2" => IconName::Undo2,
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
