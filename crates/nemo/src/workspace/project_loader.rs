use gpui::*;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::{h_flex, v_flex, ActiveTheme, IconName, WindowExt as _};
use std::path::{Path, PathBuf};
use tracing::info;

use crate::config::recent::RecentProjects;
use crate::config::NemoConfig;
use crate::theme::tokens::{radius_of, FontSize, Space, TokenStyled};

/// Action emitted when a project is selected (carries the config file path).
#[derive(Clone, Debug)]
pub struct ProjectSelected(pub PathBuf);

impl EventEmitter<ProjectSelected> for ProjectLoaderView {}

/// Relative locations, in precedence order, where a project's `app.xml` may
/// live within a repository. A root-level `app.xml` wins over `.nemo/app.xml`
/// so existing repositories keep working unchanged; when the config lives in
/// `.nemo/`, that directory becomes the project root for all relative paths.
const CONFIG_CANDIDATES: [&str; 2] = ["app.xml", ".nemo/app.xml"];

/// Resolve the project configuration file within a cloned repository, trying
/// each candidate location in precedence order. Returns the first that exists.
fn resolve_project_config(repo_root: &Path) -> Option<PathBuf> {
    CONFIG_CANDIDATES
        .iter()
        .map(|rel| repo_root.join(rel))
        .find(|p| p.exists())
}

pub struct ProjectLoaderView {
    nemo_config: NemoConfig,
    recent_projects: RecentProjects,
    clone_error: Option<String>,
}

impl ProjectLoaderView {
    pub fn new(nemo_config: NemoConfig, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let recent_projects = RecentProjects::load();

        Self {
            nemo_config,
            recent_projects,
            clone_error: None,
        }
    }

    fn open_from_disk(&mut self, _: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Select an app.xml configuration file".into()),
        });

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |_view, cx| {
                        cx.emit(ProjectSelected(path));
                    });
                }
            }
        })
        .detach();
    }

    fn clone_from_repo(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let project_dir = self.nemo_config.project_dir.clone();
        let entity = cx.entity().downgrade();
        let input_state = cx
            .new(|cx| InputState::new(window, cx).placeholder("https://github.com/user/repo.git"));

        window.open_dialog(cx, move |dialog, _window, _cx| {
            let input_for_ok = input_state.clone();
            let project_dir = project_dir.clone();
            let entity = entity.clone();

            dialog
                .title("Clone Repository")
                .width(px(500.))
                .child(
                    v_flex()
                        .gap_t(Space::Md)
                        .child(Label::new("Enter the repository URL:"))
                        .child(Input::new(&input_state)),
                )
                .on_ok(move |_, _window, cx| {
                    let url = input_for_ok.read(cx).value().to_string();
                    if url.is_empty() {
                        return false;
                    }

                    let repo_name = url
                        .trim_end_matches('/')
                        .rsplit('/')
                        .next()
                        .unwrap_or("repo")
                        .trim_end_matches(".git")
                        .to_string();

                    let target = project_dir.join(&repo_name);
                    let entity = entity.clone();

                    cx.spawn(async move |cx: &mut AsyncApp| {
                        info!("Cloning {} into {:?}", url, target);

                        let output = std::process::Command::new("git")
                            .args(["clone", &url, &target.to_string_lossy()])
                            .output();

                        match output {
                            Ok(result) if result.status.success() => {
                                match resolve_project_config(&target) {
                                    Some(app_xml) => {
                                        let _ = entity.update(cx, |_view, cx| {
                                            cx.emit(ProjectSelected(app_xml));
                                        });
                                    }
                                    None => {
                                        let _ = entity.update(cx, |view, cx| {
                                            view.clone_error = Some(format!(
                                                "Cloned successfully but no app.xml found at the \
                                                 repo root or in .nemo/ ({})",
                                                target.display()
                                            ));
                                            cx.notify();
                                        });
                                    }
                                }
                            }
                            Ok(result) => {
                                let stderr = String::from_utf8_lossy(&result.stderr).to_string();
                                let _ = entity.update(cx, |view, cx| {
                                    view.clone_error = Some(format!("Clone failed: {}", stderr));
                                    cx.notify();
                                });
                            }
                            Err(e) => {
                                let _ = entity.update(cx, |view, cx| {
                                    view.clone_error = Some(format!("Failed to run git: {}", e));
                                    cx.notify();
                                });
                            }
                        }
                    })
                    .detach();

                    true // close dialog immediately
                })
        });
    }

    fn select_recent(
        &mut self,
        config_path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.emit(ProjectSelected(config_path));
    }
}

impl Render for ProjectLoaderView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card_radius = radius_of("md", cx);
        let theme = cx.theme();
        let muted = theme.colors.muted_foreground;
        let border_color = theme.colors.border;
        let list_active_bg = theme.colors.list_active;

        let mut content = v_flex().gap_t(Space::Xl).w(px(480.)).child(
            v_flex()
                .gap_t(Space::Xs)
                .child(
                    div()
                        .text_t(FontSize::Xxl)
                        .font_weight(FontWeight::BOLD)
                        .child("Open a Project"),
                )
                .child(
                    div()
                        .text_color(muted)
                        .child("Select a recent project or open a new one"),
                ),
        );

        // Recent projects list
        let recent = self.recent_projects.list();
        if recent.is_empty() {
            content = content.child(
                div()
                    .px_t(Space::Lg)
                    .py_t(Space::Md)
                    .rounded(card_radius)
                    .border_1()
                    .border_color(border_color)
                    .text_color(muted)
                    .child("No recent projects"),
            );
        } else {
            let mut list = v_flex().gap_t(Space::Xs);
            for project in recent {
                let config_path = project.config_path.clone();
                let name = project.name.clone();
                let path_display = project.config_path.display().to_string();

                list = list.child(
                    div()
                        .id(SharedString::from(format!("recent-{}", path_display)))
                        .px_t(Space::Lg)
                        .py_t(Space::Sm)
                        .rounded(card_radius)
                        .cursor_pointer()
                        .hover(|s| s.bg(list_active_bg))
                        .child(
                            v_flex()
                                .child(div().font_weight(FontWeight::MEDIUM).child(name))
                                .child(
                                    div()
                                        .text_t(FontSize::Xs)
                                        .text_color(muted)
                                        .child(path_display),
                                ),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.select_recent(config_path.clone(), window, cx);
                        })),
                );
            }

            content = content.child(
                v_flex()
                    .gap_t(Space::Xs)
                    .child(
                        div()
                            .text_t(FontSize::Sm)
                            .font_weight(FontWeight::MEDIUM)
                            .child("Recent Projects"),
                    )
                    .child(list),
            );
        }

        // Error message
        if let Some(ref error) = self.clone_error {
            content = content.child(
                div()
                    .px_t(Space::Lg)
                    .py_t(Space::Sm)
                    .rounded(card_radius)
                    .bg(red())
                    .text_color(white())
                    .text_t(FontSize::Sm)
                    .child(error.clone()),
            );
        }

        // Action buttons
        content = content.child(
            h_flex()
                .gap_t(Space::Md)
                .child(
                    Button::new("open-disk")
                        .label("Open from Disk")
                        .icon(IconName::FolderOpen)
                        .primary()
                        .on_click(cx.listener(Self::open_from_disk)),
                )
                .child(
                    Button::new("clone-repo")
                        .label("Clone from Repository")
                        .icon(IconName::ExternalLink)
                        .on_click(cx.listener(Self::clone_from_repo)),
                ),
        );

        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .child(content)
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_project_config;
    use std::path::Path;
    use tempfile::TempDir;

    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent dir");
        }
        std::fs::write(path, "<app></app>").expect("write config");
    }

    #[test]
    fn resolves_root_config() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("app.xml"));

        assert_eq!(resolve_project_config(root), Some(root.join("app.xml")));
    }

    #[test]
    fn resolves_dot_nemo_config() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join(".nemo/app.xml"));

        assert_eq!(
            resolve_project_config(root),
            Some(root.join(".nemo/app.xml"))
        );
    }

    #[test]
    fn root_config_takes_precedence_over_dot_nemo() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();
        touch(&root.join("app.xml"));
        touch(&root.join(".nemo/app.xml"));

        assert_eq!(resolve_project_config(root), Some(root.join("app.xml")));
    }

    #[test]
    fn returns_none_when_no_config_present() {
        let dir = TempDir::new().unwrap();

        assert_eq!(resolve_project_config(dir.path()), None);
    }
}
