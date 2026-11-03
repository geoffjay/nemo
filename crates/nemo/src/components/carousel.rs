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
