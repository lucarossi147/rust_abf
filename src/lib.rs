use std::fs::File;
use std::str;
use memmap::Mmap;
mod abf;
mod conversion_util;
// TODO this will become an Abf Header
// TODO the Abf Header will be an enum, of either abf_v1 or abf_v2 

fn mmap(path: &str) -> Result<Mmap, std::io::Error> {
    Ok( unsafe {
         Mmap::map(&File::open(path)?)? 
        }
    )
}