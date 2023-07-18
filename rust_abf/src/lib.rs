use std::fs::File;
use std::io::Read;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abf_v1() {
        let result = get_file_signature("test_abf/05210017_vc_abf1.abf");
        match result {
            Ok(r) =>  assert!(matches!(r, AbfType::AbfV1)),
            _ => println!("not found"),
        }
    }

    #[test]
    fn test_abf_v2() {
        let result = get_file_signature("test_abf/18425108.abf");
        match result {
            Ok(r) =>  assert!(matches!(r, AbfType::AbfV2)),
            _ => println!("not found"),
        }
    }

}
