use crate::decode::{decode, le_f32, le_i16, EitherIter, StridedIter};
use crate::error::AbfError;
use crate::storage::Storage;
use std::fmt;
use std::sync::Arc;

/// The on-disk sample representation of a channel, mirroring ABF2's `nDataFormat`
/// header field (`0` => int16, anything else => float32).
///
/// # Examples
///
/// ```
/// use rust_abf::{Abf, FileKind};
/// use std::path::Path;
///
/// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
/// let channel = abf.channel(0).unwrap();
/// match channel.file_kind() {
///     FileKind::I16 => assert!(channel.raw_sweep(0).is_some()),
///     FileKind::F32 => assert!(channel.raw_sweep(0).is_none()),
/// }
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    /// Samples are stored as `i16`, scaled by `gain`/`offset` to produce
    /// physical units (see [`Channel::sweep`]).
    I16,
    /// Samples are stored as `f32`, already in physical units.
    F32,
}

impl FileKind {
    pub(crate) fn sample_width(self) -> usize {
        match self {
            FileKind::I16 => 2,
            FileKind::F32 => 4,
        }
    }
}

/// Where a channel's samples live in the backing [`Storage`], and how to
/// find them: they are interleaved with `channel_count - 1` other channels'
/// samples starting at `data_offset`, so the channel's `j`-th sample (across
/// all sweeps) lives at byte offset
/// `data_offset + (j * channel_count + channel_index) * file_kind.sample_width()`.
pub(crate) struct ChannelLayout {
    pub(crate) storage: Arc<Storage>,
    pub(crate) data_offset: usize,
    pub(crate) channel_index: usize,
    pub(crate) channel_count: usize,
    pub(crate) samples_per_channel: usize,
    pub(crate) file_kind: FileKind,
}

/// A single channel's metadata plus a lazy view into its samples.
///
/// No sample data is copied at construction time: every sweep is decoded
/// on demand, straight out of the shared `Storage`, when
/// [`Channel::sweep`] or [`Channel::raw_sweep`] is called. A `Channel` is
/// obtained from an [`crate::Abf`] via [`crate::Abf::channel`] or
/// [`crate::Abf::channels`].
///
/// # Examples
///
/// ```
/// use rust_abf::Abf;
/// use std::path::Path;
///
/// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
/// let channel = abf.channel(0).unwrap();
/// let sweep = channel.sweep(0).unwrap();
/// assert_eq!(sweep.len(), channel.sweep_len());
/// ```
pub struct Channel {
    storage: Arc<Storage>,
    data_offset: usize,
    channel_index: usize,
    channel_count: usize,
    file_kind: FileKind,
    sweep_len: usize,
    sweeps_count: usize,
    uom: Option<String>,
    gain: f32,
    offset: f32,
    label: Option<String>,
}

impl fmt::Debug for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Channel")
            .field("index", &self.channel_index)
            .field("label", &self.label)
            .field("uom", &self.uom)
            .field("gain", &self.gain)
            .field("offset", &self.offset)
            .field("file_kind", &self.file_kind)
            .field("sweep_len", &self.sweep_len)
            .field("sweeps_count", &self.sweeps_count)
            .finish()
    }
}

impl Channel {
    pub(crate) fn new(
        layout: ChannelLayout,
        uom: Option<String>,
        gain: f32,
        offset: f32,
        label: Option<String>,
        sweeps_count: usize,
    ) -> Self {
        let sweep_len = layout.samples_per_channel / sweeps_count.max(1);
        Self {
            storage: layout.storage,
            data_offset: layout.data_offset,
            channel_index: layout.channel_index,
            channel_count: layout.channel_count,
            file_kind: layout.file_kind,
            sweep_len,
            sweeps_count,
            uom,
            gain,
            offset,
            label,
        }
    }

    /// This channel's index within its [`crate::Abf`]'s channel list, i.e.
    /// the position [`crate::Abf::channel`]/[`crate::Abf::channels`] would
    /// return it at.
    #[must_use]
    pub fn index(&self) -> usize {
        self.channel_index
    }

    /// The channel's unit of measurement (e.g. `"pA"`, `"mV"`), or `None` if
    /// the ABF file's string table has no entry for it.
    #[must_use]
    pub fn uom(&self) -> Option<&str> {
        self.uom.as_deref()
    }

    /// Deprecated alias for [`Channel::uom`], falling back to `"nan"`
    /// instead of `None` when there is no unit of measurement.
    #[deprecated(since = "0.5.0", note = "use `Channel::uom` instead")]
    pub fn get_uom(&self) -> &str {
        self.uom().unwrap_or("nan")
    }

    /// The channel's label (e.g. `"IN 0"`), or `None` if the ABF file's
    /// string table has no entry for it.
    #[must_use]
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Deprecated alias for [`Channel::label`], falling back to `"nan"`
    /// instead of `None` when there is no label.
    #[deprecated(since = "0.5.0", note = "use `Channel::label` instead")]
    pub fn get_label(&self) -> &str {
        self.label().unwrap_or("nan")
    }

    /// The multiplier applied to raw [`FileKind::I16`] samples to produce
    /// physical units (see [`Channel::sweep`]). Unused for [`FileKind::F32`]
    /// channels.
    #[must_use]
    pub fn gain(&self) -> f32 {
        self.gain
    }

    /// Deprecated alias for [`Channel::gain`].
    #[deprecated(since = "0.5.0", note = "use `Channel::gain` instead")]
    pub fn get_gain(&self) -> f32 {
        self.gain()
    }

    /// The value added to raw [`FileKind::I16`] samples, after scaling by
    /// [`Channel::gain`], to produce physical units. Unused for
    /// [`FileKind::F32`] channels.
    #[must_use]
    pub fn offset(&self) -> f32 {
        self.offset
    }

    /// Deprecated alias for [`Channel::offset`].
    #[deprecated(since = "0.5.0", note = "use `Channel::offset` instead")]
    pub fn get_offset(&self) -> f32 {
        self.offset()
    }

    /// The on-disk sample representation of this channel.
    #[must_use]
    pub fn file_kind(&self) -> FileKind {
        self.file_kind
    }

    /// Deprecated alias for [`Channel::file_kind`].
    #[deprecated(since = "0.5.0", note = "use `Channel::file_kind` instead")]
    pub fn get_file_kind(&self) -> FileKind {
        self.file_kind()
    }

    /// Distance in bytes between two consecutive samples of this channel.
    pub(crate) fn stride(&self) -> usize {
        self.channel_count
            .saturating_mul(self.file_kind.sample_width())
    }

    /// The bytes spanning this channel's samples of `sweep`: starting at its
    /// first sample and ending right after its last one, with consecutive
    /// samples [`Channel::stride`] bytes apart.
    ///
    /// The whole range is validated here, once per sweep, with checked
    /// arithmetic, so the decode loops need no per-sample bounds checks and a
    /// corrupt header can never overflow or panic. Returns `None` if the
    /// range overflows or runs past the end of the data (a truncated file);
    /// callers then yield zeros for the whole sweep.
    fn sweep_bytes<'a>(&self, bytes: &'a [u8], sweep: usize) -> Option<&'a [u8]> {
        let width = self.file_kind.sample_width();
        let stride = self.channel_count.checked_mul(width)?;
        let start = self
            .sweep_len
            .checked_mul(sweep)?
            .checked_mul(stride)?
            .checked_add(self.data_offset)?
            .checked_add(self.channel_index.checked_mul(width)?)?;
        let len = match self.sweep_len.checked_sub(1) {
            None => 0,
            Some(n) => n.checked_mul(stride)?.checked_add(width)?,
        };
        bytes.get(start..start.checked_add(len)?)
    }

    /// [`Channel::sweep_bytes`] for this channel's own storage, or an empty
    /// slice if the range is invalid.
    fn sweep_src(&self, sweep: usize) -> Option<&[u8]> {
        self.sweep_bytes(self.storage.bytes(), sweep)
    }

    /// Byte range of *all* channels' frames for `sweep` (used by the
    /// single-pass multi-channel decoder in [`crate::Abf`]).
    pub(crate) fn frames_bytes(&self, sweep: usize) -> Option<&[u8]> {
        let stride = self.stride();
        let start = self
            .sweep_len
            .checked_mul(sweep)?
            .checked_mul(stride)?
            .checked_add(self.data_offset)?;
        let len = self.sweep_len.checked_mul(stride)?;
        self.storage.bytes().get(start..start.checked_add(len)?)
    }

    /// Decodes `sweep` (already range-checked) into `out` (already exactly
    /// `sweep_len` long) in physical units. `allow_par` lets allocating
    /// callers use the `parallel` feature; allocation-free callers pass
    /// `false`. A truncated or corrupt data range decodes as all zeros.
    pub(crate) fn decode_sweep_unchecked(&self, sweep: usize, out: &mut [f32], allow_par: bool) {
        let Some(src) = self.sweep_src(sweep) else {
            out.fill(0.0);
            return;
        };
        let stride = self.stride();
        let (gain, offset) = (self.gain, self.offset);
        let scale = move |c: &[u8]| le_i16(c) as f32 * gain + offset;
        match (self.file_kind, allow_par) {
            (FileKind::I16, true) => crate::parallel::decode_maybe_par(src, stride, out, scale),
            (FileKind::I16, false) => decode(src, stride, out, scale),
            (FileKind::F32, true) => crate::parallel::decode_maybe_par(src, stride, out, le_f32),
            (FileKind::F32, false) => decode(src, stride, out, le_f32),
        }
    }

    /// Returns the raw, unscaled int16 samples for a sweep.
    ///
    /// Only meaningful for [`FileKind::I16`] channels. Float32 channels are
    /// already stored in physical units (see [`Channel::sweep`]), so
    /// there is no int16 representation to hand back and this always
    /// returns `None` for them.
    #[must_use]
    pub fn raw_sweep(&self, sweep: usize) -> Option<Vec<i16>> {
        if sweep >= self.sweeps_count || self.file_kind != FileKind::I16 {
            return None;
        }
        let mut out = vec![0i16; self.sweep_len];
        if let Some(src) = self.sweep_src(sweep) {
            crate::parallel::decode_maybe_par(src, self.stride(), &mut out, le_i16);
        }
        Some(out)
    }

    /// Deprecated alias for [`Channel::raw_sweep`].
    #[deprecated(since = "0.5.0", note = "use `Channel::raw_sweep` instead")]
    pub fn get_raw_sweep(&self, sweep: u32) -> Option<Vec<i16>> {
        self.raw_sweep(sweep as usize)
    }

    /// Returns the sweep in physical units.
    ///
    /// Int16 data is scaled by `gain`/`offset`; float32 data is returned as
    /// stored, since pyABF does not apply gain/offset scaling to it either.
    #[must_use]
    pub fn sweep(&self, sweep: usize) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        let mut out = vec![0.0f32; self.sweep_len];
        self.decode_sweep_unchecked(sweep, &mut out, true);
        Some(out)
    }

    /// Deprecated alias for [`Channel::sweep`].
    #[deprecated(since = "0.5.0", note = "use `Channel::sweep` instead")]
    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        self.sweep(sweep as usize)
    }

    /// Decodes a sweep's samples in physical units directly into `out`,
    /// without allocating.
    ///
    /// Int16 data is scaled by `gain`/`offset`; float32 data is written as
    /// stored (see [`Channel::sweep`]).
    ///
    /// # Errors
    ///
    /// Returns `Err` without modifying `out` if:
    /// - [`AbfError::SweepOutOfRange`]: `sweep` is not a valid sweep index
    ///   for this channel.
    /// - [`AbfError::BufferLengthMismatch`]: `out`'s length does not equal
    ///   [`Channel::sweep_len`].
    ///
    /// # Examples
    ///
    /// ```
    /// use rust_abf::Abf;
    /// use std::path::Path;
    ///
    /// let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
    /// let channel = abf.channel(0).unwrap();
    /// let mut buffer = vec![0.0f32; channel.sweep_len()];
    /// channel.read_sweep_into(0, &mut buffer).unwrap();
    /// ```
    pub fn read_sweep_into(&self, sweep: usize, out: &mut [f32]) -> Result<(), AbfError> {
        if sweep >= self.sweeps_count {
            return Err(AbfError::SweepOutOfRange {
                sweep,
                sweeps_count: self.sweeps_count,
            });
        }
        if out.len() != self.sweep_len {
            return Err(AbfError::BufferLengthMismatch {
                expected: self.sweep_len,
                actual: out.len(),
            });
        }
        self.decode_sweep_unchecked(sweep, out, false);
        Ok(())
    }

    /// Returns a lazy, scaled iterator over a sweep's samples in physical
    /// units, or `None` if `sweep` is out of range.
    ///
    /// Each sample is decoded on demand as the iterator is advanced, so
    /// consuming it (e.g. via `for`, `.sum()`, or `.zip(..)`) does not
    /// allocate. See [`Channel::sweep`] for the scaling rules.
    pub fn sweep_iter(&self, sweep: usize) -> Option<impl ExactSizeIterator<Item = f32> + '_> {
        if sweep >= self.sweeps_count {
            return None;
        }
        // A truncated or corrupt range decodes as silence (0.0), like
        // `read_sweep_into`: missing samples read as raw 0, so the scaling
        // must be zeroed too or they would come out as `offset`.
        let (src, gain, offset) = match self.sweep_src(sweep) {
            Some(src) => (src, self.gain, self.offset),
            None => (&[][..], 0.0, 0.0),
        };
        let (stride, len) = (self.stride(), self.sweep_len);
        Some(match self.file_kind {
            FileKind::I16 => EitherIter::Left(StridedIter::<_, _, 2>::new(
                src,
                stride,
                len,
                move |c: &[u8]| le_i16(c) as f32 * gain + offset,
            )),
            FileKind::F32 => {
                EitherIter::Right(StridedIter::<_, _, 4>::new(src, stride, len, le_f32))
            }
        })
    }

    /// Returns a lazy iterator over a sweep's raw, unscaled int16 samples,
    /// or `None` if `sweep` is out of range or the channel is float32 (see
    /// [`Channel::raw_sweep`]).
    ///
    /// Each sample is decoded on demand as the iterator is advanced, so
    /// consuming it does not allocate.
    pub fn raw_sweep_iter(&self, sweep: usize) -> Option<impl ExactSizeIterator<Item = i16> + '_> {
        if sweep >= self.sweeps_count || self.file_kind != FileKind::I16 {
            return None;
        }
        let src = self.sweep_src(sweep).unwrap_or(&[]);
        Some(StridedIter::<_, _, 2>::new(
            src,
            self.stride(),
            self.sweep_len,
            le_i16,
        ))
    }

    /// Returns an iterator over every sweep in physical units, in order
    /// (one item per sweep recorded in the file); every item is `Some` in
    /// practice, since the indices are always in range. See
    /// [`Channel::sweep`] for the scaling rules.
    pub fn sweeps(&self) -> impl Iterator<Item = Option<Vec<f32>>> + '_ {
        (0..self.sweeps_count).map(|s| self.sweep(s))
    }

    /// Deprecated alias for [`Channel::sweeps`].
    #[deprecated(since = "0.5.0", note = "use `Channel::sweeps` instead")]
    pub fn get_sweeps(&self) -> impl Iterator<Item = Option<Vec<f32>>> + '_ {
        self.sweeps()
    }

    /// The number of samples in one sweep of this channel.
    #[must_use]
    pub fn sweep_len(&self) -> usize {
        self.sweep_len
    }

    /// Deprecated alias for [`Channel::sweep_len`].
    #[deprecated(since = "0.5.0", note = "use `Channel::sweep_len` instead")]
    pub fn get_sweep_len(&self) -> usize {
        self.sweep_len()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
pub(crate) mod test_support {
    use super::*;
    use std::io::Write;

    /// Builds a single-channel `Channel` backed by a real memory-mapped
    /// temporary file, so unit tests exercise the same lazy decode path as
    /// production code instead of a synthetic in-memory shortcut.
    fn build_channel(
        bytes: &[u8],
        file_kind: FileKind,
        sweeps_count: usize,
    ) -> (tempfile::NamedTempFile, Channel) {
        let mut file = tempfile::NamedTempFile::new().expect("create temp file");
        file.write_all(bytes).expect("write temp file");
        file.flush().expect("flush temp file");
        let mmap = unsafe { memmap2::Mmap::map(file.as_file()).expect("mmap temp file") };
        let storage = Arc::new(Storage::Mmap(mmap));
        let samples_per_channel = bytes.len() / file_kind.sample_width();
        let layout = ChannelLayout {
            storage,
            data_offset: 0,
            channel_index: 0,
            channel_count: 1,
            samples_per_channel,
            file_kind,
        };
        let channel = Channel::new(
            layout,
            Some("test".to_string()),
            1.0,
            0.0,
            Some("test".to_string()),
            sweeps_count,
        );
        (file, channel)
    }

    pub(crate) fn i16_channel(
        values: &[i16],
        sweeps_count: usize,
    ) -> (tempfile::NamedTempFile, Channel) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        build_channel(&bytes, FileKind::I16, sweeps_count)
    }

    pub(crate) fn f32_channel(
        values: &[f32],
        sweeps_count: usize,
    ) -> (tempfile::NamedTempFile, Channel) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        build_channel(&bytes, FileKind::F32, sweeps_count)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn make_channel(values: Vec<i16>, sweeps_count: usize) -> Channel {
        test_support::i16_channel(&values, sweeps_count).1
    }

    fn make_f32_channel(values: Vec<f32>, sweeps_count: usize) -> Channel {
        test_support::f32_channel(&values, sweeps_count).1
    }

    #[test]
    fn get_raw_sweep_returns_none_at_sweeps_count_boundary() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        assert_eq!(ch.raw_sweep(3), None);
        assert_eq!(ch.raw_sweep(usize::MAX), None);
    }

    #[test]
    fn get_raw_sweep_returns_exact_sweep_len_for_last_sweep() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        let last = ch.raw_sweep(2).unwrap();
        assert_eq!(last.len(), ch.sweep_len());
        assert_eq!(last, vec![4, 5]);
    }

    #[test]
    fn get_raw_sweep_does_not_absorb_remainder_into_last_sweep() {
        // 7 values over 3 sweeps: sweep_len = 7 / 3 = 2 (integer division), so
        // every sweep must be exactly 2 points long and the trailing value
        // that doesn't fit evenly is excluded rather than tacked onto the
        // last sweep.
        let ch = make_channel(vec![10, 20, 30, 40, 50, 60, 70], 3);
        assert_eq!(ch.sweep_len(), 2);
        for s in 0..3 {
            assert_eq!(ch.raw_sweep(s).unwrap().len(), 2);
        }
        assert_eq!(ch.raw_sweep(2).unwrap(), vec![50, 60]);
    }

    #[test]
    fn get_file_kind_reflects_the_stored_representation() {
        assert_eq!(make_channel(vec![0], 1).file_kind(), FileKind::I16);
        assert_eq!(make_f32_channel(vec![0.0], 1).file_kind(), FileKind::F32);
    }

    #[test]
    fn f32_channel_get_raw_sweep_is_always_none() {
        let ch = make_f32_channel(vec![1.5, -2.25, 3.0, 4.0], 2);
        assert_eq!(ch.raw_sweep(0), None);
        assert_eq!(ch.raw_sweep(1), None);
    }

    #[test]
    fn f32_channel_get_sweep_returns_values_unscaled() {
        let mut ch = make_f32_channel(vec![1.5, -2.25, 3.0, 4.0], 2);
        // gain/offset must be ignored for float data even if non-default.
        ch.gain = 2.0;
        ch.offset = 100.0;
        assert_eq!(ch.sweep(0).unwrap(), vec![1.5, -2.25]);
        assert_eq!(ch.sweep(1).unwrap(), vec![3.0, 4.0]);
    }

    #[test]
    fn f32_channel_get_sweep_returns_none_out_of_range() {
        let ch = make_f32_channel(vec![1.5, -2.25], 1);
        assert_eq!(ch.sweep(1), None);
    }

    #[test]
    fn i16_channel_get_sweep_applies_gain_and_offset() {
        let mut ch = make_channel(vec![1, 2, 3, 4], 2);
        ch.gain = 2.0;
        ch.offset = 1.0;
        assert_eq!(ch.sweep(0).unwrap(), vec![3.0, 5.0]);
        assert_eq!(ch.sweep(1).unwrap(), vec![7.0, 9.0]);
    }

    #[test]
    fn read_sweep_into_returns_err_on_sweep_out_of_range() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        let mut out = [0.0f32; 2];
        assert!(matches!(
            ch.read_sweep_into(3, &mut out),
            Err(AbfError::SweepOutOfRange {
                sweep: 3,
                sweeps_count: 3
            })
        ));
    }

    #[test]
    fn read_sweep_into_returns_err_on_wrong_buffer_length() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        let mut too_short = [0.0f32; 1];
        assert!(matches!(
            ch.read_sweep_into(0, &mut too_short),
            Err(AbfError::BufferLengthMismatch {
                expected: 2,
                actual: 1
            })
        ));
        let mut too_long = [0.0f32; 3];
        assert!(matches!(
            ch.read_sweep_into(0, &mut too_long),
            Err(AbfError::BufferLengthMismatch {
                expected: 2,
                actual: 3
            })
        ));
    }

    #[test]
    fn read_sweep_into_fills_buffer_with_scaled_i16_values() {
        let mut ch = make_channel(vec![1, 2, 3, 4], 2);
        ch.gain = 2.0;
        ch.offset = 1.0;
        let mut out = [0.0f32; 2];
        ch.read_sweep_into(0, &mut out).unwrap();
        assert_eq!(out, [3.0, 5.0]);
        ch.read_sweep_into(1, &mut out).unwrap();
        assert_eq!(out, [7.0, 9.0]);
    }

    #[test]
    fn read_sweep_into_fills_buffer_with_unscaled_f32_values() {
        let mut ch = make_f32_channel(vec![1.5, -2.25, 3.0, 4.0], 2);
        ch.gain = 2.0;
        ch.offset = 100.0;
        let mut out = [0.0f32; 2];
        ch.read_sweep_into(0, &mut out).unwrap();
        assert_eq!(out, [1.5, -2.25]);
        ch.read_sweep_into(1, &mut out).unwrap();
        assert_eq!(out, [3.0, 4.0]);
    }

    #[test]
    fn sweep_iter_returns_none_out_of_range() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        assert!(ch.sweep_iter(3).is_none());
        assert!(ch.sweep_iter(usize::MAX).is_none());
    }

    #[test]
    fn sweep_iter_yields_scaled_values_and_reports_exact_len() {
        let mut ch = make_channel(vec![1, 2, 3, 4], 2);
        ch.gain = 2.0;
        ch.offset = 1.0;
        let iter = ch.sweep_iter(1).unwrap();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.collect::<Vec<_>>(), vec![7.0, 9.0]);
    }

    #[test]
    fn raw_sweep_iter_returns_none_for_f32_channel() {
        let ch = make_f32_channel(vec![1.5, -2.25], 1);
        assert!(ch.raw_sweep_iter(0).is_none());
    }

    #[test]
    fn raw_sweep_iter_returns_none_out_of_range() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        assert!(ch.raw_sweep_iter(3).is_none());
    }

    #[test]
    fn raw_sweep_iter_yields_raw_values_and_reports_exact_len() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        let iter = ch.raw_sweep_iter(2).unwrap();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.collect::<Vec<_>>(), vec![4, 5]);
    }

    #[test]
    fn index_reflects_position_in_the_channel_layout() {
        let (_file, ch1) = test_support::i16_channel(&[1, 2, 3], 1);
        assert_eq!(ch1.index(), 0);
    }

    #[test]
    fn uom_and_label_are_none_when_not_provided() {
        let layout_channel = {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            {
                use std::io::Write;
                file.write_all(&[0, 0]).unwrap();
                file.flush().unwrap();
            }
            let mmap = unsafe { memmap2::Mmap::map(file.as_file()).unwrap() };
            let storage = Arc::new(Storage::Mmap(mmap));
            let channel = Channel::new(
                ChannelLayout {
                    storage,
                    data_offset: 0,
                    channel_index: 0,
                    channel_count: 1,
                    samples_per_channel: 1,
                    file_kind: FileKind::I16,
                },
                None,
                1.0,
                0.0,
                None,
                1,
            );
            (file, channel)
        };
        let (_file, ch) = layout_channel;
        assert_eq!(ch.uom(), None);
        assert_eq!(ch.label(), None);
    }

    #[test]
    fn multi_channel_layout_deinterleaves_by_stride() {
        // interleaved as ch0, ch1, ch0, ch1, ch0, ch1
        let values: [i16; 6] = [1, -1, 2, -2, 3, -3];
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        let mut file = tempfile::NamedTempFile::new().unwrap();
        {
            use std::io::Write;
            file.write_all(&bytes).unwrap();
            file.flush().unwrap();
        }
        let mmap = unsafe { memmap2::Mmap::map(file.as_file()).unwrap() };
        let storage = Arc::new(Storage::Mmap(mmap));

        let ch0 = Channel::new(
            ChannelLayout {
                storage: storage.clone(),
                data_offset: 0,
                channel_index: 0,
                channel_count: 2,
                samples_per_channel: 3,
                file_kind: FileKind::I16,
            },
            Some("test".to_string()),
            1.0,
            0.0,
            Some("ch0".to_string()),
            1,
        );
        let ch1 = Channel::new(
            ChannelLayout {
                storage,
                data_offset: 0,
                channel_index: 1,
                channel_count: 2,
                samples_per_channel: 3,
                file_kind: FileKind::I16,
            },
            Some("test".to_string()),
            1.0,
            0.0,
            Some("ch1".to_string()),
            1,
        );
        assert_eq!(ch0.raw_sweep(0).unwrap(), vec![1, 2, 3]);
        assert_eq!(ch1.raw_sweep(0).unwrap(), vec![-1, -2, -3]);
        assert_eq!(ch0.index(), 0);
        assert_eq!(ch1.index(), 1);
    }

    // ---- multi-channel / runtime-stride / truncation / overflow ----

    fn owned_storage(bytes: Vec<u8>) -> Arc<Storage> {
        Arc::new(Storage::Owned(Arc::from(bytes)))
    }

    fn layout(
        storage: &Arc<Storage>,
        data_offset: usize,
        channel_index: usize,
        channel_count: usize,
        samples_per_channel: usize,
        file_kind: FileKind,
    ) -> ChannelLayout {
        ChannelLayout {
            storage: storage.clone(),
            data_offset,
            channel_index,
            channel_count,
            samples_per_channel,
            file_kind,
        }
    }

    fn i16_bytes(values: &[i16]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    fn f32_bytes(values: &[f32]) -> Vec<u8> {
        values.iter().flat_map(|v| v.to_le_bytes()).collect()
    }

    #[test]
    fn three_channel_i16_runtime_stride_deinterleaves_every_api() {
        // 3 channels, 2 sweeps of 5 samples: 30 interleaved values.
        let per_channel = |c: usize| -> Vec<i16> {
            (0..10)
                .map(|j| (c as i16 + 1) * 100 + j as i16 - 4)
                .collect()
        };
        let mut values = Vec::new();
        for j in 0..10 {
            for c in 0..3 {
                values.push(per_channel(c)[j]);
            }
        }
        let storage = owned_storage(i16_bytes(&values));
        for c in 0..3usize {
            let (gain, offset) = (0.5 + c as f32, -1.25 * (c as f32 + 1.0));
            let ch = Channel::new(
                layout(&storage, 0, c, 3, 10, FileKind::I16),
                None,
                gain,
                offset,
                None,
                2,
            );
            assert_eq!(ch.stride(), 6);
            assert_eq!(ch.sweep_len(), 5);
            let expected_raw = per_channel(c);
            for s in 0..2 {
                let raw = &expected_raw[s * 5..(s + 1) * 5];
                let scaled: Vec<f32> = raw.iter().map(|&v| v as f32 * gain + offset).collect();
                assert_eq!(ch.raw_sweep(s).unwrap(), raw);
                assert_eq!(ch.sweep(s).unwrap(), scaled);
                let mut buf = [f32::NAN; 5];
                ch.read_sweep_into(s, &mut buf).unwrap();
                assert_eq!(buf.to_vec(), scaled);
                let it = ch.sweep_iter(s).unwrap();
                assert_eq!(it.len(), 5);
                assert_eq!(it.collect::<Vec<_>>(), scaled);
                let it = ch.raw_sweep_iter(s).unwrap();
                assert_eq!(it.len(), 5);
                assert_eq!(it.collect::<Vec<_>>(), raw);
            }
        }
    }

    #[test]
    fn three_channel_f32_stride_12_deinterleaves_every_api() {
        let per_channel =
            |c: usize| -> Vec<f32> { (0..6).map(|j| (c as f32 + 1.0) * 10.5 - j as f32).collect() };
        let mut values = Vec::new();
        for j in 0..6 {
            for c in 0..3 {
                values.push(per_channel(c)[j]);
            }
        }
        let storage = owned_storage(f32_bytes(&values));
        for c in 0..3usize {
            // gain/offset must be ignored for float data.
            let ch = Channel::new(
                layout(&storage, 0, c, 3, 6, FileKind::F32),
                None,
                3.0,
                99.0,
                None,
                2,
            );
            assert_eq!(ch.stride(), 12);
            let exp = per_channel(c);
            for s in 0..2 {
                let want = &exp[s * 3..(s + 1) * 3];
                assert_eq!(ch.raw_sweep(s), None);
                assert!(ch.raw_sweep_iter(s).is_none());
                assert_eq!(ch.sweep(s).unwrap(), want);
                let mut buf = [f32::NAN; 3];
                ch.read_sweep_into(s, &mut buf).unwrap();
                assert_eq!(buf, *want);
                let it = ch.sweep_iter(s).unwrap();
                assert_eq!(it.len(), 3);
                assert_eq!(it.collect::<Vec<_>>(), want);
            }
        }
    }

    #[test]
    fn data_offset_and_channel_index_are_honoured() {
        // 3 junk bytes, then 2 channels interleaved.
        let mut bytes = vec![0xAA, 0xBB, 0xCC];
        bytes.extend(i16_bytes(&[1, -1, 2, -2, 3, -3]));
        let storage = owned_storage(bytes);
        let ch1 = Channel::new(
            layout(&storage, 3, 1, 2, 3, FileKind::I16),
            None,
            1.0,
            0.0,
            None,
            1,
        );
        assert_eq!(ch1.raw_sweep(0).unwrap(), vec![-1, -2, -3]);
    }

    #[test]
    fn truncated_data_yields_zeros_for_the_incomplete_sweep() {
        // Claims 2 sweeps x 4 samples, but the bytes stop one byte short of
        // covering sweep 1's last sample.
        let mut bytes = i16_bytes(&[1, 2, 3, 4, 5, 6, 7, 8]);
        bytes.pop();
        let storage = owned_storage(bytes);
        let ch = Channel::new(
            layout(&storage, 0, 0, 1, 8, FileKind::I16),
            None,
            2.0,
            0.0,
            None,
            2,
        );
        assert_eq!(ch.sweep_len(), 4);
        assert_eq!(ch.raw_sweep(0).unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(ch.sweep(0).unwrap(), vec![2.0, 4.0, 6.0, 8.0]);

        assert_eq!(ch.sweep(1).unwrap(), vec![0.0; 4]);
        assert_eq!(ch.raw_sweep(1).unwrap(), vec![0; 4]);
        let mut buf = [7.0f32; 4];
        ch.read_sweep_into(1, &mut buf).unwrap();
        assert_eq!(buf, [0.0; 4]);
        let it = ch.sweep_iter(1).unwrap();
        assert_eq!(it.len(), 4);
        assert_eq!(it.collect::<Vec<_>>(), vec![0.0; 4]);
        let it = ch.raw_sweep_iter(1).unwrap();
        assert_eq!(it.len(), 4);
        assert_eq!(it.collect::<Vec<_>>(), vec![0; 4]);
    }

    #[test]
    fn truncated_multi_channel_f32_yields_zeros() {
        // 2 channels, 1 sweep of 3 frames claimed, only 2 frames present.
        let storage = owned_storage(f32_bytes(&[1.0, -1.0, 2.0, -2.0]));
        for c in 0..2 {
            let ch = Channel::new(
                layout(&storage, 0, c, 2, 3, FileKind::F32),
                None,
                1.0,
                0.0,
                None,
                1,
            );
            assert_eq!(ch.sweep(0).unwrap(), vec![0.0; 3]);
            let mut buf = [5.0f32; 3];
            ch.read_sweep_into(0, &mut buf).unwrap();
            assert_eq!(buf, [0.0; 3]);
            assert_eq!(ch.sweep_iter(0).unwrap().collect::<Vec<_>>(), vec![0.0; 3]);
        }
    }

    #[test]
    fn huge_data_offset_does_not_overflow_or_panic() {
        let storage = owned_storage(i16_bytes(&[1, 2, 3, 4]));
        let ch = Channel::new(
            layout(&storage, usize::MAX - 1, 0, 1, 4, FileKind::I16),
            None,
            1.0,
            0.0,
            None,
            1,
        );
        let mut buf = [9.0f32; 4];
        ch.read_sweep_into(0, &mut buf).unwrap();
        assert_eq!(buf, [0.0; 4]);
        let it = ch.sweep_iter(0).unwrap();
        assert_eq!(it.len(), 4);
        assert_eq!(it.collect::<Vec<_>>(), vec![0.0; 4]);
        assert_eq!(ch.raw_sweep(0).unwrap(), vec![0; 4]);
        assert_eq!(
            ch.raw_sweep_iter(0).unwrap().collect::<Vec<_>>(),
            vec![0; 4]
        );
    }

    #[test]
    fn huge_channel_count_does_not_overflow_or_panic() {
        let storage = owned_storage(i16_bytes(&[1, 2, 3, 4]));
        for (kind, index) in [(FileKind::I16, 1), (FileKind::F32, usize::MAX / 2 - 1)] {
            let ch = Channel::new(
                layout(&storage, 0, index, usize::MAX / 2, 4, kind),
                None,
                1.0,
                0.0,
                None,
                2,
            );
            for s in 0..2 {
                let mut buf = [9.0f32; 2];
                ch.read_sweep_into(s, &mut buf).unwrap();
                assert_eq!(buf, [0.0; 2]);
                let it = ch.sweep_iter(s).unwrap();
                assert_eq!(it.len(), 2);
                assert_eq!(it.collect::<Vec<_>>(), vec![0.0; 2]);
            }
        }
    }

    // BUG (reported, not fixed): on a truncated/invalid range `sweep_iter`
    // decodes the missing samples as raw 0 and then applies gain/offset, so it
    // yields `offset` instead of 0.0, unlike `sweep`/`read_sweep_into`.
    #[test]
    fn truncated_i16_sweep_iter_is_zero_even_with_nonzero_offset() {
        let mut bytes = i16_bytes(&[1, 2, 3, 4, 5, 6, 7, 8]);
        bytes.pop();
        let storage = owned_storage(bytes);
        let ch = Channel::new(
            layout(&storage, 0, 0, 1, 8, FileKind::I16),
            None,
            2.0,
            1.0,
            None,
            2,
        );
        assert_eq!(ch.sweep(1).unwrap(), vec![0.0; 4]);
        assert_eq!(ch.sweep_iter(1).unwrap().collect::<Vec<_>>(), vec![0.0; 4]);
    }
}
