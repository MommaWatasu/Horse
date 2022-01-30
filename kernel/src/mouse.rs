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

pub fn draw_mouse_cursor() {
    let graphics = Graphics::instance();
    debug!("resolution: {} . {}", graphics.resolution().0, graphics.resolution().1);
    for (dy, l) in MOUSE_CURSOR_SHAPE.iter().enumerate() {
        for (dx, c) in l.chars().enumerate() {
            let x = 200+dx;
            let y = 100+dy;
            match c {
                '@' => {
                    graphics.write_pixel(x, y, &PixelColor(0, 0, 0));
                },
                '.' => {
                    graphics.write_pixel(x, y, &PixelColor(255, 255, 255));
                },
                _=>{}
            }
        }
    }
}