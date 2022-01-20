use crate::flash;
use crate::partitions;
use core::slice;
use embassy_nrf::nvmc::PAGE_SIZE;

mod progress {
    use crate::flash;
    use crate::model::{SWAP_MAGIC, SWAP_REVERTED_MAGIC};
    use crate::partitions;
    use embassy_nrf::nvmc::PAGE_SIZE;

    const PROGRESS_PAGE: usize = partitions::BOOTLOADER_COPYLIST.start;
    const MAX_INDEX: usize = PAGE_SIZE / 4 - 1;

    pub fn reset() {
        // First sabotage the magic. Then erase.
        // This ensures a partial erase doesn't corrupt random data but leaving the magic intact by chance.
        unsafe { flash::write(PROGRESS_PAGE as _, &[0, 0, 0, 0]) }
        unsafe { flash::erase(PROGRESS_PAGE as _) };
    }

    pub fn set_as_reverted() {
        reset();
        unsafe { flash::write(PROGRESS_PAGE as _, &SWAP_REVERTED_MAGIC.to_le_bytes()) }
    }

    pub fn is_started() -> bool {
        unsafe { (PROGRESS_PAGE as *const u32).read_volatile() == SWAP_MAGIC }
    }

    fn addr(i: usize) -> *mut u32 {
        (PROGRESS_PAGE + 4 + i * 4) as _
    }

    pub fn get() -> usize {
        for i in 0..MAX_INDEX {
            if unsafe { addr(i).read() == 0xFFFF_FFFF } {
                return i;
            }
        }
        return MAX_INDEX;
    }

    pub fn set(i: usize) {
        unsafe { flash::write(addr(i) as _, &[0, 0, 0, 0]) }
    }
}

fn copy_page_once(progress_index: usize, from: u32, to: u32) {
    if progress::get() <= progress_index {
        unsafe {
            let data = slice::from_raw_parts(from as *const u8, PAGE_SIZE);
            flash::erase_and_write(to as _, data);
        }
        progress::set(progress_index);
    }
}

fn app(n: usize) -> u32 {
    (partitions::APP.start + n * PAGE_SIZE) as u32
}

fn dfu(n: usize) -> u32 {
    (partitions::DFU.start + n * PAGE_SIZE) as u32
}

const PAGE_COUNT: usize = partitions::APP.len() / PAGE_SIZE;

fn do_update() {
    for p in 0..PAGE_COUNT {
        copy_page_once(p * 2, app(PAGE_COUNT - 1 - p), dfu(PAGE_COUNT - p));
        copy_page_once(p * 2 + 1, dfu(PAGE_COUNT - 1 - p), app(PAGE_COUNT - 1 - p));
    }
}

fn do_revert() {
    for p in 0..PAGE_COUNT {
        copy_page_once(PAGE_COUNT * 2 + p * 2, app(p), dfu(p));
        copy_page_once(PAGE_COUNT * 2 + p * 2 + 1, dfu(p + 1), app(p));
    }
}

pub fn execute() {
    if !progress::is_started() {
        return;
    }

    if progress::get() >= PAGE_COUNT * 2 {
        // Update was already done. This means the firmware has booted once, and
        // we've rebooted since. Firmware is probably bad, revert it.
        do_revert();
        progress::set_as_reverted();
    } else {
        // do it
        do_update();
    }
}
