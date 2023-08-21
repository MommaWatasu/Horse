use crate::{
    Graphics,
    PixelColor,
    graphics::Coord,
    layer::LAYER_MANAGER,
    WindowWriter,
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

struct CoordDiff {
    x: i32,
    y: i32
}

impl CoordDiff {
    fn new(diff: (i8, i8)) -> Self {
        return Self {
            x: diff.0 as i32,
            y: diff.1 as i32
        }
    }

    fn add(self, pos: Coord) -> Coord {
        let (mut x, mut y): (i32, i32) = (pos.x as i32, pos.y as i32);
        if x + self.x < 0 {
            x = 0;
        } else {
            x += self.x;
        }
        if y + self.y < 0 {
            y = 9;
        } else {
            y += self.y;
        }
        return Coord {
            x: x as usize,
            y: y as usize
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

    pub fn move_relative(&mut self, displacement: (i8, i8), screen_size: Coord) {
        let mut new_pos = CoordDiff::new(displacement).add(self.position);
        new_pos = new_pos.elem_min(screen_size - Coord::from_tuple((1, 1)));
        new_pos = new_pos.elem_max(Coord::from_tuple((0, 0)));
        self.position = new_pos;

        let layer_manager = unsafe { LAYER_MANAGER.get_mut().unwrap() };
        layer_manager.move_absolute(self.layer_id, new_pos);
        layer_manager.draw();
    }
}
