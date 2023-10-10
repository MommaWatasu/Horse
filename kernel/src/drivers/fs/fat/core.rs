use alloc::{
    format,
    slice,
    vec,
    string::String,
};
use core::mem::size_of;

use crate::{
    drivers::fs::core::{
        FileSystem,
        STORAGE_CONTROLLERS, FILE_DESCRIPTOR_TABLE
    },
    horse_lib::{fd::{
        File,
        Path
    }, bytes::bytes2str}
};

const END_OF_CLUSTER_CHAIN: u32 = 0x0fffffff;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct BPB {
    jmp_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sec: u16,
    sec_per_clus: u8,
    rsvd_sec_cnt: u16,
    num_fats: u8,
    root_ent_cnt: u16,
    tot_sec_16: u16,
    media: u8,
    fatsz16: u16,
    sec_per_trk: u16,
    num_heads: u16,
    hiddsec: u32,
    tot_sec32: u32,
    fatsz32: u32,
    ext_flags: u16,
    fs_ver: u16,
    root_clus: u32,
    fs_info: u16,
    bk_boot_sec: u16,
    reserved: [u8; 12],
    drv_num: u8,
    reserved1: u8,
    boot_sig: u8,
    vol_id: u32,
    vol_lab: [u8; 11],
    pub fil_sys_type: [u8; 8]
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct DirectoryEntry {
    name: [u8; 11],
    attr: u8,
    nt_reserve: u8,
    crt_time_tenth: u8,
    crt_time: u16,
    crt_date: u16,
    lst_acc_date: u16,
    fst_clus_hi: u16,
    wrt_time: u16,
    wrt_date: u16,
    fst_clus_lo: u16,
    file_size: u32
}

impl DirectoryEntry {
    pub fn first_cluster(&self) -> u32 {
        return self.fst_clus_lo as u32 | ((self.fst_clus_hi as u32) << 16) 
    }
    pub fn file_size(&self) -> u32 { self.file_size }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct LFNEntry {
    ord: u8,
    name1: [u8; 10],
    attr: u8,
    lfn_type: u8,
    checksum: u8,
    name2: [u8; 12],
    fst_cluster: u16,
    name3: [u8; 4]
}

impl LFNEntry {
    pub fn ord(&self) -> usize {
        return (self.ord ^ 0x40) as usize
    }
    pub fn is_end(&self) -> bool {
        return (self.ord & 0x40) != 1
    }
    pub fn get_name(&self) -> [u8; 26] {
        let mut name = [0u8; 26];
        name[..10].copy_from_slice(&self.name1);
        name[10..22].copy_from_slice(&self.name2);
        name[22..26].copy_from_slice(&self.name3);
        return name;
    }
}


enum FATFileAttribute {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeId = 0x08,
    Directory = 0x10,
    Archive = 0x20,
    LongName = 0x0f
}

pub struct FAT {
    storage_id: usize,
    bpb: BPB,
    bpc: usize
}

impl FAT {
    pub fn new(bpb: BPB, storage_id: usize) -> Self {
        let bpc = bpb.bytes_per_sec as usize * bpb.sec_per_clus as usize;
        return Self {
            storage_id,
            bpb,
            bpc
        }
    }
    fn get_cluster_offset(&self, cluster: u32) -> u32 {
        let sector_num = self.bpb.rsvd_sec_cnt as u32  + self.bpb.num_fats as u32 * self.bpb.fatsz32 + (cluster - 2) * self.bpb.sec_per_clus as u32;
        return sector_num * self.bpb.bytes_per_sec as u32
    }
    pub fn get_cluster(&self, cluster: u32, buf: &mut [u8]) {
        let offset = self.get_cluster_offset(cluster);
        let lba = offset / 512;
        let nbytes = self.bpc;
        STORAGE_CONTROLLERS.lock()[self.storage_id].read(buf, lba, nbytes);
    }
    fn next_cluster(&self, cluster: u32) -> u32 {
        let offset = self.bpb.rsvd_sec_cnt as u32 * self.bpb.bytes_per_sec as u32 + 4 * cluster;
        let lba = offset / 512;
        let padding = offset as usize % 512;
        let mut buf = vec![0; 512];
        STORAGE_CONTROLLERS.lock()[self.storage_id].read(&mut buf, lba, 512);
        let next = u32::from_le_bytes(buf[padding..padding+4].try_into().unwrap());
        if next >= 0x0ffffff8 {
            return END_OF_CLUSTER_CHAIN
        }
        return next
    }
    
    fn sfn_cmp(sfn: [u8; 11], name: &str) -> bool {
        let mut name83 = [0x20; 11];

        let mut i = 0;
        let mut i83 = 0;
        let mut found_dot = false;
        for c in name.chars() {
            if c == '.' {
                if found_dot {
                    return false // there are more than two dots
                }
                i83 = 7;
                found_dot = true;
                continue
            }
            if !found_dot && i > 7 {
                return false // there are more than 9 characters before a dot
            }
            name83[i83] = c.to_ascii_uppercase() as u8;
            i += 1;
            i83 += 1;
        }
        return name.chars().count() == i && sfn[..] == name83[..]
    }
    pub fn find_file(&self, full_path: &Path) -> Result<DirectoryEntry, u8> {
        let mut entry: DirectoryEntry;
        let mut dir_clus = self.bpb.root_clus;
        let mut lfn_flag = false;
        let mut lfn = String::from("");
        let mut i = 0;
        let mut name = &full_path.path[i];
        let mut buf = vec![0u8; self.bpc];
        while dir_clus != END_OF_CLUSTER_CHAIN {
            self.get_cluster(dir_clus, &mut buf);
            dir_clus = self.next_cluster(dir_clus);
            for c in 0..self.bpc as usize / size_of::<DirectoryEntry>() {
                let entry_ptr = unsafe { (buf.as_ptr() as *const DirectoryEntry).add(c) };
                unsafe { entry = *entry_ptr; }
                // Long File Name
                if entry.attr == (FATFileAttribute::LongName as u8) {
                    crate::debug!("Long File Name");
                    let lfn_entry = unsafe { *(entry_ptr as *const LFNEntry) };
                        if lfn_entry.is_end() {
                            lfn = String::new();
                        } else if lfn_entry.ord() == 1 {
                            lfn_flag = true;
                        }
                    lfn = format!("{}{}", bytes2str(&lfn_entry.get_name()), lfn);
                    continue
                // Volumen ID
                } else if entry.attr & 0x08 != 0 {
                    crate::debug!("Volume Name");
                // Directory
                } else if entry.attr & 0x10 != 0 {
                    crate::debug!("Directory");
                    if (lfn_flag && &lfn == name) || (!lfn_flag && Self::sfn_cmp(entry.name, &name)) {
                        if i == full_path.path.len()-1 {
                            return Ok(entry)
                        } else {
                            dir_clus = entry.first_cluster();
                            i += 1;
                            name = &full_path.path[i];
                            break
                        }
                    }
                // Regular File
                } else {
                    crate::debug!("Regular File");
                    if (lfn_flag && &lfn == name) || (!lfn_flag && Self::sfn_cmp(entry.name, &name)) {
                        if i == full_path.path.len()-1 {
                            return Ok(entry)
                        } else {
                            return Err(1)
                        }
                    }
                }
                if lfn_flag {lfn_flag = false}
            }
        }
        return Err(3)
    }
}

impl FileSystem for FAT {
    fn open(&self, path: &str, flags: u32) -> i32 {
        let file = File::new(flags, path);
        return FILE_DESCRIPTOR_TABLE.lock().add(file)
    }
    fn close(&self, fd: i32) {
        FILE_DESCRIPTOR_TABLE.lock().remove(fd);
    }
    fn read(&self, fd: i32, buf: &mut [u8], nbytes: usize) -> isize {
        let file = FILE_DESCRIPTOR_TABLE.lock().get(fd);
        let entry = self.find_file(&file.path).unwrap();
        if entry.attr & 0x08 != 0 || entry.attr & 0x10 != 0 {
            return -1
        }
        let mut cluster = entry.first_cluster();
        let mut i = 0;
        let mut bytes_buf = vec![0u8; self.bpc];
        while cluster != END_OF_CLUSTER_CHAIN {
            self.get_cluster(cluster, &mut bytes_buf);
            if self.bpc*(i+1) <= nbytes {
                buf[self.bpc*i..self.bpc*(i+1)].copy_from_slice(&bytes_buf);
            } else {
                buf[self.bpc*i..nbytes].copy_from_slice(&bytes_buf[..(nbytes-self.bpc*i)]);
                break
            }
            cluster = self.next_cluster(cluster);
            i += 1;
        }
        return nbytes as isize
    }
}