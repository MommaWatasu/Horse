//! System Call implementation for Horse OS
//!
//! This module provides the syscall interface for user-space applications.
//!
//! Syscall calling convention (x86-64):
//! - RAX: syscall number
//! - RDI: arg1
//! - RSI: arg2
//! - RDX: arg3
//! - R10: arg4
//! - R8:  arg5
//! - R9:  arg6
//! - Return value in RAX

use alloc::vec;
use crate::drivers::fs::init::FILESYSTEM_TABLE;
use crate::console::Console;
use crate::layer::LAYER_MANAGER;
use crate::proc::PROCESS_MANAGER;

/// System call numbers (Linux-compatible)
#[repr(usize)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallNumber {
    Read = 0,
    Write = 1,
    Open = 2,
    Close = 3,
    Exit = 60,
}

impl TryFrom<usize> for SyscallNumber {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(SyscallNumber::Read),
            1 => Ok(SyscallNumber::Write),
            2 => Ok(SyscallNumber::Open),
            3 => Ok(SyscallNumber::Close),
            60 => Ok(SyscallNumber::Exit),
            _ => Err(()),
        }
    }
}

/// Syscall error codes
#[repr(isize)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallError {
    InvalidSyscall = -1,
    InvalidFd = -9,      // EBADF
    InvalidArg = -22,    // EINVAL
    NoEnt = -2,          // ENOENT
    IoError = -5,        // EIO
}

/// Syscall arguments structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SyscallArgs {
    pub syscall_num: usize,
    pub arg1: usize,
    pub arg2: usize,
    pub arg3: usize,
    pub arg4: usize,
    pub arg5: usize,
    pub arg6: usize,
}

/// Main syscall dispatcher
///
/// Called from the syscall interrupt handler with saved register state
pub fn syscall_handler(args: &SyscallArgs) -> isize {
    let syscall_num = match SyscallNumber::try_from(args.syscall_num) {
        Ok(num) => num,
        Err(_) => return SyscallError::InvalidSyscall as isize,
    };

    match syscall_num {
        SyscallNumber::Read => sys_read(
            args.arg1 as i32,       // fd
            args.arg2 as *mut u8,   // buf
            args.arg3,              // count
        ),
        SyscallNumber::Write => sys_write(
            args.arg1 as i32,       // fd
            args.arg2 as *const u8, // buf
            args.arg3,              // count
        ),
        SyscallNumber::Open => sys_open(
            args.arg1 as *const u8, // pathname
            args.arg2 as u32,       // flags
        ),
        SyscallNumber::Close => sys_close(args.arg1 as i32),
        SyscallNumber::Exit => sys_exit(args.arg1 as i32),
    }
}

/// sys_open - Open a file
///
/// # Arguments
/// * `pathname` - Pointer to null-terminated pathname string
/// * `flags` - Open flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT)
///
/// # Returns
/// * File descriptor on success (>= 0)
/// * Negative error code on failure
pub fn sys_open(pathname: *const u8, flags: u32) -> isize {
    if pathname.is_null() {
        return SyscallError::InvalidArg as isize;
    }

    // Read the pathname from user space
    let path_str = unsafe {
        let mut len = 0;
        let mut ptr = pathname;
        while *ptr != 0 {
            len += 1;
            ptr = ptr.add(1);
            if len > 4096 {
                return SyscallError::InvalidArg as isize;
            }
        }
        let slice = core::slice::from_raw_parts(pathname, len);
        match core::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return SyscallError::InvalidArg as isize,
        }
    };

    // Use the filesystem to open the file
    // Safety: FILESYSTEM_TABLE is initialized once at boot
    let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
    if !fs_table.is_empty() {
        let fd = fs_table[0].open(path_str, flags);
        if fd < 0 {
            return SyscallError::NoEnt as isize;
        }
        return fd as isize;
    }

    SyscallError::IoError as isize
}

/// sys_read - Read from a file descriptor
///
/// # Arguments
/// * `fd` - File descriptor
/// * `buf` - Buffer to read into
/// * `count` - Number of bytes to read
///
/// # Returns
/// * Number of bytes read on success (>= 0)
/// * Negative error code on failure
pub fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize {
    if buf.is_null() || count == 0 {
        return SyscallError::InvalidArg as isize;
    }

    // Validate file descriptor
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }

    // Handle stdin (fd 0)
    if fd == 0 {
        // For now, stdin reading is not implemented
        // Could integrate with keyboard driver in the future
        return 0;
    }

    // For regular files, use filesystem
    // Safety: FILESYSTEM_TABLE is initialized once at boot
    let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
    if !fs_table.is_empty() {
        let mut temp_buf = vec![0u8; count];
        let bytes_read = fs_table[0].read(fd, &mut temp_buf, count);

        if bytes_read < 0 {
            return SyscallError::IoError as isize;
        }

        // Copy to user buffer
        unsafe {
            core::ptr::copy_nonoverlapping(temp_buf.as_ptr(), buf, bytes_read as usize);
        }

        return bytes_read;
    }

    SyscallError::IoError as isize
}

/// sys_write - Write to a file descriptor
///
/// # Arguments
/// * `fd` - File descriptor
/// * `buf` - Buffer to write from
/// * `count` - Number of bytes to write
///
/// # Returns
/// * Number of bytes written on success (>= 0)
/// * Negative error code on failure
pub fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    if buf.is_null() || count == 0 {
        return SyscallError::InvalidArg as isize;
    }

    // Validate file descriptor
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }

    // Handle stdout (fd 1) and stderr (fd 2)
    if fd == 1 || fd == 2 {
        // Note: buf points to user space, which should be accessible since
        // we copied PML4[0] (identity mapping) to user page tables
        let data = unsafe {
            core::slice::from_raw_parts(buf, count)
        };

        // Convert to string and print
        if let Ok(s) = core::str::from_utf8(data) {
            // Write to console (use block scope to release lock before draw)
            {
                let mut console = Console::instance();
                if let Some(ref mut con) = *console {
                    con.put_string(s);
                }
            }
            
            // Update screen
            LAYER_MANAGER.lock().as_mut().unwrap().draw();
            return count as isize;
        } else {
            // Write raw bytes as characters (convert each byte to a char string)
            {
                let mut console = Console::instance();
                if let Some(ref mut con) = *console {
                    for &byte in data {
                        // Create a temporary buffer for single character
                        let mut buf = [0u8; 4];
                        let c = byte as char;
                        if let Some(s) = c.encode_utf8(&mut buf).get(..c.len_utf8()) {
                            con.put_string(s);
                        }
                    }
                }
            }
            // Update screen
            LAYER_MANAGER.lock().as_mut().unwrap().draw();
            return count as isize;
        }
    }

    // For regular files, write is not yet implemented in FAT
    // Return error for now
    SyscallError::IoError as isize
}

/// sys_close - Close a file descriptor
///
/// # Arguments
/// * `fd` - File descriptor to close
///
/// # Returns
/// * 0 on success
/// * Negative error code on failure
pub fn sys_close(fd: i32) -> isize {
    // Validate file descriptor
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }

    // Don't close stdin, stdout, stderr
    if fd <= 2 {
        return 0;
    }

    // Use filesystem to close
    // Safety: FILESYSTEM_TABLE is initialized once at boot
    let fs_table = unsafe { FILESYSTEM_TABLE.lock() };
    if !fs_table.is_empty() {
        fs_table[0].close(fd);
        return 0;
    }

    SyscallError::IoError as isize
}

/// sys_exit - Terminate the current process
///
/// # Arguments
/// * `status` - Exit status code (0 for success, non-zero for error)
///
/// # Returns
/// * This function should not return to the calling process
/// * Returns 0 if process termination was successful (for internal use)
pub fn sys_exit(status: i32) -> isize {
    crate::info!("sys_exit called with status: {}", status);

    // Get the process manager and terminate the current process
    let mut manager_lock = PROCESS_MANAGER.lock();
    if let Some(manager) = manager_lock.get_mut() {
        manager.terminate_current(status);
    }

    // If we return here, it means there are no more processes
    // The kernel main loop will continue
    crate::info!("All user processes terminated, returning to kernel");
    0
}

/// Entry point for syscall from assembly
///
/// This function is called from the assembly interrupt handler (syscall_handler_asm)
/// with a pointer to the SyscallArgs structure on the stack.
#[no_mangle]
pub extern "C" fn syscall_entry(args: *const SyscallArgs) -> isize {
    if args.is_null() {
        return SyscallError::InvalidArg as isize;
    }

    // Read fields individually using raw pointer arithmetic to handle any alignment
    let args_copy = unsafe {
        let base = args as *const usize;
        SyscallArgs {
            syscall_num: core::ptr::read_unaligned(base),
            arg1: core::ptr::read_unaligned(base.add(1)),
            arg2: core::ptr::read_unaligned(base.add(2)),
            arg3: core::ptr::read_unaligned(base.add(3)),
            arg4: core::ptr::read_unaligned(base.add(4)),
            arg5: core::ptr::read_unaligned(base.add(5)),
            arg6: core::ptr::read_unaligned(base.add(6)),
        }
    };
    
    syscall_handler(&args_copy)
}
