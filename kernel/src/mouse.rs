use crate::{
    Graphics,
    PixelColor
};
use crate::debug;
const K_MOUSE_CURSOR_HEIGHT: usize = 24;
pub const MOUSE_CURSOR_SHAPE: [&str; K_MOUSE_CURSOR_HEIGHT] = [
"@              ",
"@@             ",
"@.@            ",
"@..@           ",
"@...@          ",
"@....@         ",
"@.....@        ",
"@......@       ",
"@.......@      ",
"@........@     ",
"@.........@    ",
"@..........@   ",
"@...........@  ",
"@............@ ",
"@......@@@@@@@@",
"@......@       ",
"@....@@.@      ",
"@...@ @.@      ",
"@..@   @.@     ",
"@.@    @.@     ",
"@@      @.@    ",
"@       @.@    ",
"         @.@   ",
"         @@@   "
];

pub fn draw_mouse_cursor(position: [usize; 2]) {
    let graphics = Graphics::instance();
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in l.chars().enumerate() {
            match c {
                '@' => {
                    graphics.write_pixel(position[0]+dx, position[1]+dy, &PixelColor(0, 0, 0));
                },
                '.' => {
                    graphics.write_pixel(position[0]+dx, position[1]+dy, &PixelColor(255, 255, 255));
                },
                _=>{}
            }
        }
    }
}

pub fn erase_mouse_cursor(position: [usize; 2], erase_color: &PixelColor) {
    let graphics = Graphics::instance();
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in l.chars().enumerate() {
            graphics.write_pixel(position[0]+dx, position[1]+dy, erase_color);
        }
    }
}

pub struct MouseCursor {
    erase_color: PixelColor,
    position: [usize; 2]
}

impl MouseCursor {
    pub const fn new(erase_color: PixelColor, position: [usize; 2]) -> Self {
            return MouseCursor{
                erase_color,
                position
            }
    }
    
    pub fn move_relative(&mut self, displacement: [usize; 2]) {
        erase_mouse_cursor(self.position, &self.erase_color);
        self.position[0] += displacement[0]; self.position[1] += displacement[1];
        draw_mouse_cursor(self.position);
    }
}
