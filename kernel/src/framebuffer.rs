use crate::{
    Coord,
    FrameBufferWriter,
    StatusCode
};

use alloc::{
    vec::Vec,
    vec
};
use core::{
    default::Default,
    ptr::{
        copy_nonoverlapping,
        null_mut
    }
};
use libloader::PixelFormat;

/// This struct has information about FrameBuffer.
/// - fb: the base address of framebuffer
/// - stride: pixels per scan line
/// - hr: horizontal resolution
/// - vr: vertical resolution
/// - format: pixel format
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FrameBufferConfig {
    pub fb: *mut u8,
    pub stride: usize,
    pub resolution: (usize, usize),
    pub format: PixelFormat
}

impl Default for FrameBufferConfig {
    fn default() -> FrameBufferConfig {
        return FrameBufferConfig {
            fb: null_mut(),
            stride: 0,
            resolution: (0, 0),
            format: PixelFormat::Rgb
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameBuffer {
    pub config: FrameBufferConfig,
    pub buffer: Vec<u8>,
    //writer has the problem
    pub writer: FrameBufferWriter
}

impl FrameBuffer {
    pub fn new(mut config: FrameBufferConfig) -> Self {
        let bpp = Self::bits_per_pixel(config.format);
        if bpp <= 0 {
            panic!("This pixel format is not supported by the drawing demo");
        }

        let mut buffer: Vec<u8>;
        if config.fb != null_mut() {
            buffer = vec![];
        } else {
            let (hr, vr) = config.resolution;
            buffer = vec![0; ((bpp + 7) / 8) * hr * vr];
            config.fb = buffer.as_mut_ptr();
            config.stride = hr;
        }

        let mut writer = match config.format {
            PixelFormat::Rgb => FrameBufferWriter::new(config.format, config.stride, config.fb),
            PixelFormat::Bgr => FrameBufferWriter::new(config.format, config.stride, config.fb),
            _ => {
                panic!("This pixel format is not supported by the drawing demo");
            }
        };

        use crate::{
            graphics::PixelWriter,
            PixelColor
        };

        return Self {
            config,
            buffer,
            writer
        }
    }

    pub unsafe fn copy(&self, pos: Coord, src: &FrameBuffer) -> StatusCode {
        if self.config.format != src.config.format {
            return StatusCode::UnknownPixelFormat;
        }

        let mut bpp = Self::bits_per_pixel(self.config.format);
        if bpp <= 0 {
            return StatusCode::UnknownPixelFormat;
        }

        let (dst_width, dst_height) = self.config.resolution;
        let (src_width, src_height) = src.config.resolution;
        let copy_start_dst_x = pos.x.max(0);
        let copy_start_dst_y = pos.y.max(0);
        let copy_end_dst_x = dst_width.min(pos.x + src_width);
        let copy_end_dst_y = dst_height.min(pos.y + src_height);

        bpp = (bpp + 7) / 8;
        let stride = bpp * (copy_end_dst_x - copy_start_dst_x);
        let mut dst_buf: *mut u8 = self.config.fb.add(bpp * (self.config.stride * copy_start_dst_y + copy_start_dst_x));
        let mut src_buf: *const u8 = src.config.fb;

        for dy in 0..(copy_end_dst_y - copy_start_dst_y) {
            copy_nonoverlapping(src_buf, dst_buf, stride);
            dst_buf = dst_buf.add(bpp * self.config.stride);
            src_buf = src_buf.add(bpp * src.config.stride)
        }
        return StatusCode::Success
    }

    fn bits_per_pixel(format: PixelFormat) -> usize {
        return match format {
            PixelFormat::Rgb => 32,
            PixelFormat::Bgr => 32,
            _ => unreachable!()
        }
    }
}