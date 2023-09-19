#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]

mod acpi;
mod ascii_font;
mod memory_allocator;
mod paging;
mod queue;
mod segment;

pub mod console;
pub mod drivers;
pub mod fixed_vec;
pub mod framebuffer;
pub mod graphics;
pub mod interrupt;
pub mod layer;
pub mod lib;
pub mod log;
pub mod memory_manager;
pub mod mouse;
pub mod status;
pub mod volatile;
pub mod window;

use acpi::*;
use console::Console;
use drivers::{
    detect_dev::initialize_pci_devices,
    pci::*,
    timer::*,
    usb::{classdriver::mouse::MOUSE_CURSOR, memory::*, xhci::Controller},
    video::qemu::*,
};
use framebuffer::*;
use graphics::*;
use interrupt::*;
use layer::*;
use log::*;
use memory_allocator::KernelMemoryAllocator;
use memory_manager::*;
use mouse::{draw_mouse_cursor, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH, MOUSE_TRANSPARENT_COLOR};
use queue::ArrayQueue;
use status::StatusCode;
use window::*;

extern crate libloader;
use libloader::MemoryMap;

extern crate alloc;
use alloc::sync::Arc;
use core::{arch::asm, panic::PanicInfo};
use spin::{once::Once, Mutex};
use uefi::table::{Runtime, SystemTable};
use x86_64::{
    instructions::interrupts::{
        disable, //cli
        enable,  //sti
    },
    structures::idt::InterruptStackFrame,
};

const BG_COLOR: PixelColor = PixelColor(153, 76, 0);
const FG_COLOR: PixelColor = PixelColor(255, 255, 255);

#[derive(Clone, Copy, Debug)]
pub enum Message {
    NoInterruption,
    InterruptXHCI,
    TimerTimeout { timeout: u64, value: i32 },
}

pub static XHC: Mutex<Once<usize>> = Mutex::new(Once::new());
pub static INTERRUPTION_QUEUE: Mutex<ArrayQueue<Message, 32>> = Mutex::new(ArrayQueue::new());
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
    let fb_config_ref = unsafe { *fb_config };
    let resolution = fb_config_ref.resolution;
    unsafe { Graphics::initialize_instance(fb_config_ref) }
    let graphics = Graphics::instance();
    graphics.clear(&BG_COLOR);

    let mut bgwindow = Arc::new(Window::new(
        resolution.0,
        resolution.1,
        fb_config_ref.format,
    ));
    let bgwriter = Arc::get_mut(&mut bgwindow).unwrap().writer();
    Console::initialize(bgwriter, resolution, &FG_COLOR, &BG_COLOR);

    let mut mouse_window = Arc::new(Window::new(
        MOUSE_CURSOR_WIDTH,
        MOUSE_CURSOR_HEIGHT,
        fb_config_ref.format,
    ));
    Arc::get_mut(&mut mouse_window)
        .unwrap()
        .set_transparent_color(Some(MOUSE_TRANSPARENT_COLOR));
    draw_mouse_cursor(
        Arc::get_mut(&mut mouse_window).unwrap().writer(),
        Coord::new(0, 0),
    );

    unsafe { LAYER_MANAGER.call_once(|| LayerManager::new(fb_config_ref)) };
    let layer_manager = unsafe { LAYER_MANAGER.get_mut().unwrap() };

    let bglayer_id = layer_manager
        .new_layer()
        .borrow_mut()
        .set_window(bgwindow)
        .move_absolute(Coord::new(0, 0))
        .id();

    let mouse_layer_id = layer_manager
        .new_layer()
        .borrow_mut()
        .set_window(mouse_window)
        .move_absolute(Coord::new(resolution.0 / 2, resolution.1 / 2))
        .id();

    MOUSE_CURSOR.lock().set_layer_id(mouse_layer_id);
    layer_manager.up_down(bglayer_id, LayerHeight::Height(0));
    layer_manager.up_down(mouse_layer_id, LayerHeight::Height(1));
    layer_manager.draw();
}

extern "x86-interrupt" fn handler_xhci(_: InterruptStackFrame) {
    INTERRUPTION_QUEUE.lock().push(Message::InterruptXHCI);
    unsafe {
        notify_end_of_interrupt();
    }
}

extern "x86-interrupt" fn handler_lapic_timer(_: InterruptStackFrame) {
    lapic_timer_on_interrupt();
    unsafe {
        notify_end_of_interrupt();
    }
}

#[no_mangle]
extern "sysv64" fn kernel_main_virt(
    st: SystemTable<Runtime>,
    fb_config: *mut FrameBufferConfig,
    memory_map: *const MemoryMap,
) -> ! {
    //setup memory allocator
    segment::initialize();
    unsafe {
        paging::initialize();
    }
    frame_manager_instance().initialize(unsafe { *memory_map });
    //initialize allocator for usb
    initialize_usballoc();

    //initialize graphics
    initialize(fb_config);

    welcome_message();
    unsafe { debug!("fb: {:?}", (*fb_config).fb) };

    initialize_acpi(st);

    let pci_devices = find_pci_devices();
    let mut xhc = initialize_pci_devices(&pci_devices).unwrap();

    //set the IDT entry
    IDT.lock()[InterruptVector::Xhci as usize].set_handler_fn(handler_xhci);
    IDT.lock()[InterruptVector::LAPICTimer as usize].set_handler_fn(handler_lapic_timer);
    unsafe {
        IDT.lock().load_unsafe();
    }
    INTERRUPTION_QUEUE
        .lock()
        .initialize(Message::NoInterruption);

    loop {
        disable();
        if INTERRUPTION_QUEUE.lock().count == 0 {
            unsafe { asm!("sti", "hlt") }; //don't touch this line!These instructions must be in a row.
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
            Message::TimerTimeout { timeout, value } => {
                if value != -1 {
                    println!("Timer timeout: {}", value)
                };
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
