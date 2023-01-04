use alloc::{
    vec::Vec,
    vec
};
use core::fmt::Write;
use spin::{
    Mutex,
    MutexGuard,
    Once
};

use crate::{
    ascii_font::FONTS,
    graphics::{
        PixelColor,
        PixelWriter
    },
    window::WindowWriter
};

static RAW_CONSOLE: Mutex<Option<Console>> = Mutex::new(None);

pub const LINE_HEIGHT: usize = 18;
pub const MARGIN: usize = 8;

#[derive(Debug, Clone)]
pub struct Console {
    pixel_writer: usize,
    buffer: Vec<Vec<char>>,
    size: (usize, usize),
    fg_color: PixelColor,
    bg_color: PixelColor,
    pub cursor_row: usize,
    cursor_column: usize,
    buffer_row_offset: usize,
}

impl Console {
    fn new(pixel_writer: &WindowWriter, resolution: (usize, usize), fg_color: &PixelColor, bg_color: &PixelColor) -> Self {
        let size = (resolution.0 / MARGIN, resolution.1 / LINE_HEIGHT);
        Console {
            pixel_writer: pixel_writer as *const WindowWriter as usize,
            buffer: vec![vec![0.into(); size.0 + 1]; size.1],
            size,
            fg_color: *fg_color,
            bg_color: *bg_color,
            cursor_row: 0,
            cursor_column: 0,
            buffer_row_offset: 0,
        }
    }

    pub fn initialize(pixel_writer: &WindowWriter, resolution: (usize, usize), fg_color: &PixelColor, bg_color: &PixelColor) {
        *RAW_CONSOLE.lock() = Some(Console::new(pixel_writer, resolution, fg_color, bg_color));
    }

    pub fn instance() -> MutexGuard<'static, Option<Console>> {
        RAW_CONSOLE.lock()
    }

    pub fn pixel_writer(&self) -> &WindowWriter {
        unsafe { &*(self.pixel_writer as *const WindowWriter) }
    }

    pub fn columns(&self) -> usize { self.size.0 }
    pub fn rows(&self) -> usize { self.size.1 }

    pub fn actual_row(&self, row: usize) -> usize {
        (row + self.buffer_row_offset) % self.rows()
    }

    pub fn actual_cursor_row(&self) -> usize {
        self.actual_row(self.cursor_row)
    }

    pub fn newline(&mut self) {
        self.cursor_column = 0;
        if self.cursor_row < self.rows() - 1 {
            self.cursor_row += 1;
        } else {
            clear(self.pixel_writer(), &self.bg_color);
            self.buffer_row_offset = (self.buffer_row_offset + 1) % self.rows();
            for row in 0..(self.rows() - 1) {
                for column in 0..(self.columns() - 1) {
                    write_ascii(
                        self.pixel_writer(),
                        8 * column + MARGIN,
                        LINE_HEIGHT * row + MARGIN,
                        self.buffer[self.actual_row(row)][column],
                        &self.fg_color,
                    );
                }
            }
            let cursor_row = self.actual_cursor_row();
            self.buffer[cursor_row] = vec![0.into(); self.columns() + 1].clone();
        }
    }
    pub fn put_string(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                self.newline();
            }
            if self.cursor_column < self.columns() && c as u32 >= 0x20 {
                write_ascii(
                    self.pixel_writer(),
                    8 * self.cursor_column + MARGIN,
                    LINE_HEIGHT * self.cursor_row + MARGIN,
                    c,
                    &self.fg_color,
                );
                let row = self.actual_cursor_row();
                self.buffer[row][self.cursor_column] = c;
                self.cursor_column += 1;
                if self.cursor_column == self.columns()-1 {
                    self.newline();
                }
            }
        }
    }
}

impl Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.put_string(s);
        Ok(())
    }
}

use crate::graphics::Graphics;
fn clear(pixel_writer: &WindowWriter, color: &PixelColor) {
    let (width, height) = pixel_writer.size();
    for y in 0..height {
        for x in 0..width {
            pixel_writer.write(x, y, color);
            //Graphics::instance().pixel_writer().write(x, y, color);
        }
    }
}

fn write_ascii(pixel_writer: &WindowWriter, x: usize, y: usize, c: char, color: &PixelColor) {
    if (c as u32) > 0x7f {
        return;
    }
    let font: [u8; 16] = FONTS[c as usize];
    for (dy, line) in font.iter().enumerate() {
        for dx in 0..8 {
            if (line << dx) & 0x80 != 0 {
                pixel_writer.write(x + dx, y + dy, &color);
                //Graphics::instance().pixel_writer().write(x + dx, y + dy, &color);
            }
        }
    }
}