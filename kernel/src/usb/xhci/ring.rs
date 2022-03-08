use core::ptr::{
    null_mut,
    addr_of,
    addr_of_mut
};
use core::mem::{
    MaybeUninit,
    transmute
};
use crate::{
    status::StatusCode,
    bit_getter,
    bit_setter,
    trace,
    volatile::Volatile
};
use super::{
    ALLOC,
    trb::{
        GenericTrb,
        Link,
        Trb
    },
    InterrupterRegisterSet
};

pub struct Ring {
    buf: &'static mut [GenericTrb],
    pub cycle_bit: bool,
    write_idx: usize
}

impl Ring {
    pub fn with_capacity(buf_size: usize) -> Result<Self, StatusCode> {
        let buf: &mut [MaybeUninit<GenericTrb>] = unsafe {
            ALLOC
                .lock()
                .alloc_slice_ext::<GenericTrb>(buf_size, 64, Some(64 * 1024))
                .ok_or(StatusCode::NoEnoughMemory)?
                .as_mut()  
        };
        for p in buf.iter_mut() {
            *p = MaybeUninit::zeroed();
        }
        let buf = unsafe {
            transmute::<&mut [MaybeUninit<GenericTrb>], &mut [GenericTrb]>(buf)
        };
        Ok(Self{
            buf,
            cycle_bit: true,
            write_idx: 0
        })
    }
    
    pub fn buffer_ptr(&self) -> *const GenericTrb {
        self.buf.as_ptr()
    }
    
    pub fn copy_to_last(&mut self, mut trb: GenericTrb) {
        trb.set_cycle_bit(self.cycle_bit as u8);
        
        let p = &mut self.buf[self.write_idx] as *mut GenericTrb as *mut u32;
        
        for i in 0..3 {
            unsafe { p.add(i).write_volatile(trb.data[i]) };
        }
        
        unsafe { p.add(3).write_volatile(trb.data[3]) };
    }
    
    pub fn push(&mut self, trb: &GenericTrb) -> &GenericTrb {
        self.copy_to_last(trb.clone());
        let written_idx = self.write_idx;
        self.write_idx += 1;
        
        if self.write_idx + 1 == self.buf.len() {
            let mut link = Link::new(self.buffer_ptr() as usize);
            link.set_toggle_cycle(1);
            let trb = <Link as Trb>::upcast(&link);
            self.copy_to_last(trb.clone());
            self.write_idx = 0;
            self.cycle_bit = !self.cycle_bit;
        }
        trace!("TRB (type: {}) pushed: {:?}", trb.trb_type(), trb);
        &self.buf[written_idx]
    }
}

pub type CommandRing = Ring;
pub type TransferRing = Ring;

#[derive(Copy, Clone)]
pub struct EventRing {
    buf: *const [GenericTrb],
    erst: *const [EventRingSegmentTableEntry],
    cycle_bit: bool,
    interrupter: *mut InterrupterRegisterSet
}

impl EventRing {
    pub fn with_capacity(buf_size: usize) -> Result<Self, StatusCode> {
        let mut alloc = ALLOC.lock();
        
        let buf: &mut [MaybeUninit<GenericTrb>] = unsafe {
            alloc
                .alloc_slice_ext::<GenericTrb>(buf_size, 64, Some(64 * 1024))
                .ok_or(StatusCode::NoEnoughMemory)?
                .as_mut()
        };
        for p in buf.iter_mut() {
            *p = MaybeUninit::zeroed();
        }
        let buf = unsafe {
            transmute::<&mut [MaybeUninit<GenericTrb>], &mut [GenericTrb]>(buf)
        };
        let buf = buf as *const [GenericTrb];
        
        let table: &mut [MaybeUninit<EventRingSegmentTableEntry>] = unsafe {
            alloc
                .alloc_slice_ext::<EventRingSegmentTableEntry>(1, 64, Some(64 * 1024))
                .ok_or(StatusCode::NoEnoughMemory)?
                .as_mut()
        };
        for p in table.iter_mut() {
            *p = MaybeUninit::zeroed();
        }
        let table = unsafe {
            transmute::<
                &mut [MaybeUninit<EventRingSegmentTableEntry>],
                &mut [EventRingSegmentTableEntry],
            >(table)
        };
        unsafe {
            table[0].set_pointer((*buf).as_ptr() as usize);
            table[0].set_ring_segment_size((*buf).len() as u16);
        }
        let table = table as *const [EventRingSegmentTableEntry];
        
        Ok(Self {
            buf,
            erst: table,
            cycle_bit: true,
            interrupter: null_mut()
        })
    }
    
    pub fn initialize(&mut self, interrupter: *mut InterrupterRegisterSet) {
        self.interrupter = interrupter;

        let (ptr, len) = unsafe { ((*self.erst).as_ptr(), (*self.erst).len()) };

        unsafe {
            (*interrupter).erstsz.modify(|erstsz| {
                erstsz.set_event_ring_segment_table_size(len as u16);
            })
        };

        self.write_dequeue_pointer(unsafe { (*self.buf).as_ptr() });

        unsafe {
            Volatile::unaligned_modify(addr_of_mut!((*interrupter).erstba), |erstba| {
                erstba.set_pointer(ptr as usize);
            })
        };
    }
    
    pub fn front(&self) -> Option<&GenericTrb> {
        if self.has_front() {
            Some(unsafe { &*self.read_dequeue_pointer() })
        } else {
            None
        }
    }
    
    pub fn pop(&mut self) {
        let mut new_front = unsafe { self.read_dequeue_pointer().add(1) };
        
        {
            let begin = unsafe { (*self.erst)[0].pointer() as *const GenericTrb };
            let size = unsafe { (*self.erst)[0].ring_segment_size() };
            let end = unsafe { begin.add(size as usize) };
            
            if new_front == end {
                new_front = begin;
                self.cycle_bit = !self.cycle_bit;
            }
        }
        
        self.write_dequeue_pointer(new_front);
    }
    
    fn write_dequeue_pointer(&mut self, ptr: *const GenericTrb) {
        unsafe {
            Volatile::unaligned_modify(addr_of_mut!((*self.interrupter).erdp), |erdp| {
                erdp.set_pointer(ptr as usize);
            })
        };
    }
    
    fn read_dequeue_pointer(&self) -> *const GenericTrb {
        unsafe {
            Volatile::unaligned_read(addr_of!((*self.interrupter).erdp)).pointer() as *const GenericTrb
        }
    }
    
    pub fn has_front(&self) -> bool {
        unsafe { (*self.read_dequeue_pointer()).cycle_bit() == self.cycle_bit as u8 }
    }
}

#[repr(C, packed(4))]
struct EventRingSegmentTableEntry {
    data: [u64; 2],
}
impl EventRingSegmentTableEntry {
    bit_getter!(data[0]: u64; 0xFFFFFFFFFFFFFFC0; u64, ring_segment_base_address);
    bit_setter!(data[0]: u64; 0xFFFFFFFFFFFFFFC0; u64, set_ring_segment_base_address);

    bit_getter!(data[1]: u64; 0x000000000000FFFF; u16, pub ring_segment_size);
    bit_setter!(data[1]: u64; 0x000000000000FFFF; u16, pub set_ring_segment_size);

    pub fn pointer(&self) -> usize {
        (self.ring_segment_base_address() << 6) as usize
    }
    pub fn set_pointer(&mut self, ptr: usize) {
        self.set_ring_segment_base_address((ptr as u64) >> 6);
    }
}
