use super::SyscallError;
use crate::horse_lib::ringbuffer::RingBuffer;
use crate::proc::PROCESS_MANAGER;
use crate::socket::{Socket, SocketAddrUn, SocketState, GLOBAL_SOCKET_TABLE, SOCKET_RING_BUFFER_SIZE};
use crate::sync::WaitQueue;
use alloc::{string::String, sync::Arc, vec::Vec};
use spin::Mutex;

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
