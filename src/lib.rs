use std::str;
use std::fs::File;
use std::io::{Error, ErrorKind};
use memmap::Mmap;

pub mod abf;
mod conversion_util;
use conversion_util as cu;
use abf::Abf;
use abf::abf_v2::AbfV2;
// TODO this will become an Abf Header
// TODO the Abf Header will be an enum, of either abf_v1 or abf_v2 
#[derive(Debug, Clone, Copy)]
pub enum AbfKind {
    AbfV1,
    AbfV2,
}

pub struct AbfBuilder;

impl AbfBuilder {
    pub fn new(filepath: &str)-> Result<impl Abf, Error> {
        let memmap =  unsafe { Mmap::map(&File::open(filepath)?)? };
        println!("{:?}", memmap.len());
        let file_signature_str = cu::from_bytes_array_to_string(&memmap, 0, 4);
        match file_signature_str {
            Ok(v) => match v {
                "ABF " => todo!(),
                "ABF2" => Ok(AbfV2::new(memmap,  AbfKind::AbfV2)),
                _ => Err(Error::new(ErrorKind::InvalidData, "Incorrect file type"))
            },
            _ => Err(Error::new(ErrorKind::InvalidData, "Incorrect file type"))
        }
    }
}