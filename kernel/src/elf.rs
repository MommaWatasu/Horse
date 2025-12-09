//! ELF (Executable and Linkable Format) loader
//!
//! This module provides functionality to parse and load ELF64 executables
//! for user-space program execution.

use alloc::vec::Vec;
use core::mem::size_of;

/// ELF magic number
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class: 64-bit
pub const ELFCLASS64: u8 = 2;

/// ELF data encoding: little endian
pub const ELFDATA2LSB: u8 = 1;

/// ELF type: executable
pub const ET_EXEC: u16 = 2;

/// ELF machine: x86-64
pub const EM_X86_64: u16 = 62;

/// Program header type: loadable segment
pub const PT_LOAD: u32 = 1;

/// Program header flags
pub const PF_X: u32 = 1; // Execute
pub const PF_W: u32 = 2; // Write
pub const PF_R: u32 = 4; // Read

/// ELF64 file header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    /// Magic number and other info
    pub e_ident: [u8; 16],
    /// Object file type
    pub e_type: u16,
    /// Architecture
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u64,
    /// Program header table file offset
    pub e_phoff: u64,
    /// Section header table file offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size in bytes
    pub e_ehsize: u16,
    /// Program header table entry size
    pub e_phentsize: u16,
    /// Program header table entry count
    pub e_phnum: u16,
    /// Section header table entry size
    pub e_shentsize: u16,
    /// Section header table entry count
    pub e_shnum: u16,
    /// Section header string table index
    pub e_shstrndx: u16,
}

impl Elf64Header {
    /// Check if this is a valid ELF64 executable for x86-64
    pub fn is_valid(&self) -> bool {
        // Check magic number
        if self.e_ident[0..4] != ELF_MAGIC {
            return false;
        }
        // Check 64-bit
        if self.e_ident[4] != ELFCLASS64 {
            return false;
        }
        // Check little endian
        if self.e_ident[5] != ELFDATA2LSB {
            return false;
        }
        // Check executable type
        if self.e_type != ET_EXEC {
            return false;
        }
        // Check x86-64 architecture
        if self.e_machine != EM_X86_64 {
            return false;
        }
        true
    }
}

/// ELF64 program header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Segment file offset
    pub p_offset: u64,
    /// Segment virtual address
    pub p_vaddr: u64,
    /// Segment physical address
    pub p_paddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Segment alignment
    pub p_align: u64,
}

/// Parsed ELF information
#[derive(Debug)]
pub struct ElfInfo {
    /// Entry point address
    pub entry_point: u64,
    /// Loadable segments
    pub segments: Vec<LoadSegment>,
    /// Lowest virtual address (for memory allocation)
    pub load_base: u64,
    /// Highest virtual address + size (for memory allocation)
    pub load_end: u64,
}

/// A segment to be loaded into memory
#[derive(Debug, Clone)]
pub struct LoadSegment {
    /// Virtual address to load at
    pub vaddr: u64,
    /// Physical address
    pub paddr: u64,
    /// Offset in the file
    pub file_offset: u64,
    /// Size in the file
    pub file_size: u64,
    /// Size in memory (may be larger than file_size for .bss)
    pub mem_size: u64,
    /// Flags (readable, writable, executable)
    pub flags: u32,
}

/// ELF parsing error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// Invalid ELF magic number
    InvalidMagic,
    /// Not a 64-bit ELF
    Not64Bit,
    /// Not little endian
    NotLittleEndian,
    /// Not an executable
    NotExecutable,
    /// Wrong architecture
    WrongArchitecture,
    /// File too small
    FileTooSmall,
    /// Invalid program header
    InvalidProgramHeader,
}

/// Parse an ELF64 executable from raw bytes
pub fn parse_elf(data: &[u8]) -> Result<ElfInfo, ElfError> {
    // Check minimum size for ELF header
    if data.len() < size_of::<Elf64Header>() {
        return Err(ElfError::FileTooSmall);
    }

    // Parse ELF header
    let header = unsafe { &*(data.as_ptr() as *const Elf64Header) };

    // Validate header
    if header.e_ident[0..4] != ELF_MAGIC {
        return Err(ElfError::InvalidMagic);
    }
    if header.e_ident[4] != ELFCLASS64 {
        return Err(ElfError::Not64Bit);
    }
    if header.e_ident[5] != ELFDATA2LSB {
        return Err(ElfError::NotLittleEndian);
    }
    if header.e_type != ET_EXEC {
        return Err(ElfError::NotExecutable);
    }
    if header.e_machine != EM_X86_64 {
        return Err(ElfError::WrongArchitecture);
    }

    let entry_point = header.e_entry;
    let ph_offset = header.e_phoff as usize;
    let ph_size = header.e_phentsize as usize;
    let ph_num = header.e_phnum as usize;

    // Parse program headers
    let mut segments = Vec::new();
    let mut load_base = u64::MAX;
    let mut load_end = 0u64;

    for i in 0..ph_num {
        let ph_start = ph_offset + i * ph_size;
        if ph_start + size_of::<Elf64ProgramHeader>() > data.len() {
            return Err(ElfError::InvalidProgramHeader);
        }

        let ph = unsafe { &*(data.as_ptr().add(ph_start) as *const Elf64ProgramHeader) };

        // Only process loadable segments
        if ph.p_type == PT_LOAD {
            let vaddr = ph.p_vaddr;
            let memsz = ph.p_memsz;

            // Track memory bounds
            if vaddr < load_base {
                load_base = vaddr;
            }
            if vaddr + memsz > load_end {
                load_end = vaddr + memsz;
            }

            segments.push(LoadSegment {
                vaddr: ph.p_vaddr,
                paddr: ph.p_paddr,
                file_offset: ph.p_offset,
                file_size: ph.p_filesz,
                mem_size: ph.p_memsz,
                flags: ph.p_flags,
            });
        }
    }

    if load_base == u64::MAX {
        load_base = 0;
    }

    Ok(ElfInfo {
        entry_point,
        segments,
        load_base,
        load_end,
    })
}

/// Load ELF segments into memory
///
/// This function copies the loadable segments from the ELF file to the
/// specified memory regions. The caller is responsible for allocating
/// the memory at the correct virtual addresses.
///
/// # Safety
///
/// The destination memory must be valid and properly allocated.
pub unsafe fn load_segments(elf_data: &[u8], elf_info: &ElfInfo, base_addr: *mut u8) {
    for segment in &elf_info.segments {
        let dest = base_addr.add((segment.vaddr - elf_info.load_base) as usize);
        let src = elf_data.as_ptr().add(segment.file_offset as usize);

        // Copy file content
        if segment.file_size > 0 {
            core::ptr::copy_nonoverlapping(src, dest, segment.file_size as usize);
        }

        // Zero out the rest (for .bss section)
        if segment.mem_size > segment.file_size {
            let bss_start = dest.add(segment.file_size as usize);
            let bss_size = (segment.mem_size - segment.file_size) as usize;
            core::ptr::write_bytes(bss_start, 0, bss_size);
        }
    }
}

/// Calculate the total memory size needed for the ELF
pub fn calculate_memory_size(elf_info: &ElfInfo) -> usize {
    (elf_info.load_end - elf_info.load_base) as usize
}
