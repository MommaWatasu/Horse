//! User program execution
//!
//! This module provides functionality to load and execute user-space programs.

use crate::elf::{parse_elf, load_segments, calculate_memory_size, ElfError};
use crate::segment::{USER_CS, USER_SS};

/// Default user stack size (64 KB)
const USER_STACK_SIZE: usize = 64 * 1024;

/// User stack base address (below kernel space)
/// In a real OS, this would be determined by the process's virtual address space
const USER_STACK_BASE: u64 = 0x0000_7fff_ffff_0000;

// External assembly function to jump to user mode
extern "C" {
    fn jump_to_user_mode(entry: u64, user_stack: u64, user_cs: u64, user_ss: u64) -> !;
}

/// Error type for program execution
#[derive(Debug, Clone, Copy)]
pub enum ExecError {
    /// ELF parsing error
    ElfError(ElfError),
    /// Memory allocation failed
    MemoryAllocationFailed,
    /// Stack allocation failed
    StackAllocationFailed,
}

impl From<ElfError> for ExecError {
    fn from(e: ElfError) -> Self {
        ExecError::ElfError(e)
    }
}

/// Loaded program ready for execution
pub struct LoadedProgram {
    /// Entry point address
    pub entry_point: u64,
    /// Base address where program was loaded
    pub load_base: u64,
    /// Size of loaded program
    pub load_size: usize,
    /// User stack pointer (top of stack)
    pub stack_pointer: u64,
    /// User stack base (bottom of stack)
    pub stack_base: u64,
    /// User stack size
    pub stack_size: usize,
}

/// Load an ELF program into memory
///
/// This function parses the ELF file, allocates memory, and loads the program
/// segments into memory. It also allocates a user stack.
///
/// # Arguments
///
/// * `elf_data` - The raw ELF file data
///
/// # Returns
///
/// * `Ok(LoadedProgram)` - Information about the loaded program
/// * `Err(ExecError)` - Error if loading failed
pub fn load_program(elf_data: &[u8]) -> Result<LoadedProgram, ExecError> {
    // Parse ELF
    let elf_info = parse_elf(elf_data)?;

    crate::debug!("ELF parsed: entry=0x{:x}, load_base=0x{:x}, load_end=0x{:x}",
        elf_info.entry_point, elf_info.load_base, elf_info.load_end);

    // Calculate memory needed
    let program_size = calculate_memory_size(&elf_info);

    // For simplicity, we'll load the program at its specified virtual address
    // In a real OS, we would set up page tables for the process
    let load_base = elf_info.load_base;

    // Load segments
    unsafe {
        load_segments(elf_data, &elf_info, load_base as *mut u8);
    }

    crate::debug!("Program loaded at 0x{:x}, size={} bytes", load_base, program_size);

    // Allocate user stack
    // For simplicity, we use a fixed stack address
    // In a real OS, this would be allocated per-process
    let stack_base = USER_STACK_BASE - USER_STACK_SIZE as u64;
    let stack_top = USER_STACK_BASE;

    // The stack pointer starts at the top and grows downward
    // Align to 16 bytes as required by System V AMD64 ABI
    let stack_pointer = stack_top & !0xF;

    crate::debug!("User stack: base=0x{:x}, top=0x{:x}, sp=0x{:x}",
        stack_base, stack_top, stack_pointer);

    Ok(LoadedProgram {
        entry_point: elf_info.entry_point,
        load_base,
        load_size: program_size,
        stack_pointer,
        stack_base,
        stack_size: USER_STACK_SIZE,
    })
}

/// Execute a loaded program
///
/// This function transitions to user mode and starts executing the program.
/// **This function never returns!**
///
/// # Arguments
///
/// * `program` - The loaded program to execute
///
/// # Safety
///
/// This function is unsafe because it transitions to user mode and
/// the program must be properly loaded.
pub unsafe fn execute_program(program: &LoadedProgram) -> ! {
    crate::info!("Executing program at 0x{:x}", program.entry_point);

    // Jump to user mode
    jump_to_user_mode(
        program.entry_point,
        program.stack_pointer,
        USER_CS as u64,
        USER_SS as u64,
    )
}

/// Load and execute an ELF program
///
/// This is a convenience function that combines loading and execution.
/// **This function never returns!**
///
/// # Arguments
///
/// * `elf_data` - The raw ELF file data
pub fn exec(elf_data: &[u8]) -> Result<!, ExecError> {
    let program = load_program(elf_data)?;
    unsafe {
        execute_program(&program)
    }
}

/// Simple execution function for testing
///
/// Loads an ELF from the given data and executes it.
/// Prints error message if loading fails.
pub fn run_elf(elf_data: &[u8]) {
    match load_program(elf_data) {
        Ok(program) => {
            crate::info!("Starting user program...");
            unsafe {
                execute_program(&program);
            }
        }
        Err(e) => {
            crate::error!("Failed to load program: {:?}", e);
        }
    }
}
