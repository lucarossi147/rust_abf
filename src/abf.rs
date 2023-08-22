mod abf_v1;
pub mod abf_v2;
// use std::collections::HashMap;

use std::collections::HashMap;

use super::AbfKind;

// TODO the abf will be a collection of sweeps.
// sweeps will be common to both abfs
// there won't be the necessity for an abf trait
// the following struct are still a work in progress
// enum FileType{
//      I16,
//      F32,  
// }
// struct Channel{
//     values: Vec<i16>,
//     uom: String,
//     scaling_factor: f32,
//     label: String,
// }
// struct Abf{
//     abf_kind: AbfKind,
//     number_of_channels: u32,
//     number_of_sweeps: u32,
//     file_type: FileType,
//     channels: HashMap<u32, Channel>,
// }

pub trait Abf {
    fn get_channel_count(&self) -> usize;
    fn get_data(&self, channel: usize) -> Option<Vec<f32>>;
    fn get_file_signature(&self) -> AbfKind;
}