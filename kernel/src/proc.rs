use spin::Mutex;

const DEFAULT_CONTEXT: ContextWrapper = ContextWrapper(ProcessContext { cr3: 0, rip: 0, rflags: 0, reserved1: 0, cs: 0, ss: 0, fs: 0, gs: 0, rax: 0, rbx: 0, rcx: 0, rdx: 0, rdi: 0, rsi: 0, rsp: 0, rbp: 0, r8: 0, r9: 0, r10: 0, r11: 0, r12: 0, r13: 0, r14: 0, r15: 0, fxsave_area: [0; 512] });
pub static TASK_A_CONTEXT: Mutex<ContextWrapper> = Mutex::new(DEFAULT_CONTEXT);
pub static TASK_B_CONTEXT: Mutex<ContextWrapper> = Mutex::new(DEFAULT_CONTEXT);

extern "C" {
    pub fn switch_context(next_ctx: &mut ProcessContext, current_ctx: &mut ProcessContext);
    pub fn get_cr3() -> u64;
}


#[repr(align(16))]
pub struct ContextWrapper(ProcessContext);

impl ContextWrapper {
    pub fn unwrap(&mut self) -> &mut ProcessContext {
        return &mut self.0
    }
}

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

pub fn taskb() {
    let mut count: u64 = 0;
    loop {
        crate::println!("TaskB is running! - count: {}", count);
        count += 1;
        unsafe { switch_context(&mut *TASK_A_CONTEXT.lock().unwrap(), &mut *TASK_B_CONTEXT.lock().unwrap()); }
    }
}