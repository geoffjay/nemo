use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    div, px, AnyElement, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement as _, RenderOnce, SharedString, Styled as _, Window,
};
use gpui_kit::component::dock::{
    panel_handle, BasePanel, DockArea, DockLayout, DockPlacement, Panel as KitPanel, PanelEvent,
};
use gpui_kit::component::{ActiveTheme as _, Icon as KitIcon};
use nemo_layout::BuiltComponent;

use super::icon::map_icon_name;

/// Side-dock width/height when the declarative panel declares no `size`.
const DEFAULT_DOCK_SIZE: f32 = 240.;
/// The declarative panel's name in persisted layouts. Panels are rebuildable
/// from the declarative tree, so nothing here persists; the name only keeps
/// the identity contract (`panel_name` never changes after construction).
const DECLARED_PANEL_NAME: &str = "nemo_declared_panel";

/// Which region of a `<dock>` a `<dock-panel>` docks to. Center panels fill
/// the middle; the other three form collapsible, resizable side/bottom docks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPlacementKind {
    Center,
    Left,
    Right,
    Bottom,
}

impl DockPlacementKind {
    /// The kit placement this region maps to.
    pub fn placement(self) -> DockPlacement {
        match self {
            DockPlacementKind::Center => DockPlacement::Center,
            DockPlacementKind::Left => DockPlacement::Left,
            DockPlacementKind::Right => DockPlacement::Right,
            DockPlacementKind::Bottom => DockPlacement::Bottom,
        }
    }
}

/// Tolerantly reads a `placement` attribute; anything unrecognized (or
/// undeclared) docks to the center.
pub fn dock_placement_value(value: Option<&str>) -> DockPlacementKind {
    match value.map(str::trim) {
        Some("left") => DockPlacementKind::Left,
        Some("right") => DockPlacementKind::Right,
        Some("bottom") => DockPlacementKind::Bottom,
        _ => DockPlacementKind::Center,
    }
}

/// One declarative `<dock-panel>`: its body is the panel's declarative
/// children (rendered fresh every pass into the panel entity), its attributes
/// name the tab and place it in the area.
#[derive(Clone, Debug)]
pub struct DockPanelSpec {
    /// The built component's id; keys the panel entity across renders.
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub placement: DockPlacementKind,
    /// Side/bottom dock size in pixels; the center has no size of its own.
    pub size: Option<i64>,
    /// Initially displayed tab of the panel's group.
    pub active: bool,
    pub closable: bool,
    pub visible: bool,
}

/// The declared layout's identity, used by the app to detect XML edits that
/// require re-installing the area's layout (a change rebuilds the area's
/// trees through `set_center`/`set_dock`; without one, user-made drags,
/// resizes, toggles and tab picks survive re-renders untouched).
///
/// Deliberately covers only the layout-shaping attributes: panel titles,
/// icons and flags are mirrored per pass without touching the area.
pub fn dock_layout_signature(specs: &[DockPanelSpec]) -> String {
    let regions = [
        (DockPlacementKind::Center, "center"),
        (DockPlacementKind::Left, "left"),
        (DockPlacementKind::Right, "right"),
        (DockPlacementKind::Bottom, "bottom"),
    ];
    regions
        .iter()
        .filter_map(|(kind, name)| {
            let group: Vec<&DockPanelSpec> = specs
                .iter()
                .filter(|spec| spec.placement == *kind)
                .collect();
            if group.is_empty() {
                return None;
            }
            let ids = group
                .iter()
                .map(|spec| spec.id.as_str())
                .collect::<Vec<_>>()
                .join("|");
            // The center has no dock size; a side group takes the first
            // declared size or the default the area would install itself.
            // `active` is in because it selects the group's initially
            // displayed tab at layout-build time.
            let size = match kind {
                DockPlacementKind::Center => 0,
                _ => group
                    .iter()
                    .find_map(|spec| spec.size)
                    .unwrap_or(DEFAULT_DOCK_SIZE as i64),
            };
            let active = group
                .iter()
                .position(|spec| spec.active)
                .map(|ix| ix.to_string())
                .unwrap_or_default();
            Some(format!("{name}:{ids}@{size}:{active}"))
        })
        .collect::<Vec<_>>()
        .join(";")
}

/// Installs (or replaces) the declared layout on a dock area. Called when the
/// layout signature changes — and once at creation — never on plain
/// re-renders, so the user's own drags and tab picks are not reset.
pub(crate) fn apply_dock_layout(
    area: &Entity<DockArea>,
    panels: &HashMap<String, Entity<DeclarativePanel>>,
    specs: &[DockPanelSpec],
    window: &mut Window,
    cx: &mut App,
) {
    let regions = [
        DockPlacementKind::Center,
        DockPlacementKind::Left,
        DockPlacementKind::Right,
        DockPlacementKind::Bottom,
    ];
    for kind in regions {
        let group: Vec<&DockPanelSpec> =
            specs.iter().filter(|spec| spec.placement == kind).collect();
        if group.is_empty() {
            continue;
        }
        let active = group.iter().position(|spec| spec.active).unwrap_or(0);
        let mut layout = DockLayout::tabs();
        for (ix, spec) in group.iter().enumerate() {
            let Some(entity) = panels.get(&spec.id) else {
                continue;
            };
            layout = layout.panel_view(panel_handle(entity.clone()), cx);
            if ix == active {
                // Index into the panels added so far; skipped panels (a spec
                // without a station entity) would shift it, but the station is
                // rebuilt from the same specs, so the two never disagree.
                layout = layout.active_index(ix);
            }
        }
        let size = if kind == DockPlacementKind::Center {
            None
        } else {
            Some(px(group
                .iter()
                .find_map(|spec| spec.size)
                .unwrap_or(DEFAULT_DOCK_SIZE as i64)
                as f32))
        };

        area.update(cx, |area, cx| match kind {
            DockPlacementKind::Center => area.set_center(layout, window, cx),
            _ => {
                let placement = kind.placement();
                area.set_dock(placement, layout, window, cx);
                if let Some(size) = size {
                    area.set_dock_size(placement, size, window, cx);
                }
            }
        });
    }
}

/// A declarative panel as a live dock panel entity.
///
/// The body is a same-frame handoff: `AnyElement`s are one-shot, so the
/// app's render pass rebuilds the declarative children every frame and
/// deposits them in the body slot; the panel's own `render` (painted later
/// in the same frame, under the area's element tree) takes them out. A
/// redraw that reaches this panel always passes through the app's render
/// first — window draws start at the root view — so the slot is refilled
/// before it is read.
pub struct DeclarativePanel {
    title: SharedString,
    /// The `icon` attribute as declared; mapped to the kit's `IconName` when
    /// the tab title renders (`IconName` has no `PartialEq`, the string is
    /// what the mirror compares).
    icon: Option<String>,
    closable: bool,
    visible: bool,
    focus_handle: FocusHandle,
    body: Rc<RefCell<Option<AnyElement>>>,
}

impl DeclarativePanel {
    pub(crate) fn new(spec: &DockPanelSpec, cx: &mut Context<Self>) -> Self {
        Self {
            title: SharedString::from(spec.title.clone()),
            icon: spec.icon.clone(),
            closable: spec.closable,
            visible: spec.visible,
            focus_handle: cx.focus_handle(),
            body: Rc::new(RefCell::new(None)),
        }
    }

    /// Deposits the declarative body the app's render pass rebuilt this
    /// frame; the panel's own `render` takes it out at paint time.
    pub(crate) fn set_body(&mut self, body: AnyElement) {
        *self.body.borrow_mut() = Some(body);
    }

    /// Whether the panel's mirrored attributes still match the declaration
    /// (mismatches are applied via `apply` without touching the layout).
    pub(crate) fn matches(&self, spec: &DockPanelSpec) -> bool {
        self.title == spec.title
            && self.icon == spec.icon
            && self.closable == spec.closable
            && self.visible == spec.visible
    }

    /// Mirrors a changed declaration into the live panel.
    pub(crate) fn apply(&mut self, spec: &DockPanelSpec) {
        self.title = SharedString::from(spec.title.clone());
        self.icon = spec.icon.clone();
        self.closable = spec.closable;
        self.visible = spec.visible;
    }
}

impl gpui::Render for DeclarativePanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        match self.body.borrow_mut().take() {
            Some(body) => body,
            // Nothing deposited this frame (a hidden panel, say): an empty
            // fill keeps the pane from collapsing.
            None => div().flex().flex_1().min_h(px(0.)).into_any_element(),
        }
    }
}

impl EventEmitter<PanelEvent> for DeclarativePanel {}

impl Focusable for DeclarativePanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// The behavior half: identity plus the two declarative flags. Zoom stays
/// available through the kit's default (the tab bar's menu), so a declared
/// panel zooms like any native one.
impl BasePanel for DeclarativePanel {
    fn panel_name(&self) -> &'static str {
        DECLARED_PANEL_NAME
    }

    fn visible(&self, _cx: &App) -> bool {
        self.visible
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }
}

/// The presentation half: the tab title drawn from the declared `title` and
/// `icon`. Everything else takes the kit default (toolbar buttons, menu
/// entries, title suffixes are Rust seams the declarative tree cannot
/// express — see the wrapper's out-of-scope notes).
impl KitPanel for DeclarativePanel {
    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let mut title = div().flex().items_center().whitespace_nowrap();
        if let Some(icon) = &self.icon {
            title = title.child(KitIcon::new(map_icon_name(icon)));
        }
        title.child(self.title.clone())
    }
}

/// A dockable panel layout: a center region plus up to three collapsible,
/// resizable docks (left, right, bottom), each holding tab groups of
/// declarative panels.
///
/// # XML Configuration
///
/// ```xml
/// <dock id="main" height="480">
///   <dock-panel id="explorer" title="Explorer" icon="folder"
///               placement="left" size="220">
///     <stack id="explorer_body" direction="vertical" padding="12">
///       <!-- declarative body -->
///     </stack>
///   </dock-panel>
///   <dock-panel id="editor" title="Editor" active="true">
///     <!-- declarative body -->
///   </dock-panel>
/// </dock>
/// ```
///
/// Panels sharing a `placement` form that region's tab group; the group's
/// initially displayed tab is the panel with `active="true"` (the first
/// declared one otherwise). Panels can be dragged between regions, docks
/// resize by their handle and collapse via the tab bar's affordance, and a
/// panel's ellipsis menu offers zoom and close (when `closable`).
///
/// # Properties
///
/// | Property | Type | Default | Description |
/// |----------|------|---------|-------------|
/// | `height` | int | `480` | Total area height in pixels (kit tab bars and docks need a definite height) |
///
/// `<dock-panel>`: `title`, `icon`, `placement` (`center`/`left`/`right`/
/// `bottom`, default `center`), `size` (px for non-center docks, default
/// `240`), `active`, `closable` (default `true`), `visible` (default `true`).
///
/// # Body handoff
///
/// Each panel's declarative children are rendered fresh every app render
/// pass and deposited into the panel entity's body slot; the panel takes
/// them when it paints in the same frame.
///
/// # Out of scope
///
/// Kit capabilities the declarative system cannot express, kept visible
/// here rather than silently dropped:
///
/// * Layout persistence (`DockArea::dump`/`load`) and per-panel persisted
///   state (`PanelState`) — panels rebuild from the declarative tree.
/// * Registered panel builders (`register_panel`/`PanelRegistry`) — the
///   declarative panels are created directly.
/// * Nested split trees (`DockLayout::h_split`/`v_split`) — one tab group
///   per region only; splits are what a host's own layout DSL is for.
/// * Panel toolbars, title suffixes, custom menu entries and zoom-control
///   placement (`toolbar_buttons`, `title_suffix`, `dropdown_menu`,
///   `zoom_control`) — Rust seams beyond the attribute surface.
/// * Dock/persist events (`DockToggleEvent`, `DockEvent`) as handlers, and
///   the skin's `PanelStyle`/button-visibility settings.
#[derive(IntoElement)]
pub struct Dock {
    source: BuiltComponent,
    area: Option<Entity<DockArea>>,
}

impl Dock {
    pub fn new(source: BuiltComponent) -> Self {
        Self { source, area: None }
    }

    /// The dock area entity prepared by the app's render pass, or `None` for
    /// a placeholder (no declared panels).
    pub fn area(mut self, area: Option<Entity<DockArea>>) -> Self {
        self.area = area;
        self
    }
}

impl RenderOnce for Dock {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(area) = self.area else {
            return div()
                .text_color(cx.theme().colors.muted_foreground)
                .child("Dock: no dock-panel children declared")
                .into_any_element();
        };

        let props = &self.source.properties;
        // The kit's tab bars, docks and split frames size against a definite
        // height; an undeclared height collapses them to a strip.
        let height = props.get("height").and_then(|v| v.as_f64()).unwrap_or(480.) as f32;

        div().w_full().h(px(height)).child(area).into_any_element()
    }
}

#[cfg(test)]
mod probe {
    use super::*;

    fn spec(
        id: &str,
        placement: DockPlacementKind,
        size: Option<i64>,
        active: bool,
    ) -> DockPanelSpec {
        DockPanelSpec {
            id: id.to_string(),
            title: id.to_string(),
            icon: None,
            placement,
            size,
            active,
            closable: true,
            visible: true,
        }
    }

    #[test]
    fn placement_reading_defaults_to_center() {
        assert_eq!(dock_placement_value(None), DockPlacementKind::Center);
        assert_eq!(dock_placement_value(Some("")), DockPlacementKind::Center);
        assert_eq!(
            dock_placement_value(Some("center")),
            DockPlacementKind::Center
        );
        assert_eq!(dock_placement_value(Some("left")), DockPlacementKind::Left);
        assert_eq!(
            dock_placement_value(Some("right")),
            DockPlacementKind::Right
        );
        assert_eq!(
            dock_placement_value(Some("bottom")),
            DockPlacementKind::Bottom
        );
        assert_eq!(
            dock_placement_value(Some("somewhere")),
            DockPlacementKind::Center
        );
    }

    #[test]
    fn signature_groups_by_region_in_fixed_order() {
        let specs = vec![
            spec("b", DockPlacementKind::Right, None, false),
            spec("c", DockPlacementKind::Center, None, true),
            spec("a", DockPlacementKind::Left, Some(180), false),
        ];
        assert_eq!(
            dock_layout_signature(&specs),
            "center:c@0:0;left:a@180:;right:b@240:"
        );
    }

    #[test]
    fn signature_tracks_only_layout_shaping_attributes() {
        let base = vec![
            spec("a", DockPlacementKind::Center, None, true),
            spec("b", DockPlacementKind::Bottom, Some(200), false),
        ];
        let signature = dock_layout_signature(&base);

        // Presentation attributes are mirrored per pass; they must not
        // trigger a layout rebuild.
        let mut retitled = base.clone();
        retitled[0].title = "Renamed".into();
        retitled[0].icon = Some("terminal".into());
        retitled[1].closable = false;
        retitled[1].visible = false;
        assert_eq!(dock_layout_signature(&retitled), signature);

        // Structural changes do.
        let mut reactivated = base.clone();
        reactivated[1].active = true;
        assert_ne!(dock_layout_signature(&reactivated), signature);

        let mut resized = base.clone();
        resized[1].size = Some(320);
        assert_ne!(dock_layout_signature(&resized), signature);

        let mut reordered = vec![base[1].clone(), base[0].clone()];
        reordered[0].active = true;
        assert_ne!(dock_layout_signature(&reordered), signature);
    }

    #[test]
    fn side_dock_without_declared_size_uses_the_default() {
        let specs = vec![spec("a", DockPlacementKind::Bottom, None, false)];
        assert_eq!(
            dock_layout_signature(&specs),
            format!("bottom:a@{}:", DEFAULT_DOCK_SIZE as i64)
        );
    }
}
