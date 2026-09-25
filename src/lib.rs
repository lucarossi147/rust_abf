#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

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
pub use channel::{Channel, FileKind};
mod storage;

/// Which ABF format version a file was parsed as.
///
/// # Examples
///
/// ```
/// use rust_abf::{Abf, AbfKind};
/// use std::path::Path;
///
/// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
/// assert!(matches!(abf.kind(), AbfKind::AbfV2));
/// ```
#[derive(Debug, Clone, Copy)]
pub enum AbfKind {
    /// A legacy ABF1 file. `Abf::from_file` does not currently parse ABF1
    /// files (it returns `Err(AbfError::UnsupportedVersion(_))` for them
    /// instead), so this variant cannot currently be observed; it exists for
    /// forward compatibility with future ABF1 support.
    AbfV1,
    /// An ABF2 file, the only format `Abf::from_file` currently parses.
    AbfV2,
}

/// Parsed data and metadata from an Axon Binary Format (ABF) file.
///
/// An `Abf` is obtained by calling [`Abf::from_file`], and exposes the file's
/// channels either through [`Abf::channel`]/[`Abf::channels`] (which return
/// [`Channel`] values with their own accessors) or directly through
/// [`Abf::sweep`], which takes a channel and sweep index.
///
/// # Examples
///
/// ```
/// use rust_abf::Abf;
/// use std::path::Path;
///
/// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
/// assert_eq!(abf.channel_count(), 2);
/// assert_eq!(abf.sweep_count(), 1);
/// assert_eq!(abf.sampling_rate(), 25_000.0);
/// ```
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
    /// # Errors
    ///
    /// Returns `Err` instead of panicking on any malformed or truncated
    /// input, in particular:
    /// - [`AbfError::Io`] if the file cannot be opened or memory-mapped.
    /// - [`AbfError::InvalidSignature`] if the first 4 bytes are not a
    ///   recognized ABF signature.
    /// - [`AbfError::UnsupportedVersion`] if the file is a legacy ABF1 file.
    /// - [`AbfError::Truncated`] or [`AbfError::InvalidSection`] if a header
    ///   section is missing data or internally inconsistent.
    /// - [`AbfError::Unsupported`] if the file uses a recognized but
    ///   not-yet-supported feature (e.g. non-uniform event-driven sweeps).
    ///
    /// # Caveats
    ///
    /// Because the file is memory-mapped, modifying or truncating it on disk
    /// while the returned `Abf` (or any `Channel` obtained from it) is still
    /// alive is undefined behavior: the mapping may be read concurrently
    /// with the external write, with no synchronization between the two.
    /// Callers must not write to a file while an `Abf` opened from it is in
    /// use.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_abf::Abf;
    /// use std::path::Path;
    ///
    /// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
    /// assert_eq!(abf.channel_count(), 2);
    /// ```
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

    /// Deprecated alias for [`Abf::time_axis`].
    #[deprecated(since = "0.5.0", note = "use `Abf::time_axis` instead")]
    pub fn get_time_axis(&self) -> Vec<f32> {
        self.time_axis()
    }

    /// The number of channels recorded in this file.
    #[must_use]
    pub fn channel_count(&self) -> usize {
        self.channels_count
    }

    /// Deprecated alias for [`Abf::channel_count`].
    #[deprecated(since = "0.5.0", note = "use `Abf::channel_count` instead")]
    pub fn get_channels_count(&self) -> u32 {
        self.channel_count() as u32
    }

    /// The number of sweeps recorded per channel.
    #[must_use]
    pub fn sweep_count(&self) -> usize {
        self.sweeps_count
    }

    /// Deprecated alias for [`Abf::sweep_count`].
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

    /// Deprecated alias for [`Abf::sweep`], with the arguments swapped
    /// (`(sweep, channel)` instead of `(channel, sweep)`).
    #[deprecated(since = "0.5.0", note = "use `Abf::sweep` instead")]
    pub fn get_sweep_in_channel(&self, sweep: u32, channel: u32) -> Option<Vec<f32>> {
        self.sweep(channel as usize, sweep as usize)
    }

    /// Which ABF format version this file was parsed as.
    #[must_use]
    pub fn kind(&self) -> AbfKind {
        self.abf_kind
    }

    /// Deprecated alias for [`Abf::kind`].
    #[deprecated(since = "0.5.0", note = "use `Abf::kind` instead")]
    pub fn get_file_signature(&self) -> AbfKind {
        self.kind()
    }

    /// Returns the channel at `index`, or `None` if `index >= channel_count()`.
    #[must_use]
    pub fn channel(&self, index: usize) -> Option<&Channel> {
        self.channels.get(index)
    }

    /// Deprecated alias for [`Abf::channel`].
    #[deprecated(since = "0.5.0", note = "use `Abf::channel` instead")]
    pub fn get_channel(&self, index: u32) -> Option<&Channel> {
        self.channel(index as usize)
    }

    /// Returns an iterator over every channel, in recording order.
    pub fn channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels.iter()
    }

    /// Deprecated alias for [`Abf::channels`].
    #[deprecated(since = "0.5.0", note = "use `Abf::channels` instead")]
    pub fn get_channels(&self) -> impl Iterator<Item = &Channel> {
        self.channels()
    }

    /// The sampling rate, in Hz, shared by every channel in this file.
    #[must_use]
    pub fn sampling_rate(&self) -> f32 {
        self.sampling_rate
    }

    /// Deprecated alias for [`Abf::sampling_rate`].
    #[deprecated(since = "0.5.0", note = "use `Abf::sampling_rate` instead")]
    pub fn get_sampling_rate(&self) -> f32 {
        self.sampling_rate()
    }

    /// The filesystem path this `Abf` was opened from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Deprecated alias for [`Abf::path`].
    #[deprecated(since = "0.5.0", note = "use `Abf::path` instead")]
    pub fn get_path(&self) -> &Path {
        self.path()
    }

    /// The duration, in seconds, of one sweep, or `None` if there are no
    /// channels.
    #[must_use]
    pub fn time_duration(&self) -> Option<f32> {
        let data_sec_per_point = 1.0 / self.sampling_rate;
        self.channel(0)
            .map(|ch| ch.sweep_len() as f32 * data_sec_per_point)
    }

    /// Deprecated alias for [`Abf::time_duration`].
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
