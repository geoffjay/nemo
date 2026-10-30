//! Dev-mode configuration builder panel.
//!
//! Opens in a separate GPUI window under `nemo dev`. Provides a collapsible
//! file tree (left), a syntax-highlighted code editor (center), and compile
//! actions (compile current file / build whole project).
//!
//! See `docs/knowledgebase/plans/config-dev-env.md`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Editor as GpuiEditor, EditorState, InputEvent};
use gpui_kit::component::label::Label;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::notification::{Notification as Toast, NotificationType};
use gpui_kit::component::tree::{Tree as GpuiTree, TreeItem, TreeState};
use gpui_kit::component::{
    h_flex, v_flex, ActiveTheme, Disableable, IconName, Sizable as _, StyledExt, TitleBar,
    WindowExt as _,
};
use nemo_config::{ProjectManifest, MANIFEST_FILE};
use nemo_layout::BuiltComponent;

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
    editor_state: Entity<EditorState>,
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
    /// Which page the panel shows: file editing or the object browser.
    active_tab: Tab,
    /// The object-browser state: node tree + selection + snapshot.
    object_browser: ObjectBrowser,
}

/// The dev panel's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    /// File tree + editor (existing page).
    Files,
    /// Component node tree + details (the object browser).
    Objects,
}

/// The object-browser page state.
struct ObjectBrowser {
    /// The component node-tree state.
    tree_state: Entity<TreeState>,
    /// The runtime snapshot the tree was built from (id -> component),
    /// retained so the details pane can read the selected node without
    /// re-locking the layout manager.
    snapshot: HashMap<String, BuiltComponent>,
    /// Root id of the snapshot (may be `None` if the app has no layout).
    root_id: Option<String>,
    /// Active bindings per component id (component id -> description lines).
    bindings: HashMap<String, Vec<String>>,
    /// The selected node id (flat-map key), if any.
    selected: Option<String>,
    /// The last selected tree item id we processed (selection-change
    /// detection; TreeState doesn't emit events).
    last_selected_id: Option<SharedString>,
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
            EditorState::new(window, cx)
                .language("html")
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
            active_tab: Tab::Files,
            object_browser: ObjectBrowser {
                tree_state: cx.new(|cx| TreeState::new(cx).items(Vec::<TreeItem>::new())),
                snapshot: HashMap::new(),
                root_id: None,
                bindings: HashMap::new(),
                selected: None,
                last_selected_id: None,
            },
        }
    }

    /// Snapshots the runtime's layout manager into the object browser:
    /// the flat component map + root id, and rebuilds the node tree items.
    fn refresh_object_browser(&mut self, cx: &mut Context<Self>) {
        let layout_manager = self
            .runtime
            .layout_manager
            .read()
            .expect("layout_manager lock poisoned");
        let root_id = layout_manager.root_id();
        let snapshot: HashMap<String, BuiltComponent> = layout_manager
            .component_ids()
            .into_iter()
            .filter_map(|id| layout_manager.get_component(&id).cloned().map(|c| (id, c)))
            .collect();
        // Active bindings targeting each component (component id -> source
        // descriptions), captured in the same snapshot.
        let bindings: HashMap<String, Vec<String>> = layout_manager
            .component_ids()
            .into_iter()
            .map(|id| {
                let sources: Vec<String> = layout_manager
                    .bindings()
                    .bindings_for_component(&id)
                    .into_iter()
                    .map(|b| {
                        format!(
                            "{} → {}:{} ({:?})",
                            b.source, b.target.component_id, b.target.property_path, b.mode,
                        )
                    })
                    .collect();
                (id, sources)
            })
            .filter(|(_, sources)| !sources.is_empty())
            .collect();
        drop(layout_manager);

        let items = build_object_tree(&snapshot, root_id.as_deref());
        self.object_browser.root_id = root_id;
        self.object_browser.snapshot = snapshot;
        self.object_browser.bindings = bindings;
        self.object_browser
            .tree_state
            .update(cx, |state, cx| state.set_items(items, cx));
        cx.notify();
    }

    /// Processes object-tree selection changes (TreeState has no events).
    fn process_object_selection(&mut self, cx: &mut Context<Self>) {
        let selected = self
            .object_browser
            .tree_state
            .read(cx)
            .selected_item()
            .cloned();
        let current_id = selected.as_ref().map(|i| i.id.clone());

        if current_id == self.object_browser.last_selected_id {
            return;
        }
        self.object_browser.last_selected_id = current_id.clone();
        self.object_browser.selected = current_id.map(|s| s.to_string());
        cx.notify();
    }

    /// Renders the object-browser page: node tree (left) + details (right).
    fn render_object_browser(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let bg = cx.theme().colors.background;
        let panel_bg = cx.theme().colors.title_bar;
        let border = cx.theme().colors.border;
        let muted = cx.theme().colors.muted_foreground;

        // Node tree (left).
        let tree_state = self.object_browser.tree_state.clone();
        let tree = GpuiTree::new(&tree_state, |ix, entry, _selected, _window, _cx| {
            let item = entry.item();
            ListItem::new(ix)
                .pl(px(8. + 16. * entry.depth() as f32))
                .pr(px(8.))
                .child(
                    h_flex()
                        .gap_2()
                        .child(gpui_kit::component::Icon::new(IconName::Frame).size(px(12.)))
                        .child(item.label.clone()),
                )
        });
        let node_count = self.object_browser.snapshot.len();

        // Fixed 40% of the panel window width, min 320px (user spec; no
        // resizable-split component exists in the kit yet).
        let window_width = window.viewport_size().width;
        let tree_width = window_width.max(px(640.)) * 0.40;

        let tree_panel = v_flex()
            .w(tree_width.min(window_width))
            .min_w(px(320.))
            .h_full()
            .bg(panel_bg)
            .border_r_1()
            .border_color(border)
            .child(
                h_flex()
                    .h(px(32.))
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(border)
                    .child(
                        Label::new(format!("Nodes ({node_count})"))
                            .text_t(FontSize::Xs)
                            .font_semibold(),
                    )
                    .child(
                        Button::new("refresh-nodes")
                            .xsmall()
                            .ghost()
                            .icon(IconName::RefreshCw)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_object_browser(cx);
                            })),
                    ),
            )
            .child(div().flex_1().overflow_hidden().child(tree));

        // Details (right).
        let details = self
            .object_browser
            .selected
            .as_deref()
            .and_then(|id| self.object_browser.snapshot.get(id))
            .cloned();

        let details_panel = v_flex()
            .id("object-details")
            .flex_1()
            .h_full()
            .bg(bg)
            .overflow_y_scroll()
            .child(match details {
                Some(component) => render_node_details(&self.object_browser, &component, cx),
                None => v_flex().p_4().child(
                    Label::new(if self.object_browser.root_id.is_some() {
                        "Select a node from the tree"
                    } else {
                        "No layout loaded — the app has not rendered a configuration"
                    })
                    .text_t(FontSize::Sm)
                    .text_color(muted),
                ),
            });

        div()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .h_flex()
            .child(tree_panel)
            .child(details_panel)
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
        // Auto-load the project's app.nemo entry on first render.
        if self.pending_default_load {
            self.pending_default_load = false;
            let nemo = self.project_root.join("app.nemo");
            if nemo.is_file() {
                self.load_file(&nemo, window, cx);
            }
        }

        // Poll for tree selection changes (TreeState has no events).
        if self.active_tab == Tab::Objects {
            self.process_object_selection(cx);
        } else {
            self.process_tree_selection(window, cx);
        }

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
                        .child(gpui_kit::component::Icon::new(icon).size(px(14.)))
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
        let editor_input = GpuiEditor::new(&self.editor_state).h(relative(1.0));
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
                            gpui_kit::component::Icon::new(IconName::SquareTerminal)
                                .size(px(14.))
                                .text_color(cx.theme().colors.accent),
                        )
                        .child(Label::new("Dev Panel").text_t(FontSize::Xs).font_semibold())
                        .child(
                            Button::new("tab-files")
                                .xsmall()
                                .when(self.active_tab == Tab::Files, |b| b.primary())
                                .when(self.active_tab != Tab::Files, |b| b.ghost())
                                .label("Files")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.active_tab = Tab::Files;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("tab-objects")
                                .xsmall()
                                .when(self.active_tab == Tab::Objects, |b| b.primary())
                                .when(self.active_tab != Tab::Objects, |b| b.ghost())
                                .label("Objects")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.active_tab = Tab::Objects;
                                    this.refresh_object_browser(cx);
                                })),
                        ),
                ),
        );

        let body = if self.active_tab == Tab::Objects {
            self.render_object_browser(window, cx)
        } else {
            h_flex()
                .flex_1()
                .h_full()
                .overflow_hidden()
                .child(tree_panel)
                .child(editor_area)
        };

        v_flex().size_full().bg(bg).child(toolbar).child(body)
    }
}

/// Maximum depth for the object-browser node tree (mirrors `MAX_TREE_DEPTH`).
const MAX_NODE_DEPTH: usize = 40;

/// Builds the object-browser `TreeItem` list from a `LayoutManager` snapshot,
/// walking the tree from `root_id`. Label format: `type #id` (id omitted for
/// synthetic ids the panel can't attribute).
fn build_object_tree(
    components: &HashMap<String, BuiltComponent>,
    root_id: Option<&str>,
) -> Vec<TreeItem> {
    let Some(root_id) = root_id else {
        return Vec::new();
    };
    build_object_children(components, root_id, 0)
}

/// Recursively converts a component's children (by id) into tree items.
fn build_object_children(
    components: &HashMap<String, BuiltComponent>,
    id: &str,
    depth: usize,
) -> Vec<TreeItem> {
    let Some(component) = components.get(id) else {
        return Vec::new();
    };
    node_tree_item(component, components, depth)
        .map(|item| vec![item])
        .unwrap_or_default()
}

/// Converts one component into a (collapsed-by-default) tree item with its
/// descendants; `None` when the id is missing or the depth limit is hit.
fn node_tree_item(
    component: &BuiltComponent,
    components: &HashMap<String, BuiltComponent>,
    depth: usize,
) -> Option<TreeItem> {
    if depth > MAX_NODE_DEPTH {
        return None;
    }
    let label = label_for_component(component);
    let children: Vec<TreeItem> = component
        .children
        .iter()
        .filter_map(|child_id| components.get(child_id))
        .filter_map(|child| node_tree_item(child, components, depth + 1))
        .collect();
    let item = TreeItem::new(component.id.clone(), label);
    Some(if children.is_empty() {
        item
    } else {
        item.children(children)
    })
}

/// The tree label for a component: `type #id` when an authored id exists,
/// `type (anon)` otherwise.
fn label_for_component(component: &BuiltComponent) -> String {
    match component.id.is_empty() {
        _ if is_synthetic_id(&component.id) => {
            format!("{} {}", component.component_type, component.id)
        }
        true => format!("{} (anon)", component.component_type),
        false => format!("{} #{}", component.component_type, component.id),
    }
}

/// Whether an id was generated by the pipeline rather than authored
/// (`__dyn_N` from runtime list bindings, `__anon_N` from the parser).
fn is_synthetic_id(id: &str) -> bool {
    id.starts_with("__dyn_") || id.starts_with("__anon_")
}

/// Renders the details pane for a selected component.
fn render_node_details(details: &ObjectBrowser, component: &BuiltComponent, cx: &gpui::App) -> Div {
    let components = &details.snapshot;
    let muted = cx.theme().colors.muted_foreground;
    let accent = cx.theme().colors.accent;
    let border = cx.theme().colors.border;

    let mut pane = v_flex().p_4().gap_3();

    // Header: type + id + parent.
    pane = pane.child(
        v_flex()
            .gap_1()
            .child(
                Label::new(component.component_type.clone())
                    .text_t(FontSize::Lg)
                    .font_semibold(),
            )
            .child(
                Label::new(if component.id.is_empty() {
                    "(no id)".to_string()
                } else if is_synthetic_id(&component.id) {
                    format!("{} (generated)", component.id)
                } else {
                    component.id.clone()
                })
                .text_t(FontSize::Sm)
                .text_color(accent),
            )
            .when_some(component.parent.clone(), |pane, parent| {
                pane.child(
                    Label::new(format!("parent: {parent}"))
                        .text_t(FontSize::Xs)
                        .text_color(muted),
                )
            }),
    );

    // Properties (sorted for stable reading).
    pane = pane.child(section_heading("Properties").text_color(muted));
    if component.properties.is_empty() {
        pane = pane.child(muted_note("none"));
    } else {
        let mut props: Vec<(&String, &nemo_config::Value)> = component.properties.iter().collect();
        props.sort_by(|a, b| a.0.cmp(b.0));
        let rows = props
            .into_iter()
            .map(|(key, value)| {
                h_flex()
                    .gap_2()
                    .overflow_hidden()
                    .child(
                        div()
                            .w(px(160.))
                            .flex_shrink_0()
                            .truncate()
                            .child(Label::new(key.clone()).text_t(FontSize::Xs)),
                    )
                    .child(
                        div().truncate().child(
                            Label::new(display_value(value))
                                .text_t(FontSize::Xs)
                                .text_color(muted),
                        ),
                    )
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        pane = pane.child(
            v_flex()
                .border_t_1()
                .border_b_1()
                .border_color(border)
                .py_1()
                .children(rows),
        );
    }

    // Handlers.
    pane = pane.child(section_heading("Handlers").text_color(muted));
    if component.handlers.is_empty() {
        pane = pane.child(muted_note("none"));
    } else {
        let mut handlers: Vec<(&String, &String)> = component.handlers.iter().collect();
        handlers.sort_by(|a, b| a.0.cmp(b.0));
        let rows = handlers
            .into_iter()
            .map(|(event, action)| {
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .w(px(120.))
                            .flex_shrink_0()
                            .child(Label::new(event.clone()).text_t(FontSize::Xs)),
                    )
                    .child(
                        Label::new(action.clone())
                            .text_t(FontSize::Xs)
                            .text_color(muted),
                    )
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        pane = pane.child(
            v_flex()
                .border_t_1()
                .border_b_1()
                .border_color(border)
                .py_1()
                .children(rows),
        );
    }

    // Bindings, when present.
    pane = pane.children(details.bindings.get(&component.id).map(|sources| {
        v_flex()
            .gap_1()
            .child(section_heading("Bindings").text_color(muted))
            .children(
                sources
                    .iter()
                    .map(|source| {
                        Label::new(source.clone())
                            .text_t(FontSize::Xs)
                            .text_color(muted)
                    })
                    .collect::<Vec<_>>(),
            )
    }));

    // Children summary.
    if !component.children.is_empty() {
        let child_labels = component
            .children
            .iter()
            .filter_map(|id| components.get(id))
            .map(label_for_component)
            .collect::<Vec<_>>()
            .join(", ");
        pane = pane
            .child(
                section_heading(format!("Children ({})", component.children.len()))
                    .text_color(muted),
            )
            .child(
                Label::new(child_labels)
                    .text_t(FontSize::Xs)
                    .text_color(muted),
            );
    }

    pane
}

/// A small section heading in the details pane.
fn section_heading(text: impl Into<SharedString>) -> Label {
    Label::new(text).text_t(FontSize::Sm).font_semibold()
}

/// A muted fallback note ("none").
fn muted_note(text: &'static str) -> Label {
    Label::new(text).text_t(FontSize::Xs)
}

/// Renders a config `Value` compactly for the properties list.
fn display_value(value: &nemo_config::Value) -> String {
    match value {
        nemo_config::Value::String(s) => s.clone(),
        nemo_config::Value::Integer(i) => i.to_string(),
        nemo_config::Value::Float(f) => f.to_string(),
        nemo_config::Value::Bool(b) => b.to_string(),
        other => other.to_string(),
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
