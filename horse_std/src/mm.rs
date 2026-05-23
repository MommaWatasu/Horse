//! Memory management operations
//!
//! This module provides safe wrappers around memory-related system calls.

use crate::error::{check_syscall, Result};
use crate::raw::{syscall1, syscall6, SyscallNum};
pub use horse_abi::mm::{MapFlags, Prot};

/// Adjust the program break (end of the data segment).
///
/// Passing `addr = 0` returns the current break without changing it.
///
/// # Returns
///
/// * `Ok(brk)` — the new (or current) program break address
/// * `Err(e)` — error on failure
pub fn brk(addr: usize) -> Result<usize> {
    let ret = unsafe { syscall1(SyscallNum::Brk as usize, addr) };
    check_syscall(ret)
}

/// Map memory into the process address space (`mmap`).
///
/// # Arguments
///
/// * `addr`   — hint address (0 lets the kernel choose)
/// * `len`    — number of bytes to map
/// * `prot`   — memory protection ([`Prot`])
/// * `flags`  — mapping flags ([`MapFlags`])
/// * `fd`     — file descriptor (-1 for anonymous mappings)
/// * `offset` — offset into the file (must be page-aligned)
///
/// # Returns
///
/// * `Ok(addr)` — start address of the new mapping
/// * `Err(e)`   — error on failure
pub fn mmap(
    addr: usize,
    len: usize,
    prot: u64,
    flags: u64,
    fd: i32,
    offset: usize,
) -> Result<usize> {
    let ret = unsafe {
        syscall6(
            SyscallNum::Mmap as usize,
            addr,
            len,
            prot as usize,
            flags as usize,
            fd as usize,
            offset,
        )
    };
    check_syscall(ret)
}
