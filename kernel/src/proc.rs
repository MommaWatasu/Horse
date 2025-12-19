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

use crate::{drivers::timer::TIMER_MANAGER, segment::{KERNEL_CS, KERNEL_SS, USER_CS, USER_SS}};

const DEFAULT_CONTEXT: ContextWrapper = ContextWrapper(ProcessContext { cr3: 0, rip: 0, rflags: 0, reserved1: 0, cs: 0, ss: 0, fs: 0, gs: 0, rax: 0, rbx: 0, rcx: 0, rdx: 0, rdi: 0, rsi: 0, rsp: 0, rbp: 0, r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0, fxsave_area: [0; 512] });
pub static PROCESS_MANAGER: Mutex<Once<ProcessManager>> = Mutex::new(Once::new());

/// Current running process ID (0 means no user process)
pub static CURRENT_PROCESS_ID: Mutex<usize> = Mutex::new(0);

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
    run_queue: VecDeque<Arc<Mutex<Process>>>,
    pending_queue: Vec<Arc<Mutex<Process>>>
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
    pub fn new_proc(&mut self) -> Arc<Mutex<Process>> {
        self.latest_id += 1;
        let proc = Arc::new(Mutex::new(Process::new(self.latest_id)));
        self.pending_queue.push(proc.clone());
        return proc
    }
    pub fn wake_up(&mut self, proc: Arc<Mutex<Process>>) {
        if let Some(idx) = self.pending_queue.iter().position(|x| Arc::ptr_eq(x, &proc)) {
            self.run_queue.push_back(proc);
            self.pending_queue.remove(idx);
        }
    }
    pub fn id_wake_up(&mut self, id: usize) {
        if let Some(idx) = self.pending_queue.iter().position(|x| x.lock().id() == id) {
            self.run_queue.push_back(self.pending_queue[idx].clone());
            self.pending_queue.remove(idx);
        }
    }
    pub fn sleep(&mut self, proc: Arc<Mutex<Process>>) {
        if let Some(idx) = self.run_queue.iter().position(|x| Arc::ptr_eq(x, &proc)) {
            if idx == 0 {
                self.switch_process(true);
            } else {
                self.run_queue.remove(idx);
                self.pending_queue.push(proc);
            }
        }
    }
    pub fn id_sleep(&mut self, id: usize) {
        if let Some(idx) = self.run_queue.iter().position(|x| x.lock().id() == id) {
            self.pending_queue.push(self.run_queue[idx].clone());
            self.run_queue.remove(idx);
            if idx == 0 {
                self.switch_process(true);
            }
        }
    }
    pub fn switch_process(&mut self, sleep: bool) {
        let current_proc = self.run_queue.pop_front().unwrap();
        let current_proc_ptr = current_proc.lock().context().as_ptr();
        if !sleep {
            self.run_queue.push_back(current_proc)
        }
        let next_proc = self.run_queue.front_mut().unwrap();

        unsafe { switch_context(next_proc.lock().context().as_ptr(), current_proc_ptr) }
    }

    /// Terminate the current process and switch to the next one
    /// Returns the exit status if there was a process to terminate
    pub fn terminate_current(&mut self, _exit_status: i32) -> Option<i32> {
        if self.run_queue.is_empty() {
            return None;
        }

        // Remove the current process from the run queue
        let current_proc = self.run_queue.pop_front().unwrap();
        let current_id = current_proc.lock().id();

        crate::info!("Process {} terminated", current_id);

        // If there are no more processes, return
        if self.run_queue.is_empty() {
            crate::info!("No more processes in run queue");
            *CURRENT_PROCESS_ID.lock() = 0;
            return Some(_exit_status);
        }

        // Switch to the next process
        let current_proc_ptr = current_proc.lock().context().as_ptr();
        let next_proc = self.run_queue.front_mut().unwrap();
        let next_id = next_proc.lock().id();
        *CURRENT_PROCESS_ID.lock() = next_id;

        crate::info!("Switching to process {}", next_id);
        unsafe { switch_context(next_proc.lock().context().as_ptr(), current_proc_ptr) }

        Some(_exit_status)
    }

    /// Get the ID of the current running process
    pub fn current_id(&self) -> Option<usize> {
        self.run_queue.front().map(|p| p.lock().id())
    }

    /// Get the number of processes in the run queue
    pub fn run_queue_len(&self) -> usize {
        self.run_queue.len()
    }

    /// Prepare for context switch and return the context pointers
    /// This rotates the run queue (moves current to back) but doesn't switch
    /// Returns (next_context_ptr, current_context_ptr) for use with switch_context
    pub fn prepare_switch(&mut self) -> Option<(u64, u64)> {
        if self.run_queue.len() < 2 {
            return None;
        }

        let current_proc = self.run_queue.pop_front().unwrap();
        let current_proc_ptr = current_proc.lock().context().as_ptr();
        self.run_queue.push_back(current_proc);

        let next_proc = self.run_queue.front_mut().unwrap();
        let next_proc_ptr = next_proc.lock().context().as_ptr();

        Some((next_proc_ptr, current_proc_ptr))
    }

    /// Prepare to terminate current process and switch to next
    /// Returns (next_context_ptr, current_context_ptr) or None if no other processes
    pub fn prepare_terminate(&mut self, exit_status: i32) -> Option<(u64, u64)> {
        if self.run_queue.is_empty() {
            return None;
        }

        // Remove the current process from the run queue
        let current_proc = self.run_queue.pop_front().unwrap();
        let current_id = current_proc.lock().id();
        let current_proc_ptr = current_proc.lock().context().as_ptr();

        crate::info!("Process {} terminated with status {}", current_id, exit_status);

        // If there are no more processes, return None
        if self.run_queue.is_empty() {
            crate::info!("No more processes in run queue");
            *CURRENT_PROCESS_ID.lock() = 0;
            return None;
        }

        // Get the next process context
        let next_proc = self.run_queue.front_mut().unwrap();
        let next_id = next_proc.lock().id();
        let next_proc_ptr = next_proc.lock().context().as_ptr();
        *CURRENT_PROCESS_ID.lock() = next_id;

        crate::info!("Will switch to process {}", next_id);
        Some((next_proc_ptr, current_proc_ptr))
    }
}

/// Perform context switch after dropping the ProcessManager lock
/// This is safe to call after prepare_switch or prepare_terminate
pub unsafe fn do_switch_context(next_ctx: u64, current_ctx: u64) {
    switch_context(next_ctx, current_ctx);
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

    /// Initialize context for a user-mode process
    ///
    /// # Arguments
    /// * `entry_point` - Entry point address in user space
    /// * `user_stack` - User stack pointer (top of stack)
    /// * `cr3` - Page table physical address for this process
    pub fn init_user_context(&mut self, entry_point: u64, user_stack: u64, cr3: u64) {
        // Allocate kernel stack for this process (used during syscalls/interrupts)
        let stack_size = Self::DEFAULT_STACK_BYTES / size_of::<u64>();
        self.stack.resize(stack_size, 0);

        let ctx = self.context.unwrap();
        ctx.cr3 = cr3;
        ctx.rflags = 0x202;  // IF=1, reserved bit 1=1
        ctx.cs = USER_CS as u64;
        ctx.ss = USER_SS as u64;
        ctx.rsp = user_stack & !0xFu64;  // 16-byte aligned
        ctx.rip = entry_point;

        // Initialize FPU state
        ctx.fxsave_area[25] = 0x8;
        ctx.fxsave_area[26] = 0xf;
        ctx.fxsave_area[27] = 0x1;
    }

    pub fn context(&mut self) -> &mut ContextWrapper {
        return &mut self.context
    }
}

pub fn initialize_process_manager() {
    PROCESS_MANAGER.lock().call_once(|| ProcessManager::new());

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