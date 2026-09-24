use byteorder::{LittleEndian, ReadBytesExt};
use memmap2::Mmap;
use std::str;

pub fn from_bytes_array_to_string(
    mmap: &Mmap,
    from: usize,
    len: usize,
) -> Result<&str, str::Utf8Error> {
    str::from_utf8(&mmap[from..from + len])
}

pub fn from_byte_array_to_u32(mmap: &Mmap, from: usize) -> Result<u32, ()> {
    // Extract the relevant bytes (4 bytes) from the mmap slice
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<u32>()];
    // Convert the bytes to a u32 value in little-endian order
    if let Ok(u32) = bytes_slice.read_u32::<LittleEndian>() {
        Ok(u32)
    } else {
        Err(())
    }
}

pub fn from_byte_array_to_u16(mmap: &Mmap, from: usize) -> u16 {
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<u16>()];
    bytes_slice.read_u16::<LittleEndian>().unwrap()
}

pub fn byte_array_to_i16(ba: &[u8]) -> i16 {
    let mut ba = ba;
    ba.read_i16::<LittleEndian>().unwrap()
}

pub fn byte_array_to_f32(ba: &[u8]) -> f32 {
    let mut ba = ba;
    ba.read_f32::<LittleEndian>().unwrap()
}

pub fn mmap_to_i16(mmap: &Mmap, from: usize) -> i16 {
    let mut ba = &mmap[from..from + std::mem::size_of::<i16>()];
    ba.read_i16::<LittleEndian>().unwrap()
}

pub fn mmap_to_f32(mmap: &Mmap, from: usize) -> f32 {
    let mut ba = &mmap[from..from + std::mem::size_of::<f32>()];
    ba.read_f32::<LittleEndian>().unwrap()
}

pub fn mmap_to_i32(mmap: &Mmap, from: usize) -> i32 {
    let mut ba = &mmap[from..from + std::mem::size_of::<i32>()];
    ba.read_i32::<LittleEndian>().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_array_to_f32_reads_little_endian_ieee754() {
        let bytes = 1.5f32.to_le_bytes();
        assert_eq!(byte_array_to_f32(&bytes), 1.5);
    }

    #[test]
    fn byte_array_to_f32_reads_only_the_first_four_bytes() {
        let mut bytes = (-2.25f32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&[0xAA, 0xBB]);
        assert_eq!(byte_array_to_f32(&bytes), -2.25);
    }
}
