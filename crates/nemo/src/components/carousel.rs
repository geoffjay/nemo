use gpui::{
    px, AnyElement, ElementId, Entity, IntoElement, ParentElement as _, SharedString, Styled as _,
};
use gpui_kit::component::carousel::{
    Carousel as GpuiCarousel, CarouselContent, CarouselItem, CarouselNext, CarouselPagination,
    CarouselPaginationItem, CarouselPrevious, CarouselState,
};
use nemo_layout::BuiltComponent;

/// A snapping slide viewport (gpui-kit `Carousel`). Declarative XML children
/// become the slides, in order: the wrapper hands each rendered child to the
/// kit inside a `CarouselItem` so the viewport's snap geometry tracks them.
///
/// The selection lives in a shared `CarouselState` entity managed by the app
/// (seeded from `index`); swiping, wheel, keyboard arrows, the prev/next
/// buttons, and the pagination dots all drive it, and every successful
/// selection fires `CarouselEvent::Change` — wired to the `on-change`
/// handler by `get_or_create_carousel_state`.
///
/// ```xml
/// <carousel id="main" height="140">
///   <stack id="s1" ... />
///   <stack id="s2" ... />
/// </carousel>
/// ```
///
/// # Properties
///
/// | Property | Type | Description |
/// |----------|------|-------------|
/// | `height` | integer | Slide height in px (needed for a definite snap viewport) |
/// | `axis` | string | `horizontal` (default) or `vertical` |
/// | `looping` | bool | Wrap navigation past the first/last slide |
/// | `index` | integer | Initially selected slide, zero-based |
/// | `controls` | bool | Render the prev/next chevron buttons (default true) |
/// | `pagination` | bool | Render a dot-button row under the viewport |
/// | `on-change` | string | Handler invoked with the newly selected index |
#[derive(IntoElement)]
pub struct Carousel {
    source: BuiltComponent,
    state: Entity<CarouselState>,
    slides: Vec<AnyElement>,
}

impl Carousel {
    pub fn new(
        source: BuiltComponent,
        state: Entity<CarouselState>,
        slides: Vec<AnyElement>,
    ) -> Self {
        Self {
            source,
            state,
            slides,
        }
    }

    fn named_id(&self, suffix: &str) -> ElementId {
        ElementId::Name(SharedString::from(format!("{}-{}", self.source.id, suffix)))
    }
}

impl gpui::RenderOnce for Carousel {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        let props = &self.source.properties;
        let height = props
            .get("height")
            .and_then(|v| v.as_f64())
            .map(|f| px(f as f32));
        let controls = props
            .get("controls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let pagination = props
            .get("pagination")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let component_id = self.source.id.clone();
        let state = self.state.clone();
        let root_id = self.named_id("root");

        let items: Vec<AnyElement> = self
            .slides
            .into_iter()
            .enumerate()
            .map(|(ix, slide)| {
                let mut item = CarouselItem::new(
                    ElementId::Name(SharedString::from(format!("{}-slide-{}", component_id, ix))),
                    ix,
                    &state,
                )
                // Nemo slides are flex stacks that are usually left
                // unsized, so the item becomes a column that stretches
                // each slide to full width and centers it vertically;
                // otherwise content hugs the item's top-left corner and
                // the viewport looks half empty.
                .flex()
                .flex_col()
                .justify_center()
                .child(slide);
                if let Some(h) = height {
                    item = item.h(h);
                }
                item.into_any_element()
            })
            .collect();
        let slide_count = items.len();

        let mut content = CarouselContent::new(&state);
        if let Some(h) = height {
            content = content.h(h);
        }

        let mut carousel = GpuiCarousel::new(root_id, &state).child(content.children(items));

        if controls {
            carousel = carousel
                .child(CarouselPrevious::new(&state))
                .child(CarouselNext::new(&state));
        }

        if pagination {
            let dots: Vec<AnyElement> = (0..slide_count)
                .map(|ix| {
                    CarouselPaginationItem::new(
                        ElementId::Name(SharedString::from(format!("{}-dot-{}", component_id, ix))),
                        ix,
                        &state,
                    )
                    // No intrinsic dot artwork: mirror the upstream docs
                    // example and fill each item with its slide number so
                    // the compact buttons have visible content.
                    .child((ix + 1).to_string())
                    .into_any_element()
                })
                .collect();
            carousel = carousel.child(CarouselPagination::new().children(dots));
        }

        carousel
    }
}

#[cfg(test)]
mod probe {
    use std::collections::HashMap;

    use gpui::{
        div, px, AnyElement, AppContext as _, Context, ElementId, Entity, InteractiveElement as _,
        IntoElement, ParentElement as _, Render, SharedString, Styled as _, TestAppContext, Window,
    };
    use gpui_kit::component::carousel::{
        Carousel as GpuiCarousel, CarouselContent, CarouselItem, CarouselPagination,
        CarouselPaginationItem, CarouselState,
    };
    use nemo_layout::BuiltComponent;

    use crate::components::Carousel;

    fn source(id: &str, props: &[(&str, nemo_config::Value)]) -> BuiltComponent {
        BuiltComponent {
            id: id.to_string(),
            component_type: "carousel".to_string(),
            properties: props
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            handlers: HashMap::new(),
            children: Vec::new(),
            parent: None,
        }
    }

    fn slide(id: impl Into<gpui::ElementId>, text: &'static str, height: i64) -> gpui::AnyElement {
        div()
            .id(id)
            .h(px(height as f32))
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .child(text)
            .into_any_element()
    }

    struct Gallery {
        basic_state: Entity<CarouselState>,
        vertical_state: Entity<CarouselState>,
        looping_state: Entity<CarouselState>,
    }

    impl Render for Gallery {
        fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let (basic_state, vertical_state, looping_state) = self.clone_states();

            let basic = Carousel::new(
                source(
                    "c_basic",
                    &[
                        ("height", nemo_config::Value::Integer(120)),
                        ("controls", nemo_config::Value::Bool(true)),
                    ],
                ),
                basic_state.clone(),
                (0..3)
                    .map(|ix| slide(format!("s{ix}"), "slide", 120))
                    .collect(),
            );

            let pagination = Carousel::new(
                source(
                    "c_pagination",
                    &[
                        ("height", nemo_config::Value::Integer(120)),
                        ("controls", nemo_config::Value::Bool(false)),
                        ("pagination", nemo_config::Value::Bool(true)),
                    ],
                ),
                basic_state.clone(),
                (0..3)
                    .map(|ix| slide(format!("p{ix}"), "slide", 120))
                    .collect(),
            );

            let vertical = Carousel::new(
                source(
                    "c_vertical",
                    &[
                        ("height", nemo_config::Value::Integer(220)),
                        ("axis", nemo_config::Value::String("vertical".to_string())),
                        ("controls", nemo_config::Value::Bool(false)),
                        ("pagination", nemo_config::Value::Bool(true)),
                    ],
                ),
                vertical_state,
                (0..3)
                    .map(|ix| slide(format!("v{ix}"), "slide", 200))
                    .collect(),
            );

            let looping = Carousel::new(
                source(
                    "c_looping",
                    &[
                        ("height", nemo_config::Value::Integer(120)),
                        ("looping", nemo_config::Value::Bool(true)),
                        ("index", nemo_config::Value::Integer(2)),
                    ],
                ),
                looping_state,
                (0..3)
                    .map(|ix| slide(format!("l{ix}"), "slide", 120))
                    .collect(),
            );

            // The gallery chain: page stack > example stack > carousel.
            div()
                .flex()
                .flex_col()
                .w(px(1200.))
                .child(div().debug_selector(|| "m-basic".to_string()).child(basic))
                .child(
                    div()
                        .debug_selector(|| "m-after-basic".to_string())
                        .child("after-basic"),
                )
                .child(
                    div()
                        .debug_selector(|| "m-pagination".to_string())
                        .child(pagination),
                )
                .child(
                    div()
                        .debug_selector(|| "m-after-pagination".to_string())
                        .child("after-pagination"),
                )
                .child(
                    div()
                        .debug_selector(|| "m-vertical".to_string())
                        .child(vertical),
                )
                .child(
                    div()
                        .debug_selector(|| "m-after-vertical".to_string())
                        .child("after-vertical"),
                )
                .child(
                    div()
                        .debug_selector(|| "m-looping".to_string())
                        .child(looping),
                )
                .child(
                    div()
                        .debug_selector(|| "m-after-looping".to_string())
                        .child("after-looping"),
                )
        }
    }

    impl Gallery {
        fn clone_states(
            &self,
        ) -> (
            Entity<CarouselState>,
            Entity<CarouselState>,
            Entity<CarouselState>,
        ) {
            (
                self.basic_state.clone(),
                self.vertical_state.clone(),
                self.looping_state.clone(),
            )
        }
    }

    #[gpui::test]
    fn gallery_flow_and_pagination_sizing(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let basic_state = cx.update(|cx| cx.new(|_| CarouselState::new(3)));
        let vertical_state = cx.update(|cx| {
            cx.new(|_| {
                CarouselState::new(3)
                    .with_axis(gpui::Axis::Vertical)
                    .with_looping(false)
            })
        });
        let looping_state = cx.update(|cx| {
            cx.new(|_| {
                CarouselState::new(3)
                    .with_looping(true)
                    .with_selected_index(2)
            })
        });
        let (_, cx) = cx.add_window_view(move |_, _| Gallery {
            basic_state,
            vertical_state,
            looping_state,
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let mut bounds = |sel: &'static str| cx.debug_bounds(sel).expect(sel);

        // Controls-only carousels are exactly the viewport height.
        let basic = bounds("m-basic");
        assert_eq!(basic.size.height.as_f32(), 120., "basic carousel height");
        // A pagination carousel is taller than its viewport: the dot row
        // (gap + button height) must fit inside the carousel, not overflow.
        let pagination = bounds("m-pagination");
        assert!(
            pagination.size.height.as_f32() > 120.,
            "pagination row must add height beyond the 120px viewport, got {}",
            pagination.size.height.as_f32()
        );
        // Vertical viewport + dots, same rule.
        let vertical = bounds("m-vertical");
        assert!(
            vertical.size.height.as_f32() > 220.,
            "vertical carousel must fit its dots below the 220px viewport, got {}",
            vertical.size.height.as_f32()
        );
        // Looping with controls only: exactly the viewport height.
        let looping = bounds("m-looping");
        assert_eq!(looping.size.height.as_f32(), 120.);

        // The sections stack in order with no overlap: every "after" marker
        // starts below the preceding carousel's bottom edge.
        for (carousel_sel, after_sel) in [
            ("m-basic", "m-after-basic"),
            ("m-pagination", "m-after-pagination"),
            ("m-vertical", "m-after-vertical"),
            ("m-looping", "m-after-looping"),
        ] {
            let c = bounds(carousel_sel);
            let after = bounds(after_sel);
            assert!(
                after.top() >= c.bottom(),
                "{after_sel} must start below {carousel_sel} (top {:.0} < bottom {:.0})",
                after.top().as_f32(),
                c.bottom().as_f32()
            );
        }
    }

    #[gpui::test]
    fn probe_pagination_renders(cx: &mut TestAppContext) {
        cx.update(gpui_kit::component::init);
        let state = cx.update(|cx| cx.new(|_| CarouselState::new(3)));
        let (_, cx) = cx.add_window_view(move |_, _| Pager {
            state: state.clone(),
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));

        let content = cx.debug_bounds("pager-content").expect("content");
        let pager = cx.debug_bounds("pager-dots").expect("pagination");
        println!(
            "PAGER content h {:.0} w {:.0}; pagination top {:.0} bottom {:.0} h {:.0} w {:.0}",
            content.size.height.as_f32(),
            content.size.width.as_f32(),
            pager.top().as_f32(),
            pager.bottom().as_f32(),
            pager.size.height.as_f32(),
            pager.size.width.as_f32(),
        );
        // The dot row must occupy vertical space below the viewport, not
        // collapse to zero height.
        assert!(pager.size.height.as_f32() > 0., "pagination has height");
    }

    struct Pager {
        state: Entity<CarouselState>,
    }

    impl Render for Pager {
        fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            let component_id = "c_pager".to_string();
            let state = self.state.clone();
            let dots: Vec<AnyElement> = (0..3)
                .map(|ix| {
                    CarouselPaginationItem::new(
                        ElementId::Name(SharedString::from(format!("{}-dot-{}", component_id, ix))),
                        ix,
                        &state,
                    )
                    .child((ix + 1).to_string())
                    .into_any_element()
                })
                .collect();
            div().w(px(400.)).child(
                div().debug_selector(|| "pager-content".to_string()).child(
                    GpuiCarousel::new(ElementId::Name(SharedString::from("c_pager-root")), &state)
                        .child(
                            CarouselContent::new(&state)
                                .h(px(100.))
                                .children((0..3).map(|ix| {
                                    CarouselItem::new(
                                        ElementId::Name(SharedString::from(format!(
                                            "c_pager-slide-{ix}"
                                        ))),
                                        ix,
                                        &state,
                                    )
                                    .flex()
                                    .flex_col()
                                    .justify_center()
                                    .child("slide")
                                    .into_any_element()
                                })),
                        )
                        .child(
                            div()
                                .debug_selector(|| "pager-dots".to_string())
                                .child(CarouselPagination::new().children(dots)),
                        ),
                ),
            )
        }
    }
}
