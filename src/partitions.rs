#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Partition {
    pub start: usize,
    pub end: usize,
}

impl Partition {
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn bytes(&self) -> &'static [u8] {
        unsafe { core::slice::from_raw_parts(self.start as *const u8, self.len()) }
    }
}

#[rustfmt::skip]
mod partitions {
    use super::*;

    // =================================================
    // IMPORTANT IMPORTANT IMPORTANT IMPORTANT IMPORTANT IMPORTANT IMPORTANT !!!!
    // =================================================
    //
    // - Must NEVER OVERLAP
    // - Must be kept in sync with linker script (ak_link_app.x, ak_link_bootloader.x)
    // - DO NOT MOVE factory config or the device won't be able to find it after an upgrade!
    // - DFU must be EXACTLY ONE page bigger than APP, for bootloader power-fail-safe swap.

    pub const MBR:                  Partition = Partition{ start: 0x00000, end:  0x01000 };
    pub const SOFTDEVICE:           Partition = Partition{ start: 0x01000, end:  0x27000 };
    pub const APP:                  Partition = Partition{ start: 0x27000, end:  0x85000 };
    pub const DFU:                  Partition = Partition{ start: 0x85000, end:  0xe4000 };

    pub const BOOTLOADER:           Partition = Partition{ start: 0xf8000, end:  0xfe000 };
    pub const MBR_PARAMS_PAGE:      Partition = Partition{ start: 0xfe000, end:  0xff000 };
    pub const BOOTLOADER_COPYLIST:  Partition = Partition{ start: 0xff000, end: 0x100000 };
}

pub use partitions::*;
