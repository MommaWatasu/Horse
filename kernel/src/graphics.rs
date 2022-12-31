use crate::{
    ascii_font::FONTS,
    println
};
use core::{
    mem::MaybeUninit,
    ops::{
        Add,
        AddAssign,
        Sub
    }
};
use libloader::{
    FrameBufferInfo,
    TSFrameBuffer,
    ModeInfo,
    PixelFormat
};

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct PixelColor(pub u8, pub u8, pub u8); // RGB

#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Coord {
    pub x: usize,
    pub y: usize
}

impl Coord {
    pub const fn new(x: usize, y: usize) -> Self { Self{x, y} }
    pub fn from_tuple(pos: (usize, usize)) -> Self {
        Self{
            x: pos.0,
            y: pos.1
        }
    }
    pub fn elem_min(self, other: Self) -> Self {
        return Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y)
        }
    }
    pub fn elem_max(self, other: Self) -> Self {
        return Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y)
        }
    }
}

impl Add for Coord {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        return Self {
            x: self.x + other.x,
            y: self.y + other.y
        }
    }
}

impl AddAssign for Coord {
    fn add_assign(&mut self, other: Self) {
        self.x += other.x;
        self.y += other.y;
    }
}

impl Sub for Coord {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        return Self {
            x: self.x - other.x,
            y: self.y - other.y
        }
    }
}

pub trait PixelWriter {
    fn write(&mut self, x: usize, y: usize, c: &PixelColor);
}

#[derive(Clone, Copy, Default)]
pub struct FrameBufferWriter {
    format: PixelFormat,
    stride: usize,
    fb: TSFrameBuffer
}

impl FrameBufferWriter {
    pub fn new(format: PixelFormat, stride: u32, fb: &mut FrameBufferInfo) -> Self {
        Self {
            format,
            stride: stride as usize,
            fb: unsafe { TSFrameBuffer::new(fb) }
        }
    }
}

impl PixelWriter for FrameBufferWriter {
    fn write(&mut self, x: usize, y: usize, c: &PixelColor) {
        let pixel_index = y * self.stride + x;
        let base = 4 * pixel_index;
        match &self.format {
            PixelFormat::Rgb => {
                unsafe {
                    self.fb.write_value(base, [c.0, c.1, c.2]);
                }
            },
            PixelFormat::Bgr => {
                unsafe {
                    self.fb.write_value(base, [c.2, c.1, c.0]);
                }
            }
            _ => panic!("not supported")
        }
    }
}

// static singleton pointer
static mut RAW_GRAPHICS: MaybeUninit<Graphics> = MaybeUninit::<Graphics>::uninit();
static mut GRAPHICS_INITIALIZED: bool = false;

#[derive(Clone)]
pub struct Graphics {
    fb: FrameBufferInfo,
    mi: ModeInfo,
    pixel_writer: FrameBufferWriter,
    rotated: bool,
    double_scaled: bool,
}

impl Graphics {
    pub fn new(mut fb: FrameBufferInfo, mi: ModeInfo) -> Self {
        let pixel_writer = match mi.format {
            PixelFormat::Rgb => FrameBufferWriter::new(mi.format, mi.stride, &mut fb),
            PixelFormat::Bgr => FrameBufferWriter::new(mi.format, mi.stride, &mut fb),
            _ => {
                panic!("This pixel format is not supported by the drawing demo");
            }
        };

        // Hardcode for GPD Pocket resolution
        let rotated = mi.resolution() == (1200, 1920);
        let double_scaled = mi.resolution() == (1200, 1920);
        Graphics {
            fb,
            mi,
            pixel_writer,
            rotated,
            double_scaled,
        }
    }

    pub fn instance() -> &'static mut Self {
        if unsafe { !GRAPHICS_INITIALIZED } {
            panic!("graphics not initialized");
        }
        unsafe { &mut *RAW_GRAPHICS.as_mut_ptr() }
    }

    ///
    /// # Safety
    /// This is unsafe : handle raw pointers.
    pub unsafe fn initialize_instance(fb: *mut FrameBufferInfo, mi: *mut ModeInfo) {
        RAW_GRAPHICS.write(Graphics::new(*fb, *mi).clone());
        GRAPHICS_INITIALIZED = true;
    }

    /// Write to the pixel of the buffer
    ///
    pub fn write_pixel(&mut self, mut x: usize, mut y: usize, color: &PixelColor) {
        let (width, height) = self.resolution();
        if x > width {
            println!("bad x coord: {}", x);
            return;
        }
        if y > height as usize {
            println!("bad y coord: {}", y);
            return;
        }

        if self.rotated {
            let oy = y;
            y = x;
            x = height - oy;
        }
        if self.double_scaled {
            x *= 2;
            y *= 2;
            self.pixel_writer.write(x, y, color);
            self.pixel_writer.write(x + 1, y, color);
            self.pixel_writer.write(x, y + 1, color);
            self.pixel_writer.write(x + 1, y + 1, color);
        } else {
            self.pixel_writer.write(x, y, color);
        }
    }
    
    pub fn write_ascii(&mut self, x: usize, y: usize, c: char, color: &PixelColor) {
        if (c as u32) > 0x7f {
            return;
        }
        let font: [u8; 16] = FONTS[c as usize];
        for (dy, line) in font.iter().enumerate() {
            for dx in 0..8 {
                if (line << dx) & 0x80 != 0 {
                    self.write_pixel(x + dx, y + dy, &color);
                }
            }
        }
    }

    pub fn write_string(
        &mut self,
        mut x: usize,
        mut y: usize,
        str: &str,
        color: &PixelColor,
    ) -> (usize, usize) {
        let first_x = x;
        let (width, height) = self.resolution();
        for c in str.chars() {
            self.write_ascii(x, y, c, color);
            x += 8;
            if x > width {
                x = first_x;
                y += 20;
            }
            if y > height {
                // can not write
                return (x, y);
            }
        }
        (x, y)
    }

    pub fn resolution(&self) -> (usize, usize) {
        let r = self.mi.resolution();
        let r = if self.rotated { (r.1, r.0) } else { r };
        if self.double_scaled {
            (r.0 / 2, r.1 / 2)
        } else {
            r
        }
    }

    pub fn clear(&mut self, color: &PixelColor) {
        let (width, height) = self.resolution();
        for y in 0..height {
            for x in 0..width {
                self.write_pixel(x, y, color);
            }
        }
    }

    pub fn fb(&self) -> FrameBufferInfo {
        self.fb
    }

    pub fn mi(&self) -> ModeInfo {
        self.mi
    }

    pub fn pixel_writer(&self) -> FrameBufferWriter {
        self.pixel_writer
    }

    pub fn text_writer(
        &mut self,
        first_x: usize,
        first_y: usize,
        color: &PixelColor,
    ) -> TextWriter {
        TextWriter::new(self, first_x, first_y, color)
    }
}

pub struct TextWriter<'a> {
    graphics: &'a mut Graphics,
    first_x: usize,
    first_y: usize,
    x: usize,
    y: usize,
    color: PixelColor,
}

impl<'a> TextWriter<'a> {
    pub fn new(
        graphics: &'a mut Graphics,
        first_x: usize,
        first_y: usize,
        color: &PixelColor,
    ) -> Self {
        TextWriter {
            graphics,
            first_x,
            first_y,
            x: first_x,
            y: first_y,
            color: *color,
        }
    }

    pub fn reset_coord(&mut self) {
        self.x = self.first_x;
        self.y = self.first_y;
    }

    pub fn change_color(&mut self, color: &PixelColor) {
        self.color = *color;
    }
}

impl<'a> core::fmt::Write for TextWriter<'a> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let (x, y) = self.graphics.write_string(self.x, self.y, s, &self.color);
        self.x = x;
        self.y = y;
        Ok(())
    }
}
