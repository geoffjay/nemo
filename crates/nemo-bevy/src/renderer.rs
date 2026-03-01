use std::thread;
use std::time::Duration;

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_graph::{self, NodeRunError, RenderGraphContext, RenderLabel};
use bevy::render::render_resource::{
    Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, ImageCopyBuffer,
    ImageDataLayout, Maintain, MapMode, TextureDimension, TextureFormat, TextureUsages,
};
use bevy::render::renderer::{RenderContext, RenderDevice, RenderQueue};
use bevy::render::{Render, RenderApp, RenderSet};
use crossbeam_channel::Sender;

use crate::frame::{FrameData, LatestFrame};
use crate::input::BevyInputEvent;

/// Configuration for the headless Bevy renderer.
pub struct BevyRendererConfig {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for BevyRendererConfig {
    fn default() -> Self {
        Self {
            width: 800,
            height: 600,
            fps: 60,
        }
    }
}

/// Handle to a running headless Bevy renderer on a background thread.
///
/// Frames are published to `latest_frame` which the GPUI component reads.
pub struct BevyRenderer {
    pub latest_frame: LatestFrame,
    input_tx: Sender<BevyInputEvent>,
}

impl BevyRenderer {
    /// Spawns a headless Bevy app on a background thread.
    ///
    /// `scene_setup` is called with the Bevy `App` before it runs, allowing
    /// the caller to add systems, spawn entities, etc. If `None`, a default
    /// spinning cube demo scene is used.
    pub fn spawn(
        config: BevyRendererConfig,
        scene_setup: Option<Box<dyn FnOnce(&mut App) + Send + 'static>>,
    ) -> Self {
        let latest_frame = LatestFrame::new();
        let frame_writer = latest_frame.clone();
        let (input_tx, _input_rx) = crossbeam_channel::unbounded::<BevyInputEvent>();
        let (frame_tx, frame_rx) = crossbeam_channel::unbounded::<RawFrame>();

        // Forwarding thread: converts RGBA (padded) -> BGRA and stores in LatestFrame
        let forward_frame = frame_writer.clone();
        thread::Builder::new()
            .name("nemo-bevy-frame-fwd".into())
            .spawn(move || {
                while let Ok(raw) = frame_rx.recv() {
                    let frame = FrameData::from_rgba_padded(
                        &raw.data,
                        raw.width,
                        raw.height,
                        raw.padded_bytes_per_row,
                    );
                    forward_frame.store(frame);
                }
            })
            .expect("failed to spawn frame forwarding thread");

        // Bevy background thread
        let width = config.width;
        let height = config.height;
        let fps = config.fps;

        thread::Builder::new()
            .name("nemo-bevy-render".into())
            .spawn(move || {
                let mut app = App::new();

                // Headless: no window, don't exit when no windows
                app.add_plugins(
                    DefaultPlugins
                        .set(bevy::window::WindowPlugin {
                            primary_window: None,
                            exit_condition: bevy::window::ExitCondition::DontExit,
                            close_when_requested: false,
                        })
                        .set(bevy::render::RenderPlugin {
                            render_creation: bevy::render::settings::WgpuSettings {
                                power_preference:
                                    bevy::render::settings::PowerPreference::HighPerformance,
                                ..default()
                            }
                            .into(),
                            ..default()
                        }),
                );
                app.add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                    1.0 / fps as f64,
                )));

                // Create the render target image
                let size = Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                };
                let mut render_image = Image::new_fill(
                    size,
                    TextureDimension::D2,
                    &[0, 0, 0, 255],
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::default(),
                );
                render_image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT;

                let image_handle =
                    app.world_mut()
                        .resource_mut::<Assets<Image>>()
                        .add(render_image);

                // Insert resources for the render graph node
                app.insert_resource(ImageCopyState {
                    image_handle: image_handle.clone(),
                    width,
                    height,
                    frame_tx,
                });

                // Apply user scene setup or default demo
                if let Some(setup) = scene_setup {
                    // Insert the image handle so the user's setup can reference it
                    app.insert_resource(RenderTargetHandle(image_handle.clone()));
                    setup(&mut app);
                } else {
                    // Default demo scene: spinning cube
                    let image_h = image_handle.clone();
                    app.add_systems(
                        Startup,
                        move |mut commands: Commands,
                              mut meshes: ResMut<Assets<Mesh>>,
                              mut materials: ResMut<Assets<StandardMaterial>>| {
                            // Camera rendering to our image
                            commands.spawn((
                                Camera3d::default(),
                                Camera {
                                    target: RenderTarget::Image(image_h.clone()),
                                    ..default()
                                },
                                Transform::from_xyz(0.0, 2.0, 5.0)
                                    .looking_at(Vec3::ZERO, Vec3::Y),
                            ));

                            // Cube
                            commands.spawn((
                                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                                MeshMaterial3d(materials.add(StandardMaterial {
                                    base_color: Color::srgb(0.3, 0.5, 0.8),
                                    ..default()
                                })),
                                Transform::default(),
                                SpinningCube,
                            ));

                            // Light
                            commands.spawn((
                                DirectionalLight {
                                    illuminance: 10000.0,
                                    ..default()
                                },
                                Transform::from_rotation(Quat::from_euler(
                                    EulerRot::XYZ,
                                    -0.5,
                                    0.5,
                                    0.0,
                                )),
                            ));
                        },
                    );
                    app.add_systems(Update, spin_cube);
                }

                // Add the image copy plugin to the render app
                app.add_plugins(ImageCopyPlugin);

                app.run();
            })
            .expect("failed to spawn Bevy render thread");

        Self {
            latest_frame,
            input_tx,
        }
    }

    /// Sends an input event to the Bevy scene (non-blocking).
    pub fn send_input(&self, event: BevyInputEvent) {
        let _ = self.input_tx.try_send(event);
    }
}

/// Resource exposing the render target image handle for custom scene setups.
#[derive(Resource)]
pub struct RenderTargetHandle(#[allow(dead_code)] pub Handle<Image>);

// ── Demo scene helpers ──────────────────────────────────────────────────

#[derive(Component)]
struct SpinningCube;

fn spin_cube(time: Res<Time>, mut query: Query<&mut Transform, With<SpinningCube>>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() * 1.0);
        transform.rotate_x(time.delta_secs() * 0.3);
    }
}

// ── Image copy plugin ───────────────────────────────────────────────────

/// Raw frame data from GPU before RGBA->BGRA conversion.
struct RawFrame {
    data: Vec<u8>,
    width: u32,
    height: u32,
    padded_bytes_per_row: u32,
}

/// Shared state for the image copy render graph node.
#[derive(Resource, Clone)]
struct ImageCopyState {
    image_handle: Handle<Image>,
    width: u32,
    height: u32,
    frame_tx: Sender<RawFrame>,
}

/// Staging buffer resource created in the render world.
#[derive(Resource)]
struct ImageCopyStagingBuffer {
    buffer: Buffer,
    padded_bytes_per_row: u32,
}

/// Plugin that adds a render graph node to copy the rendered image to a CPU buffer.
struct ImageCopyPlugin;

impl Plugin for ImageCopyPlugin {
    fn build(&self, app: &mut App) {
        // Clone the state before getting the mutable sub-app reference
        let state = app.world().resource::<ImageCopyState>().clone();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.insert_resource(state);
        render_app.add_systems(Render, create_staging_buffer.in_set(RenderSet::PrepareResources));

        // Add the copy node to the render graph
        let mut render_graph = render_app
            .world_mut()
            .resource_mut::<bevy::render::render_graph::RenderGraph>();
        render_graph.add_node(ImageCopyLabel, ImageCopyNode);
        render_graph.add_node_edge(bevy::render::graph::CameraDriverLabel, ImageCopyLabel);
    }
}

fn create_staging_buffer(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    state: Res<ImageCopyState>,
    existing: Option<Res<ImageCopyStagingBuffer>>,
) {
    if existing.is_some() {
        return;
    }

    let bytes_per_pixel = 4u32;
    let unpadded_bytes_per_row = state.width * bytes_per_pixel;
    // wgpu requires buffer copy rows to be aligned to 256 bytes
    let align = 256u32;
    let padded_bytes_per_row = (unpadded_bytes_per_row + align - 1) & !(align - 1);
    let buffer_size = (padded_bytes_per_row * state.height) as u64;

    let buffer = render_device.create_buffer(&BufferDescriptor {
        label: Some("nemo_bevy_staging_buffer"),
        size: buffer_size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    commands.insert_resource(ImageCopyStagingBuffer {
        buffer,
        padded_bytes_per_row,
    });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
struct ImageCopyLabel;

struct ImageCopyNode;

impl render_graph::Node for ImageCopyNode {
    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Some(state) = world.get_resource::<ImageCopyState>() else {
            return Ok(());
        };
        let Some(staging) = world.get_resource::<ImageCopyStagingBuffer>() else {
            return Ok(());
        };
        let gpu_images = world
            .resource::<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>(
            );
        let Some(gpu_image) = gpu_images.get(&state.image_handle) else {
            return Ok(());
        };

        let mut encoder =
            render_context
                .render_device()
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("nemo_bevy_copy_encoder"),
                });

        encoder.copy_texture_to_buffer(
            gpu_image.texture.as_image_copy(),
            ImageCopyBuffer {
                buffer: &staging.buffer,
                layout: ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(staging.padded_bytes_per_row),
                    rows_per_image: Some(state.height),
                },
            },
            Extent3d {
                width: state.width,
                height: state.height,
                depth_or_array_layers: 1,
            },
        );

        let render_queue = world.resource::<RenderQueue>();
        render_queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer and read the data synchronously
        let buffer_slice = staging.buffer.slice(..);
        let (tx, rx) = crossbeam_channel::bounded(1);
        buffer_slice.map_async(MapMode::Read, move |result| {
            let _ = tx.send(result);
        });

        render_context.render_device().poll(Maintain::Wait);

        if let Ok(Ok(())) = rx.recv() {
            let data = buffer_slice.get_mapped_range();
            let _ = state.frame_tx.try_send(RawFrame {
                data: data.to_vec(),
                width: state.width,
                height: state.height,
                padded_bytes_per_row: staging.padded_bytes_per_row,
            });
            drop(data);
            staging.buffer.unmap();
        }

        Ok(())
    }
}
