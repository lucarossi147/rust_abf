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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_file_signature_abf_v1() {
        let result = get_file_signature("test_abf/05210017_vc_abf1.abf");
        match result {
            Ok(r) =>  assert!(matches!(r, AbfType::AbfV1)),
            _ => println!("not found"),
        }
    }

    #[test]
    fn test_get_file_signature_abf_v2() {
        let result = get_file_signature("test_abf/18425108.abf");
        match result {
            Ok(r) =>  assert!(matches!(r, AbfType::AbfV2)),
            _ => println!("not found"),
        }
    }

    #[test]
    fn test_get_number_of_sweep_from_example_abf(){
        let result = get_sweep_number("test_abf/14o08011_ic_pair.abf");
        match result {
            Ok(r) =>  assert_eq!(3, r),
            _ => println!("not found"),
        }
    }

    #[test]
    fn test_get_number_of_sweep_from_abf_v1(){
        let result = get_sweep_number("test_abf/05210017_vc_abf1.abf");
        match result {
            Ok(r) =>  assert_eq!(10, r),
            _ => println!("not found"),
        }
    }

    #[test]
    fn test_get_number_of_sweep_from_abf_v2(){
        let result = get_sweep_number("test_abf/18425108.abf");
        match result {
            Ok(r) => assert_eq!(1, r),
            _ => println!("not found"),
        }
    }
}
