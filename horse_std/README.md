# horse_syscall

System call wrapper library for Horse OS user-space applications.

## Overview

This crate provides a safe and ergonomic interface to Horse OS system calls, allowing `#![no_std]` applications to interact with the kernel.

## Features

- **Raw syscall functions**: `syscall0` through `syscall6` for direct system call invocation
- **File operations**: `open`, `read`, `write`, `close`
- **RAII File wrapper**: Automatic resource cleanup
- **I/O utilities**: `print!`, `println!`, `eprint!`, `eprintln!` macros
- **Error handling**: Rust-style `Result` types

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
horse_syscall = { path = "../horse_syscall" }
```

## Quick Start

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;
use horse_syscall::prelude::*;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Write to stdout
    let _ = write(STDOUT, b"Hello from Horse OS!\n");

    // Open and read a file
    if let Ok(fd) = open("/test.txt", OpenFlags::RDONLY) {
        let mut buf = [0u8; 256];
        if let Ok(n) = read(fd, &mut buf) {
            let _ = write(STDOUT, &buf[..n]);
        }
        let _ = close(fd);
    }

    loop {}
}

#[panic_handler]
fn panic(_: &PanicInfo) -> ! {
    loop {}
}
```

## API Reference

### System Call Numbers

| Number | Name  | Description           |
|--------|-------|-----------------------|
| 0      | read  | Read from file        |
| 1      | write | Write to file         |
| 2      | open  | Open file             |
| 3      | close | Close file descriptor |

### File Descriptors

| FD | Name   | Description     |
|----|--------|-----------------|
| 0  | STDIN  | Standard input  |
| 1  | STDOUT | Standard output |
| 2  | STDERR | Standard error  |

### Open Flags

| Flag        | Value  | Description                  |
|-------------|--------|------------------------------|
| RDONLY      | 0x0000 | Open for reading only        |
| WRONLY      | 0x0001 | Open for writing only        |
| RDWR        | 0x0002 | Open for reading and writing |
| CREAT       | 0x0100 | Create if doesn't exist      |
| TRUNC       | 0x0200 | Truncate to zero length      |
| APPEND      | 0x0400 | Append to file               |

## Calling Convention

Horse OS uses the following x86-64 calling convention for system calls:

- `RAX`: System call number
- `RDI`: First argument
- `RSI`: Second argument
- `RDX`: Third argument
- `R10`: Fourth argument
- `R8`:  Fifth argument
- `R9`:  Sixth argument
- Return value in `RAX`

System calls are invoked using `int 0x80`.

## License

MIT License
