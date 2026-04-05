//! # horse_abi
//!
//! Shared ABI types between the Horse OS kernel and user-space programs.
//!
//! This crate is `no_std` and has no external dependencies, making it safe
//! to use in both the kernel and user-space contexts.

#![no_std]

pub mod error;
pub mod fb;
pub mod ioctl;
pub mod socket;
pub mod syscall;
