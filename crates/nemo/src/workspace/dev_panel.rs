//! Dev-mode configuration builder panel.
//!
//! Opens in a separate GPUI window under `nemo dev`. Provides a collapsible
//! file tree (left), a syntax-highlighted code editor (center), and compile
//! actions (compile current file / build whole project).
//!
//! See `docs/knowledgebase/plans/config-dev-env.md`.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input as GpuiInput, InputEvent, InputState};
use gpui_component::label::Label;
use gpui_component::list::ListItem;
use gpui_component::notification::{Notification as Toast, NotificationType};
use gpui_component::tree::{Tree as GpuiTree, TreeItem, TreeState};
use gpui_component::{
    h_flex, v_flex, ActiveTheme, Disableable, IconName, Sizable as _, StyledExt, TitleBar,
    WindowExt as _,
};
use nemo_config::{ProjectManifest, MANIFEST_FILE};

use crate::runtime::NemoRuntime;
use crate::theme::tokens::{FontSize, Space, TokenStyled};

/// Directories to exclude from the file tree.
const EXCLUDED_DIRS: &[&str] = &["target", ".git", "node_modules", "dist", ".nemo"];

/// Maximum tree depth to prevent runaway recursion in huge trees.
const MAX_TREE_DEPTH: usize = 15;

/// The dev panel entity — file tree + code editor + compile toolbar.
pub struct DevPanel {
    /// The project root directory (tree root + compile context).
    project_root: PathBuf,
    /// The currently-open file path, if any.
    selected_file: Option<PathBuf>,
    /// Whether the editor has unsaved changes.
    dirty: bool,
    /// The code editor state (syntax-highlighted multi-line input).
    editor_state: Entity<InputState>,
    /// The file-tree state.
    tree_state: Entity<TreeState>,
    /// The runtime (for future reload coordination after save).
    #[allow(dead_code)]
    runtime: Arc<NemoRuntime>,
    /// The last selected tree item id we processed (to detect selection
    /// changes, since TreeState doesn't emit events).
    last_selected_id: Option<SharedString>,
    /// Whether to auto-load app.xml on the first render.
    pending_default_load: bool,
}

impl DevPanel {
    /// Creates a new dev panel rooted at the given project directory.
    pub fn new(
        project_root: PathBuf,
        runtime: Arc<NemoRuntime>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Build the file tree.
        let tree_items = build_file_tree(&project_root);
        let tree_state = cx.new(|cx| TreeState::new(cx).items(tree_items));

        // Build the code editor state — start in code-editor mode with HTML
        // highlighting (the default for .nemo/.xml files).
        let editor_state = cx.new(|cx| {
            InputState::new(window, cx)
                .multi_line(true)
                .code_editor("html")
                .line_number(true)
                .searchable(true)
                .placeholder("Select a file from the tree to edit…")
        });

        // Subscribe to editor events to track unsaved changes.
        {
            let entity = cx.entity().downgrade();
            cx.subscribe_in(
                &editor_state,
                window,
                move |this: &mut DevPanel, _state, event: &InputEvent, _window, cx| {
                    if matches!(event, InputEvent::Change | InputEvent::PressEnter { .. }) {
                        this.dirty = true;
                        cx.notify();
                    }
                    let _ = entity; // keep the weak handle alive
                },
            )
            .detach();
        }

        Self {
            project_root: project_root.clone(),
            selected_file: None,
            dirty: false,
            editor_state,
            tree_state,
            runtime,
            last_selected_id: None,
            pending_default_load: true,
        }
    }

    /// Saves the current editor content to disk if a file is open.
    fn save_file(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.selected_file.clone() else {
            return;
        };
        let content = self.editor_state.read(cx).value().to_string();
        match std::fs::write(&path, &content) {
            Ok(()) => {
                self.dirty = false;
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to save {}: {}", path.display(), e);
            }
        }
    }

    /// Loads a file into the editor.
    fn load_file(&mut self, path: &Path, window: &mut Window, cx: &mut Context<Self>) {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                let language = language_for_path(path);
                self.editor_state.update(cx, |state, cx| {
                    state.set_highlighter(language, cx);
                });
                self.editor_state.update(cx, |state, cx| {
                    state.set_value(content, window, cx);
                });
                self.selected_file = Some(path.to_path_buf());
                self.dirty = false;
                cx.notify();
            }
            Err(e) => {
                tracing::error!("Failed to load {}: {}", path.display(), e);
            }
        }
    }

    /// Compiles the currently-open `.nemo` file to a component artifact.
    fn compile_file(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(path) = self.selected_file.clone() else {
            window_push_error(_window, cx, "No file open to compile.");
            return;
        };
        // Reuse the build command's compile logic.
        match crate::commands::build::build_single_component(&path) {
            Ok(()) => {
                window_push_success(
                    _window,
                    cx,
                    format!(
                        "Compiled {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ),
                );
            }
            Err(e) => {
                window_push_error(_window, cx, &format!("Compile failed: {e}"));
            }
        }
    }

    /// Builds the whole project to `dist/`.
    fn build_project(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let root = self.project_root.clone();
        // Resolve the manifest.
        let manifest_path = root.join(MANIFEST_FILE);
        match ProjectManifest::load(&manifest_path) {
            Ok(manifest) => {
                if let Some(pkg) = &manifest.package {
                    match crate::commands::build::build_package(&root, &manifest, pkg) {
                        Ok(()) => window_push_success(_window, cx, "Package built successfully."),
                        Err(e) => window_push_error(_window, cx, &format!("Build failed: {e}")),
                    }
                } else {
                    match crate::commands::build::build_project(&root, &manifest) {
                        Ok(()) => window_push_success(_window, cx, "Project built to dist/."),
                        Err(e) => window_push_error(_window, cx, &format!("Build failed: {e}")),
                    }
                }
            }
            Err(e) => {
                window_push_error(_window, cx, &format!("No nemo.toml found: {e}"));
            }
        }
    }

    /// Processes a tree selection change by checking TreeState's selected_item.
    /// Called from render since TreeState doesn't emit events.
    fn process_tree_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.tree_state.read(cx).selected_item().cloned();
        let current_id = selected.as_ref().map(|i| i.id.clone());

        if current_id == self.last_selected_id {
            return;
        }
        self.last_selected_id = current_id.clone();

        if let Some(id) = current_id {
            // The tree item id is the full file path.
            let path = PathBuf::from(id.to_string());
            if path.is_file() {
                self.load_file(&path, window, cx);
            }
        }
    }
}

impl Render for DevPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Auto-load app.xml on first render.
        if self.pending_default_load {
            self.pending_default_load = false;
            let default_file = self.project_root.join("app.xml");
            if default_file.is_file() {
                self.load_file(&default_file, window, cx);
            }
        }

        // Poll for tree selection changes (TreeState has no events).
        self.process_tree_selection(window, cx);

        let bg = cx.theme().colors.background;
        let panel_bg = cx.theme().colors.title_bar;
        let border = cx.theme().colors.border;
        let title_text = self
            .selected_file
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "No file".to_string());

        // File tree (left sidebar).
        let tree_state = self.tree_state.clone();
        let tree = GpuiTree::new(&tree_state, |ix, entry, _selected, _window, _cx| {
            let item = entry.item();
            let icon = if item.is_folder() {
                IconName::Folder
            } else {
                file_icon(item.id.as_ref())
            };
            ListItem::new(ix)
                .pl(px(8. + 16. * entry.depth() as f32))
                .pr(px(8.))
                .child(
                    h_flex()
                        .gap_2()
                        .child(gpui_component::Icon::new(icon).size(px(14.)))
                        .child(item.label.clone()),
                )
        });

        let tree_panel = v_flex()
            .w(px(240.))
            .h_full()
            .bg(panel_bg)
            .border_r_1()
            .border_color(border)
            .child(
                h_flex()
                    .h(px(32.))
                    .items_center()
                    .px_3()
                    .border_b_1()
                    .border_color(border)
                    .child(Label::new("Files").text_t(FontSize::Xs).font_semibold()),
            )
            .child(div().flex_1().overflow_hidden().child(tree));
        // Editor area (center).
        let editor_input = GpuiInput::new(&self.editor_state).h_full();
        let editor_area = v_flex()
            .flex_1()
            .h_full()
            .bg(bg)
            .child(
                h_flex()
                    .h(px(32.))
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(Label::new(title_text).text_t(FontSize::Xs))
                            .when(self.dirty, |this| {
                                this.child(
                                    div()
                                        .size(px(6.))
                                        .rounded_full()
                                        .bg(cx.theme().colors.accent),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_t(Space::Xs)
                            .child(
                                Button::new("save")
                                    .xsmall()
                                    .icon(IconName::Check)
                                    .disabled(!self.dirty)
                                    .on_click(cx.listener(Self::save_file)),
                            )
                            .child(
                                Button::new("compile-file")
                                    .label("Compile File")
                                    .xsmall()
                                    .ghost()
                                    .on_click(cx.listener(Self::compile_file)),
                            )
                            .child(
                                Button::new("build-project")
                                    .label("Build Project")
                                    .xsmall()
                                    .ghost()
                                    .on_click(cx.listener(Self::build_project)),
                            ),
                    ),
            )
            .child(div().flex_1().overflow_hidden().child(editor_input));

        let toolbar = TitleBar::new().child(
            h_flex()
                .w_full()
                .h(px(32.))
                .items_center()
                .px_3()
                .border_b_1()
                .border_color(border)
                .bg(panel_bg)
                .child(
                    h_flex()
                        .items_center()
                        .gap_2()
                        .child(
                            gpui_component::Icon::new(IconName::SquareTerminal)
                                .size(px(14.))
                                .text_color(cx.theme().colors.accent),
                        )
                        .child(Label::new("Dev Panel").text_t(FontSize::Xs).font_semibold()),
                ),
        );

        v_flex().size_full().bg(bg).child(toolbar).child(
            h_flex()
                .flex_1()
                .h_full()
                .overflow_hidden()
                .child(tree_panel)
                .child(editor_area),
        )
    }
}

/// Builds a `TreeItem` list from a directory tree, filtering out build/VCS dirs.
fn build_file_tree(root: &Path) -> Vec<TreeItem> {
    let mut items = build_dir_items(root, 0);
    // Sort: directories first, then files, both alphabetical.
    items.sort_by(|a, b| {
        let a_is_dir = !a.children.is_empty();
        let b_is_dir = !b.children.is_empty();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.label.cmp(&b.label),
        }
    });
    items
}

/// Recursively builds tree items for a directory's children.
fn build_dir_items(dir: &Path, depth: usize) -> Vec<TreeItem> {
    if depth >= MAX_TREE_DEPTH {
        return Vec::new();
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut items = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip excluded directories and hidden files.
        if EXCLUDED_DIRS.contains(&name.as_str()) || name.starts_with('.') {
            continue;
        }

        let id = path.to_string_lossy().to_string();
        if path.is_dir() {
            let children = build_dir_items(&path, depth + 1);
            let item = TreeItem::new(id, name)
                .expanded(depth == 0)
                .children(children);
            items.push(item);
        } else {
            // Only show config-relevant files.
            if is_editable_file(&path) {
                items.push(TreeItem::new(id, name));
            }
        }
    }
    items
}

/// Whether a file should appear in the dev tree.
fn is_editable_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("xml") | Some("nemo") | Some("toml") | Some("rhai") | Some("rs")
    )
}

/// Determines the tree-sitter language for syntax highlighting by extension.
fn language_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("nemo") | Some("xml") | Some("html") => "html",
        Some("toml") => "toml",
        Some("rs") => "rust",
        Some("rhai") => "rust", // Rhai is JS-like; rust is closest available
        Some("json") => "json",
        Some("js") | Some("ts") => "typescript",
        Some("md") => "markdown",
        Some("py") => "python",
        _ => "text",
    }
}

/// Maps a file extension to an icon name for the tree.
fn file_icon(id: &str) -> IconName {
    let ext = Path::new(id).extension().and_then(|e| e.to_str());
    match ext {
        Some("nemo") | Some("xml") | Some("rhai") | Some("rs") => IconName::File,
        Some("toml") => IconName::File,
        _ => IconName::File,
    }
}

/// Pushes a success toast notification.
fn window_push_success(window: &mut Window, cx: &mut App, msg: impl Into<SharedString>) {
    let toast = Toast::new()
        .message(msg)
        .with_type(NotificationType::Success);
    window.push_notification(toast, cx);
}

/// Pushes an error toast notification.
fn window_push_error(window: &mut Window, cx: &mut App, msg: &str) {
    let toast = Toast::new()
        .message(SharedString::from(msg.to_string()))
        .with_type(NotificationType::Error);
    window.push_notification(toast, cx);
}

#[cfg(test)]
mod tests {
    use super::{is_editable_file, language_for_path};
    use std::path::Path;

    #[test]
    fn language_inference_by_extension() {
        assert_eq!(language_for_path(Path::new("app.xml")), "html");
        assert_eq!(language_for_path(Path::new("button.nemo")), "html");
        assert_eq!(language_for_path(Path::new("nemo.toml")), "toml");
        assert_eq!(language_for_path(Path::new("main.rs")), "rust");
        assert_eq!(language_for_path(Path::new("handler.rhai")), "rust");
        assert_eq!(language_for_path(Path::new("data.json")), "json");
        assert_eq!(language_for_path(Path::new("unknown.xyz")), "text");
    }

    #[test]
    fn editable_file_filter() {
        assert!(is_editable_file(Path::new("app.xml")));
        assert!(is_editable_file(Path::new("button.nemo")));
        assert!(is_editable_file(Path::new("nemo.toml")));
        assert!(is_editable_file(Path::new("handler.rhai")));
        assert!(!is_editable_file(Path::new("README.md")));
        assert!(!is_editable_file(Path::new("data.csv")));
    }
}
