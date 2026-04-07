use crate::paging::{phys_to_ptr, PageTable, PAGE_SIZE_4K, PAGE_TABLE_MANAGER};

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
