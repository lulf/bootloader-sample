#![no_std]
#![no_main]
#![feature(
    generic_associated_types,
    type_alias_impl_trait,
    naked_functions,
    try_blocks
)]

use core::mem;
use cortex_m_rt::{entry, exception};
use embassy::util::Steal;
use embassy_nrf::{pac, peripherals, wdt};
use nrf_softdevice_mbr as mbr;

mod fmt;

mod copylist;
mod crc32;
mod flash;
#[cfg(feature = "defmt")]
mod log;
mod model;
mod partitions;
mod swap;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {
        core::arch::asm!("udf #0");
        core::hint::unreachable_unchecked();
    }
}

#[no_mangle]
#[cfg_attr(target_os = "none", link_section = ".HardFault.user")]
unsafe extern "C" fn HardFault() {
    cortex_m::peripheral::SCB::sys_reset();
}

#[exception]
unsafe fn DefaultHandler(_: i16) -> ! {
    const SCB_ICSR: *const u32 = 0xE000_ED04 as *const u32;
    let irqn = core::ptr::read_volatile(SCB_ICSR) as u8 as i16 - 16;

    panic!("DefaultHandler #{:?}", irqn);
}

#[used]
#[no_mangle]
#[link_section = ".uicr_bootloader_start_address"]
pub static UICR_BOOTLOADER_START_ADDRESS: usize = partitions::BOOTLOADER.start;
#[used]
#[no_mangle]
#[link_section = ".uicr_mbr_params_page"]
pub static UICR_MBR_PARAMS_PAGE: usize = partitions::MBR_PARAMS_PAGE.start;

const NRF_SUCCESS: u32 = 0;

unsafe fn boot_app() -> ! {
    let addr: u32 = 0x1000;
    info!("boot_app {=u32:x}", addr);

    let mut cmd = mbr::sd_mbr_command_t {
        command: mbr::NRF_MBR_COMMANDS_SD_MBR_COMMAND_IRQ_FORWARD_ADDRESS_SET,
        params: mbr::sd_mbr_command_t__bindgen_ty_1 {
            irq_forward_address_set: mbr::sd_mbr_command_irq_forward_address_set_t {
                address: addr,
            },
        },
    };
    let ret = mbr::sd_mbr_command(&mut cmd);
    assert_eq!(ret, NRF_SUCCESS);

    let msp = *(addr as *const u32);
    let rv = *((addr + 4) as *const u32);

    info!("msp = {=u32:x}, rv = {=u32:x}", msp, rv);

    core::arch::asm!(
        "mrs {tmp}, CONTROL",
        "bics {tmp}, {spsel}",
        "msr CONTROL, {tmp}",
        "isb",
        "msr MSP, {msp}",
        "mov lr, {new_lr}",
        "bx {rv}",
        // `out(reg) _` is not permitted in a `noreturn` asm! call,
        // so instead use `in(reg) 0` and don't restore it afterwards.
        tmp = in(reg) 0,
        spsel = in(reg) 2,
        new_lr = in(reg) 0xFFFFFFFFu32,
        msp = in(reg) msp,
        rv = in(reg) rv,
        options(noreturn),
    );
}

fn watchdog_pet() {
    unsafe { wdt::WatchdogHandle::steal(0) }.pet();
}

fn watchdog_start(wdt: peripherals::WDT) {
    let mut config = wdt::Config::default();
    config.timeout_ticks = 32768 * 5; // 5 seconds
    config.run_during_sleep = true;
    config.run_during_debug_halt = false;
    let (_wdt, [_handle]) = match wdt::Watchdog::try_new(wdt, config) {
        Ok(x) => x,
        Err(_) => {
            // In case the watchdog is already running, just spin and let it expire, since
            // we can't configure it anyway. This usually happens when we first program
            // the device and the watchdog was previously active
            info!("Watchdog already active with wrong config, waiting for it to timeout...");
            loop {}
        }
    };
}

#[entry]
unsafe fn main() -> ! {
    let p: pac::Peripherals = mem::transmute(());

    let nvmc = p.NVMC;

    // SET APPROTECT
    if *(0x10001208 as *mut u32) != 0 && *(0x100010fc as *mut u32) != 0xe35c38e7 {
        nvmc.config.write(|w| w.wen().wen());
        while nvmc.ready.read().ready().is_busy() {}
        core::ptr::write_volatile(0x10001208 as *mut u32, 0);
        while nvmc.ready.read().ready().is_busy() {}
        nvmc.config.reset();
        while nvmc.ready.read().ready().is_busy() {}
        cortex_m::peripheral::SCB::sys_reset();
    }

    info!("Hello from Bootloader!");

    watchdog_start(peripherals::WDT::steal());

    swap::execute();
    copylist::execute();

    boot_app();
}
