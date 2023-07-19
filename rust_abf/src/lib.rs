use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::str;

pub enum AbfType {
    AbfV1,
    AbfV2,
    Invalid,
}

pub fn get_file_signature(path: &str) -> Result<AbfType, std::io::Error> {
    let mut buffer = [0; 4];
    // Open the file and read the first 4 bytes into a buffer
    File::open(path)?.read_exact(&mut buffer)?;
    // Convert the bytes to an ASCII string
    let ascii_string = str::from_utf8(&buffer);
    if let Ok(s) = ascii_string {
        println!("{}", s);
    }
    Ok(match  ascii_string {
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
