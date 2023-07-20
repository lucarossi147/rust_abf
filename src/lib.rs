use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str;
use memmap::Mmap;

pub enum AbfType {
    AbfV1,
    AbfV2,
    Invalid,
}

fn read_file_to_end(path: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    // Process the buffer here
    Ok(buffer)
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

fn from_bytes_array_to_string2(vec: &Vec<u8>, from: usize, len: usize) -> Result<&str, str::Utf8Error>{
    str::from_utf8(&vec[from..from+len])
}

pub fn get_file_signature(path: &str) -> Result<AbfType, std::io::Error> {
    let mmap = mmap(path)?;
    let file_signature = from_bytes_array_to_string(&mmap, 0, 4);
    // let vec = read_file_to_end(path)?;
    // let file_signature = from_bytes_array_to_string2(&vec, 0, 4);
    Ok(match  file_signature {
        Ok(v) => match v {
            "ABF " => AbfType::AbfV1,
            "ABF2" => AbfType::AbfV2,
            _ => AbfType::Invalid,
            }
        _ => AbfType::Invalid,
    })       
}

pub fn get_sweep_number(path: &str)->Result<u32, std::io::Error>{
    let mut file = File::open(path)?;
    // Seek to the desired position (12 bytes offset)
    file.seek(SeekFrom::Start(12))?;

    // Read 4 bytes from the file into a buffer
    let mut buffer = [0u8; 4];
    file.read_exact(&mut buffer)?;
    // Convert the bytes to an integer
    Ok(u32::from_le_bytes(buffer))
}
