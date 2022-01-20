pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xffffffff;
    for &b in data {
        crc = crc ^ b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = crc >> 1 ^ 0xedb88320;
            } else {
                crc = crc >> 1;
            }
        }
    }
    return !crc;
}
