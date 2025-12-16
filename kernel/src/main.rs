#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(never_type)]

mod acpi;
mod ascii_font;
mod memory_allocator;
pub mod paging;
mod queue;
mod segment;
mod syscall;

pub mod console;
pub mod elf;
pub mod exec;
pub mod drivers;
pub mod fixed_vec;
pub mod framebuffer;
pub mod graphics;
pub mod interrupt;
pub mod layer;
pub mod horse_lib;
pub mod log;
pub mod memory_manager;
pub mod mouse;
pub mod proc;
pub mod status;
pub mod volatile;
pub mod window;

use acpi::*;
use console::Console;
use drivers::{
    detect_dev::initialize_pci_devices,
    pci::*,
    timer::*,
    usb::{classdriver::mouse::MOUSE_CURSOR, memory::*},
    fs::init::initialize_filesystem,
};
use framebuffer::*;
use graphics::*;
use interrupt::*;
use layer::*;
use log::*;
use memory_allocator::KernelMemoryAllocator;
use memory_manager::*;
use mouse::{draw_mouse_cursor, MOUSE_CURSOR_HEIGHT, MOUSE_CURSOR_WIDTH, MOUSE_TRANSPARENT_COLOR};
use proc::{PROCESS_MANAGER, initialize_process_manager};
use queue::ArrayQueue;
use status::StatusCode;
use window::*;

extern crate libloader;
use libloader::MemoryMap;

extern crate alloc;
use alloc::sync::Arc;
use core::{arch::asm, panic::PanicInfo, ops::DerefMut};
use spin::{once::Once, Mutex};
use uefi::table::{Runtime, SystemTable};
use x86_64::{
    instructions::interrupts::{
        disable, //cli
        enable,  //sti
    },
    structures::idt::{InterruptStackFrame, PageFaultErrorCode},
    PrivilegeLevel,
    VirtAddr,
};

use crate::drivers::fs::core::{FILE_DESCRIPTOR_TABLE, FileSystem};
use crate::drivers::fs::init::FILESYSTEM_TABLE;
use alloc::vec;

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

/// Load and execute the gallop user program from filesystem as a process
fn run_gallop() {
    info!("Loading gallop user program...");

    // Access filesystem - we need at least one filesystem to be available
    let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
    if fs_table.is_empty() {
        error!("No filesystem available to load gallop");
        return;
    }

    // Try to open the gallop file from the root of the filesystem
    let fs = &fs_table[0];
    let fd = fs.open("/gallop", 0);
    if fd < 0 {
        error!("Failed to open gallop: fd={}", fd);
        return;
    }

    // Read the ELF file into a buffer
    // Allocate a buffer large enough for the ELF (gallop is ~9KB)
    const MAX_ELF_SIZE: usize = 64 * 1024; // 64KB max
    let mut elf_buffer = vec![0u8; MAX_ELF_SIZE];

    let bytes_read = fs.read(fd, &mut elf_buffer, MAX_ELF_SIZE);
    fs.close(fd);

    if bytes_read <= 0 {
        error!("Failed to read gallop: bytes_read={}", bytes_read);
        return;
    }

    let elf_size = bytes_read as usize;
    info!("Loaded gallop ELF: {} bytes", elf_size);

    // Truncate buffer to actual size
    elf_buffer.truncate(elf_size);

    // Drop the filesystem lock
    drop(fs_table);

    // Load the program (parse ELF and set up page tables)
    let program = match exec::load_program(&elf_buffer) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to load program: {:?}", e);
            return;
        }
    };

    info!("Program loaded: entry=0x{:x}, stack=0x{:x}, cr3=0x{:x}",
          program.entry_point, program.stack_pointer, program.cr3);

    // Create a new process and initialize it with user context
    {
        let mut manager_lock = PROCESS_MANAGER.lock();
        let manager = manager_lock.get_mut().expect("ProcessManager not initialized");

        // Create a new process
        let proc = manager.new_proc();
        let proc_id = proc.lock().id();

        // Initialize the process with user-mode context
        proc.lock().init_user_context(
            program.entry_point,
            program.stack_pointer,
            program.cr3,
        );

        // Wake up the process (add to run queue)
        manager.wake_up(proc);

        info!("User process {} created, switching to it...", proc_id);

        // Update current process ID
        *proc::CURRENT_PROCESS_ID.lock() = proc_id;
    }

    // Switch from kernel process to user process
    // IMPORTANT: Lock must be dropped before calling switch_context because:
    // 1. switch_context doesn't return until the user process exits
    // 2. When user calls exit, sys_exit needs to acquire PROCESS_MANAGER lock
    // 3. If we hold the lock here, it would cause a deadlock
    let switch_ptrs = {
        let mut manager_lock = PROCESS_MANAGER.lock();
        let manager = manager_lock.get_mut().expect("ProcessManager not initialized");
        manager.prepare_switch()
    };
    // Lock is now dropped, safe to switch
    if let Some((next_ctx, current_ctx)) = switch_ptrs {
        unsafe { proc::do_switch_context(next_ctx, current_ctx); }
    }

    // When we return here, it means the user process has exited
    // and control has returned to the kernel
    info!("User process finished, returned to kernel");
}

fn initialize(fb_config: *mut FrameBufferConfig) {
    let fb_config_ref = unsafe { *fb_config };
    let resolution = fb_config_ref.resolution;
    unsafe { Graphics::initialize_instance(fb_config_ref) }
    let mut graphics_lock = RAW_GRAPHICS.lock();
    let graphics = graphics_lock.as_mut().unwrap();
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

    // initialize layer manager
    let mut layer_manager_lock = LAYER_MANAGER.lock();
    let layer_manager_ref = layer_manager_lock.deref_mut();
    *layer_manager_ref = Some(LayerManager::new(fb_config_ref));
    let layer_manager = layer_manager_ref.as_mut().unwrap();

    let bglayer_id = layer_manager
        .new_layer()
        .lock()
        .set_window(bgwindow)
        .move_absolute(Coord::new(0, 0))
        .id();

    let mouse_layer_id = layer_manager
        .new_layer()
        .lock()
        .set_window(mouse_window)
        .move_absolute(Coord::new(resolution.0 / 2, resolution.1 / 2))
        .id();

    MOUSE_CURSOR.lock().set_layer_id(mouse_layer_id);
    layer_manager.up_down(bglayer_id, LayerHeight::Height(0)).expect("failed to set bg layer height");
    layer_manager.up_down(mouse_layer_id, LayerHeight::Height(1)).expect("failed to set mouse layer height");
    layer_manager.draw();
}

extern "x86-interrupt" fn handler_xhci(_: InterruptStackFrame) {
    INTERRUPTION_QUEUE.lock().push(Message::InterruptXHCI);
    unsafe {
        notify_end_of_interrupt();
    }
}

extern "x86-interrupt" fn handler_lapic_timer(_: InterruptStackFrame) {
    // Save user CR3 and switch to kernel CR3
    // This is necessary because when interrupted from user mode, CR3 still points to user page table
    let user_cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            "mov cr3, {}",
            out(reg) user_cr3,
            in(reg) paging::KERNEL_CR3,
            options(nostack, preserves_flags)
        );
    }

    let should_switch = TIMER_MANAGER.lock().get_mut().unwrap().tick();

    unsafe {
        notify_end_of_interrupt();
    }

    // Handle process switching if timer says we should
    if should_switch {
        // Use try_lock to avoid deadlock if another context holds the lock
        // (e.g., run_gallop is setting up a process)
        // If we can't get the lock, skip process switching for this tick
        let switch_ptrs = {
            if let Some(mut manager_lock) = PROCESS_MANAGER.try_lock() {
                if let Some(manager) = manager_lock.get_mut() {
                    // Only switch if there are multiple processes
                    if manager.run_queue_len() > 1 {
                        manager.prepare_switch()
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                // Lock is held elsewhere, skip this tick
                None
            }
        };
        // Lock is dropped here

        // Perform context switch if needed
        // Note: When switch happens, this function won't return normally
        // Instead, control transfers to the next process
        if let Some((next_ctx, current_ctx)) = switch_ptrs {
            unsafe { proc::do_switch_context(next_ctx, current_ctx); }
            // If we somehow return here (shouldn't happen), restore CR3
        }
    }

    // Restore user CR3 before returning (only reached if no switch happened)
    unsafe {
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) user_cr3,
            options(nostack, preserves_flags)
        );
    }
}

extern "x86-interrupt" fn handler_page_fault(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    // Get the faulting address from CR2
    let faulting_address = paging::get_cr2();

    error!(
        "PAGE FAULT!\n  Faulting address: 0x{:016x}\n  Error code: {:?}\n  Stack frame: {:#?}",
        faulting_address, error_code, stack_frame
    );

    // Call the paging module's page fault handler
    paging::handle_page_fault(error_code.bits());
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
    initialize_filesystem();

    FILE_DESCRIPTOR_TABLE.lock().initialize();

    //set the IDT entry
    IDT.lock()[InterruptVector::Xhci as usize].set_handler_fn(handler_xhci);
    IDT.lock()[InterruptVector::LAPICTimer as usize].set_handler_fn(handler_lapic_timer);
    // Set up page fault handler
    IDT.lock().page_fault.set_handler_fn(handler_page_fault);
    // Set up syscall handler (int 0x80)
    // DPL must be 3 to allow user mode (Ring 3) to invoke int 0x80
    let syscall_handler_addr = interrupt::syscall_handler_asm as *const () as u64;
    debug!("syscall_handler_asm address: {:#x}", syscall_handler_addr);
    unsafe {
        IDT.lock()[InterruptVector::Syscall as usize]
            .set_handler_addr(VirtAddr::new(syscall_handler_addr))
            .set_privilege_level(PrivilegeLevel::Ring3);
    }
    unsafe {
        IDT.lock().load_unsafe();
    }
    INTERRUPTION_QUEUE
        .lock()
        .initialize(Message::NoInterruption);

    initialize_process_manager();

    // Load and execute the gallop user program
    run_gallop();

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
            Message::TimerTimeout { timeout: _, value } => {
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
