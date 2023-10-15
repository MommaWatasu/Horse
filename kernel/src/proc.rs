use core::{arch::asm, sync::atomic::{
    Ordering,
    AtomicPtr
}};
use spin::Mutex;
use x86_64::instructions::interrupts::enable;

use crate::drivers::timer::TIMER_MANAGER;

const DEFAULT_CONTEXT: ContextWrapper = ContextWrapper(ProcessContext { cr3: 0, rip: 0, rflags: 0, reserved1: 0, cs: 0, ss: 0, fs: 0, gs: 0, rax: 0, rbx: 0, rcx: 0, rdx: 0, rdi: 0, rsi: 0, rsp: 0, rbp: 0, r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0, fxsave_area: [0; 512] });
pub static mut TASK_A_CONTEXT: ContextWrapper = DEFAULT_CONTEXT;
pub static mut TASK_B_CONTEXT: ContextWrapper = DEFAULT_CONTEXT;
pub static mut CURRENT_PROCESS: u64 = 0;

extern "C" {
    pub fn switch_context(next_ctx: u64, current_ctx: u64);
    pub fn get_cr3() -> u64;
}


#[derive(Clone, Copy)]
#[repr(align(16))]
pub struct ContextWrapper(ProcessContext);

impl ContextWrapper {
    pub fn unwrap(&mut self) -> &mut ProcessContext {
        return &mut self.0
    }
    pub fn as_ptr(&mut self) -> u64 {
        return self.unwrap() as *mut ProcessContext as u64
    }
    pub fn from_ptr(ptr: u64) -> ProcessContext {
        return unsafe { *(ptr as *mut ProcessContext) }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(C, packed)]
pub struct ProcessContext {
    pub cr3: u64,
    pub rip: u64,
    pub rflags: u64,
    reserved1: u64,
    pub cs: u64,
    pub ss: u64,
    fs: u64,
    gs: u64,
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rsp: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    pub fxsave_area: [u8; 512]
}

pub fn initialize_process_manager() {
    unsafe { CURRENT_PROCESS = TASK_A_CONTEXT.as_ptr() };

    unsafe {
        asm!("cli");
        TIMER_MANAGER.lock().get_mut().unwrap().add_timer(10, -1, true);
        asm!("sti");
    }
}

pub unsafe fn switch_process() {
    let old_process = CURRENT_PROCESS;
    let task_a = TASK_A_CONTEXT.as_ptr();
    if old_process == task_a {
        CURRENT_PROCESS = TASK_B_CONTEXT.as_ptr();
    } else {
        CURRENT_PROCESS = TASK_A_CONTEXT.as_ptr();
    }
    switch_context(CURRENT_PROCESS, old_process)
}

pub fn taskb() {
    let mut count: u64 = 0;
    loop {
        crate::println!("TaskB is running! - count: {}", count);
        count += 1;
    }
}