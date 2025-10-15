use alloc::{vec, vec::Vec};
use core::ptr::null_mut;

use crate::{
    container_of,
    framebuffer::{FrameBuffer, FrameBufferConfig},
    graphics::{Coord, PixelColor, PixelWriter},
};
use libloader::PixelFormat;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WindowWriter(usize, usize);

impl WindowWriter {
    pub fn write(&self, x: usize, y: usize, c: &PixelColor) {
        #[allow(deref_nullptr)]
        let window = container_of!(self, mutable Window, writer);
        window.data[x][y] = *c;
        window.shadow_buffer.writer.write(x, y, c);
    }

    pub fn move_buffer(&self, dst: Coord, src: Coord, size: Coord) {
        #[allow(deref_nullptr)]
        let window = container_of!(self, mutable Window, writer);
        unsafe {
            window.shadow_buffer.move_buffer(dst, src, size);
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.0, self.1)
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Window {
    pub writer: WindowWriter,
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    pub shadow_buffer: FrameBuffer,
    transparent_color: Option<PixelColor>,
}

impl Window {
    pub fn new(width: usize, height: usize, format: PixelFormat) -> Self {
        let config = FrameBufferConfig {
            fb: null_mut(),
            stride: 0,
            resolution: (width, height),
            format,
        };
        let mut shadow_buffer = FrameBuffer::new(config);
        Self {
            writer: WindowWriter(width, height),
            width,
            height,
            data: vec![vec![PixelColor::default(); height]; width],
            shadow_buffer,
            transparent_color: None,
        }
    }

    pub fn writer(&mut self) -> &mut WindowWriter {
        &mut self.writer
    }

    pub fn set_transparent_color(&mut self, c: Option<PixelColor>) {
        self.transparent_color = c;
    }

    fn at(&self, x: usize, y: usize) -> &PixelColor {
        &self.data[x][y]
    }

    pub fn draw_to(&self, fb: &mut FrameBuffer, position: Coord) {
        if self.transparent_color.is_none() {
            unsafe {
                fb.copy(position, &self.shadow_buffer);
            }
        } else {
            let mut writer = fb.writer;
            let tc = &self.transparent_color.unwrap();
            let mut c: &PixelColor;
            for y in 0..self.height {
                for x in 0..self.width {
                    c = self.at(x, y);
                    if c != tc {
                        writer.write(position.x + x, position.y + y, &c);
                    }
                }
            }
        }
    }
}
