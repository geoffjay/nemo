//! A chrome-free page **router**.
//!
//! A `<router>` renders exactly one of its `<route>` children — the one whose
//! `path` pattern matches the router's current path — and nothing else. Unlike
//! [`AppShell`](super::AppShell), it draws no decoration of its own: the active
//! route body fills the router's box. Routers can be nested (a `<router>` inside
//! a route body is just another component the render recursion reaches when that
//! route is active) and switched declaratively via `<nav-link>` or from Rhai via
//! `navigate()` / `back()` / `forward()`.
//!
//! Router *state* (history + params) is authoritative host-side in the
//! `RouterRegistry` on `NemoRuntime`; navigation is applied through a deferred
//! queue so a `navigate()` call from inside a handler never re-enters the
//! extension lock. See the router methods on `NemoRuntime` and the pure
//! [`match_route`] matcher below.
//!
//! # XML Configuration
//!
//! ```xml
//! <router id="main" default="/home">
//!   <route path="/home"> <!-- ... --> </route>
//!   <route path="/users/:id" on-enter="load_user" on-leave="save_scroll"> … </route>
//!   <route path="*"> <!-- not-found fallback --> </route>
//! </router>
//!
//! <nav-link router="main" route="/users/42" label="User 42"/>
//! ```

use gpui::*;
use gpui_component::ActiveTheme;
use nemo_layout::BuiltComponent;
use std::collections::HashMap;
use std::sync::Arc;

use crate::runtime::NemoRuntime;
use crate::theme::tokens::{radius_of, FontSize, Space, TokenStyled};

/// Matches a `<route>` `path` pattern against a concrete path.
///
/// Both are split on `/`. A `:name` segment captures that path segment as a
/// param; a `*` segment matches the remainder of the path (any number of
/// segments, including none); literal segments must match exactly. On a full
/// match the captured params are returned (empty if there are none).
///
/// ```text
/// match_route("/users/:id", "/users/42") => Some({id: "42"})
/// match_route("/files/*",   "/files/a/b") => Some({})
/// match_route("*",          "/anything")  => Some({})
/// match_route("/home",      "/users")     => None
/// ```
pub fn match_route(pattern: &str, path: &str) -> Option<HashMap<String, String>> {
    let pat_segs: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let path_segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    let mut params = HashMap::new();
    for (i, pat) in pat_segs.iter().enumerate() {
        // A `*` segment swallows the rest of the path (including nothing).
        if *pat == "*" {
            return Some(params);
        }
        let seg = *path_segs.get(i)?;
        if let Some(name) = pat.strip_prefix(':') {
            params.insert(name.to_string(), seg.to_string());
        } else if *pat != seg {
            return None;
        }
    }

    // No wildcard consumed the tail, so the path must be exactly as long.
    if path_segs.len() != pat_segs.len() {
        return None;
    }
    Some(params)
}

/// Resolves which route matches `path`, scanning `patterns` in document order;
/// the first match wins. Returns the matching index and its captured params.
///
/// A `path="*"` pattern matches anything, so placing it last makes it the
/// not-found fallback.
pub fn resolve_route(patterns: &[String], path: &str) -> Option<(usize, HashMap<String, String>)> {
    patterns
        .iter()
        .enumerate()
        .find_map(|(i, pat)| match_route(pat, path).map(|params| (i, params)))
}

/// The active-route body of a `<router>`, rendered full-size with no chrome.
#[derive(IntoElement)]
pub struct Router {
    /// Pre-rendered body of the currently active route.
    body: Vec<AnyElement>,
}

impl Router {
    pub fn new(body: Vec<AnyElement>) -> Self {
        Self { body }
    }
}

impl RenderOnce for Router {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // Render like a plain vertical container (mirrors the generic
        // `render_component` fallback): content-sized, so it stays transparent
        // to the surrounding layout. Growth (`flex`) and scrolling are driven by
        // the `<router>`'s own attributes via `apply_layout_styles` — forcing
        // `size_full` here would pin the body to the parent's height and defeat
        // an enclosing `scroll` container.
        div()
            .flex()
            .flex_col()
            .children(self.body)
            .into_any_element()
    }
}

/// A clickable navigation link that enqueues a navigation to `route` on the
/// target `router` (or the primary router when unset). Highlights when its
/// `route` is the router's current path.
#[derive(IntoElement)]
pub struct NavLink {
    label: String,
    route: String,
    /// Target router id; `None` means the primary router.
    router: Option<String>,
    is_active: bool,
    entity_id: Option<EntityId>,
    runtime: Option<Arc<NemoRuntime>>,
}

impl NavLink {
    pub fn new(source: &BuiltComponent) -> Self {
        let props = &source.properties;
        Self {
            label: props
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            route: props
                .get("route")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            router: props
                .get("router")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            is_active: false,
            entity_id: None,
            runtime: None,
        }
    }

    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    pub fn entity_id(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    pub fn runtime(mut self, runtime: Arc<NemoRuntime>) -> Self {
        self.runtime = Some(runtime);
        self
    }
}

impl RenderOnce for NavLink {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = &cx.theme().colors;
        let hover_bg = colors.list_hover;
        let active_bg = colors.list_active;
        let fg = colors.foreground;
        let muted = colors.muted_foreground;

        let mut row = div()
            .flex()
            .flex_row()
            .items_center()
            .rounded(radius_of("md", cx))
            .cursor_pointer()
            .px_t(Space::Sm)
            .py_t(Space::Xs)
            .text_t(FontSize::Sm)
            .text_color(if self.is_active { fg } else { muted })
            .hover(move |s| s.bg(hover_bg))
            .child(self.label.clone());

        if self.is_active {
            row = row.bg(active_bg).text_color(fg);
        }

        // Clicking enqueues a deferred navigation; the poll loop applies it and
        // triggers a re-render. Mirrors the app_shell sidenav click closure.
        if let (Some(entity_id), Some(runtime)) = (self.entity_id, self.runtime) {
            let route = self.route.clone();
            let router = self.router.clone();
            row = row.on_mouse_down(MouseButton::Left, move |_event, _window, cx| {
                runtime.enqueue_navigation(router.clone(), route.clone());
                cx.notify(entity_id);
            });
        }

        row.into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{match_route, resolve_route};
    use std::collections::HashMap;

    #[test]
    fn matches_literal_path() {
        assert_eq!(match_route("/home", "/home"), Some(HashMap::new()));
        assert_eq!(match_route("/a/b", "/a/b"), Some(HashMap::new()));
    }

    #[test]
    fn literal_mismatch_is_none() {
        assert_eq!(match_route("/home", "/users"), None);
        // A longer path than the pattern (no wildcard) does not match.
        assert_eq!(match_route("/a", "/a/b"), None);
        // A shorter path than the pattern does not match.
        assert_eq!(match_route("/a/b", "/a"), None);
    }

    #[test]
    fn captures_named_params() {
        let params = match_route("/users/:id", "/users/42").unwrap();
        assert_eq!(params.get("id"), Some(&"42".to_string()));

        let params = match_route("/users/:id/posts/:pid", "/users/7/posts/9").unwrap();
        assert_eq!(params.get("id"), Some(&"7".to_string()));
        assert_eq!(params.get("pid"), Some(&"9".to_string()));
    }

    #[test]
    fn trailing_wildcard_matches_rest() {
        assert_eq!(
            match_route("/files/*", "/files/a/b/c"),
            Some(HashMap::new())
        );
        assert_eq!(match_route("/files/*", "/files"), Some(HashMap::new()));
        assert_eq!(match_route("*", "/anything/at/all"), Some(HashMap::new()));
        assert_eq!(match_route("*", "/"), Some(HashMap::new()));
    }

    #[test]
    fn resolve_picks_first_match_in_document_order() {
        let patterns = vec![
            "/home".to_string(),
            "/users/:id".to_string(),
            "*".to_string(),
        ];
        assert_eq!(resolve_route(&patterns, "/home").map(|(i, _)| i), Some(0));
        let (idx, params) = resolve_route(&patterns, "/users/42").unwrap();
        assert_eq!(idx, 1);
        assert_eq!(params.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn resolve_falls_back_to_wildcard() {
        let patterns = vec!["/home".to_string(), "*".to_string()];
        assert_eq!(resolve_route(&patterns, "/nope").map(|(i, _)| i), Some(1));
    }

    #[test]
    fn resolve_returns_none_when_unmatched_and_no_fallback() {
        let patterns = vec!["/home".to_string()];
        assert!(resolve_route(&patterns, "/nope").is_none());
    }
}
