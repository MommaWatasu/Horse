use super::user_mem::{copy_from_user, copy_to_user};
use super::SyscallError;
use crate::drivers::dev::fb::FrameBufferDevice;
use crate::drivers::dev::null::NullDevice;
use crate::drivers::dev::stdin::StdinDevice;
use crate::drivers::dev::stdout::{StderrDevice, StdoutDevice};
use crate::drivers::dev::zero::ZeroDevice;
use crate::drivers::fs::init::FILESYSTEM_TABLE;
use crate::drivers::fs::regular::RegularFile;
use crate::horse_lib::fd::Path;
use crate::proc::PROCESS_MANAGER;
use alloc::{sync::Arc, vec};
use horse_abi::ioctl::IoctlRequest;

/// sys_open - Open a file or device
///
/// Routes /dev/* paths to the appropriate device, all others to FAT32.
///
/// # Returns
/// * File descriptor on success (>= 0)
/// * Negative error code on failure
pub fn sys_open(pathname: *const u8, len: usize, flags: u32) -> isize {
    if pathname.is_null() || len == 0 || len > 4096 {
        return SyscallError::InvalidArg as isize;
    }

    // Copy the path from user space via USER_CR3 translation, so this works
    // regardless of whether the user's .rodata is identity-mapped or not.
    let mut path_buf = alloc::vec![0u8; len];
    unsafe {
        copy_from_user(path_buf.as_mut_ptr(), pathname, len);
    }
    let path_str = match core::str::from_utf8(&path_buf[..]) {
        Ok(s) => s,
        Err(_) => return SyscallError::InvalidArg as isize,
    };

    // Route /dev/* to device files
    if let Some(dev_name) = path_str.strip_prefix("/dev/") {
        let fd_entry: Arc<dyn crate::horse_lib::fd::FileDescriptor> = match dev_name {
            "null" => Arc::new(NullDevice),
            "zero" => Arc::new(ZeroDevice),
            "stdin" => Arc::new(StdinDevice),
            "stdout" => Arc::new(StdoutDevice),
            "stderr" => Arc::new(StderrDevice),
            "fb" => Arc::new(FrameBufferDevice),
            _ => return SyscallError::NoEntry as isize,
        };
        let fd = {
            let proc_manager = PROCESS_MANAGER.lock();
            let proc = proc_manager
                .get()
                .expect("failed to get process manager")
                .current_proc();
            let mut proc_guard = proc.lock();
            proc_guard.fd_table.add(fd_entry)
        };
        if fd < 0 {
            return SyscallError::IoError as isize;
        }
        return fd as isize;
    }

    // Regular file: verify existence then register
    let path = Path::new(alloc::string::String::from(path_str));
    let exists = {
        let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
        !fs_table.is_empty() && fs_table[0].exists(&path)
    };
    if !exists {
        return SyscallError::NoEntry as isize;
    }

    let file = Arc::new(RegularFile::new(flags, path_str));
    let fd = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let mut proc_guard = proc.lock();
        proc_guard.fd_table.add(file)
    };
    if fd < 0 {
        return SyscallError::IoError as isize;
    }
    fd as isize
}

/// sys_read - Read from a file descriptor
///
/// # Returns
/// * Number of bytes read on success (>= 0)
/// * Negative error code on failure
pub fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize {
    if buf.is_null() || count == 0 {
        return SyscallError::InvalidArg as isize;
    }
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }

    // Clone the Arc out of the process fd_table, then drop all spin locks
    // before calling read() which may block.
    let fd_entry = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        match proc_guard.fd_table.get(fd) {
            Some(e) => e,
            None => match proc_guard.fd_table.get_socket(fd) {
                Some(e) => e,
                None => return SyscallError::InvalidFd as isize,
            },
        }
    }; // proc_guard, proc, and proc_manager locks all dropped here

    let mut tmp = vec![0u8; count];
    let bytes_read = fd_entry.read(&mut tmp);
    if bytes_read > 0 {
        unsafe {
            copy_to_user(buf, tmp.as_ptr(), bytes_read as usize);
        }
    }
    bytes_read
}

/// sys_write - Write to a file descriptor
///
/// # Returns
/// * Number of bytes written on success (>= 0)
/// * Negative error code on failure
pub fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    if buf.is_null() || count == 0 {
        return SyscallError::InvalidArg as isize;
    }
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }

    let fd_entry = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        match proc_guard.fd_table.get(fd) {
            Some(e) => e,
            None => match proc_guard.fd_table.get_socket(fd) {
                Some(e) => e,
                None => return SyscallError::InvalidFd as isize,
            },
        }
    };

    // Copy user buffer to kernel memory first, because the user buffer may be
    // on the stack (arbitrary physical address, not accessible via identity map).
    let mut tmp = vec![0u8; count];
    unsafe {
        copy_from_user(tmp.as_mut_ptr(), buf, count);
    }
    fd_entry.write(&tmp)
}

/// sys_close - Close a file descriptor
///
/// # Returns
/// * 0 on success
/// * Negative error code on failure
pub fn sys_close(fd: i32) -> isize {
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }
    let valid = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        proc_guard.fd_table.get(fd).is_some()
    };
    if !valid {
        return SyscallError::InvalidFd as isize;
    }
    {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let mut proc_guard = proc.lock();
        proc_guard.fd_table.remove(fd);
    }
    0
}

pub fn sys_ioctl(fd: i32, request: u64, arg: u64) -> isize {
    let req = match IoctlRequest::try_from(request) {
        Ok(req) => req,
        Err(_) => return SyscallError::InvalidArg as isize,
    };

    let fd_entry = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        match proc_guard.fd_table.get(fd) {
            Some(e) => e,
            None => return SyscallError::InvalidFd as isize,
        }
    };

    fd_entry.ioctl(req, arg)
}
