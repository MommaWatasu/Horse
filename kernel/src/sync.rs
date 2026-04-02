use alloc::vec::Vec;

use crate::proc::{do_switch_context, PROCESS_MANAGER};

pub struct WaitQueue {
    waiters: Vec<usize>,
}

impl WaitQueue {
    pub fn new() -> Self {
        Self {
            waiters: Vec::new()
        }
    }

    pub fn wait(&mut self) {
        let current_id = {
            let proc_manager = PROCESS_MANAGER.lock();
            let proc = proc_manager.get().expect("failed to get process manager").current_proc();
            let proc_guard = proc.lock();
            proc_guard.id()
        };
        self.waiters.push(current_id);
        let switch_ptrs = {
            let mut proc_manager = PROCESS_MANAGER.lock();
            proc_manager.get_mut().expect("failed to get process manager lock").prepare_sleep()
        };

        if let Some((next_ctx, current_ctx, next_kstack_top)) = switch_ptrs {
            unsafe { do_switch_context(next_ctx, current_ctx, next_kstack_top); }
        }
    }

    pub fn  wake(&mut self) {
        if let Some(id) = self.waiters.pop() {
            let mut proc_manager = PROCESS_MANAGER.lock();
            proc_manager.get_mut().expect("failed to get process manager lock").id_wake_up(id);
        }
    }
}