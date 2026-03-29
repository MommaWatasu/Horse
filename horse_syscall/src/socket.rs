//! Socket operations
//!
//! This module provides safe wrappers around socket-related system calls
//! for Unix domain socket communication.

use crate::error::{check_syscall, Result};
use crate::raw::{syscall1, syscall2, syscall3, Fd, SyscallNum};

/// Unix domain socket address
#[derive(Clone, Copy)]
#[repr(C)]
pub struct SocketAddrUn {
    /// Address family (AF_UNIX = 1)
    pub sun_family: u16,
    /// Socket path (null-terminated, max 108 bytes)
    pub sun_path: [u8; 108],
}

impl SocketAddrUn {
    /// Create a new Unix domain socket address from a path string
    ///
    /// Returns `None` if the path is too long (> 107 bytes).
    pub fn new(path: &str) -> Option<Self> {
        let bytes = path.as_bytes();
        if bytes.len() > 107 {
            return None;
        }
        let mut addr = Self {
            sun_family: 1, // AF_UNIX
            sun_path: [0u8; 108],
        };
        addr.sun_path[..bytes.len()].copy_from_slice(bytes);
        Some(addr)
    }
}

/// Address families
pub const AF_UNIX: i32 = 1;

/// Socket types
pub const SOCK_STREAM: i32 = 1;
pub const SOCK_DGRAM: i32 = 2;

/// Create a new socket
///
/// # Arguments
///
/// * `domain` - Address family (e.g. `AF_UNIX`)
/// * `socket_type` - Socket type (e.g. `SOCK_STREAM`)
/// * `protocol` - Protocol (typically 0)
///
/// # Returns
///
/// * `Ok(fd)` - Socket file descriptor on success
/// * `Err(e)` - Error on failure
pub fn socket(domain: i32, socket_type: i32, protocol: i32) -> Result<Fd> {
    let ret = unsafe {
        syscall3(
            SyscallNum::Socket as usize,
            domain as usize,
            socket_type as usize,
            protocol as usize,
        )
    };
    check_syscall(ret).map(|fd| fd as Fd)
}

/// Bind a socket to a local address
///
/// # Arguments
///
/// * `fd` - Socket file descriptor
/// * `addr` - Address to bind to
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(e)` - Error on failure
pub fn bind(fd: Fd, addr: &SocketAddrUn) -> Result<()> {
    let ret = unsafe {
        syscall2(
            SyscallNum::Bind as usize,
            fd as usize,
            addr as *const SocketAddrUn as usize,
        )
    };
    check_syscall(ret).map(|_| ())
}

/// Mark a socket as passive, ready to accept incoming connections
///
/// # Arguments
///
/// * `fd` - Socket file descriptor (must already be bound)
/// * `backlog` - Maximum length of the pending connection queue
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(e)` - Error on failure
pub fn listen(fd: Fd, backlog: i32) -> Result<()> {
    let ret = unsafe {
        syscall2(
            SyscallNum::Listen as usize,
            fd as usize,
            backlog as usize,
        )
    };
    check_syscall(ret).map(|_| ())
}

/// Connect a socket to a remote address
///
/// # Arguments
///
/// * `fd` - Socket file descriptor
/// * `addr` - Address to connect to
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(e)` - Error on failure
pub fn connect(fd: Fd, addr: &SocketAddrUn) -> Result<()> {
    let ret = unsafe {
        syscall2(
            SyscallNum::Connect as usize,
            fd as usize,
            addr as *const SocketAddrUn as usize,
        )
    };
    check_syscall(ret).map(|_| ())
}

/// Accept an incoming connection on a listening socket
///
/// Blocks until a connection is available.
///
/// # Arguments
///
/// * `fd` - Listening socket file descriptor
///
/// # Returns
///
/// * `Ok(fd)` - New file descriptor for the accepted connection
/// * `Err(e)` - Error on failure
pub fn accept(fd: Fd) -> Result<Fd> {
    let ret = unsafe { syscall1(SyscallNum::Accept as usize, fd as usize) };
    check_syscall(ret).map(|fd| fd as Fd)
}
