use core::{
    mem::MaybeUninit,
    ops::{
        Index,
        IndexMut
    }
};

const PAGE_DIRECTORY_COUNT: usize = 64;
const PAGE_SIZE_4K: usize = 4096;
const PAGE_SIZE_2M: usize = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: usize = 512 * PAGE_SIZE_2M;

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct PageTable {
    table: [MaybeUninit<u64>; 512]
}

impl Index<usize> for PageTable {
    type Output = MaybeUninit<u64>;

    fn index(&self, index: usize) -> &Self::Output {
        return &self.table[index];
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        return &mut self.table[index];
    }
}

impl PageTable {
    const fn new() -> Self {
        return Self{ table: [MaybeUninit::<u64>::uninit(); 512] }
    }
}

static mut PML4_TABLE: PageTable = PageTable::new();
static mut PDP_TABLE: PageTable = PageTable::new();
static mut PAGE_DIRECTORY: [PageTable; PAGE_DIRECTORY_COUNT] = [PageTable::new(); PAGE_DIRECTORY_COUNT];

pub unsafe fn initialize() {
    PML4_TABLE[0].write(&PDP_TABLE[0] as *const MaybeUninit<u64> as u64 | 0x003);
    for i_pdpt in 0..PAGE_DIRECTORY_COUNT {
        PDP_TABLE[i_pdpt].write(&PAGE_DIRECTORY[i_pdpt] as *const PageTable as u64 | 0x003);
        for i_pd in 0..512 {
            PAGE_DIRECTORY[i_pdpt][i_pd].write((i_pdpt * PAGE_SIZE_1G + i_pd * PAGE_SIZE_2M) as u64 | 0x083);
        }
    }
    set_cr3(&PML4_TABLE[0] as *const MaybeUninit<u64> as u64);
}

//assembly function in asm.s
extern "C" {
    fn set_cr3(value: u64);
}