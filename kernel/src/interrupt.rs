use core::ptr::write_volatile;
use spin::Mutex;
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};
use x86_64::{PrivilegeLevel, VirtAddr};

use crate::drivers::timer::TIMER_MANAGER;
use crate::proc::{self, PROCESS_MANAGER};
use crate::{debug, debugcon_println, error, paging, Message, INTERRUPTION_QUEUE};

pub static IDT: Mutex<InterruptDescriptorTable> = Mutex::new(InterruptDescriptorTable::new());

#[repr(usize)]
pub enum InterruptVector {
    Xhci = 0x40,
    LAPICTimer = 0x41,
    Syscall = 0x80,
}

pub unsafe fn notify_end_of_interrupt() {
    let end_of_interrupt: *mut u32 = 0xfee000b0 as *mut u32;
    write_volatile(end_of_interrupt, 0);
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

pub extern "x86-interrupt" fn handler_divide_error(frame: InterruptStackFrame) {
    print_exception("DIVIDE ERROR (#DE, vec 0)", &frame);
    error!(
        "DIVIDE ERROR at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_overflow(frame: InterruptStackFrame) {
    print_exception("OVERFLOW (#OF, vec 4)", &frame);
    error!("OVERFLOW at RIP: {:#x}", frame.instruction_pointer.as_u64());
    loop {}
}

pub extern "x86-interrupt" fn handler_bound_range_exceeded(frame: InterruptStackFrame) {
    print_exception("BOUND RANGE EXCEEDED (#BR, vec 5)", &frame);
    error!(
        "BOUND RANGE EXCEEDED at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_invalid_opcode(frame: InterruptStackFrame) {
    print_exception("INVALID OPCODE (#UD, vec 6)", &frame);
    error!(
        "INVALID OPCODE at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_device_not_available(frame: InterruptStackFrame) {
    print_exception("DEVICE NOT AVAILABLE (#NM, vec 7)", &frame);
    error!(
        "DEVICE NOT AVAILABLE at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_double_fault(frame: InterruptStackFrame, code: u64) -> ! {
    print_exception_with_code("DOUBLE FAULT (#DF, vec 8)", &frame, code);
    error!(
        "DOUBLE FAULT at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_invalid_tss(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("INVALID TSS (#TS, vec 10)", &frame, code);
    error!(
        "INVALID TSS at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_segment_not_present(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("SEGMENT NOT PRESENT (#NP, vec 11)", &frame, code);
    error!(
        "SEGMENT NOT PRESENT at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_stack_segment_fault(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("STACK-SEGMENT FAULT (#SS, vec 12)", &frame, code);
    error!(
        "STACK-SEGMENT FAULT at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_general_protection_fault(
    frame: InterruptStackFrame,
    code: u64,
) {
    print_exception_with_code("GENERAL PROTECTION FAULT (#GP, vec 13)", &frame, code);
    error!(
        "GENERAL PROTECTION FAULT at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_x87_floating_point(frame: InterruptStackFrame) {
    print_exception("x87 FLOATING POINT (#MF, vec 16)", &frame);
    error!(
        "x87 FLOATING POINT at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_alignment_check(frame: InterruptStackFrame, code: u64) {
    print_exception_with_code("ALIGNMENT CHECK (#AC, vec 17)", &frame, code);
    error!(
        "ALIGNMENT CHECK at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_simd_floating_point(frame: InterruptStackFrame) {
    print_exception("SIMD FLOATING POINT (#XF, vec 19)", &frame);
    error!(
        "SIMD FLOATING POINT at RIP: {:#x}",
        frame.instruction_pointer.as_u64()
    );
    loop {}
}

pub extern "x86-interrupt" fn handler_page_fault(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let faulting_address = paging::get_cr2();
    print_exception_with_code("PAGE FAULT (#PF, vec 14)", &stack_frame, error_code.bits());
    debugcon_println!("  Faulting address: {:#018x}", faulting_address);
    debugcon_println!(
        "  Present:           {}",
        error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION)
    );
    debugcon_println!(
        "  Write:             {}",
        error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE)
    );
    debugcon_println!(
        "  User:              {}",
        error_code.contains(PageFaultErrorCode::USER_MODE)
    );
    debugcon_println!(
        "  Instruction fetch: {}",
        error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH)
    );

    error!(
        "PAGE FAULT at RIP: {:#x}, faulting addr: {:#x}, code: {:#x}",
        stack_frame.instruction_pointer.as_u64(),
        faulting_address,
        error_code.bits()
    );

    paging::handle_page_fault(error_code.bits());
}

// ── device / timer interrupt handlers ────────────────────────────────────────

pub extern "x86-interrupt" fn handler_xhci(_: InterruptStackFrame) {
    INTERRUPTION_QUEUE.lock().push(Message::InterruptXHCI);
    unsafe {
        notify_end_of_interrupt();
    }
}

pub extern "x86-interrupt" fn handler_lapic_timer(_: InterruptStackFrame) {
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
        PROCESS_MANAGER
            .try_lock()
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

// Assembly syscall handler
extern "C" {
    pub fn syscall_handler_asm();
}

pub fn initialize_idt() {
    IDT.lock().divide_error.set_handler_fn(handler_divide_error);
    IDT.lock().overflow.set_handler_fn(handler_overflow);
    IDT.lock()
        .bound_range_exceeded
        .set_handler_fn(handler_bound_range_exceeded);
    IDT.lock()
        .invalid_opcode
        .set_handler_fn(handler_invalid_opcode);
    IDT.lock()
        .device_not_available
        .set_handler_fn(handler_device_not_available);
    IDT.lock().double_fault.set_handler_fn(handler_double_fault);
    IDT.lock().invalid_tss.set_handler_fn(handler_invalid_tss);
    IDT.lock()
        .segment_not_present
        .set_handler_fn(handler_segment_not_present);
    IDT.lock()
        .stack_segment_fault
        .set_handler_fn(handler_stack_segment_fault);
    IDT.lock()
        .general_protection_fault
        .set_handler_fn(handler_general_protection_fault);
    IDT.lock()
        .x87_floating_point
        .set_handler_fn(handler_x87_floating_point);
    IDT.lock()
        .alignment_check
        .set_handler_fn(handler_alignment_check);
    IDT.lock()
        .simd_floating_point
        .set_handler_fn(handler_simd_floating_point);
    IDT.lock().page_fault.set_handler_fn(handler_page_fault);
    IDT.lock()[InterruptVector::Xhci as usize].set_handler_fn(handler_xhci);
    IDT.lock()[InterruptVector::LAPICTimer as usize].set_handler_fn(handler_lapic_timer);
    // Set up syscall handler (int 0x80)
    // DPL must be 3 to allow user mode (Ring 3) to invoke int 0x80
    let syscall_handler_addr = syscall_handler_asm as *const () as u64;
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
}
