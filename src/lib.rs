use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str;
use memmap::Mmap;
use byteorder::{LittleEndian, ReadBytesExt};

// TODO this will become an Abf Header
// TODO the Abf Header will be an enum, of either abf_v1 or abf_v2 
pub struct Abf{
    pub file_signature: AbfType,            //  0
    pub file_version_number: Vec<i8>,       //  4
    pub file_info_size: u32,                //  8
    pub actual_episodes: u32,               //  12
    pub file_start_date: u32,               //  16
    pub file_start_time_ms: u32,            //  20
    pub stopwatch_time: u32,                //  24
    pub file_type: u16,                     //  28
    pub data_format: u16,                   //  30
    pub simultaneus_scan: u16,              //  32
    pub crc_enable: u16,                    //  34
    pub file_crc: u32,                      //  38
    pub file_guid: u32,                     //  42
}

impl Abf {
    pub fn new(path: &str) -> Self {
        let memmap = mmap(path).unwrap();
        let file_signature = match from_bytes_array_to_string(&memmap, 0, 4) {
            Ok(v) => get_file_signature(v),
            _ => AbfType::Invalid,
        };    
        let file_version_number = (4..=8).map(|i| memmap[i] as i8).collect();
        let file_info_size = from_byte_array_to_u32(&memmap, 8).unwrap();
        let actual_episodes = from_byte_array_to_u32(&memmap, 12).unwrap();
        let file_start_date = from_byte_array_to_u32(&memmap, 16).unwrap();
        let file_start_time_ms = from_byte_array_to_u32(&memmap, 20).unwrap();
        let stopwatch_time = from_byte_array_to_u32(&memmap, 24).unwrap();
        let file_type = from_byte_array_to_u16(&memmap, 28);
        let data_format: u16 = from_byte_array_to_u16(&memmap, 30);
        let simultaneus_scan: u16 = from_byte_array_to_u16(&memmap, 32);
        let crc_enable: u16 = from_byte_array_to_u16(&memmap, 34);
        let file_crc: u32 = from_byte_array_to_u32(&memmap, 38).unwrap();
        let file_guid: u32= from_byte_array_to_u32(&memmap, 42).unwrap();
        Abf {
            file_signature,
            file_version_number,
            file_info_size,
            actual_episodes,
            file_start_date,
            file_start_time_ms,
            stopwatch_time,
            file_type,
            data_format,
            simultaneus_scan,
            crc_enable,
            file_crc,
            file_guid,
        }
    }
}

pub enum AbfType {
    AbfV1,
    AbfV2,
    Invalid,
}

fn mmap(path: &str) -> Result<Mmap, std::io::Error> {
    Ok( unsafe {
         Mmap::map(&File::open(path)?)? 
        }
    )
}

fn from_bytes_array_to_string(mmap: &Mmap, from: usize, len: usize) -> Result<&str, str::Utf8Error>{
    str::from_utf8(&mmap[from..from+len])
}

fn from_byte_array_to_u32(mmap: &Mmap, from: usize) -> Result<u32, ()> {
    // Extract the relevant bytes (4 bytes) from the mmap slice
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<u32>()];
    // Convert the bytes to a u32 value in little-endian order
    if let Ok(u32) = bytes_slice.read_u32::<LittleEndian>(){
        Ok(u32)
    } else {
        Err(())
    }
}

fn from_byte_array_to_u16(mmap: &Mmap, from: usize) -> u16 {
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<u16>()];
    bytes_slice.read_u16::<LittleEndian>().unwrap()
}

fn get_file_signature(file_signature_str: &str) -> AbfType {
    match file_signature_str {
        "ABF " => AbfType::AbfV1,
        "ABF2" => AbfType::AbfV2,
        _ => AbfType::Invalid,
    }
}
