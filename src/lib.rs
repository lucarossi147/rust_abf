// ! # Abf Crate
// !
// ! This crate defines the `Abf` struct, representing data from Axon Binary Format (ABF) files.
// ! ABF files are typically used in electrophysiological recordings.
// !
// ! ## Example Usage
// !
// ! ```rust,no_run
// ! use rust_abf::Abf;
// ! use std::path::Path;
// !
// ! // Create an Abf instance
// ! let abf = Abf::from_file(Path::new("recording.abf")).unwrap();
// !
// ! // Access information about the ABF file
// ! println!("File Signature: {:?}", abf.kind());
// ! println!("Channels Count: {}", abf.channel_count());
// ! println!("Sweeps Count: {}", abf.sweep_count());
// !
// ! // Access data from the ABF file
// ! for channel in abf.channels() {
// !     for sweep in channel.sweeps() {
// !         assert_eq!(sweep.unwrap().len(), 250_000);
// !     }
// ! }
// ! let channel_data = abf.sweep(0, 0);
// ! if let Some(data) = channel_data {
// !     println!("Channel 0, Sweep 0 data: {:?}", data);
// ! }
// ! ```
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

use channel::Channel;
use memmap2::Mmap;
use std::{
    fmt,
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
};
use storage::Storage;

mod byte_reader;
mod error;
pub use error::AbfError;

mod abf_v1;
mod abf_v2;
mod channel;
mod storage;

/// Which ABF format version a file was parsed as.
#[derive(Debug, Clone, Copy)]
pub enum AbfKind {
    AbfV1,
    AbfV2,
}

/// The `Abf` struct represents data from an ABF file.
pub struct Abf {
    abf_kind: AbfKind,
    channels_count: usize,
    sweeps_count: usize,
    sampling_rate: f32,
    channels: Vec<Channel>,
    path: PathBuf,
    /// Keeps the backing storage alive even for a zero-channel file (where no
    /// `Channel` would otherwise hold a reference to it); every `Channel`
    /// also holds its own clone, which is what makes lazy, per-sweep decoding
    /// possible.
    #[allow(dead_code)]
    storage: Arc<Storage>,
}

impl fmt::Debug for Abf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Abf")
            .field("kind", &self.abf_kind)
            .field("channel_count", &self.channels_count)
            .field("sweep_count", &self.sweeps_count)
            .field("sampling_rate", &self.sampling_rate)
            .field("path", &self.path)
            .finish()
    }
}

impl Abf {
    /// Opens and parses an ABF file.
    ///
    /// The file is memory-mapped rather than read into memory, so sample
    /// data is decoded lazily as sweeps are requested instead of being
    /// copied onto the heap at open time.
    ///
    /// # Caveats
    ///
    /// Because the file is memory-mapped, modifying or truncating it on disk
    /// while the returned `Abf` (or any `Channel` obtained from it) is still
    /// alive is undefined behavior: the mapping may be read concurrently
    /// with the external write, with no synchronization between the two.
    /// Callers must not write to a file while an `Abf` opened from it is in
    /// use.
    pub fn from_file(filepath: &Path) -> Result<Abf, AbfError> {
        let path = PathBuf::from(filepath);
        let file = File::open(&path)?;
        let memmap = unsafe { Mmap::map(&file)? };
        let storage = Arc::new(Storage::Mmap(memmap));
        let signature = byte_reader::ByteReader::new(storage.bytes())
            .read_str("file_signature", 0, 4)
            .ok();
        match signature {
            Some("ABF2") => Abf::from_abf_v2(storage, path),
            Some("ABF ") => Err(AbfError::UnsupportedVersion("ABF1".to_string())),
            _ => Err(AbfError::InvalidSignature),
        }
    }

    /// Returns one time point (in seconds) per sample of a single sweep,
    /// computed from the sampling rate; empty if there are no channels.
    #[must_use]
    pub fn time_axis(&self) -> Vec<f32> {
        let Some(sweep_len) = self.channels.first().map(Channel::sweep_len) else {
            return Vec::new();
        };
        let data_sec_per_point = 1.0_f64 / self.sampling_rate as f64;
        (0..sweep_len)
            .map(|n| (n as f64 * data_sec_per_point) as f32)
            .collect()
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::time_axis` instead")]
    pub fn get_time_axis(&self) -> Vec<f32> {
        self.time_axis()
    }

    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channels_count
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::channel_count` instead")]
    pub fn get_channels_count(&self) -> u32 {
        self.channel_count() as u32
    }

    #[must_use]
    pub fn sweep_count(&self) -> usize {
        self.sweeps_count
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::sweep_count` instead")]
    pub fn get_sweeps_count(&self) -> u32 {
        self.sweep_count() as u32
    }

    /// Returns channel `channel`'s sweep number `sweep`, in physical units.
    #[must_use]
    pub fn sweep(&self, channel: usize, sweep: usize) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        self.channels.get(channel)?.sweep(sweep)
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::sweep` instead")]
    pub fn get_sweep_in_channel(&self, sweep: u32, channel: u32) -> Option<Vec<f32>> {
        self.sweep(channel as usize, sweep as usize)
    }

    #[must_use]
    pub fn kind(&self) -> AbfKind {
        self.abf_kind
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::kind` instead")]
    pub fn get_file_signature(&self) -> AbfKind {
        self.kind()
    }

    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&Channel> {
        self.channels.get(index)
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::channel` instead")]
    pub fn get_channel(&self, index: u32) -> Option<&Channel> {
        self.channel(index as usize)
    }

    pub fn channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter()
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::channels` instead")]
    pub fn get_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels()
    }

    #[must_use]
    pub fn sampling_rate(&self) -> f32 {
        self.sampling_rate
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::sampling_rate` instead")]
    pub fn get_sampling_rate(&self) -> f32 {
        self.sampling_rate()
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::path` instead")]
    pub fn get_path(&self) -> &Path {
        self.path()
    }

    #[must_use]
    pub fn time_duration(&self) -> Option<f32> {
        let data_sec_per_point = 1.0 / self.sampling_rate;
        self.channel(0)
            .map(|ch| ch.sweep_len() as f32 * data_sec_per_point)
    }

    #[deprecated(since = "0.5.0", note = "use `Abf::time_duration` instead")]
    pub fn get_time_duration(&self) -> Option<f32> {
        self.time_duration()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn empty_storage() -> Arc<Storage> {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mmap = unsafe { Mmap::map(file.as_file()).unwrap() };
        Arc::new(Storage::Mmap(mmap))
    }

    fn abf_with_channels(channels: Vec<Channel>) -> Abf {
        Abf {
            abf_kind: AbfKind::AbfV2,
            channels_count: channels.len(),
            sweeps_count: 1,
            sampling_rate: 10_000.0,
            channels,
            path: PathBuf::new(),
            storage: empty_storage(),
        }
    }

    #[test]
    fn get_time_axis_is_empty_when_there_are_no_channels() {
        let abf = abf_with_channels(Vec::new());
        assert!(abf.time_axis().is_empty());
    }

    #[test]
    fn get_time_axis_uses_sweep_len_from_first_available_channel() {
        let (_file, channel) = channel::test_support::i16_channel(&[1, 2, 3], 1);
        let abf = abf_with_channels(vec![channel]);
        assert_eq!(abf.time_axis().len(), 3);
    }

    #[test]
    fn abf_and_channel_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Abf>();
        assert_send_sync::<Channel>();
    }

    #[test]
    fn abf_and_channel_implement_debug() {
        fn assert_debug<T: fmt::Debug>() {}
        assert_debug::<Abf>();
        assert_debug::<Channel>();
        assert_debug::<AbfKind>();
        assert_debug::<channel::FileKind>();
    }

    #[test]
    fn debug_format_does_not_panic() {
        let abf = abf_with_channels(Vec::new());
        assert!(format!("{abf:?}").contains("Abf"));
    }
}
