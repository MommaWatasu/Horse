//! Paging implementation for HorseOS
//!
//! This module provides 4-level paging support for x86_64:
//! - PML4 (Page Map Level 4) - 512 GB per entry
//! - PDPT (Page Directory Pointer Table) - 1 GB per entry
//! - PD (Page Directory) - 2 MB per entry
//! - PT (Page Table) - 4 KB per entry

use core::ops::{Index, IndexMut};
use core::ptr::addr_of_mut;
use spin::Mutex;

use crate::memory_manager::{frame_manager_instance, FrameID};

// Page sizes
pub const PAGE_SIZE_4K: usize = 4096;
pub const PAGE_SIZE_2M: usize = 512 * PAGE_SIZE_4K;
pub const PAGE_SIZE_1G: usize = 512 * PAGE_SIZE_2M;

// Number of entries in a page table
const PAGE_TABLE_ENTRIES: usize = 512;

// Initial page directory count for kernel identity mapping
const PAGE_DIRECTORY_COUNT: usize = 64;

// Virtual address space layout
pub const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000; // Upper half for kernel (optional)
pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FFFF_0000;
pub const USER_STACK_SIZE: usize = 64 * 1024; // 64 KB

/// Page table entry flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PageTableFlags(u64);

impl PageTableFlags {
    pub const PRESENT: Self = Self(1 << 0);
    pub const WRITABLE: Self = Self(1 << 1);
    pub const USER_ACCESSIBLE: Self = Self(1 << 2);
    pub const WRITE_THROUGH: Self = Self(1 << 3);
    pub const NO_CACHE: Self = Self(1 << 4);
    pub const ACCESSED: Self = Self(1 << 5);
    pub const DIRTY: Self = Self(1 << 6);
    pub const HUGE_PAGE: Self = Self(1 << 7);
    pub const GLOBAL: Self = Self(1 << 8);
    pub const NO_EXECUTE: Self = Self(1 << 63);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn bits(&self) -> u64 {
        self.0
    }

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn intersection(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl core::ops::BitOr for PageTableFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for PageTableFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for PageTableFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

/// Common flag combinations
impl PageTableFlags {
    /// Kernel read-only page
    pub const KERNEL_RO: Self = Self(Self::PRESENT.0);
    /// Kernel read-write page
    pub const KERNEL_RW: Self = Self(Self::PRESENT.0 | Self::WRITABLE.0);
    /// User read-only page
    pub const USER_RO: Self = Self(Self::PRESENT.0 | Self::USER_ACCESSIBLE.0);
    /// User read-write page
    pub const USER_RW: Self = Self(Self::PRESENT.0 | Self::WRITABLE.0 | Self::USER_ACCESSIBLE.0);
    /// Kernel 2MB page
    pub const KERNEL_HUGE: Self = Self(Self::PRESENT.0 | Self::WRITABLE.0 | Self::HUGE_PAGE.0);
    /// User 2MB page
    pub const USER_HUGE: Self = Self(Self::PRESENT.0 | Self::WRITABLE.0 | Self::USER_ACCESSIBLE.0 | Self::HUGE_PAGE.0);
}

/// A single page table entry
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;

    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub const fn raw(&self) -> u64 {
        self.0
    }

    pub fn set(&mut self, addr: u64, flags: PageTableFlags) {
        self.0 = (addr & Self::ADDR_MASK) | flags.bits();
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn addr(&self) -> u64 {
        self.0 & Self::ADDR_MASK
    }

    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits(self.0 & !Self::ADDR_MASK)
    }

    pub fn is_present(&self) -> bool {
        self.flags().contains(PageTableFlags::PRESENT)
    }

    pub fn is_huge(&self) -> bool {
        self.flags().contains(PageTableFlags::HUGE_PAGE)
    }

    pub fn is_user(&self) -> bool {
        self.flags().contains(PageTableFlags::USER_ACCESSIBLE)
    }
}

/// A page table (512 entries, 4KB aligned)
#[repr(C, align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::new(); PAGE_TABLE_ENTRIES],
        }
    }

    pub fn clear(&mut self) {
        for entry in self.entries.iter_mut() {
            entry.clear();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }

    /// Get entry at index (safe method)
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }

    /// Get mutable entry at index (safe method)
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

/// Virtual address helper functions
pub struct VirtAddr(u64);

impl VirtAddr {
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    pub const fn as_u64(&self) -> u64 {
        self.0
    }

    /// Get PML4 index (bits 39-47)
    pub const fn pml4_index(&self) -> usize {
        ((self.0 >> 39) & 0x1FF) as usize
    }

    /// Get PDPT index (bits 30-38)
    pub const fn pdpt_index(&self) -> usize {
        ((self.0 >> 30) & 0x1FF) as usize
    }

    /// Get PD index (bits 21-29)
    pub const fn pd_index(&self) -> usize {
        ((self.0 >> 21) & 0x1FF) as usize
    }

    /// Get PT index (bits 12-20)
    pub const fn pt_index(&self) -> usize {
        ((self.0 >> 12) & 0x1FF) as usize
    }

    /// Get page offset (bits 0-11)
    pub const fn page_offset(&self) -> u64 {
        self.0 & 0xFFF
    }

    /// Align down to 4KB page boundary
    pub const fn align_down_4k(&self) -> Self {
        Self(self.0 & !0xFFF)
    }

    /// Align up to 4KB page boundary
    pub const fn align_up_4k(&self) -> Self {
        Self((self.0 + 0xFFF) & !0xFFF)
    }
}

// Static page tables for kernel identity mapping
static mut KERNEL_PML4: PageTable = PageTable::new();
static mut KERNEL_PDPT: PageTable = PageTable::new();
static mut KERNEL_PD: [PageTable; PAGE_DIRECTORY_COUNT] = [const { PageTable::new() }; PAGE_DIRECTORY_COUNT];

// Global page table manager
pub static PAGE_TABLE_MANAGER: Mutex<PageTableManager> = Mutex::new(PageTableManager::new());

/// Page table manager for creating and managing page tables
pub struct PageTableManager {
    initialized: bool,
}

impl PageTableManager {
    pub const fn new() -> Self {
        Self { initialized: false }
    }

    /// Initialize the page table manager
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Allocate a new page table (4KB aligned)
    pub fn allocate_page_table(&self) -> Option<*mut PageTable> {
        let frame = frame_manager_instance().allocate(1).ok()?;
        let ptr = frame.phys_addr() as *mut PageTable;

        // Zero out the page table
        unsafe {
            core::ptr::write_bytes(ptr, 0, 1);
        }

        Some(ptr)
    }

    /// Free a page table
    pub fn free_page_table(&self, table: *mut PageTable) {
        let frame = FrameID::from_phys_addr(table as *mut u8);
        frame_manager_instance().free(frame, 1);
    }

    /// Create a new user page table with kernel mappings
    pub fn create_user_page_table(&self) -> Option<*mut PageTable> {
        use core::ptr::addr_of;

        let pml4 = self.allocate_page_table()?;

        unsafe {
            // Copy kernel mappings (first entry for identity mapping)
            // This ensures kernel code/data is accessible in user space
            let kernel_pml4 = addr_of!(KERNEL_PML4);
            (*pml4).entries[0] = (*kernel_pml4).entries[0];
        }

        Some(pml4)
    }

    /// Map a 4KB page
    pub fn map_4k(
        &self,
        pml4: *mut PageTable,
        virt_addr: u64,
        phys_addr: u64,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let vaddr = VirtAddr::new(virt_addr);

        unsafe {
            // Get or create PDPT
            let pml4_ref = &mut *pml4;
            let pdpt = self.get_or_create_table(&mut pml4_ref.entries[vaddr.pml4_index()], flags)?;

            // Get or create PD
            let pdpt_ref = &mut *pdpt;
            let pd = self.get_or_create_table(&mut pdpt_ref.entries[vaddr.pdpt_index()], flags)?;

            // Get or create PT
            let pd_ref = &mut *pd;
            let pt = self.get_or_create_table(&mut pd_ref.entries[vaddr.pd_index()], flags)?;

            // Set the page table entry
            let pt_ref = &mut *pt;
            pt_ref.entries[vaddr.pt_index()].set(phys_addr, flags);
        }

        Ok(())
    }

    /// Map a 2MB huge page
    pub fn map_2m(
        &self,
        pml4: *mut PageTable,
        virt_addr: u64,
        phys_addr: u64,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let vaddr = VirtAddr::new(virt_addr);
        let huge_flags = flags | PageTableFlags::HUGE_PAGE;

        unsafe {
            // Get or create PDPT
            let pml4_ref = &mut *pml4;
            let pdpt = self.get_or_create_table(&mut pml4_ref.entries[vaddr.pml4_index()], flags)?;

            // Get or create PD
            let pdpt_ref = &mut *pdpt;
            let pd = self.get_or_create_table(&mut pdpt_ref.entries[vaddr.pdpt_index()], flags)?;

            // Set the page directory entry as a huge page
            let pd_ref = &mut *pd;
            pd_ref.entries[vaddr.pd_index()].set(phys_addr, huge_flags);
        }

        Ok(())
    }

    /// Get or create a page table for the given entry
    unsafe fn get_or_create_table(
        &self,
        entry: &mut PageTableEntry,
        flags: PageTableFlags,
    ) -> Result<*mut PageTable, &'static str> {
        if entry.is_present() {
            // Table already exists
            Ok(entry.addr() as *mut PageTable)
        } else {
            // Create new table
            let table = self.allocate_page_table()
                .ok_or("Failed to allocate page table")?;

            // Set entry to point to new table
            // Use USER_ACCESSIBLE flag if the mapping is for user space
            let table_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE |
                if flags.contains(PageTableFlags::USER_ACCESSIBLE) {
                    PageTableFlags::USER_ACCESSIBLE
                } else {
                    PageTableFlags::empty()
                };
            entry.set(table as u64, table_flags);

            Ok(table)
        }
    }

    /// Unmap a 4KB page
    pub fn unmap_4k(&self, pml4: *mut PageTable, virt_addr: u64) -> Result<(), &'static str> {
        let vaddr = VirtAddr::new(virt_addr);

        unsafe {
            let pml4_ref = &*pml4;
            let pml4_entry = &pml4_ref.entries[vaddr.pml4_index()];
            if !pml4_entry.is_present() {
                return Err("PML4 entry not present");
            }

            let pdpt = pml4_entry.addr() as *const PageTable;
            let pdpt_ref = &*pdpt;
            let pdpt_entry = &pdpt_ref.entries[vaddr.pdpt_index()];
            if !pdpt_entry.is_present() {
                return Err("PDPT entry not present");
            }

            let pd = pdpt_entry.addr() as *const PageTable;
            let pd_ref = &*pd;
            let pd_entry = &pd_ref.entries[vaddr.pd_index()];
            if !pd_entry.is_present() {
                return Err("PD entry not present");
            }

            if pd_entry.is_huge() {
                return Err("Cannot unmap 4K page from 2M mapping");
            }

            let pt = pd_entry.addr() as *mut PageTable;
            let pt_ref = &mut *pt;
            pt_ref.entries[vaddr.pt_index()].clear();

            // Invalidate TLB for this address
            invalidate_page(virt_addr);
        }

        Ok(())
    }

    /// Translate virtual address to physical address
    pub fn translate(&self, pml4: *const PageTable, virt_addr: u64) -> Option<u64> {
        let vaddr = VirtAddr::new(virt_addr);

        unsafe {
            let pml4_ref = &*pml4;
            let pml4_entry = &pml4_ref.entries[vaddr.pml4_index()];
            if !pml4_entry.is_present() {
                return None;
            }

            let pdpt = pml4_entry.addr() as *const PageTable;
            let pdpt_ref = &*pdpt;
            let pdpt_entry = &pdpt_ref.entries[vaddr.pdpt_index()];
            if !pdpt_entry.is_present() {
                return None;
            }

            // Check for 1GB huge page
            if pdpt_entry.is_huge() {
                let offset = virt_addr & (PAGE_SIZE_1G as u64 - 1);
                return Some(pdpt_entry.addr() + offset);
            }

            let pd = pdpt_entry.addr() as *const PageTable;
            let pd_ref = &*pd;
            let pd_entry = &pd_ref.entries[vaddr.pd_index()];
            if !pd_entry.is_present() {
                return None;
            }

            // Check for 2MB huge page
            if pd_entry.is_huge() {
                let offset = virt_addr & (PAGE_SIZE_2M as u64 - 1);
                return Some(pd_entry.addr() + offset);
            }

            let pt = pd_entry.addr() as *const PageTable;
            let pt_ref = &*pt;
            let pt_entry = &pt_ref.entries[vaddr.pt_index()];
            if !pt_entry.is_present() {
                return None;
            }

            Some(pt_entry.addr() + vaddr.page_offset())
        }
    }

    /// Map a range of pages (4KB pages)
    pub fn map_range(
        &self,
        pml4: *mut PageTable,
        virt_start: u64,
        phys_start: u64,
        size: usize,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let pages = (size + PAGE_SIZE_4K - 1) / PAGE_SIZE_4K;

        for i in 0..pages {
            let virt = virt_start + (i * PAGE_SIZE_4K) as u64;
            let phys = phys_start + (i * PAGE_SIZE_4K) as u64;
            self.map_4k(pml4, virt, phys, flags)?;
        }

        Ok(())
    }

    /// Allocate and map pages for a given virtual address range
    pub fn allocate_and_map(
        &self,
        pml4: *mut PageTable,
        virt_start: u64,
        size: usize,
        flags: PageTableFlags,
    ) -> Result<(), &'static str> {
        let pages = (size + PAGE_SIZE_4K - 1) / PAGE_SIZE_4K;

        for i in 0..pages {
            let virt = virt_start + (i * PAGE_SIZE_4K) as u64;

            // Allocate a physical frame
            let frame = frame_manager_instance()
                .allocate(1)
                .map_err(|_| "Failed to allocate frame")?;

            let phys = frame.phys_addr() as u64;

            // Zero the frame
            unsafe {
                core::ptr::write_bytes(phys as *mut u8, 0, PAGE_SIZE_4K);
            }

            self.map_4k(pml4, virt, phys, flags)?;
        }

        Ok(())
    }
}

/// Invalidate TLB entry for a specific address
#[inline]
pub fn invalidate_page(addr: u64) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}

/// Flush the entire TLB by reloading CR3
#[inline]
pub fn flush_tlb() {
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack, preserves_flags));
    }
}

/// Get current CR3 value
#[inline]
pub fn get_cr3() -> u64 {
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nostack, preserves_flags));
        cr3
    }
}

/// Set CR3 value
#[inline]
pub unsafe fn set_cr3_inline(value: u64) {
    core::arch::asm!("mov cr3, {}", in(reg) value, options(nostack, preserves_flags));
}

/// Get kernel PML4 address
pub fn get_kernel_pml4() -> *mut PageTable {
    unsafe { addr_of_mut!(KERNEL_PML4) }
}

// External assembly function for CR3
extern "C" {
    fn set_cr3(value: u64);
}

/// Initialize the kernel page tables with identity mapping
/// This is called early in the boot process
pub unsafe fn initialize() {
    let pml4_ptr = addr_of_mut!(KERNEL_PML4);
    let pdpt_ptr = addr_of_mut!(KERNEL_PDPT);
    let pd_ptr = addr_of_mut!(KERNEL_PD);

    // Set up PML4 -> PDPT mapping
    (*pml4_ptr).entries[0].set(
        pdpt_ptr as u64,
        PageTableFlags::KERNEL_RW,
    );

    // Set up PDPT -> PD mappings and PD entries for 2MB pages
    for i_pdpt in 0..PAGE_DIRECTORY_COUNT {
        let pd_entry_ptr = addr_of_mut!((*pd_ptr)[i_pdpt]);
        (*pdpt_ptr).entries[i_pdpt].set(
            pd_entry_ptr as u64,
            PageTableFlags::KERNEL_RW,
        );

        for i_pd in 0..PAGE_TABLE_ENTRIES {
            let phys_addr = (i_pdpt * PAGE_SIZE_1G + i_pd * PAGE_SIZE_2M) as u64;
            (*pd_entry_ptr).entries[i_pd].set(phys_addr, PageTableFlags::KERNEL_HUGE);
        }
    }

    // Load the new page table
    set_cr3(pml4_ptr as u64);

    // Initialize the page table manager
    PAGE_TABLE_MANAGER.lock().initialize();
}

/// Set up user space page tables for a process
/// Returns the physical address of the PML4 table
pub fn setup_user_page_table(
    program_start: u64,
    program_size: usize,
    stack_top: u64,
    stack_size: usize,
) -> Result<u64, &'static str> {
    let manager = PAGE_TABLE_MANAGER.lock();

    // Create new PML4 with kernel mappings
    let pml4 = manager.create_user_page_table()
        .ok_or("Failed to create user page table")?;

    // Map program area with user read/write/execute permissions
    // For simplicity, we use identity mapping for the program
    let program_pages = (program_size + PAGE_SIZE_4K - 1) / PAGE_SIZE_4K;
    for i in 0..program_pages {
        let addr = program_start + (i * PAGE_SIZE_4K) as u64;
        manager.map_4k(pml4, addr, addr, PageTableFlags::USER_RW)?;
    }

    // Allocate and map user stack
    let stack_bottom = stack_top - stack_size as u64;
    manager.allocate_and_map(
        pml4,
        stack_bottom,
        stack_size,
        PageTableFlags::USER_RW,
    )?;

    Ok(pml4 as u64)
}

/// Page fault error code flags
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct PageFaultErrorCode(u64);

impl PageFaultErrorCode {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const RESERVED_WRITE: u64 = 1 << 3;
    pub const INSTRUCTION_FETCH: u64 = 1 << 4;
    pub const PROTECTION_KEY: u64 = 1 << 5;
    pub const SHADOW_STACK: u64 = 1 << 6;
    pub const SGX: u64 = 1 << 15;

    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn bits(&self) -> u64 {
        self.0
    }

    pub fn is_present(&self) -> bool {
        self.0 & Self::PRESENT != 0
    }

    pub fn is_write(&self) -> bool {
        self.0 & Self::WRITE != 0
    }

    pub fn is_user(&self) -> bool {
        self.0 & Self::USER != 0
    }

    pub fn is_instruction_fetch(&self) -> bool {
        self.0 & Self::INSTRUCTION_FETCH != 0
    }
}

impl core::fmt::Display for PageFaultErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PageFaultError {{ ")?;
        if self.is_present() {
            write!(f, "PROTECTION_VIOLATION ")?;
        } else {
            write!(f, "PAGE_NOT_PRESENT ")?;
        }
        if self.is_write() {
            write!(f, "WRITE ")?;
        } else {
            write!(f, "READ ")?;
        }
        if self.is_user() {
            write!(f, "USER ")?;
        } else {
            write!(f, "KERNEL ")?;
        }
        if self.is_instruction_fetch() {
            write!(f, "INSTRUCTION_FETCH ")?;
        }
        write!(f, "}}")
    }
}

/// Get the faulting address from CR2
#[inline]
pub fn get_cr2() -> u64 {
    unsafe {
        let cr2: u64;
        core::arch::asm!("mov {}, cr2", out(reg) cr2, options(nostack, preserves_flags));
        cr2
    }
}

/// Page fault handler
/// This is called when a page fault exception occurs
pub fn handle_page_fault(error_code: u64) {
    let faulting_address = get_cr2();
    let error = PageFaultErrorCode::from_bits(error_code);

    crate::error!(
        "PAGE FAULT at address 0x{:016x}",
        faulting_address
    );
    crate::error!("Error code: {}", error);
    crate::error!("  Present: {}", error.is_present());
    crate::error!("  Write: {}", error.is_write());
    crate::error!("  User: {}", error.is_user());
    crate::error!("  Instruction fetch: {}", error.is_instruction_fetch());

    // Check if this is a user-mode page fault that we can handle
    if error.is_user() && !error.is_present() {
        // This might be a demand paging situation
        // For now, we'll try to handle stack growth
        let stack_bottom = USER_STACK_TOP - USER_STACK_SIZE as u64;
        let extended_stack_bottom = stack_bottom - (64 * PAGE_SIZE_4K) as u64; // Allow 256KB extra

        if faulting_address >= extended_stack_bottom && faulting_address < USER_STACK_TOP {
            // This is a stack access - try to allocate the page
            crate::info!("Attempting to handle stack page fault at 0x{:016x}", faulting_address);

            let page_addr = faulting_address & !0xFFF; // Align to page boundary

            // Get current page table
            let cr3 = get_cr3();
            let pml4 = cr3 as *mut PageTable;

            let manager = PAGE_TABLE_MANAGER.lock();

            // Allocate and map the page
            match manager.allocate_and_map(pml4, page_addr, PAGE_SIZE_4K, PageTableFlags::USER_RW) {
                Ok(()) => {
                    crate::info!("Successfully mapped page at 0x{:016x}", page_addr);
                    return; // Page fault handled, return to continue execution
                }
                Err(e) => {
                    crate::error!("Failed to map page: {}", e);
                }
            }
        }
    }

    // Unhandled page fault - this is fatal
    crate::error!("Unhandled page fault - halting");

    // Print additional debug info
    crate::error!("CR3: 0x{:016x}", get_cr3());

    // Halt the system
    loop {
        unsafe {
            core::arch::asm!("hlt");
        }
    }
}
