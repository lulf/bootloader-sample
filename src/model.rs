use core::mem;
use core::slice;

use crate::crc32::crc32;

pub const SWAP_MAGIC: u32 = 0x55e53c7d;
pub const SWAP_REVERTED_MAGIC: u32 = 0xc7d1034a;

pub const MAGIC: u32 = 0x6cda65cc;
pub const MAX_ITEMS: usize = 16;

pub const FLAG_COMPRESSED: u32 = 1;
pub const FLAG_BOOTLOADER: u32 = 2; // Use MBR copy bootloader for this item. This entry must NOT be compressed.

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Item {
    pub flags: u32,
    pub src: u32,
    pub src_size: u32, // Compressed size (equal to dst_size if not compressed)
    pub dst: u32,
    pub dst_size: u32, // Decompressed size
    pub dst_crc: u32,  // Decompressed crc
    pub reserved1: u32,
    pub reserved2: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Copylist {
    pub magic: u32,
    pub count: u32,
    pub crc: u32,
    pub items: [Item; MAX_ITEMS],
}

impl Copylist {
    pub fn calc_crc(&self) -> u32 {
        let s = unsafe { transmute_slice(&self.items[..self.count as usize]) };
        crc32(s)
    }

    pub fn is_valid(&self) -> bool {
        if self.magic != MAGIC {
            info!("copylist: bad magic");
            return false;
        }

        if self.count > MAX_ITEMS as u32 {
            info!("copylist: too many items");
            return false;
        }

        if self.calc_crc() != self.crc {
            info!("copylist: bad crc");
            return false;
        }

        true
    }
}

unsafe fn transmute_slice<T>(s: &[T]) -> &[u8] {
    slice::from_raw_parts(s.as_ptr() as *const u8, s.len() * mem::size_of::<T>())
}
