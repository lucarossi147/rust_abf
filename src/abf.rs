mod abf_v1;
pub mod abf_v2;
// use std::collections::HashMap;

use std::collections::HashMap;

use super::AbfKind;

// TODO the abf will be a collection of sweeps.
// sweeps will be common to both abfs
// there won't be the necessity for an abf trait
// the following struct are still a work in progress
// enum ChannelKind {
//     Adc,
//     Dac,
// }
enum FileKind {
     I16,
     F32,  
}
struct Channel {
    // channel_kind: ChannelKind,
    values: Vec<i16>,
    uom: String,
    gain: f32,
    offset: f32,
    label: String,
}

impl Channel {
    fn new(
        // channel_kind: ChannelKind,
        values: Vec<i16>,
        uom: String,
        gain: f32,
        offset: f32,
        label: String,
    ) -> Self {
        Self{
            // channel_kind,
            values,
            uom,
            gain,
            offset,
            label,
        }
    }
    // fn get_channel_kind(&self) -> ChannelKind {
    //     self.channel_kind
    // }
}

pub struct Abf {
    abf_kind: AbfKind,
    file_kind: FileKind,
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

    pub fn get_sweeps_count(&self )-> u32 {
        self.sweeps_count
    }

    pub fn get_sweep_in_channel(&self, sweep: u32, channel: u32)->Option<Vec<f32>> {
        let ch =  self.channels.get(&channel)?;
        let len = &ch.values.len();
        let data = ch.values
        .chunks(len/self.sweeps_count as usize)
        .enumerate()
        .filter_map(|(idx, chunck)| if idx == sweep as usize {Some(chunck)} else {None})
        .flatten()
        .map(|v| *v as f32);
        Some(match self.file_kind {
            // data in int, needs to be multiplied for the scaling factors
            FileKind::I16 => data.map(|v| v * ch.gain + ch.offset).collect(),
            FileKind::F32 => data.collect(),
        })
    }

    pub fn get_file_signature(&self) -> AbfKind {
        self.abf_kind
    }
}

// pub trait Abf {
//     fn get_channel_count(&self) -> usize;
//     fn get_data(&self, channel: usize) -> Option<Vec<f32>>;
//     fn get_file_signature(&self) -> AbfKind;
// }