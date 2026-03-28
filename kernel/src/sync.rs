use alloc::vec::Vec;

use crate::proc::{do_switch_context, PROCESS_MANAGER, CURRENT_PROCESS_ID};

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
        let current_id = *CURRENT_PROCESS_ID.lock();
        self.waiters.push(current_id);
        let switch_ptrs = {
            let mut proc_manager = PROCESS_MANAGER.lock();
            proc_manager.get_mut().expect("failed to get process manager lock").prepare_sleep()
        };

        if let Some((next_ctx, current_ctx)) = switch_ptrs {
            unsafe { do_switch_context(next_ctx, current_ctx); }
        }
    }

    pub fn  wake(&mut self) {
        if let Some(id) = self.waiters.pop() {
            let mut proc_manager = PROCESS_MANAGER.lock();
            proc_manager.get_mut().expect("failed to get process manager lock").id_wake_up(id);
        }
    }
}