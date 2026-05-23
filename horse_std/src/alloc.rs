use crate::mm::mmap;
use core::alloc::{GlobalAlloc, Layout};
use horse_abi::mm::{MapFlags, Prot};
use spin::Mutex;

const INITIAL_HEAP_SIZE: usize = 8 * 1024 * 1024;
const GROW_SIZE: usize = 4 * 1024 * 1024;
const PAGE_SIZE: usize = 4096;

struct HeapState {
    heap_start: usize,
    heap_end: usize,
    next: usize,
}

pub struct BumpAllocator {
    inner: Mutex<HeapState>,
}

impl BumpAllocator {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(HeapState {
                heap_start: 0,
                heap_end: 0,
                next: 0,
            }),
        }
    }
}

#[inline]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

fn map_chunk(min_len: usize) -> Option<(usize, usize)> {
    let len = align_up(min_len, PAGE_SIZE);
    let addr = mmap(
        0,
        len,
        Prot::Read as u64 | Prot::Write as u64,
        MapFlags::Anonymous as u64 | MapFlags::Private as u64,
        -1,
        0,
    )
    .ok()?;
    if (addr as isize) <= 0 {
        return None;
    }
    Some((addr, len))
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut state = self.inner.lock();

        if state.heap_start == 0 {
            let need = INITIAL_HEAP_SIZE.max(layout.size() + layout.align());
            let (start, len) = match map_chunk(need) {
                Some(v) => v,
                None => return core::ptr::null_mut(),
            };
            state.heap_start = start;
            state.next = start;
            state.heap_end = start + len;
        }

        let aligned = align_up(state.next, layout.align());
        let new_next = aligned.checked_add(layout.size());
        let fits = matches!(new_next, Some(n) if n <= state.heap_end);

        if !fits {
            let need = GROW_SIZE.max(layout.size() + layout.align());
            let (start, len) = match map_chunk(need) {
                Some(v) => v,
                None => return core::ptr::null_mut(),
            };
            state.heap_start = start;
            state.next = start;
            state.heap_end = start + len;

            let aligned = align_up(state.next, layout.align());
            let new_next = match aligned.checked_add(layout.size()) {
                Some(n) if n <= state.heap_end => n,
                _ => return core::ptr::null_mut(),
            };
            state.next = new_next;
            return aligned as *mut u8;
        }

        state.next = new_next.unwrap();
        aligned as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
