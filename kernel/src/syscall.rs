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

use alloc::{sync::Arc, vec};
use crate::drivers::fs::core::FILE_DESCRIPTOR_TABLE;
use crate::drivers::fs::init::FILESYSTEM_TABLE;
use crate::drivers::fs::regular::RegularFile;
use crate::drivers::dev::null::NullDevice;
use crate::drivers::dev::zero::ZeroDevice;
use crate::drivers::dev::stdin::StdinDevice;
use crate::drivers::dev::stdout::{StdoutDevice, StderrDevice};
use crate::horse_lib::fd::Path;
use crate::paging::{PAGE_TABLE_MANAGER, PAGE_SIZE_4K, phys_to_ptr, PageTable};
use crate::proc::{PROCESS_MANAGER, do_switch_context};

// ── user memory helpers ──────────────────────────────────────────────────────
//
// During a syscall the CPU runs with KERNEL_CR3 (identity map VA=PA for 0-64GB).
// User stack pages are allocated at *arbitrary* physical addresses by the frame
// manager and are mapped into the user page table only – they are NOT accessible
// through the kernel's identity map at the pointer value the user passes.
//
// These helpers translate each page of a user VA range to its physical address
// using the saved USER_CR3, then access the physical frame directly via the
// kernel's identity map (VA == PA for low addresses).

/// Copy `len` bytes from `kernel_src` to the user-space buffer at `user_dst`.
/// Uses USER_CR3 to translate each page of the destination.
unsafe fn copy_to_user(user_dst: *mut u8, kernel_src: *const u8, len: usize) {
    let user_cr3 = crate::paging::USER_CR3;
    let pml4 = phys_to_ptr::<PageTable>(user_cr3) as *const PageTable;
    let manager = PAGE_TABLE_MANAGER.lock();
    let mut remaining = len;
    let mut src_off = 0usize;
    let mut dst_va = user_dst as u64;

    while remaining > 0 {
        let pa = match manager.translate(pml4, dst_va) {
            Some(pa) => pa,
            None => break, // unmapped page – stop silently
        };
        // how many bytes fit in this page from the current offset
        let page_offset = (dst_va & (PAGE_SIZE_4K as u64 - 1)) as usize;
        let chunk = (PAGE_SIZE_4K - page_offset).min(remaining);

        // write to the physical frame via the kernel identity map (VA == PA)
        core::ptr::copy_nonoverlapping(
            kernel_src.add(src_off),
            pa as *mut u8,
            chunk,
        );

        remaining -= chunk;
        src_off   += chunk;
        dst_va    += chunk as u64;
    }
}

/// Copy `len` bytes from the user-space buffer at `user_src` to `kernel_dst`.
/// Uses USER_CR3 to translate each page of the source.
unsafe fn copy_from_user(kernel_dst: *mut u8, user_src: *const u8, len: usize) {
    let user_cr3 = crate::paging::USER_CR3;
    let pml4 = phys_to_ptr::<PageTable>(user_cr3) as *const PageTable;
    let manager = PAGE_TABLE_MANAGER.lock();
    let mut remaining = len;
    let mut dst_off = 0usize;
    let mut src_va = user_src as u64;

    while remaining > 0 {
        let pa = match manager.translate(pml4, src_va) {
            Some(pa) => pa,
            None => break,
        };
        let page_offset = (src_va & (PAGE_SIZE_4K as u64 - 1)) as usize;
        let chunk = (PAGE_SIZE_4K - page_offset).min(remaining);

        core::ptr::copy_nonoverlapping(
            pa as *const u8,
            kernel_dst.add(dst_off),
            chunk,
        );

        remaining -= chunk;
        dst_off   += chunk;
        src_va    += chunk as u64;
    }
}

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
            args.arg2,              // len
            args.arg3 as u32,       // flags
        ),
        SyscallNumber::Close => sys_close(args.arg1 as i32),
        SyscallNumber::Exit => sys_exit(args.arg1 as i32),
    }
}

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

    let path_str = unsafe {
        let slice = core::slice::from_raw_parts(pathname, len);
        match core::str::from_utf8(slice) {
            Ok(s) => s,
            Err(_) => return SyscallError::InvalidArg as isize,
        }
    };

    // Route /dev/* to device files
    if let Some(dev_name) = path_str.strip_prefix("/dev/") {
        let fd_entry: Arc<dyn crate::horse_lib::fd::FileDescriptor> = match dev_name {
            "null"   => Arc::new(NullDevice),
            "zero"   => Arc::new(ZeroDevice),
            "stdin"  => Arc::new(StdinDevice),
            "stdout" => Arc::new(StdoutDevice),
            "stderr" => Arc::new(StderrDevice),
            _ => return SyscallError::NoEnt as isize,
        };
        let fd = FILE_DESCRIPTOR_TABLE.lock().add(fd_entry);
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
        return SyscallError::NoEnt as isize;
    }

    let file = Arc::new(RegularFile::new(flags, path_str));
    let fd = FILE_DESCRIPTOR_TABLE.lock().add(file);
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

    let fd_entry = match FILE_DESCRIPTOR_TABLE.lock().get(fd) {
        Some(e) => e,
        None => return SyscallError::InvalidFd as isize,
    };

    let mut tmp = vec![0u8; count];
    let bytes_read = fd_entry.read(&mut tmp);
    if bytes_read > 0 {
        // Use page-table-aware copy so that user stack buffers (which live at
        // arbitrary physical addresses) are written correctly.
        unsafe { copy_to_user(buf, tmp.as_ptr(), bytes_read as usize); }
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

    let fd_entry = match FILE_DESCRIPTOR_TABLE.lock().get(fd) {
        Some(e) => e,
        None => return SyscallError::InvalidFd as isize,
    };

    // Copy user buffer to kernel memory first, because the user buffer may be
    // on the stack (arbitrary physical address, not accessible via identity map).
    let mut tmp = vec![0u8; count];
    unsafe { copy_from_user(tmp.as_mut_ptr(), buf, count); }
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
    let valid = FILE_DESCRIPTOR_TABLE.lock().get(fd).is_some();
    if !valid {
        return SyscallError::InvalidFd as isize;
    }
    FILE_DESCRIPTOR_TABLE.lock().remove(fd);
    0
}

/// sys_exit - Terminate the current process
pub fn sys_exit(status: i32) -> isize {
    let switch_ptrs = {
        let mut manager_lock = PROCESS_MANAGER.lock();
        if let Some(manager) = manager_lock.get_mut() {
            manager.prepare_terminate(status)
        } else {
            None
        }
    };

    if let Some((next_ctx, current_ctx)) = switch_ptrs {
        unsafe { do_switch_context(next_ctx, current_ctx); }
    }

    0
}

/// Entry point for syscall from assembly
#[no_mangle]
pub extern "C" fn syscall_entry(args: *const SyscallArgs) -> isize {
    if args.is_null() {
        return SyscallError::InvalidArg as isize;
    }
    syscall_handler(unsafe { &*args })
}
