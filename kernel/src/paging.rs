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

// Virtual address space layout (kernel code model compatible - top 2GB)
pub const KERNEL_BASE: u64 = 0xFFFF_FFFF_8000_0000; // Higher half kernel (top 2GB)
pub const USER_STACK_TOP: u64 = 0x0000_7FFF_FFFF_0000;
pub const USER_STACK_SIZE: usize = 64 * 1024; // 64 KB

// PML4 index for higher half kernel (0xFFFFFFFF80000000 >> 39) & 0x1FF = 511
pub const KERNEL_PML4_INDEX: usize = 511;

/// Convert physical address to kernel virtual address
#[inline]
pub const fn phys_to_virt(phys: u64) -> u64 {
    phys + KERNEL_BASE
}

/// Convert kernel virtual address to physical address
#[inline]
pub const fn virt_to_phys(virt: u64) -> u64 {
    virt - KERNEL_BASE
}

/// Convert physical address to pointer (for kernel access)
#[inline]
pub fn phys_to_ptr<T>(phys: u64) -> *mut T {
    phys_to_virt(phys) as *mut T
}

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

    /// Remove flags from self (self & !other)
    pub const fn difference(self, other: Self) -> Self {
        Self(self.0 & !other.0)
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
static mut KERNEL_PDPT: PageTable = PageTable::new();       // For higher half (PML4[511])
static mut KERNEL_PDPT_LOW: PageTable = PageTable::new();   // For identity mapping (PML4[0])
static mut KERNEL_PD: [PageTable; PAGE_DIRECTORY_COUNT] = [const { PageTable::new() }; PAGE_DIRECTORY_COUNT];

// Global page table manager
pub static PAGE_TABLE_MANAGER: Mutex<PageTableManager> = Mutex::new(PageTableManager::new());

// Kernel CR3 value (physical address of kernel PML4)
// This is set during initialization and used by syscall handler to switch to kernel page table
#[no_mangle]
pub static mut KERNEL_CR3: u64 = 0;

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
    /// Returns the physical address of the allocated page table
    pub fn allocate_page_table_phys(&self) -> Option<u64> {
        let frame = frame_manager_instance().allocate(1).ok()?;
        let phys = frame.phys_addr() as u64;
        let ptr = phys_to_ptr::<PageTable>(phys);

        // Zero out the page table via virtual address
        unsafe {
            core::ptr::write_bytes(ptr, 0, 1);
        }

        Some(phys)
    }

    /// Allocate a new page table and return virtual pointer
    /// For backward compatibility
    pub fn allocate_page_table(&self) -> Option<*mut PageTable> {
        let phys = self.allocate_page_table_phys()?;
        Some(phys_to_ptr::<PageTable>(phys))
    }

    /// Free a page table (takes virtual address)
    pub fn free_page_table(&self, table: *mut PageTable) {
        // Convert virtual address to physical for frame manager
        let phys = virt_to_phys(table as u64);
        let frame = FrameID::from_phys_addr(phys as *mut u8);
        frame_manager_instance().free(frame, 1);
    }

    /// Create a new user page table with kernel mappings
    /// Returns (physical_address, virtual_pointer)
    pub fn create_user_page_table(&self) -> Option<(u64, *mut PageTable)> {
        let phys = self.allocate_page_table_phys()?;
        let pml4 = phys_to_ptr::<PageTable>(phys);

        unsafe {
            // Copy kernel mappings from kernel page table
            let kernel_pml4 = core::ptr::addr_of!(KERNEL_PML4);

            // For PML4[0] (low addresses where user programs live), we need to create
            // a COPY of the page table hierarchy, not just copy the pointer.
            // This is because we may need to modify entries (e.g., split huge pages)
            // without affecting the kernel's page tables.
            let kernel_pml4_0 = (*kernel_pml4).entries[0];
            if kernel_pml4_0.is_present() {
                // Create a new PDPT for this user process
                let user_pdpt_phys = self.allocate_page_table_phys()?;
                let user_pdpt = phys_to_ptr::<PageTable>(user_pdpt_phys);
                
                // Copy entries from kernel's PDPT
                let kernel_pdpt = phys_to_ptr::<PageTable>(kernel_pml4_0.addr());
                for i in 0..PAGE_TABLE_ENTRIES {
                    let kernel_pdpt_entry = (*kernel_pdpt).entries[i];
                    if kernel_pdpt_entry.is_present() {
                        // Create a new PD for this PDPT entry
                        let user_pd_phys = self.allocate_page_table_phys()?;
                        let user_pd = phys_to_ptr::<PageTable>(user_pd_phys);
                        
                        // Copy PD entries from kernel
                        let kernel_pd = phys_to_ptr::<PageTable>(kernel_pdpt_entry.addr());
                        core::ptr::copy_nonoverlapping(
                            (*kernel_pd).entries.as_ptr(),
                            (*user_pd).entries.as_mut_ptr(),
                            PAGE_TABLE_ENTRIES,
                        );
                        
                        // Set user PDPT entry to point to copied PD
                        (*user_pdpt).entries[i].set(user_pd_phys, kernel_pdpt_entry.flags());
                    }
                }
                
                // Set user PML4[0] to point to new PDPT
                (*pml4).entries[0].set(user_pdpt_phys, kernel_pml4_0.flags());
            }

            // Copy higher half kernel mappings (PML4[511])
            // This ensures kernel code/data is accessible when switching to kernel mode
            // We can share this directly since user code won't modify kernel mappings
            (*pml4).entries[KERNEL_PML4_INDEX] = (*kernel_pml4).entries[KERNEL_PML4_INDEX];
        }

        Some((phys, pml4))
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

            // Check if PD entry is a huge page - if so, we need to split it
            let pd_ref = &mut *pd;
            let pd_entry = &mut pd_ref.entries[vaddr.pd_index()];
            
            let pt = if pd_entry.is_present() && pd_entry.is_huge() {
                // Split the 2MB huge page into 512 x 4KB pages
                self.split_huge_page(pd_entry, flags)?
            } else {
                // Get or create PT normally
                self.get_or_create_table(pd_entry, flags)?
            };

            // Set the page table entry
            let pt_ref = &mut *pt;
            pt_ref.entries[vaddr.pt_index()].set(phys_addr, flags);
        }

        Ok(())
    }

    /// Split a 2MB huge page into 512 x 4KB pages
    /// Returns the virtual pointer to the new page table
    unsafe fn split_huge_page(
        &self,
        pd_entry: &mut PageTableEntry,
        flags: PageTableFlags,
    ) -> Result<*mut PageTable, &'static str> {
        // Get the base physical address of the huge page
        let huge_page_phys = pd_entry.addr();
        let old_flags = pd_entry.flags();
        
        // Allocate a new page table
        let pt_phys = self.allocate_page_table_phys()
            .ok_or("Failed to allocate page table for splitting huge page")?;
        let pt = phys_to_ptr::<PageTable>(pt_phys);
        
        // Fill the page table with 512 entries mapping the same physical region
        // Preserve the original flags but remove HUGE_PAGE and add USER_ACCESSIBLE if needed
        let base_flags = if flags.contains(PageTableFlags::USER_ACCESSIBLE) {
            old_flags.union(PageTableFlags::USER_ACCESSIBLE).difference(PageTableFlags::HUGE_PAGE)
        } else {
            old_flags.difference(PageTableFlags::HUGE_PAGE)
        };
        
        for i in 0..PAGE_TABLE_ENTRIES {
            let page_phys = huge_page_phys + (i * PAGE_SIZE_4K) as u64;
            (*pt).entries[i].set(page_phys, base_flags);
        }
        
        // Update the PD entry to point to the new page table
        let table_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE |
            if flags.contains(PageTableFlags::USER_ACCESSIBLE) {
                PageTableFlags::USER_ACCESSIBLE
            } else {
                PageTableFlags::empty()
            };
        pd_entry.set(pt_phys, table_flags);
        
        // Flush TLB for the entire 2MB region
        let virt_base = huge_page_phys; // For identity mapping, virt == phys
        for i in 0..PAGE_TABLE_ENTRIES {
            invalidate_page(virt_base + (i * PAGE_SIZE_4K) as u64);
        }
        
        Ok(pt)
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
    /// Returns virtual pointer to the table
    unsafe fn get_or_create_table(
        &self,
        entry: &mut PageTableEntry,
        flags: PageTableFlags,
    ) -> Result<*mut PageTable, &'static str> {
        if entry.is_present() {
            // Table already exists - entry contains physical address
            // If we need USER_ACCESSIBLE and it's not set, update the entry
            let current_flags = entry.flags();
            if flags.contains(PageTableFlags::USER_ACCESSIBLE) 
                && !current_flags.contains(PageTableFlags::USER_ACCESSIBLE) 
            {
                // Add USER_ACCESSIBLE flag to existing entry
                let new_flags = current_flags | PageTableFlags::USER_ACCESSIBLE;
                entry.set(entry.addr(), new_flags);
            }
            // Convert to virtual address for access
            let phys = entry.addr();
            Ok(phys_to_ptr::<PageTable>(phys))
        } else {
            // Create new table - get physical address
            let phys = self.allocate_page_table_phys()
                .ok_or("Failed to allocate page table")?;

            // Set entry to point to new table (store physical address)
            // Use USER_ACCESSIBLE flag if the mapping is for user space
            let table_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE |
                if flags.contains(PageTableFlags::USER_ACCESSIBLE) {
                    PageTableFlags::USER_ACCESSIBLE
                } else {
                    PageTableFlags::empty()
                };
            entry.set(phys, table_flags);

            // Return virtual pointer for access
            Ok(phys_to_ptr::<PageTable>(phys))
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

            // Convert physical addresses from entries to virtual for access
            let pdpt = phys_to_ptr::<PageTable>(pml4_entry.addr());
            let pdpt_ref = &*pdpt;
            let pdpt_entry = &pdpt_ref.entries[vaddr.pdpt_index()];
            if !pdpt_entry.is_present() {
                return Err("PDPT entry not present");
            }

            let pd = phys_to_ptr::<PageTable>(pdpt_entry.addr());
            let pd_ref = &*pd;
            let pd_entry = &pd_ref.entries[vaddr.pd_index()];
            if !pd_entry.is_present() {
                return Err("PD entry not present");
            }

            if pd_entry.is_huge() {
                return Err("Cannot unmap 4K page from 2M mapping");
            }

            let pt = phys_to_ptr::<PageTable>(pd_entry.addr());
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

            // Convert physical addresses from entries to virtual for access
            let pdpt = phys_to_ptr::<PageTable>(pml4_entry.addr()) as *const PageTable;
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

            let pd = phys_to_ptr::<PageTable>(pdpt_entry.addr()) as *const PageTable;
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

            let pt = phys_to_ptr::<PageTable>(pd_entry.addr()) as *const PageTable;
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

            // Zero the frame via virtual address
            unsafe {
                let virt_ptr = phys_to_virt(phys) as *mut u8;
                core::ptr::write_bytes(virt_ptr, 0, PAGE_SIZE_4K);
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

/// Initialize the kernel page tables with higher half mapping
/// This is called early in the boot process (after boot page tables are set up)
/// The kernel is now running at virtual address KERNEL_BASE + physical_address
/// 
/// For 0xFFFFFFFF80000000:
/// - PML4 index = 511 (bits 39-47)
/// - PDPT index = 510 (bits 30-38)
pub unsafe fn initialize() {
    let pml4_ptr = addr_of_mut!(KERNEL_PML4);
    let pdpt_ptr = addr_of_mut!(KERNEL_PDPT);
    let pdpt_low_ptr = addr_of_mut!(KERNEL_PDPT_LOW);
    let pd_ptr = addr_of_mut!(KERNEL_PD);

    // PDPT index for 0xFFFFFFFF80000000 is 510
    const PDPT_START_INDEX: usize = 510;

    // ===== Set up identity mapping (PML4[0]) for low addresses =====
    // This is needed to access bootloader-provided data (fb_config, memory_map, etc.)
    (*pml4_ptr).entries[0].set(
        virt_to_phys(pdpt_low_ptr as u64),
        PageTableFlags::KERNEL_RW,
    );

    // Set up PDPT_LOW -> PD mappings for identity mapping (first 64GB)
    for i in 0..PAGE_DIRECTORY_COUNT {
        let pd_entry_ptr = addr_of_mut!((*pd_ptr)[i]);
        (*pdpt_low_ptr).entries[i].set(
            virt_to_phys(pd_entry_ptr as u64),
            PageTableFlags::KERNEL_RW,
        );
    }

    // ===== Set up higher half mapping (PML4[511]) for kernel =====
    // Note: pml4_ptr, pdpt_ptr, pd_ptr are virtual addresses (linker placed them there)
    // Page table entries need physical addresses
    (*pml4_ptr).entries[KERNEL_PML4_INDEX].set(
        virt_to_phys(pdpt_ptr as u64),
        PageTableFlags::KERNEL_RW,
    );

    // Set up PDPT -> PD mappings and PD entries for 2MB pages
    // PDPT[510] and PDPT[511] map to our PD entries (2GB total for kernel space)
    // This maps physical memory starting at 0 to virtual KERNEL_BASE
    // 
    // We have PAGE_DIRECTORY_COUNT (64) page directories, which can map 64GB
    // But for the higher half kernel at 0xFFFFFFFF80000000, we only have 
    // PDPT entries 510 and 511 available (2GB of virtual address space)
    // So we limit to 2 PDPT entries
    let pdpt_entries_to_use = PAGE_DIRECTORY_COUNT.min(2); // Only 2 entries (510, 511) for top 2GB
    
    for i in 0..pdpt_entries_to_use {
        let pdpt_index = PDPT_START_INDEX + i;
        let pd_entry_ptr = addr_of_mut!((*pd_ptr)[i]);
        (*pdpt_ptr).entries[pdpt_index].set(
            virt_to_phys(pd_entry_ptr as u64),
            PageTableFlags::KERNEL_RW,
        );
    }

    // Set up PD entries with 2MB huge pages
    // These are shared between identity mapping and higher half mapping
    for i in 0..PAGE_DIRECTORY_COUNT {
        let pd_entry_ptr = addr_of_mut!((*pd_ptr)[i]);
        for i_pd in 0..PAGE_TABLE_ENTRIES {
            // Physical address: i-th GB + i_pd * 2MB
            let phys_addr = (i * PAGE_SIZE_1G + i_pd * PAGE_SIZE_2M) as u64;
            (*pd_entry_ptr).entries[i_pd].set(phys_addr, PageTableFlags::KERNEL_HUGE);
        }
    }

    // Load the new page table (CR3 needs physical address)
    let kernel_cr3_value = virt_to_phys(pml4_ptr as u64);
    set_cr3(kernel_cr3_value);

    // Save kernel CR3 for syscall handler to use
    KERNEL_CR3 = kernel_cr3_value;

    // Initialize the page table manager
    PAGE_TABLE_MANAGER.lock().initialize();
}

/// Set up user space page tables for a process
/// Returns the physical address of the PML4 table (for CR3)
pub fn setup_user_page_table(
    program_start: u64,
    program_size: usize,
    stack_top: u64,
    stack_size: usize,
) -> Result<u64, &'static str> {
    let manager = PAGE_TABLE_MANAGER.lock();

    // Create new PML4 with kernel mappings
    // Returns (physical_address, virtual_pointer)
    let (pml4_phys, pml4) = manager.create_user_page_table()
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

    // Return physical address for CR3
    Ok(pml4_phys)
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

            // Get current page table - CR3 contains physical address
            let cr3 = get_cr3();
            let pml4 = phys_to_ptr::<PageTable>(cr3);

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
