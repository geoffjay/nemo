use gpui::*;
use image::{Frame, ImageBuffer, Rgba};
use smallvec::SmallVec;
use std::sync::Arc;

use nemo_bevy::{BevyInputEvent, BevyRenderer, BevyRendererConfig, MouseButton as BevyMouseButton};
use nemo_layout::BuiltComponent;

/// GPUI entity holding the Bevy renderer and cached frame data.
pub struct BevySceneState {
    renderer: BevyRenderer,
    cached_image: Option<Arc<RenderImage>>,
    width: u32,
    height: u32,
}

impl BevySceneState {
    pub fn new(width: u32, height: u32) -> Self {
        let config = BevyRendererConfig {
            width,
            height,
            fps: 60,
        };
        let renderer = BevyRenderer::spawn(config, None);

        Self {
            renderer,
            cached_image: None,
            width,
            height,
        }
    }

    /// Checks for a new frame from Bevy and converts it to a RenderImage.
    /// Returns the latest available image (new or cached).
    pub fn poll_frame(&mut self) -> Option<Arc<RenderImage>> {
        if let Some(frame_data) = self.renderer.latest_frame.take() {
            // The FrameData is BGRA, but RenderImage expects Rgba<u8> ImageBuffers
            // because GPUI's platform renderer handles the BGRA conversion internally.
            // We need to swap B and R back to RGBA for the ImageBuffer<Rgba<u8>> type.
            let mut rgba_data = frame_data.data.clone();
            for pixel in rgba_data.chunks_exact_mut(4) {
                pixel.swap(0, 2); // BGRA -> RGBA
            }

            if let Some(buffer) = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
                frame_data.width,
                frame_data.height,
                rgba_data,
            ) {
                let frame = Frame::new(buffer);
                let frames: SmallVec<[Frame; 1]> = SmallVec::from_elem(frame, 1);
                let render_image = Arc::new(RenderImage::new(frames));
                self.cached_image = Some(render_image.clone());
                return Some(render_image);
            }
        }
        self.cached_image.clone()
    }

    /// Forward an input event to the Bevy renderer.
    pub fn send_input(&self, event: BevyInputEvent) {
        self.renderer.send_input(event);
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

/// A GPUI component that renders a Bevy 3D scene.
#[derive(IntoElement)]
pub struct BevyScene {
    source: BuiltComponent,
    bevy_state: Entity<BevySceneState>,
}

impl BevyScene {
    pub fn new(source: BuiltComponent, bevy_state: Entity<BevySceneState>) -> Self {
        Self { source, bevy_state }
    }
}

impl RenderOnce for BevyScene {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let id = ElementId::Name(self.source.id.clone().into());
        let bevy_state = self.bevy_state.clone();
        let bevy_state_input = self.bevy_state.clone();

        let (width, height) = _cx.read_entity(&self.bevy_state, |state, _| {
            (state.width(), state.height())
        });

        let width_f = width as f32;
        let height_f = height as f32;

        // Request continuous repaints so we keep polling for new frames
        window.request_animation_frame();

        let image_source = ImageSource::Custom(Arc::new(move |_window, cx| {
            let image = cx.update_entity(&bevy_state, |state: &mut BevySceneState, _cx| {
                state.poll_frame()
            });
            image.map(Ok)
        }));

        div()
            .id(id)
            .w(px(width_f))
            .h(px(height_f))
            .child(img(image_source).w(px(width_f)).h(px(height_f)))
            .on_mouse_move({
                let state = bevy_state_input.clone();
                move |event: &MouseMoveEvent, _window, cx| {
                    let x = f32::from(event.position.x) / width_f;
                    let y = f32::from(event.position.y) / height_f;
                    cx.update_entity(&state, |s, _| {
                        s.send_input(BevyInputEvent::MouseMove {
                            x: x.clamp(0.0, 1.0),
                            y: y.clamp(0.0, 1.0),
                        });
                    });
                }
            })
            .on_mouse_down(gpui::MouseButton::Left, {
                let state = bevy_state_input.clone();
                move |event: &MouseDownEvent, _window, cx| {
                    let x = f32::from(event.position.x) / width_f;
                    let y = f32::from(event.position.y) / height_f;
                    cx.update_entity(&state, |s, _| {
                        s.send_input(BevyInputEvent::MouseDown {
                            button: BevyMouseButton::Left,
                            x: x.clamp(0.0, 1.0),
                            y: y.clamp(0.0, 1.0),
                        });
                    });
                }
            })
            .on_mouse_up(gpui::MouseButton::Left, {
                let state = bevy_state_input.clone();
                move |event: &MouseUpEvent, _window, cx| {
                    let x = f32::from(event.position.x) / width_f;
                    let y = f32::from(event.position.y) / height_f;
                    cx.update_entity(&state, |s, _| {
                        s.send_input(BevyInputEvent::MouseUp {
                            button: BevyMouseButton::Left,
                            x: x.clamp(0.0, 1.0),
                            y: y.clamp(0.0, 1.0),
                        });
                    });
                }
            })
            .on_scroll_wheel({
                let state = bevy_state_input;
                move |event: &ScrollWheelEvent, _window, cx| {
                    let (dx, dy) = match event.delta {
                        ScrollDelta::Lines(delta) => (delta.x, delta.y),
                        ScrollDelta::Pixels(delta) => {
                            (f32::from(delta.x), f32::from(delta.y))
                        }
                    };
                    cx.update_entity(&state, |s, _| {
                        s.send_input(BevyInputEvent::Scroll { dx, dy });
                    });
                }
            })
    }
}
