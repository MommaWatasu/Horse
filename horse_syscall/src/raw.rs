//! Raw system call interface
//!
//! This module provides low-level access to Horse OS system calls using inline assembly.
//!
//! ## Calling Convention (x86-64)
//!
//! - `RAX`: System call number
//! - `RDI`: First argument
//! - `RSI`: Second argument
//! - `RDX`: Third argument
//! - `R10`: Fourth argument
//! - `R8`:  Fifth argument
//! - `R9`:  Sixth argument
//! - Return value in `RAX`
//!
//! System calls are invoked using `int 0x80`.

use core::arch::asm;

/// System call numbers (Linux-compatible)
#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallNum {
    /// Read from a file descriptor
    Read = 0,
    /// Write to a file descriptor
    Write = 1,
    /// Open a file
    Open = 2,
    /// Close a file descriptor
    Close = 3,
    /// Exit the process
    Exit = 60,
}

/// Perform a system call with no arguments
#[inline(always)]
pub unsafe fn syscall0(num: usize) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        options(nostack, preserves_flags)
    );
    ret
}

/// Perform a system call with one argument
#[inline(always)]
pub unsafe fn syscall1(num: usize, arg1: usize) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        in("rdi") arg1,
        options(nostack, preserves_flags)
    );
    ret
}

/// Perform a system call with two arguments
#[inline(always)]
pub unsafe fn syscall2(num: usize, arg1: usize, arg2: usize) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        options(nostack, preserves_flags)
    );
    ret
}

/// Perform a system call with three arguments
#[inline(always)]
pub unsafe fn syscall3(num: usize, arg1: usize, arg2: usize, arg3: usize) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        options(nostack, preserves_flags)
    );
    ret
}

/// Perform a system call with four arguments
#[inline(always)]
pub unsafe fn syscall4(num: usize, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        options(nostack, preserves_flags)
    );
    ret
}

/// Perform a system call with five arguments
#[inline(always)]
pub unsafe fn syscall5(
    num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        options(nostack, preserves_flags)
    );
    ret
}

/// Perform a system call with six arguments
#[inline(always)]
pub unsafe fn syscall6(
    num: usize,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
    arg6: usize,
) -> isize {
    let ret: isize;
    asm!(
        "int 0x80",
        inout("rax") num => ret,
        in("rdi") arg1,
        in("rsi") arg2,
        in("rdx") arg3,
        in("r10") arg4,
        in("r8") arg5,
        in("r9") arg6,
        options(nostack, preserves_flags)
    );
    ret
}

// Convenient type aliases
pub type Fd = i32;
