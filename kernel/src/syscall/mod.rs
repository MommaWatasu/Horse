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

use crate::socket::SocketAddrUn;
use horse_abi::syscall::SyscallNum;
use self::fs::{sys_close, sys_ioctl, sys_open, sys_read, sys_write};
use self::net::{sys_accept, sys_bind, sys_connect, sys_listen, sys_socket};
use self::proc::{sys_exit, sys_spawn};

pub mod fs;
pub mod net;
pub mod proc;
pub mod user_mem;

pub use user_mem::{copy_from_user, copy_to_user};

/// Syscall error codes
#[repr(isize)]
#[derive(Debug, Clone, Copy)]
pub enum SyscallError {
    InvalidSyscall = -1,
    NoEntry = -2,       // ENOENT
    IoError = -5,       // EIO
    InvalidFd = -9,     // EBADF
    InvalidArg = -22,   // EINVAL
    NotSocket = -88,    // ENOTSOCK
    OpNotSupp = -95,    // EOPNOTSUPP
    AddrInUse = -98,    // EADDRINUSE
    AddrNotAvail = -99, // EADDRNOTAVAIL
    IsConn = -106,      // EISCONN
    NotConn = -107,     // ENOTCONN
    ConnRefused = -111, // ECONNREFUSED
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
    let syscall_num = match SyscallNum::try_from(args.syscall_num) {
        Ok(num) => num,
        Err(_) => return SyscallError::InvalidSyscall as isize,
    };

    match syscall_num {
        SyscallNum::Read => sys_read(
            args.arg1 as i32,     // fd
            args.arg2 as *mut u8, // buf
            args.arg3,            // count
        ),
        SyscallNum::Write => sys_write(
            args.arg1 as i32,       // fd
            args.arg2 as *const u8, // buf
            args.arg3,              // count
        ),
        SyscallNum::Open => sys_open(
            args.arg1 as *const u8, // pathname
            args.arg2,              // len
            args.arg3 as u32,       // flags
        ),
        SyscallNum::Close => sys_close(args.arg1 as i32),
        SyscallNum::Socket => sys_socket(args.arg1 as i32, args.arg2 as i32, args.arg3 as i32),
        SyscallNum::Connect => sys_connect(args.arg1 as i32, unsafe {
            &*(args.arg2 as *const SocketAddrUn)
        }),
        SyscallNum::Accept => sys_accept(args.arg1 as i32),
        SyscallNum::Bind => sys_bind(args.arg1 as i32, unsafe {
            &*(args.arg2 as *const SocketAddrUn)
        }),
        SyscallNum::Listen => sys_listen(args.arg1 as i32, args.arg2 as i32),
        SyscallNum::Ioctl => sys_ioctl(args.arg1 as i32, args.arg2 as u64, args.arg3 as u64),
        SyscallNum::Exit => sys_exit(args.arg1 as i32),
        SyscallNum::Spawn => sys_spawn(
            args.arg1 as *const u8, // path
            args.arg2,              // path_len
        ),
    }
}

/// Entry point for syscall from assembly
#[no_mangle]
pub extern "C" fn syscall_entry(args: *const SyscallArgs) -> isize {
    if args.is_null() {
        return SyscallError::InvalidArg as isize;
    }
    syscall_handler(unsafe { &*args })
}
