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
use pci::{read_bar, scan_all_bus, read_class_code, read_vendor_id, ClassCode, PciDevices};

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

fn list_pci_devices() -> PciDevices {
    let pci_devices: PciDevices;
    match scan_all_bus() {
        Ok(v) => {
            pci_devices = v;
            info!("ScanBus: Success");
        },
        Err(_code) => {
            panic!("ScanBus: Failed")
        }
    }
    debug!("scanned pci devices.");
    for dev in pci_devices.iter() {
        let vendor_id = read_vendor_id(dev.bus, dev.device, dev.function);
        let class_code = read_class_code(dev.bus, dev.device, dev.function);
        info!(
            "{}.{}.{}:, vend {:04x}, class {}, head {:02x}",
            dev.bus,
            dev.device,
            dev.function,
            vendor_id,
            class_code,
            dev.header_type
        );
    }
    pci_devices
}

fn find_xhc(pci_devices: &PciDevices) -> Option<Device> {
    let mut xhc_dev = None;
    const XHC_CLASS: ClassCode = ClassCode {
        base: 0x0c,
        sub: 0x03,
        interface: 0x30
    };
    for dev in pci_devices.iter() {
        if dev.class_code == XHC_CLASS {
            xhc_dev = Some(dev);
            if dev.get_vendor_id() == 0x8086 {
                break;
            }
        }
    }
    xhc_dev
}

#[no_mangle]
extern "sysv64" fn kernel_main(fb: *mut FrameBuffer, mi: *mut ModeInfo) -> ! {
    initialize(fb, mi);
    welcome_message();

    let pci_devices = list_pci_devices();
    let xhc = find_xhc(&pci_devices);
    let xhc = match xhc {
        Some(xhc) => {
            info!(
                "xHC has been found: {}.{}.{}",
                xhc.bus, xhc.device, xhc.function
            );
            xhc
        }
        None => {
            panic!("no xHC device");
        }
    };
    let xhc_bar = read_bar(&xhc, 0).unwrap();

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
