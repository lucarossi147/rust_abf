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
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use channel::Channel;
use memmap2::Mmap;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

mod byte_reader;
mod error;
pub use error::AbfError;

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
    channels: Vec<Channel>,
    path: PathBuf,
}

impl Abf {
    pub fn from_file(filepath: &Path) -> Result<Abf, AbfError> {
        let path = PathBuf::from(filepath);
        let file = File::open(&path)?;
        let memmap = unsafe { Mmap::map(&file)? };
        let signature = byte_reader::ByteReader::new(&memmap)
            .read_str("file_signature", 0, 4)
            .ok();
        match signature {
            Some("ABF2") => Abf::from_abf_v2(memmap, path),
            Some("ABF ") => Err(AbfError::UnsupportedVersion("ABF1".to_string())),
            _ => Err(AbfError::InvalidSignature),
        }
    }

    pub fn get_time_axis(&self) -> Vec<f32> {
        let Some(sweep_len) = self.channels.first().map(|ch| ch.get_sweep_len()) else {
            return Vec::new();
        };
        let data_sec_per_point = 1.0_f64 / self.sampling_rate as f64;
        (0..sweep_len)
            .map(|n| (n as f64 * data_sec_per_point) as f32)
            .collect()
    }

    pub fn get_channels_count(&self) -> u32 {
        self.channels_count
    }

    pub fn get_sweeps_count(&self) -> u32 {
        self.sweeps_count
    }

    pub fn get_sweep_in_channel(&self, sweep: u32, channel: u32) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        self.channels.get(channel as usize)?.get_sweep(sweep)
    }

    pub fn get_file_signature(&self) -> AbfKind {
        self.abf_kind
    }

    pub fn get_channel(&self, index: u32) -> Option<&Channel> {
        self.channels.get(index as usize)
    }

    pub fn get_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter()
    }

    pub fn get_sampling_rate(&self) -> f32 {
        self.sampling_rate
    }

    pub fn get_path(&self) -> &Path {
        &self.path
    }

    pub fn get_time_duration(&self) -> Option<f32> {
        let data_sec_per_point = 1.0 / self.sampling_rate;
        self.get_channel(0)
            .map(|ch| ch.get_sweep_len() as f32 * data_sec_per_point)
    }
}

// pub trait Abf {
//     fn get_channel_count(&self) -> usize;
//     fn get_data(&self, channel: usize) -> Option<Vec<f32>>;
//     fn get_file_signature(&self) -> AbfKind;
// }

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn abf_with_channels(channels: Vec<Channel>) -> Abf {
        Abf {
            abf_kind: AbfKind::AbfV2,
            channels_count: channels.len() as u32,
            sweeps_count: 1,
            sampling_rate: 10_000.0,
            channels,
            path: PathBuf::new(),
        }
    }

    #[test]
    fn get_time_axis_is_empty_when_there_are_no_channels() {
        let abf = abf_with_channels(Vec::new());
        assert!(abf.get_time_axis().is_empty());
    }

    #[test]
    fn get_time_axis_uses_sweep_len_from_first_available_channel() {
        let channel = Channel::new(
            channel::ChannelValues::I16(std::sync::Arc::from(vec![1_i16, 2, 3])),
            "mV".to_string(),
            1.0,
            0.0,
            "IN 0".to_string(),
            1,
        );
        let abf = abf_with_channels(vec![channel]);
        assert_eq!(abf.get_time_axis().len(), 3);
    }
}
