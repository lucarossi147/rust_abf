mod abf_v1;
pub mod abf_v2;
use super::AbfKind;

pub trait Abf {
    fn get_channel_count(&self) -> usize;
    fn get_data(&self, channel: usize) -> Option<&Vec<i16>>;
    fn get_file_signature(&self) -> AbfKind;
}