use super::SyscallError;
use crate::paging::PageTableFlags;
use crate::paging::PAGE_SIZE_4K;
use crate::paging::{align_up, VirtAddr, PAGE_TABLE_MANAGER};
use crate::PROCESS_MANAGER;
use alloc::vec::Vec;
use horse_abi::mm::{MapFlags, Prot};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VmAreaType {
    AnonymousPrivate, // e.g. heap
    Reserved,         // kernel-side mappings the user must not clobber via mmap
                      //SharedMemory,   // e.g. shared memory
                      //FileMapping,    // e.g. memory-mapped files}
}

#[derive(Debug, Clone)]
pub struct VmArea {
    pub start: VirtAddr,
    pub end: VirtAddr,
    pub flags: PageTableFlags,
    pub area_type: VmAreaType,
}

impl VmArea {
    pub fn contains(&self, addr: VirtAddr) -> bool {
        self.start <= addr && addr < self.end
    }
}

pub struct VmAreaList {
    areas: Vec<VmArea>,
}

impl VmAreaList {
    pub fn new() -> Self {
        Self { areas: Vec::new() }
    }

    pub fn find(&self, addr: VirtAddr) -> Option<&VmArea> {
        self.areas.iter().find(|area| area.contains(addr))
    }

    pub fn insert(&mut self, area: VmArea) {
        self.areas.push(area);
        self.areas.sort_by_key(|a| a.start);
    }

    pub fn extend_heap(&mut self, new_end: VirtAddr) -> Result<(), ()> {
        if let Some(area) = self
            .areas
            .iter_mut()
            .find(|a| a.area_type == VmAreaType::AnonymousPrivate)
        {
            if new_end > area.end {
                area.end = new_end;
                return Ok(());
            }
        }
        Err(())
    }
}

pub fn sys_brk(addr: u64) -> isize {
    let pm_once = PROCESS_MANAGER.lock();
    let pm = pm_once.get().expect("failed to get process manager");
    let current_proc = pm.current_proc();
    drop(pm_once); // release lock before locking process

    let mut proc_guard = current_proc.lock();
    let current_brk = match proc_guard.brk {
        Some(b) => b,
        None => return SyscallError::NoMem as isize,
    };

    if addr == 0 {
        return current_brk.as_u64() as isize;
    }

    let new_brk = VirtAddr::new(addr);

    // For simplicity, we only allow increasing the break (i.e. expanding the heap).
    if new_brk < current_brk {
        return new_brk.as_u64() as isize;
    }

    if proc_guard.vm_areas.extend_heap(new_brk).is_err() {
        return current_brk.as_u64() as isize;
    }

    proc_guard.brk = Some(new_brk);
    new_brk.as_u64() as isize
}

pub fn sys_mmap(addr: u64, length: u64, prot: u64, flags: u64, fd: i64, offset: u64) -> isize {
    if flags & MapFlags::Anonymous as u64 == 0 || flags & MapFlags::Private as u64 == 0 {
        return SyscallError::InvalidArg as isize; // Only support anonymous private mappings for now
    }

    if fd != -1 || offset != 0 {
        return SyscallError::InvalidArg as isize; // File-backed mappings not supported
    }

    if length == 0 {
        return SyscallError::InvalidArg as isize; // Length must be non-zero
    }

    let length = align_up(length, PAGE_SIZE_4K as u64);

    let pm_once = PROCESS_MANAGER.lock();
    let pm = pm_once.get().expect("failed to get process manager");
    let current_proc = pm.current_proc();
    drop(pm_once); // release lock before locking process

    let mut proc = current_proc.lock();

    let virt_start = if addr != 0 {
        VirtAddr::new(addr)
    } else {
        match find_free_virt_addr(&proc.vm_areas, proc.brk, proc.user_stack_bottom, length) {
            Some(a) => a,
            None => return SyscallError::NoMem as isize,
        }
    };

    let mut flags_pt = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;

    if prot & Prot::Write as u64 != 0 {
        flags_pt |= PageTableFlags::WRITABLE;
    }
    if prot & Prot::Exec as u64 != 0 {
        flags_pt |= PageTableFlags::NO_EXECUTE; // For simplicity, we use the execute-disable bit to indicate executable pages
    }

    // Get the user's page table pointer before dropping the lock.
    let pml4 = proc.page_table() as *mut _;

    proc.vm_areas.insert(VmArea {
        start: virt_start,
        end: VirtAddr::new(virt_start.as_u64() + length),
        flags: flags_pt,
        area_type: VmAreaType::AnonymousPrivate,
    });

    drop(proc);

    // todo: we should ideally allocate physical frames lazily on page faults, but that requires more complex handling in check_vmarea. For now, we eagerly allocate and map the frames here.

    // The user's page table inherits the kernel's identity map (VA == PA for 0–64 GB)
    // via 2 MB huge pages. Those huge pages are already *present*, so a heap write
    // never triggers a page fault and lazy allocation in check_vmarea is never reached.
    // We must therefore allocate real physical frames now and install them in the user's
    // PT, splitting any 2 MB huge-page entries that cover the requested range.
    {
        let manager = PAGE_TABLE_MANAGER.lock();
        if manager
            .allocate_and_map(pml4, virt_start.as_u64(), length as usize, flags_pt)
            .is_err()
        {
            return SyscallError::NoMem as isize;
        }
    }

    virt_start.as_u64() as isize
}

fn find_free_virt_addr(
    vm_areas: &VmAreaList,
    brk: Option<VirtAddr>,
    user_stack_bottom: Option<u64>,
    length: u64,
) -> Option<VirtAddr> {
    let current_brk = match brk {
        Some(b) => b,
        None => return None,
    };
    let current_stack_bottom = user_stack_bottom.unwrap_or(0xFFFF_FFFF_FFFF_F000);

    let search_start = current_brk;
    let search_end = VirtAddr::new(current_stack_bottom);

    let mut candidate = search_start;

    for area in vm_areas.areas.iter() {
        if candidate.as_u64() + length <= area.start.as_u64() {
            // Found a gap large enough for the new area
            return Some(candidate);
        }
        if area.end > candidate {
            candidate = area.end; // Move candidate to the end of the current area
        }
    }
    // Check if we can fit the new area after the last existing area
    if candidate.as_u64() + length <= search_end.as_u64() {
        return Some(candidate);
    } else {
        return None; // No suitable gap found
    }
}
