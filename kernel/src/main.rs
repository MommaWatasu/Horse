#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(core_intrinsics)]
#![feature(abi_x86_interrupt)]

mod ascii_font;
pub mod bit_macros;
pub mod status;
pub mod log;
pub mod graphics;
pub mod console;
pub mod pci;
pub mod usb;
pub mod volatile;
pub mod mouse;
pub mod fixed_vec;
pub mod interrupt;

use spin::Mutex;
use status::StatusCode;
use log::*;
use console::Console;
use core::panic::PanicInfo;
use graphics::{FrameBuffer, Graphics, ModeInfo, PixelColor};
use pci::*;
use usb::xhci::Controller;
use interrupt::*;
use x86_64::{
    instructions::interrupts::enable,
    structures::idt::InterruptStackFrame
};

const BG_COLOR: PixelColor = PixelColor(0, 0, 0);
const FG_COLOR: PixelColor = PixelColor(255, 255, 255);

static XHC: Mutex<usize> = Mutex::new(0);

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
            status_log!(StatusCode::Success, "Scanning Bus")
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

extern "x86-interrupt" fn handler_xhci(_: InterruptStackFrame) {
    debug!("interruption!");
    let xhc = unsafe {
        let xhc_addr = XHC.lock();
        &mut *(*xhc_addr as *mut Controller)
    };
    while xhc.get_er().has_front() {
        if let Err(e) = xhc.process_event() {
            error!("Error occurs during process_event: {:?}", e);
        }
    }
    unsafe { notify_end_of_interrupt(); }
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
                StatusCode::Success,
                "xHC has been found: {}.{}.{}",
                xhc_dev.bus, xhc_dev.device, xhc_dev.function
            );
            xhc_dev
        }
        None => {
            panic!("no xHC device");
        }
    };
    
    IDT.lock()[InterruptVector::KXHCI as usize].set_handler_fn(handler_xhci);
    unsafe { IDT.lock().load_unsafe(); }
    let bsp_local_apic_id: u8 = unsafe { (*(0xFEE00020 as *const u32) >> 24) as u8 };
    debug!("bsp id: {}", bsp_local_apic_id);
    
    status_log!(configure_msi_fixed_destination(
        &xhc_dev.clone(),
        bsp_local_apic_id,
        MSITriggerMode::Level,
        MSIDeliveryMode::Fixed,
        InterruptVector::KXHCI as u8, 0
    ), "Configure msi");
    
    switch_echi_to_xhci(&xhc_dev, &pci_devices);
    let xhc_bar = read_bar(&xhc_dev, 0).unwrap();
    let xhc_mmio_base = (xhc_bar & !0xf) as usize;
    let mut xhc: Controller;
    unsafe {
        xhc = Controller::new(xhc_mmio_base).unwrap();
    }
    debug!("xHC initalized");
    unsafe {
        status_log!(xhc.run().unwrap(), "xHC started");
        xhc.configure_ports();
    }
    info!("ports configured");
    
    let mut xhc_addr = XHC.lock();
    *xhc_addr = &mut xhc as *mut Controller as usize;
    enable();
    
    /*
    loop {
        if let Err(e) = xhc.process_event() {
            error!("Error occurs during process_event: {:?}", e);
        }
    }
    */
    
    hlt_loop();
    
    info!("DONE ALL PROCESSING");
}

fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}
