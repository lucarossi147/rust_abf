use self::abf_kind::AbfKind;

mod abf_v1;
pub mod abf_v2;
mod abf_kind;

pub trait Abf {
    fn open<T,E>(filepath: &str) -> Result<T, E>;
    fn get_data(self, channel: usize) -> Option<&'static Vec<i16>>;
    fn get_file_signature(self, file_signature_str: &str) -> AbfKind;
}