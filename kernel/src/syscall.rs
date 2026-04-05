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

use crate::drivers::dev::fb::FrameBufferDevice;
use crate::drivers::dev::null::NullDevice;
use crate::drivers::dev::stdin::StdinDevice;
use crate::drivers::dev::stdout::{StderrDevice, StdoutDevice};
use crate::drivers::dev::zero::ZeroDevice;
use crate::drivers::fs::init::FILESYSTEM_TABLE;
use crate::drivers::fs::regular::RegularFile;
use crate::horse_lib::fd::{FDTable, Path};
use crate::horse_lib::ringbuffer::*;
use crate::paging::{phys_to_ptr, PageTable, PAGE_SIZE_4K, PAGE_TABLE_MANAGER};
use crate::proc::{do_switch_context, PROCESS_MANAGER};
use crate::socket::*;
use crate::sync::WaitQueue;
use alloc::{string::String, sync::Arc, vec, vec::Vec};
use horse_abi::{ioctl::IoctlRequest, syscall::SyscallNum};
use spin::Mutex;

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
pub unsafe fn copy_to_user(user_dst: *mut u8, kernel_src: *const u8, len: usize) {
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
        core::ptr::copy_nonoverlapping(kernel_src.add(src_off), pa as *mut u8, chunk);

        remaining -= chunk;
        src_off += chunk;
        dst_va += chunk as u64;
    }
}

/// Copy `len` bytes from the user-space buffer at `user_src` to `kernel_dst`.
/// Uses USER_CR3 to translate each page of the source.
pub unsafe fn copy_from_user(kernel_dst: *mut u8, user_src: *const u8, len: usize) {
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

        core::ptr::copy_nonoverlapping(pa as *const u8, kernel_dst.add(dst_off), chunk);

        remaining -= chunk;
        dst_off += chunk;
        src_va += chunk as u64;
    }
}

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

pub fn sys_socket(domain: i32, socket_type: i32, _protocol: i32) -> isize {
    let socket = Socket::new(domain, socket_type);
    let fd = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let mut proc_guard = proc.lock();
        proc_guard.fd_table.add_socket(Arc::new(socket))
    };
    if fd < 0 {
        return SyscallError::InvalidFd as isize;
    }
    fd as isize
}

pub fn sys_bind(fd: i32, addr: &SocketAddrUn) -> isize {
    // read path name until null character
    let path_len = addr.sun_path.iter().position(|&b| b == 0).unwrap_or(108);
    let path = core::str::from_utf8(&addr.sun_path[..path_len]).unwrap();

    let mut socket = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        match proc_guard.fd_table.get_socket(fd) {
            Some(s) => s,
            None => return SyscallError::NotSocket as isize,
        }
    };

    if unsafe { GLOBAL_SOCKET_TABLE.lock().get(path).is_some() } {
        return SyscallError::AddrInUse as isize;
    }

    unsafe {
        GLOBAL_SOCKET_TABLE
            .lock()
            .insert(String::from(path), socket.clone());
    }

    Arc::<Socket>::get_mut(&mut socket)
        .expect("failed to get mut ref of socket")
        .set_state(SocketState::Bound(String::from(path)));

    0
}

pub fn sys_listen(fd: i32, _backlog: i32) -> isize {
    let mut socket = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        match proc_guard.fd_table.get_socket(fd) {
            Some(s) => s,
            None => return SyscallError::NotSocket as isize,
        }
    };

    match *socket.state.lock() {
        SocketState::Bound(_) => {}
        SocketState::Listening => return 0,
        _ => return SyscallError::InvalidArg as isize,
    }

    Arc::<Socket>::get_mut(&mut socket)
        .expect("failed to get mut ref of socket")
        .set_state(SocketState::Listening);

    0
}

pub fn sys_connect(fd: i32, addr: &SocketAddrUn) -> isize {
    // read path name until null character
    let path_len = addr.sun_path.iter().position(|&b| b == 0).unwrap_or(108);
    let path = core::str::from_utf8(&addr.sun_path[..path_len]).unwrap();

    let mut client_socket = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        proc_guard
            .fd_table
            .get_socket(fd)
            .expect("failed to get client socket")
    };

    let mut server_socket = {
        match unsafe { GLOBAL_SOCKET_TABLE.lock().get(path) } {
            None => return SyscallError::ConnRefused as isize,
            Some(socket) => socket,
        }
    };

    if *server_socket.state.lock() != SocketState::Listening {
        return SyscallError::InvalidArg as isize;
    }

    let a2b = Arc::new(Mutex::new(RingBuffer::new(SOCKET_RING_BUFFER_SIZE)));
    let b2a = Arc::new(Mutex::new(RingBuffer::new(SOCKET_RING_BUFFER_SIZE)));

    let server_wait_queue = Arc::new(Mutex::new(WaitQueue::new()));
    let client_wait_queue = Arc::new(Mutex::new(WaitQueue::new()));

    let server_conn = Socket {
        state: Mutex::new(SocketState::Connected),
        accept_queue: Mutex::new(Vec::new()),
        wait_queue: Arc::new(Mutex::new(WaitQueue::new())),
        read_buf: Some(b2a.clone()),
        write_buf: Some(a2b.clone()),
        peer_wait_queue: Some(client_wait_queue.clone()),
    };
    let server_socket_mut =
        Arc::<Socket>::get_mut(&mut server_socket).expect("failed to get mut ref of server socket");
    server_socket_mut.accept_queue.lock().push(server_conn);
    server_socket_mut.wait_queue.lock().wake();

    {
        let client_socket_mut = Arc::<Socket>::get_mut(&mut client_socket)
            .expect("failed to get mut ref of client socket");
        client_socket_mut.set_buffer(a2b, b2a);
        client_socket_mut.set_state(SocketState::Connected);
        client_socket_mut.peer_wait_queue = Some(server_wait_queue);
    }

    0
}

pub fn sys_accept(fd: i32) -> isize {
    let socket = {
        let proc_manager = PROCESS_MANAGER.lock();
        let proc = proc_manager
            .get()
            .expect("failed to get process manager")
            .current_proc();
        let proc_guard = proc.lock();
        match proc_guard.fd_table.get_socket(fd) {
            Some(s) => s,
            None => return SyscallError::NotSocket as isize,
        }
    };

    if *socket.state.lock() != SocketState::Listening {
        return SyscallError::InvalidArg as isize;
    }

    loop {
        if let Some(connected) = socket.accept_queue.lock().pop() {
            let new_fd = {
                let proc_manager = PROCESS_MANAGER.lock();
                let proc = proc_manager
                    .get()
                    .expect("failed to get process manager")
                    .current_proc();
                let mut proc_guard = proc.lock();
                proc_guard.fd_table.add_socket(Arc::new(connected))
            };
            return new_fd as isize;
        }

        socket.wait_queue.lock().wait();
    }
}

fn sys_ioctl(fd: i32, request: u64, arg: u64) -> isize {
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

/// Entry point for syscall from assembly
#[no_mangle]
pub extern "C" fn syscall_entry(args: *const SyscallArgs) -> isize {
    if args.is_null() {
        return SyscallError::InvalidArg as isize;
    }
    syscall_handler(unsafe { &*args })
}
