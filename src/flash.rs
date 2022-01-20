use core::slice;
use embassy::util::Steal;
use embassy_nrf::nvmc::{Nvmc, PAGE_SIZE};
use embassy_nrf::peripherals::NVMC;
use embedded_storage::nor_flash::NorFlash;

use super::watchdog_pet;

pub unsafe fn erase_and_write_if_different(dst: *mut u8, src: &[u8]) {
    watchdog_pet();

    let dst_data = slice::from_raw_parts(dst, src.len());
    if dst_data == src {
        info!("erase_and_write_if_different {=u32:x}: equal, skipping", dst as u32);
        return;
    }

    erase_and_write(dst, src);
}

pub unsafe fn erase_and_write(dst: *mut u8, src: &[u8]) {
    watchdog_pet();

    info!(
        "erase_and_write {=u32:x} from {=u32:x} {=u32:x}",
        dst as u32,
        src.as_ptr() as u32,
        src.len() as u32
    );

    let mut f = Nvmc::new(NVMC::steal());
    unwrap!(f.erase(dst as u32, dst as u32 + PAGE_SIZE as u32));
    unwrap!(f.write(dst as u32, src));
}

pub unsafe fn write(dst: *mut u8, src: &[u8]) {
    watchdog_pet();

    info!(
        "write {=u32:x} from {=u32:x} {=u32:x}",
        dst as u32,
        src.as_ptr() as u32,
        src.len() as u32
    );

    let mut f = Nvmc::new(NVMC::steal());
    unwrap!(f.write(dst as u32, src));
}

pub unsafe fn erase(dst: *mut u8) {
    watchdog_pet();

    info!("erase {=u32:x} ", dst as u32);

    let mut f = Nvmc::new(NVMC::steal());
    unwrap!(f.erase(dst as u32, dst as u32 + PAGE_SIZE as u32));
}
