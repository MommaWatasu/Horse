use alloc::{
    vec::Vec,
    vec
};
use core::{fmt::Write, mem::MaybeUninit};

use crate::graphics::{Graphics, PixelColor};

static mut RAW_CONSOLE: MaybeUninit<Console> = MaybeUninit::<Console>::uninit();

pub const LINE_HEIGHT: usize = 18;
pub const MARGIN: usize = 8;

#[derive(Debug, Clone)]
pub struct Console {
    buffer: Vec<Vec<char>>,
    size: (usize, usize),
    fg_color: PixelColor,
    bg_color: PixelColor,
    pub cursor_row: usize,
    cursor_column: usize,
    buffer_row_offset: usize,
}

impl Console {
    fn new(resolution: (usize, usize), fg_color: &PixelColor, bg_color: &PixelColor) -> Self {
        let size = (resolution.0 / MARGIN, resolution.1 / LINE_HEIGHT);
        Console {
            buffer: vec![vec![0.into(); size.0 + 1]; size.1],
            size,
            fg_color: *fg_color,
            bg_color: *bg_color,
            cursor_row: 0,
            cursor_column: 0,
            buffer_row_offset: 0,
        }
    }

    pub fn initialize(resolution: (usize, usize), fg_color: &PixelColor, bg_color: &PixelColor) {
        unsafe { RAW_CONSOLE.write(Console::new(resolution, fg_color, bg_color).clone()) };
    }

    pub fn instance() -> &'static mut Console {
        unsafe { &mut *RAW_CONSOLE.as_mut_ptr() }
    }

    pub fn columns(&self) -> usize { self.size.0 }
    pub fn rows(&self) -> usize { self.size.1 }

    pub fn actual_row(&self, row: usize) -> usize {
        (row + self.buffer_row_offset) % self.rows()
    }

    pub fn actual_cursor_row(&self) -> usize {
        self.actual_row(self.cursor_row)
    }

    pub fn newline(&mut self, graphics: &mut Graphics) {
        self.cursor_column = 0;
        if self.cursor_row < self.rows() - 1 {
            self.cursor_row += 1;
        } else {
            // clear
            Graphics::instance().clear(&self.bg_color);
            self.buffer_row_offset = (self.buffer_row_offset + 1) % self.rows();
            for row in 0..(self.rows() - 1) {
                for column in 0..(self.columns() - 1) {
                    graphics.write_ascii(
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
        let graphics = Graphics::instance();
        for c in s.chars() {
            if c == '\n' {
                self.newline(graphics);
            }
            if self.cursor_column < self.columns() && c as u32 >= 0x20 {
                graphics.write_ascii(
                    8 * self.cursor_column + MARGIN,
                    LINE_HEIGHT * self.cursor_row + MARGIN,
                    c,
                    &self.fg_color,
                );
                let row = self.actual_cursor_row();
                self.buffer[row][self.cursor_column] = c;
                self.cursor_column += 1;
                if self.cursor_column == self.columns()-1 {
                    self.newline(graphics);
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
