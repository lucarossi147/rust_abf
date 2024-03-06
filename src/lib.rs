// ! # Abf Crate
// !
// ! This crate defines the `Abf` struct, representing data from Axon Binary Format (ABF) files.
// ! ABF files are typically used in electrophysiological recordings.
// !
// ! ## Example Usage
// !
// ! ```rust
// ! use abf::{Abf, AbfKind};
// ! use std::path::Path;
// ! // Create an Abf instance
// ! let abf = Abf::from_file(Path::new(filepath)).unwrap();
// !
// ! // Access information about the ABF file
// ! println!("File Signature: {:?}", abf.get_file_signature());
// ! println!("Channels Count: {}", abf.get_channels_count());
// ! println!("Sweeps Count: {}", abf.get_sweeps_count());
// !
// ! // Access data from the ABF file
// ! abf.get_channels()
// ! .map(|c| c.get_sweeps()) 
// ! .flatten()
// ! .for_each(|s| assert_eq!(s.unwrap().len(), 250_000));
// ! let channel_data = abf.get_sweep_in_channel(0, 0);
// ! if let Some(data) = channel_data {
// !     println!("Channel 0, Sweep 0 data: {:?}", data);
// ! }
// ! ```

use memmap2::Mmap;
use std::{
    collections::HashMap, 
    path::{Path, PathBuf},
    fs::File,
    io::{Error, ErrorKind}
};
use channel::Channel;

mod conversion_util;
use conversion_util as cu;
mod abf_v1;
pub mod abf_v2;
mod channel;

// use abf::abf_v2::AbfV2;
// TODO this will become an Abf Header
// TODO the Abf Header will be an enum, of either abf_v1 or abf_v2 
#[derive(Debug, Clone, Copy)]
pub enum AbfKind {
    AbfV1,
    AbfV2,
}

/// The `Abf` struct represents data from an ABF file.
pub struct Abf {
    abf_kind: AbfKind,
    channels_count: u32,
    sweeps_count: u32,
    sampling_rate: f32,
    channels: HashMap<u32, Channel>,
    path: PathBuf,
}

impl Abf {

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

    pub fn get_time_axis(&self) -> Vec<f32> {
        let data_sec_per_point = 1.0 / self.sampling_rate;
        let data_len = self.get_sweep_in_channel(0, 0).unwrap().len();
        let number_of_points = data_len / self.sweeps_count as usize;
        (0..number_of_points).map(|n| n as f32).map(|n| n * data_sec_per_point).collect()
    }

    pub fn get_channels_count(&self) -> u32 {
        self.channels_count
    }

    pub fn get_sweeps_count(&self ) -> u32 {
        self.sweeps_count
    }

    pub fn get_sweep_in_channel(&self, sweep: u32, channel: u32) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        self.channels.get(&channel)?.get_sweep(sweep)
    }

    pub fn get_file_signature(&self) -> AbfKind {
        self.abf_kind
    }

    pub fn get_channel(&self, index: u32) -> Option<&Channel> {
        self.channels.get(&index)
    }

    pub fn get_channels(&self) -> impl Iterator<Item = &Channel>{
        self.channels.values().into_iter()
    }
    
    pub fn get_sampling_rate(&self) -> f32 {
        self.sampling_rate
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }
}

// pub trait Abf {
//     fn get_channel_count(&self) -> usize;
//     fn get_data(&self, channel: usize) -> Option<Vec<f32>>;
//     fn get_file_signature(&self) -> AbfKind;
// }