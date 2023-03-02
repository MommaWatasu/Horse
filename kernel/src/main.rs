#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(default_free_fn)]

mod ascii_font;
mod memory_allocator;
mod paging;
mod queue;
mod segment;
pub mod console;
pub mod container_of;
pub mod bit_macros;
pub mod fixed_vec;
pub mod framebuffer;
pub mod graphics;
pub mod interrupt;
pub mod layer;
pub mod log;
pub mod memory_manager;
pub mod mouse;
pub mod pci;
pub mod status;
pub mod timer;
pub mod usb;
pub mod volatile;
pub mod window;

use console::Console;
use framebuffer::*;
use graphics::*;
use interrupt::*;
use layer::*;
use log::*;
use memory_manager::*;
use memory_allocator::KernelMemoryAllocator;
use mouse::{
    MOUSE_CURSOR_HEIGHT,
    MOUSE_CURSOR_WIDTH,
    MOUSE_TRANSPARENT_COLOR,
    draw_mouse_cursor
};
use pci::*;
use queue::ArrayQueue;
use status::StatusCode;
use timer::*;
use usb::{
    memory::*,
    classdriver::mouse::MOUSE_CURSOR,
    xhci::Controller
};
use window::*;

extern crate libloader;
use libloader::MemoryMap;

extern crate alloc;
use alloc::sync::Arc;
use core::{
    arch::asm,
    panic::PanicInfo,
};
use spin::{
    Mutex,
    once::Once
};
use x86_64::{
    instructions::{
        interrupts::{
            enable,//sti
            disable//cli
        },
    },
    structures::idt::InterruptStackFrame
};

const BG_COLOR: PixelColor = PixelColor(153, 76, 0);
const FG_COLOR: PixelColor = PixelColor(255, 255, 255);

static XHC: Once<usize> = Once::new();

#[derive(Clone, Copy, Debug)]
enum Message {
    NoInterruption,
    InterruptXHCI
}
static INTERRUPTION_QUEUE: Mutex<ArrayQueue<Message, 32>> = Mutex::new(ArrayQueue::new());
#[global_allocator]
static ALLOCATOR: KernelMemoryAllocator = KernelMemoryAllocator::new();

fn welcome_message() {
    print!(
        r"
        ___    ___
       /  /   /  /
      /  /   /  / _______  _____  _____  ______
     /  /___/  / / ___  / / ___/ / ___/ / __  /
    /  ____   / / /  / / / /     \_ \  / /___/
   /  /   /  / / /__/ / / /     __/ / / /___
  /__/   /__/ /______/ /_/     /___/ /_____/
"
    );
    println!("Horse is the OS made by Momma Watasu. This OS is distributed under the MIT license.")
}

fn initialize(fb_config: *mut FrameBufferConfig) {
    unsafe { Graphics::initialize_instance(fb_config) }
    let graphics = Graphics::instance();
    let shadow_format = unsafe { (*fb_config).format };
    let resolution = graphics.resolution();
    graphics.clear(&BG_COLOR);

    let mut bgwindow = Arc::new(Window::new(resolution.0, resolution.1, shadow_format));
    let bgwriter = Arc::get_mut(&mut bgwindow).unwrap().writer();
    Console::new(bgwriter, resolution, &FG_COLOR, &BG_COLOR);
    Console::initialize(bgwriter, resolution, &FG_COLOR, &BG_COLOR);

    let mut mouse_window = Arc::new(Window::new(MOUSE_CURSOR_WIDTH, MOUSE_CURSOR_HEIGHT, shadow_format));
    Arc::get_mut(&mut mouse_window).unwrap().set_transparent_color(Some(MOUSE_TRANSPARENT_COLOR));
    draw_mouse_cursor(Arc::get_mut(&mut mouse_window).unwrap().writer(), Coord::new(0, 0));

    unsafe { LAYER_MANAGER.call_once(|| LayerManager::new(graphics.pixel_writer())) };
    let layer_manager = unsafe { LAYER_MANAGER.get_mut().unwrap() };

    let bglayer_id = layer_manager.new_layer()
        .borrow_mut()
        .set_window(bgwindow)
        .move_absolute(Coord::new(0, 0))
        .id();

    let mouse_layer_id = layer_manager.new_layer()
        .borrow_mut()
        .set_window(mouse_window)
        .move_absolute(Coord::new(resolution.0 / 2, resolution.1 / 2))
        .id();
    
    MOUSE_CURSOR.lock().set_layer_id(mouse_layer_id);
    layer_manager.up_down(bglayer_id, LayerHeight::Height(0));
    layer_manager.up_down(mouse_layer_id, LayerHeight::Height(1));
    layer_manager.draw();
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
    INTERRUPTION_QUEUE.lock().push(Message::InterruptXHCI);
    unsafe { notify_end_of_interrupt(); }
}

#[no_mangle]
extern "sysv64" fn kernel_main_virt(fb_config: *mut FrameBufferConfig, memory_map: *const MemoryMap) -> ! {
    //setup memory allocator
    segment::initialize();
    unsafe { paging::initialize(); }
    frame_manager_instance().initialize(unsafe { *memory_map });
    //initialize allocator for usb
    initialize_usballoc();

    //initialize graphics
    initialize(fb_config);

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
    
    //set the IDT entry
    IDT.lock()[InterruptVector::KXHCI as usize].set_handler_fn(handler_xhci);
    unsafe { IDT.lock().load_unsafe(); }
    let bsp_local_apic_id: u8 = unsafe { (*(0xFEE00020 as *const u32) >> 24) as u8 };
    debug!("bsp id: {}", bsp_local_apic_id);
    
    status_log!(configure_msi_fixed_destination(
        &xhc_dev,
        bsp_local_apic_id,
        MSITriggerMode::Level,
        MSIDeliveryMode::Fixed,
        InterruptVector::KXHCI as u8, 0
    ), "Configure msi");
    
    switch_echi_to_xhci(&xhc_dev, &pci_devices);
    let xhc_bar = read_bar(&xhc_dev, 0).unwrap();
    let xhc_mmio_base = (xhc_bar & !0xf) as usize;
    let mut xhc = unsafe { Controller::new(xhc_mmio_base).unwrap() };//there is a problem here
    debug!("xHC initalized");
    unsafe {
        status_log!(xhc.run().unwrap(), "xHC started");
        xhc.configure_ports();
    }
    info!("ports configured");
    
    XHC.call_once(|| &mut xhc as *mut Controller as usize);
    INTERRUPTION_QUEUE.lock().initialize(Message::NoInterruption);

    loop {
        disable();
        if INTERRUPTION_QUEUE.lock().count == 0 {
            unsafe { asm!("sti", "hlt") };//don't touch this line!These instructions must be in a row.
            continue;
        }
        let msg = INTERRUPTION_QUEUE.lock().pop().unwrap();
        enable();

        match msg {
            Message::InterruptXHCI => {
                while xhc.get_er().has_front() {
                    if let Err(e) = xhc.process_event() {
                        error!("Error occurs during processing event: {:?}", e);
                    }
                }
            }
            Message::NoInterruption => {}
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    loop {}
}
