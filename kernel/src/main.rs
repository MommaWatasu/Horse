#![no_std]
#![no_main]
#![feature(abi_efiapi)]

mod ascii_font;
pub mod bit_macros;
pub mod status;
pub mod log;
pub mod graphics;
pub mod console;
pub mod pci;
pub mod usb;
pub mod volatile;
pub mod register;
pub mod mouse;

use status::StatusCode;
use log::*;
use mouse::*;
use console::Console;
use core::panic::PanicInfo;
use core::arch::asm;
use graphics::{FrameBuffer, Graphics, ModeInfo, PixelColor};
use pci::Device;
use pci::{read_bar, scan_all_bus, read_class_code, read_vendor_id, ClassCode, PciDevices};
use usb::xhci::Controller;
use usb::xhci::port::Port;

const BG_COLOR: PixelColor = PixelColor(0, 0, 0);
const FG_COLOR: PixelColor = PixelColor(255, 255, 255);

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

fn find_pci_devices() -> PciDevices {
    let pci_devices: PciDevices;
    match scan_all_bus() {
        Ok(v) => {
            pci_devices = v;
            status_log!(StatusCode::KSuccess, "Scanning Bus")
        },
        Err(_code) => {
            panic!("Scanning Bus");
        }
    }
    for dev in pci_devices.iter() {
        let vendor_id = read_vendor_id(dev.bus, dev.device, dev.function);
        let class_code = read_class_code(dev.bus, dev.device, dev.function);
        trace!(
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

fn switch_echi_to_xhci(_xhc_dev: &Device, pci_devices: &PciDevices) {
    let ehciclass = ClassCode {
        base: 0x0c,
        sub: 0x03,
        interface: 0x20,
    };
    let ehci = pci_devices
        .iter()
        .find(|&dev| dev.class_code == ehciclass && dev.get_vendor_id() == 0x8086);
    if ehci.is_none() {
        info!("no ehci");
    } else {
        panic!("ehci found, but do nothing for the present");
    }
}

static mouse_cursor: spin::Mutex<MouseCursor> = spin::Mutex::new(MouseCursor::new(BG_COLOR, [300, 200]));

fn mouse_observer(displacement_x: i8, displacement_y: i8) {
    mouse_cursor.lock().move_relative([displacement_x.try_into().unwrap(),
        displacement_y.try_into().unwrap()]);
}

#[no_mangle]
extern "sysv64" fn kernel_main(fb: *mut FrameBuffer, mi: *mut ModeInfo) -> ! {
    initialize(fb, mi);
    welcome_message();

    let pci_devices = find_pci_devices();
    let xhc_dev = find_xhc(&pci_devices);
    let xhc_dev = match xhc_dev {
        Some(xhc_dev) => {
            status_log!(
                StatusCode::KSuccess,
                "xHC has been found: {}.{}.{}",
                xhc_dev.bus, xhc_dev.device, xhc_dev.function
            );
            xhc_dev
        }
        None => {
            panic!("no xHC device");
        }
    };
    switch_echi_to_xhci(&xhc_dev, &pci_devices);
    let xhc_bar = read_bar(&xhc_dev, 0).unwrap();
    let xhc_mmio_base = (xhc_bar & !0xf) as usize;
    let mut xhc: Controller;
    unsafe {
        xhc = Controller::new(xhc_mmio_base);
    };
    status_log!(xhc.run().unwrap(), "XHC starting");

    let mut port: Port;
    let mut is_connected: bool;
    for i in 1..xhc.max_ports() {
        unsafe {
            port = xhc.port_at(i);
            is_connected = port.is_connected();
        }

        if is_connected {
            match xhc.configure_port(&port) {
                Ok(sc) => {
                    status_log!(StatusCode::KSuccess, "Configure Port: {}", sc);
                },
                Err(sc) => {
                    status_log!(StatusCode::KFailure, "Configure Port: {}", sc);
                    continue;
                }
            }
        }
    }
    info!("DONE ALL PROCESSING");
    
    /*
    loop {
        match xhc.process_event(); {
            Ok(StatusCode::KSucess) => {},
            Err(code) => {
                status_log!(code, "Error while process_event")
            }
        }
    }
    */

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
