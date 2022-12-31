use alloc::{
    vec::Vec,
    vec
};
use crate::{
    container_of,
    graphics::{
        Coord,
        PixelColor,
        PixelWriter
    }
};

#[derive(Clone, Default, PartialEq)]
pub struct WindowWriter {}

impl PixelWriter for WindowWriter {
    fn write(&mut self, x: usize, y: usize, c: &PixelColor) {
        container_of!(self, mutable Window, writer).data[x][y] = *c;
    }
}

#[derive(Clone, Default, PartialEq)]
pub struct Window {
    writer: WindowWriter,
    width: usize,
    height: usize,
    data: Vec<Vec<PixelColor>>,
    transparent_color: Option<PixelColor>
}

impl Window {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            writer: WindowWriter{},
            width,
            height,
            data: vec![vec![PixelColor::default(); width]; height],
            transparent_color:  None
        }
    }

    pub fn writer(&mut self) -> &mut WindowWriter {&mut self.writer}

    pub fn set_transparent_color(&mut self, c: Option<PixelColor>) {
        self.transparent_color = c;
    }

    fn at(&self, x: usize, y: usize) -> &PixelColor {
        &self.data[y][x]
    }

    pub fn draw_to<T: PixelWriter>(&self, writer: &mut T, position: Coord) {
        if self.transparent_color.is_none() {
            for y in 0..self.height {
                for x in 0..self.width {
                    writer.write(position.x + x, position.y + y, self.at(x, y))
                }
            }
        } else {
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