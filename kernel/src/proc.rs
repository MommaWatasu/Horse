use alloc::sync::Arc;
use alloc::{
    collections::VecDeque,
    vec::Vec,
};
use core::{arch::asm, mem::size_of};
use spin::{
    Mutex,
    Once
};
use core::cell::{Cell, RefCell, Ref};

use crate::{drivers::timer::TIMER_MANAGER, segment::{KERNEL_CS, KERNEL_SS}};

const DEFAULT_CONTEXT: ContextWrapper = ContextWrapper(ProcessContext { cr3: 0, rip: 0, rflags: 0, reserved1: 0, cs: 0, ss: 0, fs: 0, gs: 0, rax: 0, rbx: 0, rcx: 0, rdx: 0, rdi: 0, rsi: 0, rsp: 0, rbp: 0, r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0, fxsave_area: [0; 512] });
pub static mut PROCESS_MANAGER: Once<ProcessManager> = Once::new();

extern "C" {
    pub fn switch_context(next_ctx: u64, current_ctx: u64);
    pub fn get_cr3() -> u64;
}


#[derive(Clone, Copy, Eq, PartialEq)]
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

#[derive(Clone, Copy, Eq, PartialEq)]
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

pub struct ProcessManager {
    latest_id: usize,
    run_queue: VecDeque<Arc<RefCell<Process>>>,
    pending_queue: Vec<Arc<RefCell<Process>>>
}

impl ProcessManager {
    pub fn new() -> Self {
        let mut manager = Self {
            latest_id: 0,
            run_queue: VecDeque::new(),
            pending_queue: Vec::new(),
        };
        manager.new_proc();
        manager.id_wake_up(1);
        return manager
    }
    pub fn new_proc(&mut self) -> Arc<RefCell<Process>> {
        self.latest_id += 1;
        let proc = Arc::new(RefCell::new(Process::new(self.latest_id)));
        self.pending_queue.push(proc.clone());
        return proc
    }
    pub fn wake_up(&mut self, proc: Arc<RefCell<Process>>) {
        if let Some(idx) = self.pending_queue.iter().position(|x| *x == proc) {
            self.run_queue.push_back(proc);
            self.pending_queue.remove(idx);
        }
    }
    pub fn id_wake_up(&mut self, id: usize) {
        if let Some(idx) = self.pending_queue.iter().position(|x| x.borrow().id() == id) {
            self.run_queue.push_back(self.pending_queue[idx].clone());
            self.pending_queue.remove(idx);
        }
    }
    pub fn sleep(&mut self, proc: Arc<RefCell<Process>>) {
        if let Some(idx) = self.run_queue.iter().position(|x| *x == proc) {
            if idx == 0 {
                self.switch_process(true);
            } else {
                self.run_queue.remove(idx);
                self.pending_queue.push(proc);
            }
        }
    }
    pub fn id_sleep(&mut self, id: usize) {
        if let Some(idx) = self.run_queue.iter().position(|x| x.borrow().id() == id) {
            self.pending_queue.push(self.run_queue[idx].clone());
            self.run_queue.remove(idx);
            if idx == 0 {
                self.switch_process(true);
            }
        }
    }
    pub fn switch_process(&mut self, sleep: bool) {
        let current_proc = self.run_queue.pop_front().unwrap();
        let current_proc_ptr = current_proc.borrow_mut().context().as_ptr();
        if !sleep {
            self.run_queue.push_back(current_proc)
        }
        let next_proc = self.run_queue.front_mut().unwrap();

        unsafe { switch_context(next_proc.borrow_mut().context().as_ptr(), current_proc_ptr) }
    }
}

#[derive(Eq, PartialEq)]
pub struct Process {
    id: usize,
    stack: Vec<u64>,
    context: ContextWrapper
}

impl Process {
    const DEFAULT_STACK_BYTES: usize = 4096;
    pub fn new(id: usize) -> Self {
        return Self {
            id,
            stack: Vec::new(),
            context: DEFAULT_CONTEXT
        }
    }
    pub fn id(&self) -> usize { self.id }
    pub fn init_context(&mut self, f: fn()) {
        let stack_size = Self::DEFAULT_STACK_BYTES / size_of::<u64>();
        self.stack.resize(stack_size, 0);
        let stack_end = self.stack.as_ptr() as u64 + stack_size as u64;

        let ctx = self.context.unwrap();
        ctx.cr3 = unsafe { get_cr3() };
        ctx.rflags = 0x202;
        ctx.cs = KERNEL_CS as u64;
        ctx.ss = KERNEL_SS as u64;
        ctx.rsp = (stack_end & !0xfu64) - 8;
        ctx.rip = f as *const () as u64;
        
        ctx.fxsave_area[25] = 0x8;
        ctx.fxsave_area[26] = 0xf;
        ctx.fxsave_area[27] = 0x1;
    }
    pub fn context(&mut self) -> &mut ContextWrapper {
        return &mut self.context
    }
}

pub fn initialize_process_manager() {
    unsafe { PROCESS_MANAGER.call_once(|| ProcessManager::new()); }

    unsafe {
        asm!("cli");
        TIMER_MANAGER.lock().get_mut().unwrap().add_timer(2, -1, true);
        asm!("sti");
    }
}

pub fn taskb() {
    let mut count: u64 = 0;
    loop {
        crate::println!("TaskB is running! - count: {}", count);
        count += 1;
    }
}