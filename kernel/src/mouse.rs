use crate::{
    Graphics,
    PixelColor
};
pub const K_MOUSE_CURSOR_HEIGHT: usize = 24;
pub const K_MOUSE_CURSOR_WIDTH: usize = 15;
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

pub fn draw_mouse_cursor(position: (usize, usize)) {
    let graphics = Graphics::instance();
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in l.chars().enumerate() {
            match c {
                '@' => {
                    graphics.write_pixel(position.0 + dx, position.1 + dy, &PixelColor(0, 0, 0));
                },
                '.' => {
                    graphics.write_pixel(position.0 + dx, position.1 + dy, &PixelColor(255, 255, 255));
                },
                _=>{}
            }
        }
    }
}

pub fn erase_mouse_cursor(position: (usize, usize), erase_color: &PixelColor) {
    let graphics = Graphics::instance();
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, _c) in l.chars().enumerate() {
            graphics.write_pixel(position.0 + dx, position.1 + dy, erase_color);
        }
    }
}

pub struct MouseCursor {
    erase_color: PixelColor,
    position: (usize, usize)
}

impl MouseCursor {
    pub const fn new(erase_color: PixelColor, position: (usize, usize)) -> Self {
        return MouseCursor{
            erase_color,
            position
        }
    }
    
    pub fn pos(&mut self) -> (usize, usize) { self.position }
    
    pub fn move_relative(&mut self, displacement: (usize, usize), limit: (usize, usize)) {
        erase_mouse_cursor(self.position, &self.erase_color);
        if displacement.0 >= 128 {
            if self.position.0 + displacement.0 < 256 {
                self.position.0 = 0;
            } else {
                self.position.0 += displacement.0;
                self.position.0 -= 256;
            }
        } else {
            if self.position.0 + displacement.0 > limit.0 {
                self.position.0 = limit.0;
            } else {
                self.position.0 += displacement.0;
            }
        }
        if displacement.1 >= 128 {
            if self.position.1 + displacement.1 < 256 {
                self.position.1 = 0;
            } else {
                self.position.1 += displacement.1;
                self.position.1 -= 256;
            }
        } else {
            if self.position.1 + displacement.1 > limit.1 {
                self.position.1 = limit.1;
            } else {
                self.position.1 += displacement.1;
            }
        }
        draw_mouse_cursor(self.position);
    }
}
