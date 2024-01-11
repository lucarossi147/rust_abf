use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::{Error, ErrorKind};
use memmap2::Mmap;

pub mod abf;
mod conversion_util;
use conversion_util as cu;
use abf::Abf;
// use abf::abf_v2::AbfV2;
// TODO this will become an Abf Header
// TODO the Abf Header will be an enum, of either abf_v1 or abf_v2 
#[derive(Debug, Clone, Copy)]
pub enum AbfKind {
    AbfV1,
    AbfV2,
}

pub struct AbfBuilder;

impl AbfBuilder {
    pub fn from_file(filepath: &Path)-> Result<Abf, Error> {
        let path = PathBuf::from(filepath);
        let memmap =  unsafe { Mmap::map(&File::open(&path)?)? };
        let file_signature_str = cu::from_bytes_array_to_string(&memmap, 0, 4);
        match file_signature_str {
            Ok(v) => match v {
                "ABF " => todo!(),
                "ABF2" => Ok(Abf::from_abf_v2(memmap, path)),
                _ => Err(Error::new(ErrorKind::InvalidData, "Incorrect file type"))
            },
            _ => Err(Error::new(ErrorKind::InvalidData, "Incorrect file type"))
        }
    }
}