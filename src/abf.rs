mod abf_v1;
pub mod abf_v2;
mod channel;

use std::collections::HashMap;
use super::AbfKind;
use channel::Channel;

// TODO the abf will be a collection of sweeps.
// sweeps will be common to both abfs
// there won't be the necessity for an abf trait
// the following struct are still a work in progress
// enum ChannelKind {
//     Adc,
//     Dac,
// }

pub struct Abf {
    abf_kind: AbfKind,
    channels_count: u32,
    sweeps_count: u32,
    sampling_rate: f32,
    channels: HashMap<u32, Channel>,
}

impl Abf {
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
}

// pub trait Abf {
//     fn get_channel_count(&self) -> usize;
//     fn get_data(&self, channel: usize) -> Option<Vec<f32>>;
//     fn get_file_signature(&self) -> AbfKind;
// }