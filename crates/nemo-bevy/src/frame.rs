use std::sync::{Arc, Mutex};

/// Tightly-packed BGRA pixel data for a single frame.
#[derive(Clone)]
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    /// BGRA pixels, length = width * height * 4.
    pub data: Vec<u8>,
}

impl FrameData {
    /// Converts RGBA data (potentially with GPU row padding) to tightly-packed BGRA.
    ///
    /// `padded_bytes_per_row` is the stride of each row in the source buffer,
    /// which may be larger than `width * 4` due to GPU alignment requirements.
    pub fn from_rgba_padded(
        rgba: &[u8],
        width: u32,
        height: u32,
        padded_bytes_per_row: u32,
    ) -> Self {
        let tight_stride = (width * 4) as usize;
        let padded_stride = padded_bytes_per_row as usize;
        let mut data = Vec::with_capacity(tight_stride * height as usize);

        for row in 0..height as usize {
            let src_start = row * padded_stride;
            let src_end = src_start + tight_stride;
            if src_end > rgba.len() {
                break;
            }
            let row_data = &rgba[src_start..src_end];
            // Swap R and B channels: RGBA -> BGRA
            for pixel in row_data.chunks_exact(4) {
                data.push(pixel[2]); // B
                data.push(pixel[1]); // G
                data.push(pixel[0]); // R
                data.push(pixel[3]); // A
            }
        }

        Self {
            width,
            height,
            data,
        }
    }
}

/// Atomic frame slot shared between the Bevy render thread and the GPUI component.
///
/// The Bevy side calls `store()` to publish new frames, while the GPUI side
/// calls `take()` to consume them. Only the latest frame is kept.
#[derive(Clone)]
pub struct LatestFrame(Arc<Mutex<Option<FrameData>>>);

impl LatestFrame {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }

    /// Stores a new frame, replacing any unconsumed previous frame.
    pub fn store(&self, frame: FrameData) {
        *self.0.lock().unwrap() = Some(frame);
    }

    /// Takes the latest frame, leaving `None` in its place.
    pub fn take(&self) -> Option<FrameData> {
        self.0.lock().unwrap().take()
    }

    /// Peeks at the latest frame without consuming it.
    pub fn peek(&self) -> Option<FrameData> {
        self.0.lock().unwrap().clone()
    }
}

impl Default for LatestFrame {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_data_from_rgba_no_padding() {
        // 2x2 image, no padding (stride = width * 4 = 8)
        let rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 0, 255, // yellow
        ];
        let frame = FrameData::from_rgba_padded(&rgba, 2, 2, 8);
        assert_eq!(frame.width, 2);
        assert_eq!(frame.height, 2);
        assert_eq!(frame.data.len(), 16);
        // First pixel: red RGBA -> blue BGRA
        assert_eq!(&frame.data[0..4], &[0, 0, 255, 255]);
        // Second pixel: green RGBA -> green BGRA
        assert_eq!(&frame.data[4..8], &[0, 255, 0, 255]);
    }

    #[test]
    fn test_frame_data_from_rgba_with_padding() {
        // 1x2 image with stride=8 (4 bytes padding per row)
        let rgba = vec![
            255, 0, 0, 255, 0, 0, 0, 0, // row 0 + 4 padding bytes
            0, 255, 0, 255, 0, 0, 0, 0, // row 1 + 4 padding bytes
        ];
        let frame = FrameData::from_rgba_padded(&rgba, 1, 2, 8);
        assert_eq!(frame.data.len(), 8);
        assert_eq!(&frame.data[0..4], &[0, 0, 255, 255]); // red -> BGRA
        assert_eq!(&frame.data[4..8], &[0, 255, 0, 255]); // green -> BGRA
    }

    #[test]
    fn test_latest_frame_store_and_take() {
        let lf = LatestFrame::new();
        assert!(lf.take().is_none());

        lf.store(FrameData {
            width: 1,
            height: 1,
            data: vec![0, 0, 0, 255],
        });
        let frame = lf.take().unwrap();
        assert_eq!(frame.width, 1);
        assert!(lf.take().is_none());
    }

    #[test]
    fn test_latest_frame_peek() {
        let lf = LatestFrame::new();
        lf.store(FrameData {
            width: 2,
            height: 2,
            data: vec![0; 16],
        });
        assert!(lf.peek().is_some());
        assert!(lf.peek().is_some()); // still there
        assert!(lf.take().is_some());
        assert!(lf.peek().is_none());
    }
}
