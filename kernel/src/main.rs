#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(core_intrinsics)]
#![feature(never_type)]

mod acpi;
mod ascii_font;
pub mod debugcon;
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
pub mod socket;
pub mod sync;
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
    // Read the ELF file into a buffer
    // Allocate a buffer large enough for the ELF (gallop is ~9KB)
    const MAX_ELF_SIZE: usize = 64 * 1024; // 64KB max
    let mut elf_buffer = vec![0u8; MAX_ELF_SIZE];

    let gallop_path = crate::horse_lib::fd::Path::new(alloc::string::String::from("gallop"));
    let bytes_read = {
        let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
        if fs_table.is_empty() {
            error!("No filesystem available to load gallop");
            return;
        }
        fs_table[0].read_file(&gallop_path, &mut elf_buffer, MAX_ELF_SIZE)
    };

    if bytes_read <= 0 {
        error!("Failed to read gallop: bytes_read={}", bytes_read);
        return;
    }

    let elf_size = bytes_read as usize;
    info!("Loaded gallop ELF: {} bytes", elf_size);

    // Truncate buffer to actual size
    elf_buffer.truncate(elf_size);

    // Load the program (parse ELF and set up page tables)
    let program = match exec::load_program(&elf_buffer[..]) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to load program: {:?}", e);
            return;
        }
    };

    info!("Program loaded: entry=0x{:x}, stack=0x{:x}, cr3=0x{:x}",
          program.entry_point, program.stack_pointer, program.cr3);


    // disable interrupts before manipulating process manager
    disable();

    {
        // Build a parent fd_table with stdin/stdout/stderr for the child to inherit
        let mut parent_fds = crate::horse_lib::fd::FDTable::new();
        parent_fds.add(Arc::new(crate::drivers::dev::stdin::StdinDevice));
        parent_fds.add(Arc::new(crate::drivers::dev::stdout::StdoutDevice));
        parent_fds.add(Arc::new(crate::drivers::dev::stdout::StderrDevice));

        // Create a new process
        let mut manager_lock = PROCESS_MANAGER.lock();
        let manager = manager_lock.get_mut().expect("ProcessManager not initialized");
        let proc = manager.new_proc(&parent_fds);

        // Initialize the process with user-mode context
        proc.lock().init_user_context(
            program.entry_point,
            program.stack_pointer,
            program.cr3,
        );

        // Wake up the process
        manager_lock.get_mut().unwrap().wake_up(proc);
    }

    // re-enable interrupts
    enable();
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

// ── exception helpers ────────────────────────────────────────────────────────

fn print_exception(name: &str, frame: &InterruptStackFrame) {
    debugcon_println!("\n=== {} ===", name);
    debugcon_println!("RIP:    {:#018x}", frame.instruction_pointer.as_u64());
    debugcon_println!("RSP:    {:#018x}", frame.stack_pointer.as_u64());
    debugcon_println!("RFLAGS: {:#018x}", frame.cpu_flags);
    debugcon_println!("CS:     {:#018x}", frame.code_segment as u64);
    debugcon_println!("SS:     {:#018x}", frame.stack_segment as u64);
    debugcon_println!("==================");
}

fn print_exception_with_code(name: &str, frame: &InterruptStackFrame, code: u64) {
    debugcon_println!("\n=== {} ===", name);
    debugcon_println!("Error code: {:#018x}", code);
    debugcon_println!("RIP:    {:#018x}", frame.instruction_pointer.as_u64());
    debugcon_println!("RSP:    {:#018x}", frame.stack_pointer.as_u64());
    debugcon_println!("RFLAGS: {:#018x}", frame.cpu_flags);
    debugcon_println!("CS:     {:#018x}", frame.code_segment as u64);
    debugcon_println!("SS:     {:#018x}", frame.stack_segment as u64);
    debugcon_println!("==================");
}

// ── CPU exception handlers ───────────────────────────────────────────────────

extern "x86-interrupt" fn handler_divide_error(frame: InterruptStackFrame) {
    print_exception("DIVIDE ERROR (#DE, vec 0)", &frame);
    error!("DIVIDE ERROR at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_overflow(frame: InterruptStackFrame) {
    print_exception("OVERFLOW (#OF, vec 4)", &frame);
    error!("OVERFLOW at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_bound_range_exceeded(frame: InterruptStackFrame) {
    print_exception("BOUND RANGE EXCEEDED (#BR, vec 5)", &frame);
    error!("BOUND RANGE EXCEEDED at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_invalid_opcode(frame: InterruptStackFrame) {
    print_exception("INVALID OPCODE (#UD, vec 6)", &frame);
    error!("INVALID OPCODE at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_device_not_available(frame: InterruptStackFrame) {
    print_exception("DEVICE NOT AVAILABLE (#NM, vec 7)", &frame);
    error!("DEVICE NOT AVAILABLE at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_double_fault(frame: InterruptStackFrame, code: u64) -> ! {
    print_exception_with_code("DOUBLE FAULT (#DF, vec 8)", &frame, code);
    error!("DOUBLE FAULT at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_invalid_tss(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("INVALID TSS (#TS, vec 10)", &frame, code);
    error!("INVALID TSS at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_segment_not_present(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("SEGMENT NOT PRESENT (#NP, vec 11)", &frame, code);
    error!("SEGMENT NOT PRESENT at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_stack_segment_fault(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("STACK-SEGMENT FAULT (#SS, vec 12)", &frame, code);
    error!("STACK-SEGMENT FAULT at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_general_protection_fault(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("GENERAL PROTECTION FAULT (#GP, vec 13)", &frame, code);
    error!("GENERAL PROTECTION FAULT at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_x87_floating_point(frame: InterruptStackFrame) {
    print_exception("x87 FLOATING POINT (#MF, vec 16)", &frame);
    error!("x87 FLOATING POINT at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_alignment_check(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("ALIGNMENT CHECK (#AC, vec 17)", &frame, code);
    error!("ALIGNMENT CHECK at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_simd_floating_point(frame: InterruptStackFrame) {
    print_exception("SIMD FLOATING POINT (#XF, vec 19)", &frame);
    error!("SIMD FLOATING POINT at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

extern "x86-interrupt" fn handler_page_fault(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let faulting_address = paging::get_cr2();
    print_exception_with_code("PAGE FAULT (#PF, vec 14)", &stack_frame, error_code.bits());
    debugcon_println!("  Faulting address: {:#018x}", faulting_address);
    debugcon_println!("  Present:           {}", error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION));
    debugcon_println!("  Write:             {}", error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE));
    debugcon_println!("  User:              {}", error_code.contains(PageFaultErrorCode::USER_MODE));
    debugcon_println!("  Instruction fetch: {}", error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH));

    error!("PAGE FAULT at RIP: {:#x}, faulting addr: {:#x}, code: {:#x}",
        stack_frame.instruction_pointer.as_u64(), faulting_address, error_code.bits());

    paging::handle_page_fault(error_code.bits());
}

// ── device / timer interrupt handlers ────────────────────────────────────────

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
    
    // Get context pointers while holding the lock, then release it before switch_context
    // This is critical because switch_context doesn't return (it uses iretq),
    // so any locks held would never be released, causing deadlock.
    let switch_contexts = if should_switch {
        // Use try_lock to avoid deadlock: sys_exit holds PROCESS_MANAGER while calling
        // prepare_terminate, which does screen I/O. If the timer fires during that window
        // and uses .lock() here, it spins forever. Skip this tick if the lock is busy.
        PROCESS_MANAGER.try_lock()
            .and_then(|mut guard| guard.get_mut().and_then(|m| m.prepare_switch()))
    } else {
        None
    };

    unsafe {
        notify_end_of_interrupt();
        
        if let Some((next_ctx, current_ctx, next_kstack_top)) = switch_contexts {
            // Lock is already released here, safe to call switch_context
            proc::do_switch_context(next_ctx, current_ctx, next_kstack_top);
        }
    }
    
    // Restore user CR3 before returning
    unsafe {
        core::arch::asm!(
            "mov cr3, {}",
            in(reg) user_cr3,
            options(nostack, preserves_flags)
        );
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
    initialize_filesystem();

    //set the IDT entry
    IDT.lock().divide_error.set_handler_fn(handler_divide_error);
    IDT.lock().overflow.set_handler_fn(handler_overflow);
    IDT.lock().bound_range_exceeded.set_handler_fn(handler_bound_range_exceeded);
    IDT.lock().invalid_opcode.set_handler_fn(handler_invalid_opcode);
    IDT.lock().device_not_available.set_handler_fn(handler_device_not_available);
    IDT.lock().double_fault.set_handler_fn(handler_double_fault);
    IDT.lock().invalid_tss.set_handler_fn(handler_invalid_tss);
    IDT.lock().segment_not_present.set_handler_fn(handler_segment_not_present);
    IDT.lock().stack_segment_fault.set_handler_fn(handler_stack_segment_fault);
    IDT.lock().general_protection_fault.set_handler_fn(handler_general_protection_fault);
    IDT.lock().x87_floating_point.set_handler_fn(handler_x87_floating_point);
    IDT.lock().alignment_check.set_handler_fn(handler_alignment_check);
    IDT.lock().simd_floating_point.set_handler_fn(handler_simd_floating_point);
    IDT.lock().page_fault.set_handler_fn(handler_page_fault);
    IDT.lock()[InterruptVector::Xhci as usize].set_handler_fn(handler_xhci);
    IDT.lock()[InterruptVector::LAPICTimer as usize].set_handler_fn(handler_lapic_timer);
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
