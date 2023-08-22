mod abf_v1;
pub mod abf_v2;
// use std::collections::HashMap;

use super::AbfKind;

// TODO the abf will be a collection of sweeps.
// sweeps will be common to both abfs
// there won't be the necessity for an abf trait
// the following struct are still a work in progress
// struct Sweep{
//     values: Vec<i32>,
//     scaling_factor: f32,
//     uom: String,
//     label: String,
// }

// struct Abf{
//     abf_kind: AbfKind,
//     channels: Vec<usize>,
//     sweeps: HashMap<usize, Sweep>
// }

pub trait Abf {
    fn get_channel_count(&self) -> usize;
    fn get_data(&self, channel: usize) -> Option<Vec<f32>>;
    fn get_file_signature(&self) -> AbfKind;
}