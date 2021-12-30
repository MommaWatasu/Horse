#![no_std]
#![no_main]
#![feature(asm)]
#![feature(abi_efiapi)]

mod ascii_font;
pub mod log;
pub mod graphics;
pub mod console;
pub mod pci;

use log::*;

use console::Console;
use core::panic::PanicInfo;
use graphics::{FrameBuffer, Graphics, ModeInfo, PixelColor};
use pci::Device;
use pci::{scan_all_bus, read_class_code, PciDevices};

const BG_COLOR: PixelColor = PixelColor(0, 80, 80);
const FG_COLOR: PixelColor = PixelColor(255, 128, 0);

const K_MOUSE_CURSOR_HEIGHT: usize = 24;
const MOUSE_CURSOR_SHAPE: [&str; K_MOUSE_CURSOR_HEIGHT] = [
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

fn draw_mouse_cursor() {
    let graphics = Graphics::instance();
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

fn welcome_message() {
    print!(
        r"
        ___    ___
       /  /   /  /
      /  /   /  / _______  _____  _____   ______
     /  /___/  / / ___  / / ___/ / ___/ / __  /
    /  ____   / / /  / / / /     \_ \  / /___/
   /  /   /  / / /__/ / / /     __/ / / /___
  /__/   /__/ /______/ /_/     /___/ /_____/
"
    );
    println!("Horse is the OS made by Momma Watasu. This OS is distributed under the MIT license.")
}

fn initialize(fb: *mut FrameBuffer, mi: *mut ModeInfo) {
    unsafe { Graphics::initialize_instance(fb, mi) }
    Console::initialize(&FG_COLOR, &BG_COLOR);
    Graphics::instance().clear(&BG_COLOR);
}

#[no_mangle]
extern "sysv64" fn kernel_main(fb: *mut FrameBuffer, mi: *mut ModeInfo) -> ! {
    initialize(fb, mi);
    welcome_message();
    draw_mouse_cursor();
    loop {
        unsafe {
            asm!("hlt")
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}
