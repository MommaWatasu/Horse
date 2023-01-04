use crate::{
    Graphics,
    PixelColor,
    PixelWriter,
    graphics::Coord,
    layer::LAYER_MANAGER,
    WindowWriter
};
pub const MOUSE_CURSOR_HEIGHT: usize = 24;
pub const MOUSE_CURSOR_WIDTH: usize = 15;
pub const MOUSE_TRANSPARENT_COLOR: PixelColor = PixelColor(0, 0, 1);
pub const MOUSE_CURSOR_SHAPE: [&str; MOUSE_CURSOR_HEIGHT] = [
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
/*
pub fn draw_mouse_cursor(position: (usize, usize), limit: (usize, usize)) {
    let graphics = Graphics::instance();
    let lx: usize = limit.0 - position.0;
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        if position.1 + dy > limit.1 { break; }
        for (dx, c) in l.chars().enumerate() {
            if dx > lx { break; }
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
*/
pub fn draw_mouse_cursor(pixel_writer: &WindowWriter, position: Coord) {
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in l.chars().enumerate() {
            //crate::debug!("dx: {}", dx);
            match c {
                '@' => {
                    pixel_writer.write(position.x + dx, position.y + dy, &PixelColor(0, 0, 0));
                },
                '.' => {
                    pixel_writer.write(position.x + dx, position.y + dy, &PixelColor(255, 255, 255));
                },
                _ => {
                    pixel_writer.write(position.x + dx, position.y + dy, &MOUSE_TRANSPARENT_COLOR)
                }
            }
        }
    }
}

pub fn erase_mouse_cursor(position: (usize, usize), limit: (usize, usize), erase_color: &PixelColor) {
    let graphics = Graphics::instance();
    let lx: usize = limit.0 - position.0;
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        if position.1 + dy > limit.1 { break; }
        for (dx, _c) in l.chars().enumerate() {
            if dx > lx { break; }
            graphics.write_pixel(position.0 + dx, position.1 + dy, erase_color);
        }
    }
}

pub struct MouseCursor {
    layer_id: u32,
    erase_color: PixelColor,
    position: Coord
}

impl MouseCursor {
    pub const fn new(erase_color: PixelColor, position: Coord) -> Self {
        return MouseCursor{
            layer_id: 0,
            erase_color,
            position
        }
    }

    pub fn set_layer_id(&mut self, id: u32) {
        self.layer_id = id;
    }
    
    pub fn pos(&mut self) -> Coord { self.position }
/*
    pub fn move_relative(&mut self, displacement: (usize, usize), limit: (usize, usize)) {
        erase_mouse_cursor(self.position, limit, &self.erase_color);
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
        draw_mouse_cursor(self.position, limit);
    }
*/
    pub fn move_relative(&mut self, displacement: Coord, screen_size: Coord) {
        let mut new_pos = self.position + displacement;
        new_pos = new_pos.elem_min(screen_size - Coord::from_tuple((1, 1)));
        new_pos = new_pos.elem_max(Coord::from_tuple((0, 0)));

        let mut layer_manager = unsafe { LAYER_MANAGER.get_mut().unwrap() };
        layer_manager.move_absolute(self.layer_id, new_pos);
        layer_manager.draw();
    }
}
