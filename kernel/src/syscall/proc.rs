use super::SyscallError;
use super::user_mem::copy_from_user;
use crate::drivers::fs::init::FILESYSTEM_TABLE;
use crate::horse_lib::fd::{FDTable, Path};
use crate::proc::{do_switch_context, PROCESS_MANAGER};
use alloc::vec;

/// sys_exit - Terminate the current process
pub fn sys_exit(status: i32) -> isize {
    let (_current_proc_keeper, switch_ptrs) = {
        let mut manager_lock = PROCESS_MANAGER.lock();
        if let Some(manager) = manager_lock.get_mut() {
            manager.prepare_terminate(status)
        } else {
            (None, None)
        }
    };

    if let Some((next_ctx, current_ctx, next_kstack_top)) = switch_ptrs {
        unsafe {
            do_switch_context(next_ctx, current_ctx, next_kstack_top);
        }
    }

    0
}

/// sys_spawn - Load and execute an ELF binary as a new process
///
/// Inherits stdin (fd 0), stdout (fd 1), and stderr (fd 2) from the calling process.
///
/// # Arguments
/// * `path_ptr` — pointer to the path string in user space
/// * `path_len` — byte length of the path string
///
/// # Returns
/// * New process ID on success (> 0)
/// * Negative error code on failure
pub fn sys_spawn(path_ptr: *const u8, path_len: usize) -> isize {
    if path_ptr.is_null() || path_len == 0 || path_len > 4096 {
        return SyscallError::InvalidArg as isize;
    }

    // Copy path string from user space
    let mut path_buf = vec![0u8; path_len];
    unsafe {
        copy_from_user(path_buf.as_mut_ptr(), path_ptr, path_len);
    }
    let path_str = match core::str::from_utf8(&path_buf[..]) {
        Ok(s) => s,
        Err(_) => return SyscallError::InvalidArg as isize,
    };

    // Read ELF binary from filesystem
    const MAX_ELF_SIZE: usize = 64 * 1024;
    let mut elf_buffer = vec![0u8; MAX_ELF_SIZE];
    let path = Path::new(alloc::string::String::from(path_str));
    let bytes_read = {
        let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
        if fs_table.is_empty() {
            return SyscallError::IoError as isize;
        }
        fs_table[0].read_file(&path, &mut elf_buffer, MAX_ELF_SIZE)
    };

    if bytes_read <= 0 {
        return SyscallError::NoEntry as isize;
    }
    elf_buffer.truncate(bytes_read as usize);

    // Parse and load the ELF into memory
    let program = match crate::exec::load_program(&elf_buffer[..]) {
        Ok(p) => p,
        Err(_) => return SyscallError::IoError as isize,
    };

    // Clone stdin/stdout/stderr Arcs from the calling process
    let (fd0, fd1, fd2) = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        (
            proc_guard.fd_table.get(0),
            proc_guard.fd_table.get(1),
            proc_guard.fd_table.get(2),
        )
    };

    let mut parent_fds = FDTable::new();
    if let Some(f) = fd0 {
        parent_fds.add(f);
    }
    if let Some(f) = fd1 {
        parent_fds.add(f);
    }
    if let Some(f) = fd2 {
        parent_fds.add(f);
    }

    // Create the child process, initialize its context, and put it on the run queue
    let new_id = {
        let mut proc_manager = PROCESS_MANAGER.lock();
        let manager = proc_manager
            .get_mut()
            .expect("failed to get process manager");
        let proc = manager.new_proc(&parent_fds);
        let new_id = {
            let mut p = proc.lock();
            p.init_user_context(program.entry_point, program.stack_pointer, program.cr3);
            p.id()
        };
        manager.wake_up(proc);
        new_id
    };

    new_id as isize
}
