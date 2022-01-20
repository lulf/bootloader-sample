use core::mem::MaybeUninit;
use core::slice;
use embassy_nrf::nvmc::PAGE_SIZE;
use nrf_softdevice_mbr as mbr;

use crate::crc32::crc32;
use crate::flash;
use crate::model::*;
use crate::partitions;

unsafe fn copy(dst: *mut u8, src: &[u8]) {
    info!(
        "copy {=u32:x} {=u32:x}:{=u32:x}",
        dst as u32,
        src.as_ptr() as u32,
        src.len() as u32
    );

    for (i, chunk) in src.chunks(PAGE_SIZE).enumerate() {
        flash::erase_and_write_if_different(dst.add(i * PAGE_SIZE), chunk);
    }

    info!("copy: done")
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Error {
    InflateCorrupted,
    InflateOverflow,
    InflateUnderflow,
}

unsafe fn copy_uncompress(mut dst: &mut [u8], src: &[u8]) -> Result<(), Error> {
    info!(
        "copy_uncompress {=u32:x}:{=u32:x} {=u32:x}:{=u32:x}",
        dst.as_ptr() as u32,
        dst.len() as u32,
        src.as_ptr() as u32,
        src.len() as u32
    );

    const UNCOMP_DICT_SIZE: usize = 32 * 1024;
    let mut uncomp: MaybeUninit<uzlib::uzlib_uncomp> = MaybeUninit::uninit();
    let mut uncomp_dict = [0u8; UNCOMP_DICT_SIZE];

    uzlib::uzlib_uncompress_init(
        uncomp.as_mut_ptr(),
        uncomp_dict.as_mut_ptr() as _,
        UNCOMP_DICT_SIZE as _,
    );
    let uncomp = &mut *uncomp.as_mut_ptr();

    let mut buf = [0u8; PAGE_SIZE];

    uncomp.source = src.as_ptr();
    uncomp.source_limit = src.as_ptr().add(src.len());

    loop {
        uncomp.dest = buf.as_mut_ptr();
        uncomp.dest_start = buf.as_mut_ptr();
        uncomp.dest_limit = buf.as_mut_ptr().add(PAGE_SIZE);

        let ret = uzlib::uzlib_uncompress(uncomp) as u32;
        if ret != uzlib::TINF_OK && ret != uzlib::TINF_DONE {
            return Err(Error::InflateCorrupted);
        }

        let size: isize = uncomp.dest.offset_from(uncomp.dest_start);
        assert!(size >= 0);
        let mut size = size as usize;

        if size > dst.len() {
            return Err(Error::InflateOverflow);
        }

        // round up to word
        size = (size + 3) / 4 * 4;

        flash::erase_and_write_if_different(dst.as_mut_ptr(), &buf[..size]);
        dst = &mut dst[size..];

        if ret == uzlib::TINF_DONE {
            break;
        }

        // if we're not done, uzlib should have given us an entire page
        // otherwise something fishy 🐟 is going on.
        assert!(size == PAGE_SIZE);
    }

    if dst.len() != 0 {
        return Err(Error::InflateUnderflow);
    }

    info!("copy_uncompress: done");
    Ok(())
}

const NRF_SUCCESS: u32 = 0;

unsafe fn copy_bootloader(dst: *mut u8, src: &[u8]) {
    info!(
        "copy_bootloader {=u32:x} {=u32:x}:{=u32:x}",
        dst as u32,
        src.as_ptr() as u32,
        src.len() as u32
    );

    // Check dst is in the bootloader partition.
    let part = partitions::BOOTLOADER;
    assert!(dst as u32 == part.start as u32);
    assert!((dst as u32 + src.len() as u32) <= part.end as u32);

    let dst_data = slice::from_raw_parts(dst, src.len());
    if dst_data == src {
        info!("copy_bootloader: equal, skipping");
        return;
    }

    // Length is IN WORDS, round up.
    let bl_src = src.as_ptr() as *mut u32;
    let bl_len = ((src.len() as u32) + 3) / 4;

    let mut cmd = mbr::sd_mbr_command_t {
        command: mbr::NRF_MBR_COMMANDS_SD_MBR_COMMAND_COPY_BL,
        params: mbr::sd_mbr_command_t__bindgen_ty_1 {
            copy_bl: mbr::sd_mbr_command_copy_bl_t { bl_src, bl_len },
        },
    };
    info!("copy_bootloader: running sd_mbr_command, see you on the other side!");
    let ret = mbr::sd_mbr_command(&mut cmd);
    assert_eq!(ret, NRF_SUCCESS);

    // COPY_BL command never returns if successful.
    unreachable!()
}

unsafe fn execute_item(item: &Item) {
    info!("execute_item {:?}", item);
    if item.flags & FLAG_BOOTLOADER != 0 {
        assert!(item.flags & FLAG_COMPRESSED == 0);
        assert_eq!(item.dst_size, item.src_size);

        let dst = item.dst as *mut u8;
        let src = slice::from_raw_parts(item.src as *const u8, item.src_size as usize);
        copy_bootloader(dst, src);
    } else if item.flags & FLAG_COMPRESSED != 0 {
        let dst = slice::from_raw_parts_mut(item.dst as *mut u8, item.dst_size as usize);
        let src = slice::from_raw_parts(item.src as *const u8, item.src_size as usize);
        copy_uncompress(dst, src).unwrap()
    } else {
        assert_eq!(item.dst_size, item.src_size);
        let dst = item.dst as *mut u8;

        let mut size = item.dst_size as usize;
        // round up to word
        size = (size + 3) / 4 * 4;

        let src = slice::from_raw_parts(item.src as *const u8, size);
        copy(dst, src)
    }

    let dst = slice::from_raw_parts(item.dst as *const u8, item.dst_size as usize);
    assert_eq!(crc32(dst), item.dst_crc);
}

pub unsafe fn do_execute(copylist: &Copylist) {
    for item in &copylist.items[..copylist.count as usize] {
        execute_item(item)
    }
}

pub unsafe fn execute() {
    let copylist_addr = partitions::BOOTLOADER_COPYLIST.start;
    let copylist = &*(copylist_addr as *const Copylist);
    info!("copylist addr {=u32:x}", copylist as *const _ as u32);

    if copylist.is_valid() {
        info!("copylist valid, executing");
        do_execute(copylist);
        flash::erase(copylist_addr as _);
    }
}
