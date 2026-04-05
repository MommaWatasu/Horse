//! # horse_syscall
//!
//! System call wrapper library for Horse OS user-space applications.
//!
//! This crate provides a safe and ergonomic interface to Horse OS system calls,
//! allowing `#![no_std]` applications to interact with the kernel.
//!
//! ## Example
//!
//! ```rust,ignore
//! #![no_std]
//! #![no_main]
//!
//! use horse_syscall::prelude::*;
//!
//! #[no_mangle]
//! pub extern "C" fn _start() -> ! {
//!     // Write to stdout
//!     write(STDOUT, b"Hello from Horse OS!\n").unwrap();
//!
//!     // Open and read a file
//!     let fd = open("/test.txt", OpenFlags::RDONLY).unwrap();
//!     let mut buf = [0u8; 256];
//!     let n = read(fd, &mut buf).unwrap();
//!     close(fd).unwrap();
//!
//!     exit(0);
//! }
//! ```

#![no_std]

pub mod error;
pub mod fb;
pub mod fs;
pub mod io;
pub mod raw;
pub mod socket;

/// Prelude module - import everything you need with `use horse_syscall::prelude::*`
pub mod prelude {
    pub use crate::error::{Error, Result};
    pub use crate::fb::{
        fb_get_fscreeninfo, fb_get_vscreeninfo, fb_put_vscreeninfo, ioctl, FbBitfield,
        FbFixScreenInfo, FbVarScreenInfo, FBIOGET_FSCREENINFO, FBIOGET_VSCREENINFO,
        FBIOPUT_VSCREENINFO,
    };
    pub use crate::fs::{close, exit, open, read, spawn, write, OpenFlags};
    pub use crate::io::{print, println, STDERR, STDIN, STDOUT};
    pub use crate::raw::{syscall0, syscall1, syscall2, syscall3, syscall4, syscall5, syscall6};
    pub use crate::socket::{accept, bind, connect, listen, socket, SocketAddrUn, AF_UNIX, SOCK_DGRAM, SOCK_STREAM};
    pub use horse_abi::ioctl::IoctlRequest;
}

pub use error::{Error, Result};
pub use fs::OpenFlags;
