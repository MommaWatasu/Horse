use uefi::proto::console::gop::{FrameBuffer, ModeInfo, PixelFormat};

/// This struct has information about FrameBuffer.
/// - fb: the base address of framebuffer
/// - stride: pixels per scan line
/// - hr: horizontal resolution
/// - vr: vertical resolution
/// - format: pixel format
#[allow(dead_code)]
#[derive(Copy, Clone)]
pub struct FrameBufferConfig {
    fb: *mut u8,
    stride: usize,
    resolution: (usize, usize),
    format: PixelFormat
}

impl FrameBufferConfig {
    pub fn new(mut fb: FrameBuffer, mi: ModeInfo) -> Self {
        return Self {
            fb: fb.as_mut_ptr(),
            stride: mi.stride(),
            resolution: mi.resolution(),
            format: mi.pixel_format()
        }
    }
}