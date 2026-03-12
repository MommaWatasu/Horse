# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Horse is an x86_64 operating system written in Rust. It consists of four main components:

- **bootloader** — UEFI bootloader that loads the kernel ELF into memory and sets up framebuffer/memory map
- **kernel** — The core OS kernel (no_std, runs in ring 0)
- **horse_syscall** — User-space system call library (no_std)
- **gallop** — Example user-space program demonstrating syscall usage

There is also a **libloader** shared crate used by both the bootloader and kernel to pass boot-time data (framebuffer config, memory map).

## Prerequisites

```
rustup install nightly
sudo apt install nasm lld qemu qemu-utils qemu-system-x86
rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu
cargo install cargo-make
```

## Build & Run Commands

```bash
# Build all components and run in QEMU (most common workflow)
cargo make run

# Build and run with debugcon output to stdio (for kernel debug! logging)
cargo make run-debugcon

# Build only (no QEMU)
# Equivalent: cd bootloader && cargo build, cd kernel && cargo build, cd gallop && cargo build --release
cargo make BUILD

# Create disk image only (no QEMU)
cargo make make-image

# Keep image after run (for inspection)
cargo make save-img

# Run with GDB attached (waits for connection on port 1234)
cargo make gdb
```

Each sub-crate also builds independently:
```bash
cd kernel && cargo build
cd bootloader && cargo build
cd gallop && cargo build --release
```

## Architecture Overview

### Boot Flow

1. UEFI firmware loads `bootloader/` → `horse-bootloader.efi`
2. Bootloader reads `horse-kernel` and `gallop` ELFs from the FAT32 disk image, sets up GOP framebuffer, then jumps to the kernel entry point
3. `kernel/asm.s` (`kernel_main`) sets up boot page tables (identity map + higher-half mapping at `0xFFFFFFFF80000000`) via 2MB huge pages, switches stacks, then calls `kernel_main_virt`
4. `kernel/src/main.rs` initializes: paging, memory manager, heap allocator, segments/GDT, IDT, ACPI/HPET timer, PCI/USB/xHCI, filesystem, process manager, then loads and executes `gallop`

### Memory Layout

- Kernel runs in the **higher half** at `0xFFFFFFFF80000000` (physical base `0x100000`)
- Identity mapping covers physical `0x0–64GB` via `PML4[0]`
- Higher-half kernel mapping at `PML4[511]` covers the top 2GB
- User stack is placed at `0x4000_0000` (1GB physical, within identity map so kernel can access via syscalls)
- User programs load at their ELF-specified virtual addresses (also in identity-mapped range)

### Syscall Interface

System calls use `int 0x80` (interrupt vector `0x80`). The calling convention follows Linux x86-64:
- `RAX` = syscall number, `RDI/RSI/RDX/R10/R8/R9` = args 1–6
- Return value in `RAX`

Implemented syscalls: `read(0)`, `write(1)`, `open(2)`, `close(3)`, `exit(60)`

The kernel-side handler (`kernel/src/syscall.rs:syscall_entry`) is called from assembly (`kernel/asm.s`). On entry, the handler saves kernel CR3 and switches to the kernel page table via `KERNEL_CR3`.

The user-side wrappers live in `horse_syscall/src/raw.rs` and high-level helpers in `horse_syscall/src/fs.rs`. User programs use `use horse_syscall::prelude::*`.

### Page Tables

`kernel/src/paging.rs` owns all page table management:
- `PAGE_TABLE_MANAGER` (global `Mutex<PageTableManager>`) handles allocation/mapping
- `setup_user_page_table()` creates per-process PML4s by deep-copying the low identity-map entries (so user programs can read kernel-mapped memory) and sharing the higher-half kernel entry
- Huge pages (2MB) are split into 4KB pages on demand when finer-grained mappings are needed
- `KERNEL_CR3` is a `#[no_mangle]` static read by the assembly syscall handler

### Process Management

`kernel/src/proc.rs` manages processes via `PROCESS_MANAGER` (`Mutex<Once<ProcessManager>>`). Context switches save/restore all GP registers plus `fxsave` state in `ProcessContext`. The LAPIC timer (vector `0x41`) triggers preemption via `switch_context` (assembly).

### Drivers

Located under `kernel/src/drivers/`:
- `pci.rs` — PCI device enumeration
- `usb/xhci/` — xHCI USB host controller
- `usb/classdriver/` — keyboard, mouse (HID), CDC
- `timer/` — HPET, LAPIC timer, IO-APIC
- `ata/` — PATA and virtual ATA
- `fs/` — GPT partition table, FAT32 filesystem, file descriptor table

### Logging / Debug

The kernel exposes `crate::info!`, `crate::debug!`, `crate::error!` macros (`kernel/src/log.rs`). Debug output goes to the on-screen console. With `cargo make run-debugcon`, `crate::debug!` also writes to QEMU's debugcon (stdio).

### Adding a New User-Space Program

1. Create a new `no_std` binary crate with `horse_syscall` as a dependency
2. Use the custom linker script (`gallop/link.ld`) and target spec (`gallop/x86_64-horse-user.json`)
3. Entry point must be `#[no_mangle] pub extern "C" fn _start() -> !`
4. Build with `cargo build --release --target x86_64-horse-user.json`
5. Copy the output binary to `dev-tools/` and add it to the disk image in `Makefile.toml`
