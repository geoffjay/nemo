/// Mouse button identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Input events forwarded from GPUI to the Bevy scene.
///
/// Coordinates are normalized to 0.0..1.0 within the render area.
#[derive(Debug, Clone)]
pub enum BevyInputEvent {
    MouseMove { x: f32, y: f32 },
    MouseDown { button: MouseButton, x: f32, y: f32 },
    MouseUp { button: MouseButton, x: f32, y: f32 },
    Scroll { dx: f32, dy: f32 },
}
