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

mod abf_v2;
mod channel;
mod decode;
mod parallel;
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
        Abf::from_storage(storage, path)
    }

    /// Parses ABF data already resident in memory, without touching the
    /// filesystem.
    ///
    /// Unlike [`Abf::from_file`], this does not memory-map anything and
    /// contains no `unsafe` code, so it works in contexts with no filesystem
    /// access (e.g. WebAssembly) or when the bytes come from the network
    /// rather than a local file. [`Abf::path`] returns an empty path on the
    /// result, since there is no file it was opened from.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Abf::from_file`], except
    /// [`AbfError::Io`] never occurs (there is no file to open).
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_abf::Abf;
    ///
    /// let bytes = std::fs::read("tests/test_abf/18425108.abf").unwrap();
    /// let abf = Abf::from_bytes(bytes).unwrap();
    /// assert_eq!(abf.channel_count(), 2);
    /// ```
    pub fn from_bytes(bytes: impl Into<Arc<[u8]>>) -> Result<Abf, AbfError> {
        let storage = Arc::new(Storage::Owned(bytes.into()));
        Abf::from_storage(storage, PathBuf::new())
    }

    fn from_storage(storage: Arc<Storage>, path: PathBuf) -> Result<Abf, AbfError> {
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

    /// Returns sweep number `sweep` of **every** channel, in physical units
    /// (one `Vec` per channel, in channel order), or `None` if `sweep` is out
    /// of range.
    ///
    /// Prefer this over calling [`Channel::sweep`] once per channel when you
    /// need all channels: ABF stores channels interleaved sample by sample,
    /// so reading them one at a time streams the same bytes from memory
    /// once per channel. This method picks the fastest strategy on its own
    /// (see [`Abf::read_sweep_all_channels_into`]) and, with the `parallel`
    /// feature enabled, also spreads large sweeps across threads.
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_abf::Abf;
    /// use std::path::Path;
    ///
    /// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
    /// let channels = abf.sweep_all_channels(0).unwrap();
    /// assert_eq!(channels.len(), abf.channel_count());
    /// assert_eq!(channels[1], abf.channel(1).unwrap().sweep(0).unwrap());
    /// ```
    #[must_use]
    pub fn sweep_all_channels(&self, sweep: usize) -> Option<Vec<Vec<f32>>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        let mut outs: Vec<Vec<f32>> = self
            .channels
            .iter()
            .map(|c| vec![0.0f32; c.sweep_len()])
            .collect();
        self.decode_all_channels_par(sweep, &mut outs);
        Some(outs)
    }

    /// Decodes sweep number `sweep` of every channel into `outs` (one
    /// buffer per channel, in channel order, each exactly
    /// [`Channel::sweep_len`] long), without allocating.
    ///
    /// `outs` can be anything that yields `&mut [f32]` per channel, e.g.
    /// `&mut [Vec<f32>]`, `&mut [Box<[f32]>]` or `&mut [&mut [f32]]`, so the
    /// same buffers can be reused for every sweep.
    ///
    /// # Strategy
    ///
    /// The decode strategy is chosen automatically:
    ///
    /// - **Single pass** (when the file has two or more channels, they all
    ///   share one sweep length, and one sweep's interleaved data is larger
    ///   than [`Abf::SINGLE_PASS_MIN_BYTES`]): the sweep's frames are walked
    ///   once, in cache-sized blocks, and every channel's samples are pulled
    ///   out of each block while it is still in cache. Memory traffic no
    ///   longer grows with the channel count.
    /// - **Per channel** otherwise (single-channel files, small sweeps that
    ///   fit in cache anyway, or channels with different sweep lengths):
    ///   equivalent to calling [`Channel::read_sweep_into`] for each channel.
    ///
    /// Both strategies produce bit-identical results. This method always
    /// runs on the calling thread, even with the `parallel` feature, so it
    /// never allocates.
    ///
    /// # Errors
    ///
    /// Returns `Err` without modifying `outs` if:
    /// - [`AbfError::SweepOutOfRange`]: `sweep` is not a valid sweep index.
    /// - [`AbfError::ChannelCountMismatch`]: `outs.len()` is not
    ///   [`Abf::channel_count`].
    /// - [`AbfError::BufferLengthMismatch`]: some `outs[c]`'s length is not
    ///   channel `c`'s [`Channel::sweep_len`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_abf::Abf;
    /// use std::path::Path;
    ///
    /// let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
    /// let mut buffers: Vec<Vec<f32>> = abf
    ///     .channels()
    ///     .map(|c| vec![0.0; c.sweep_len()])
    ///     .collect();
    /// for sweep in 0..abf.sweep_count() {
    ///     abf.read_sweep_all_channels_into(sweep, &mut buffers).unwrap();
    ///     // buffers[0], buffers[1], ... now hold this sweep of each channel
    /// }
    /// ```
    pub fn read_sweep_all_channels_into<B: AsMut<[f32]>>(
        &self,
        sweep: usize,
        outs: &mut [B],
    ) -> Result<(), AbfError> {
        if sweep >= self.sweeps_count {
            return Err(AbfError::SweepOutOfRange {
                sweep,
                sweeps_count: self.sweeps_count,
            });
        }
        if outs.len() != self.channels.len() {
            return Err(AbfError::ChannelCountMismatch {
                expected: self.channels.len(),
                actual: outs.len(),
            });
        }
        for (channel, out) in self.channels.iter().zip(outs.iter_mut()) {
            let actual = out.as_mut().len();
            if actual != channel.sweep_len() {
                return Err(AbfError::BufferLengthMismatch {
                    expected: channel.sweep_len(),
                    actual,
                });
            }
        }
        self.decode_all_channels_seq(sweep, outs);
        Ok(())
    }

    /// The single-pass multi-channel decoder is only used when one sweep's
    /// interleaved data (all channels) is at least this many bytes. Below
    /// it, the data already fits in the CPU's L2 cache, so reading it once
    /// per channel costs nothing extra and the simpler per-channel path is
    /// used instead.
    pub const SINGLE_PASS_MIN_BYTES: usize = 256 * 1024;

    /// Whether [`Abf::read_sweep_all_channels_into`] /
    /// [`Abf::sweep_all_channels`] will use the single-pass strategy for
    /// this file.
    fn uses_single_pass(&self) -> bool {
        let Some(first) = self.channels.first() else {
            return false;
        };
        self.channels.len() >= 2
            && self
                .channels
                .iter()
                .all(|c| c.sweep_len() == first.sweep_len())
            && first.sweep_len().saturating_mul(first.stride()) >= Self::SINGLE_PASS_MIN_BYTES
    }

    /// Per-channel decode parameters for the single-pass decoder.
    fn channel_params(&self, c: usize) -> decode::ChannelParams {
        let channel = self.channels.get(c);
        let width = channel.map_or(2, |ch| ch.file_kind().sample_width());
        decode::ChannelParams {
            offset_in_frame: c.saturating_mul(width),
            gain: channel.map_or(1.0, Channel::gain),
            offset: channel.map_or(0.0, Channel::offset),
        }
    }

    /// Decodes `sweep` of every channel into already-validated `outs` on the
    /// calling thread, without allocating.
    fn decode_all_channels_seq<B: AsMut<[f32]>>(&self, sweep: usize, outs: &mut [B]) {
        if !self.uses_single_pass() {
            for (channel, out) in self.channels.iter().zip(outs.iter_mut()) {
                channel.decode_sweep_unchecked(sweep, out.as_mut(), false);
            }
            return;
        }
        let Some(first) = self.channels.first() else {
            return;
        };
        let Some(src) = first.frames_bytes(sweep) else {
            // Truncated or corrupt data section: silence, as per channel.
            outs.iter_mut().for_each(|out| out.as_mut().fill(0.0));
            return;
        };
        let is_f32 = first.file_kind() == FileKind::F32;
        decode::decode_frames_blocked(src, first.stride(), is_f32, outs, |c| {
            self.channel_params(c)
        });
    }

    /// Like [`Abf::decode_all_channels_seq`], but spreads the work across
    /// threads when the `parallel` feature is enabled: over channels on the
    /// per-channel path, over large frame ranges on the single-pass path.
    /// May allocate; only used by [`Abf::sweep_all_channels`].
    fn decode_all_channels_par(&self, sweep: usize, outs: &mut [Vec<f32>]) {
        if !self.uses_single_pass() {
            let mut jobs: Vec<(&Channel, &mut Vec<f32>)> =
                self.channels.iter().zip(outs.iter_mut()).collect();
            parallel::for_each_maybe_par(&mut jobs, |(channel, out)| {
                channel.decode_sweep_unchecked(sweep, out, true);
            });
            return;
        }
        let Some(first) = self.channels.first() else {
            return;
        };
        let frames = first.sweep_len();
        let range = parallel::frame_range_len(frames);
        if range >= frames {
            self.decode_all_channels_seq(sweep, outs);
            return;
        }
        let Some(src) = first.frames_bytes(sweep) else {
            outs.iter_mut().for_each(|out| out.fill(0.0));
            return;
        };
        let stride = first.stride();
        let is_f32 = first.file_kind() == FileKind::F32;
        // tasks[k].1[c] = frames [k*range, (k+1)*range) of channel c
        let task_count = (frames + range - 1) / range;
        let mut tasks: Vec<(usize, Vec<&mut [f32]>)> = (0..task_count)
            .map(|k| (k, Vec::with_capacity(outs.len())))
            .collect();
        for out in outs.iter_mut() {
            for (task, piece) in tasks.iter_mut().zip(out.chunks_mut(range)) {
                task.1.push(piece);
            }
        }
        parallel::for_each_maybe_par(&mut tasks, |(k, pieces)| {
            let start = k.saturating_mul(range).saturating_mul(stride);
            if let Some(src_k) = src.get(start..) {
                decode::decode_frames_blocked(src_k, stride, is_f32, pieces, |c| {
                    self.channel_params(c)
                });
            }
        });
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

    /// The filesystem path this `Abf` was opened from, or an empty path if
    /// it was constructed with [`Abf::from_bytes`].
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

    // ---- sweep_all_channels / read_sweep_all_channels_into ----

    fn abf_with_sweeps(channels: Vec<Channel>, sweeps_count: usize) -> Abf {
        let mut abf = abf_with_channels(channels);
        abf.sweeps_count = sweeps_count;
        abf
    }

    /// Deterministic sample value for frame `j` of channel `c`.
    fn i16_sample(j: usize, c: usize) -> i16 {
        (j.wrapping_mul(31).wrapping_add(c.wrapping_mul(7919)) % 65_536) as u16 as i16
    }

    fn f32_sample(j: usize, c: usize) -> f32 {
        j as f32 * 0.25 - c as f32 * 3.0
    }

    /// Builds `gains.len()` interleaved channels of `kind` sharing one owned
    /// storage; the storage holds `stored_frames` frames but the channels
    /// claim `frames` (so `stored_frames < frames` models a truncated file).
    fn shared_channels(
        kind: channel::FileKind,
        gains: &[(f32, f32)],
        frames: usize,
        stored_frames: usize,
        sweeps_count: usize,
    ) -> Vec<Channel> {
        let n = gains.len();
        let mut bytes = Vec::new();
        for j in 0..stored_frames {
            for c in 0..n {
                match kind {
                    channel::FileKind::I16 => bytes.extend(i16_sample(j, c).to_le_bytes()),
                    channel::FileKind::F32 => bytes.extend(f32_sample(j, c).to_le_bytes()),
                }
            }
        }
        let storage = Arc::new(Storage::Owned(Arc::from(bytes)));
        gains
            .iter()
            .enumerate()
            .map(|(c, &(gain, offset))| {
                Channel::new(
                    channel::ChannelLayout {
                        storage: storage.clone(),
                        data_offset: 0,
                        channel_index: c,
                        channel_count: n,
                        samples_per_channel: frames,
                        file_kind: kind,
                    },
                    None,
                    gain,
                    offset,
                    None,
                    sweeps_count,
                )
            })
            .collect()
    }

    fn bits(v: &[f32]) -> Vec<u32> {
        v.iter().map(|x| x.to_bits()).collect()
    }

    /// Asserts both all-channel entry points match per-channel
    /// `Channel::sweep` bit for bit, for every sweep.
    fn assert_all_channels_match(abf: &Abf) {
        let mut bufs: Vec<Vec<f32>> = abf
            .channels()
            .map(|c| vec![f32::NAN; c.sweep_len()])
            .collect();
        for s in 0..abf.sweep_count() {
            let expected: Vec<Vec<f32>> = abf.channels().map(|c| c.sweep(s).unwrap()).collect();
            let all = abf.sweep_all_channels(s).unwrap();
            assert_eq!(all.len(), expected.len());
            abf.read_sweep_all_channels_into(s, &mut bufs).unwrap();
            for c in 0..expected.len() {
                assert_eq!(
                    bits(&all[c]),
                    bits(&expected[c]),
                    "sweep_all_channels c{c} s{s}"
                );
                assert_eq!(bits(&bufs[c]), bits(&expected[c]), "read_into c{c} s{s}");
            }
        }
    }

    #[test]
    fn all_channels_per_channel_path_matches_channel_sweep() {
        let channels = shared_channels(
            channel::FileKind::I16,
            &[(0.5, 1.0), (-2.0, 0.25)],
            20,
            20,
            2,
        );
        let abf = abf_with_sweeps(channels, 2);
        assert!(!abf.uses_single_pass());
        assert_all_channels_match(&abf);
    }

    #[test]
    fn all_channels_single_channel_file_uses_per_channel_path() {
        let channels = shared_channels(channel::FileKind::I16, &[(1.5, 0.5)], 10, 10, 1);
        let abf = abf_with_sweeps(channels, 1);
        assert!(!abf.uses_single_pass());
        assert_all_channels_match(&abf);
    }

    #[test]
    fn all_channels_single_pass_i16_three_channels_matches() {
        // Ragged tail: a multiple of BLOCK_FRAMES plus 123 frames.
        let frames = decode::BLOCK_FRAMES * 11 + 123;
        let channels = shared_channels(
            channel::FileKind::I16,
            &[(0.125, -3.0), (0.03, 10.5), (-1.75, 0.0)],
            frames * 2,
            frames * 2,
            2,
        );
        let abf = abf_with_sweeps(channels, 2);
        assert!(abf.uses_single_pass());
        assert!(frames * 6 >= Abf::SINGLE_PASS_MIN_BYTES);
        assert_all_channels_match(&abf);
    }

    #[test]
    fn all_channels_long_sweeps_split_into_frame_ranges_match() {
        // Long enough (> 1M frames per sweep) that, with the `parallel`
        // feature, `sweep_all_channels` splits the single-pass decode into
        // several frame ranges, with a ragged last range. Without the
        // feature this covers the same sequential path as the tests above.
        let frames = (1 << 20) + 4_099;
        let channels = shared_channels(
            channel::FileKind::I16,
            &[(0.5, 1.0), (-2.0, 3.0)],
            frames,
            frames,
            1,
        );
        let abf = abf_with_sweeps(channels, 1);
        assert!(abf.uses_single_pass());
        assert_all_channels_match(&abf);
    }

    #[test]
    fn all_channels_single_pass_f32_two_channels_matches() {
        let frames = decode::BLOCK_FRAMES * 8 + 77;
        let channels = shared_channels(
            channel::FileKind::F32,
            &[(1.0, 0.0), (7.0, 7.0)],
            frames * 2,
            frames * 2,
            2,
        );
        let abf = abf_with_sweeps(channels, 2);
        assert!(abf.uses_single_pass());
        assert_all_channels_match(&abf);
    }

    #[test]
    fn all_channels_single_pass_truncated_yields_zeros() {
        let frames = decode::BLOCK_FRAMES * 11 + 123;
        // Sweep 0 is complete; sweep 1 is cut off a few frames short.
        let channels = shared_channels(
            channel::FileKind::I16,
            &[(0.5, 2.0), (1.0, -1.0), (2.0, 4.0)],
            frames * 2,
            frames * 2 - 5,
            2,
        );
        let abf = abf_with_sweeps(channels, 2);
        assert!(abf.uses_single_pass());
        assert_all_channels_match(&abf);
        let last = abf.sweep_all_channels(1).unwrap();
        assert!(last
            .iter()
            .all(|c| c.len() == frames && c.iter().all(|&v| v == 0.0)));
        let first = abf.sweep_all_channels(0).unwrap();
        assert!(first.iter().any(|c| c.iter().any(|&v| v != 0.0)));

        let mut bufs = vec![vec![9.0f32; frames]; 3];
        abf.read_sweep_all_channels_into(1, &mut bufs).unwrap();
        assert!(bufs.iter().all(|b| b.iter().all(|&v| v == 0.0)));
    }

    #[test]
    fn all_channels_single_pass_f32_truncated_yields_zeros() {
        let frames = decode::BLOCK_FRAMES * 8 + 77;
        let channels = shared_channels(
            channel::FileKind::F32,
            &[(1.0, 0.0), (1.0, 0.0)],
            frames,
            frames / 2,
            1,
        );
        let abf = abf_with_sweeps(channels, 1);
        assert!(abf.uses_single_pass());
        let all = abf.sweep_all_channels(0).unwrap();
        assert!(all
            .iter()
            .all(|c| c.len() == frames && c.iter().all(|&v| v == 0.0)));
        let mut bufs = vec![vec![9.0f32; frames]; 2];
        abf.read_sweep_all_channels_into(0, &mut bufs).unwrap();
        assert!(bufs.iter().all(|b| b.iter().all(|&v| v == 0.0)));
    }

    #[test]
    fn all_channels_unequal_sweep_lengths_use_per_channel_path() {
        let (_f0, a) = channel::test_support::i16_channel(&[1, 2, 3, 4], 1);
        let (_f1, b) = channel::test_support::i16_channel(&[5, 6], 1);
        let abf = abf_with_channels(vec![a, b]);
        assert!(!abf.uses_single_pass());
        assert_all_channels_match(&abf);
    }

    #[test]
    fn read_sweep_all_channels_into_reports_errors_and_leaves_buffers_untouched() {
        let channels = shared_channels(channel::FileKind::I16, &[(1.0, 0.0), (1.0, 0.0)], 8, 8, 2);
        let abf = abf_with_sweeps(channels, 2);

        let mut ok = vec![vec![7.0f32; 4], vec![7.0f32; 4]];
        assert!(matches!(
            abf.read_sweep_all_channels_into(2, &mut ok),
            Err(AbfError::SweepOutOfRange {
                sweep: 2,
                sweeps_count: 2
            })
        ));
        assert!(abf.sweep_all_channels(2).is_none());
        assert!(abf.sweep_all_channels(usize::MAX).is_none());

        let mut one = vec![vec![7.0f32; 4]];
        assert!(matches!(
            abf.read_sweep_all_channels_into(0, &mut one),
            Err(AbfError::ChannelCountMismatch {
                expected: 2,
                actual: 1
            })
        ));
        let mut three = vec![vec![7.0f32; 4]; 3];
        assert!(matches!(
            abf.read_sweep_all_channels_into(0, &mut three),
            Err(AbfError::ChannelCountMismatch {
                expected: 2,
                actual: 3
            })
        ));

        // The first buffer is valid, the second is wrong: nothing is written.
        let mut bad = vec![vec![7.0f32; 4], vec![7.0f32; 3]];
        assert!(matches!(
            abf.read_sweep_all_channels_into(0, &mut bad),
            Err(AbfError::BufferLengthMismatch {
                expected: 4,
                actual: 3
            })
        ));

        for b in ok.iter().chain(one.iter()).chain(three.iter()) {
            assert!(b.iter().all(|&v| v == 7.0));
        }
        assert!(bad[0].iter().all(|&v| v == 7.0));
        assert!(bad[1].iter().all(|&v| v == 7.0));
    }

    #[test]
    fn all_channels_errors_leave_buffers_untouched_on_single_pass_path() {
        let frames = decode::BLOCK_FRAMES * 11 + 123;
        let channels = shared_channels(
            channel::FileKind::I16,
            &[(1.0, 0.0), (1.0, 0.0), (1.0, 0.0)],
            frames,
            frames,
            1,
        );
        let abf = abf_with_sweeps(channels, 1);
        assert!(abf.uses_single_pass());
        let mut bad = vec![vec![7.0f32; frames], vec![7.0f32; frames], vec![7.0f32; 1]];
        assert!(matches!(
            abf.read_sweep_all_channels_into(0, &mut bad),
            Err(AbfError::BufferLengthMismatch { .. })
        ));
        assert!(bad.iter().all(|b| b.iter().all(|&v| v == 7.0)));
    }

    #[test]
    fn all_channels_zero_channel_abf() {
        let abf = abf_with_channels(Vec::new());
        assert!(!abf.uses_single_pass());
        let mut empty: Vec<Vec<f32>> = Vec::new();
        abf.read_sweep_all_channels_into(0, &mut empty).unwrap();
        assert_eq!(abf.sweep_all_channels(0), Some(Vec::new()));
        assert!(abf.sweep_all_channels(1).is_none());
        assert!(matches!(
            abf.read_sweep_all_channels_into(1, &mut empty),
            Err(AbfError::SweepOutOfRange { .. })
        ));
        let mut one = vec![vec![0.0f32; 0]];
        assert!(matches!(
            abf.read_sweep_all_channels_into(0, &mut one),
            Err(AbfError::ChannelCountMismatch {
                expected: 0,
                actual: 1
            })
        ));
    }

    #[test]
    fn read_sweep_all_channels_into_accepts_vec_box_and_slice_buffers() {
        let channels = shared_channels(channel::FileKind::I16, &[(0.5, 1.0), (2.0, -1.0)], 6, 6, 1);
        let abf = abf_with_sweeps(channels, 1);
        let expected = abf.sweep_all_channels(0).unwrap();
        assert_eq!(expected[0].len(), 6);

        let mut vecs = vec![vec![f32::NAN; 6]; 2];
        abf.read_sweep_all_channels_into(0, &mut vecs).unwrap();
        assert_eq!(vecs, expected);

        let mut boxes: Vec<Box<[f32]>> = vec![vec![f32::NAN; 6].into_boxed_slice(); 2];
        abf.read_sweep_all_channels_into(0, &mut boxes).unwrap();
        assert_eq!(boxes[0].to_vec(), expected[0]);
        assert_eq!(boxes[1].to_vec(), expected[1]);

        let mut a = [f32::NAN; 6];
        let mut b = [f32::NAN; 6];
        let mut slices: [&mut [f32]; 2] = [&mut a, &mut b];
        abf.read_sweep_all_channels_into(0, &mut slices).unwrap();
        assert_eq!(a.to_vec(), expected[0]);
        assert_eq!(b.to_vec(), expected[1]);
    }
}
