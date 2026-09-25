use crate::error::AbfError;
use crate::storage::Storage;
use std::sync::Arc;

/// The on-disk sample representation of a channel, mirroring ABF2's `nDataFormat`
/// header field (`0` => int16, anything else => float32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    I16,
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
/// on demand, straight out of the shared [`Storage`], when
/// [`Channel::get_sweep`] or [`Channel::get_raw_sweep`] is called.
pub struct Channel {
    storage: Arc<Storage>,
    data_offset: usize,
    channel_index: usize,
    channel_count: usize,
    file_kind: FileKind,
    sweep_len: usize,
    sweeps_count: u32,
    uom: String,
    gain: f32,
    offset: f32,
    label: String,
}

impl Channel {
    pub(crate) fn new(
        layout: ChannelLayout,
        uom: String,
        gain: f32,
        offset: f32,
        label: String,
        sweeps_count: u32,
    ) -> Self {
        let sweep_len = layout.samples_per_channel / sweeps_count.max(1) as usize;
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

    pub fn get_uom(&self) -> &str {
        &self.uom
    }

    pub fn get_label(&self) -> &str {
        &self.label
    }

    pub fn get_gain(&self) -> f32 {
        self.gain
    }

    pub fn get_offset(&self) -> f32 {
        self.offset
    }

    pub fn get_file_kind(&self) -> FileKind {
        self.file_kind
    }

    /// Absolute byte offset of this channel's `per_channel_index`-th sample
    /// (0-indexed, across all sweeps concatenated).
    fn sample_byte_offset(&self, per_channel_index: usize) -> usize {
        self.data_offset
            + (per_channel_index * self.channel_count + self.channel_index)
                * self.file_kind.sample_width()
    }

    /// Decodes the raw int16 sample at `per_channel_index`, or `0` if the
    /// computed offset somehow falls outside the backing storage. The latter
    /// should not happen for a `Channel` built from validated section
    /// metadata, but decoding never panics either way.
    fn read_i16(&self, per_channel_index: usize) -> i16 {
        let offset = self.sample_byte_offset(per_channel_index);
        self.storage
            .bytes()
            .get(offset..offset + 2)
            .and_then(|b| <[u8; 2]>::try_from(b).ok())
            .map(i16::from_le_bytes)
            .unwrap_or(0)
    }

    /// Decodes the raw float32 sample at `per_channel_index`, or `0.0` if the
    /// computed offset somehow falls outside the backing storage (see
    /// [`Channel::read_i16`]).
    fn read_f32(&self, per_channel_index: usize) -> f32 {
        let offset = self.sample_byte_offset(per_channel_index);
        self.storage
            .bytes()
            .get(offset..offset + 4)
            .and_then(|b| <[u8; 4]>::try_from(b).ok())
            .map(f32::from_le_bytes)
            .unwrap_or(0.0)
    }

    /// Returns the raw, unscaled int16 samples for a sweep.
    ///
    /// Only meaningful for [`FileKind::I16`] channels. Float32 channels are
    /// already stored in physical units (see [`Channel::get_sweep`]), so
    /// there is no int16 representation to hand back and this always
    /// returns `None` for them.
    pub fn get_raw_sweep(&self, sweep: u32) -> Option<Vec<i16>> {
        Some(self.raw_sweep_iter(sweep as usize)?.collect())
    }

    /// Returns the sweep in physical units.
    ///
    /// Int16 data is scaled by `gain`/`offset`; float32 data is returned as
    /// stored, since pyABF does not apply gain/offset scaling to it either.
    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        let mut out = vec![0.0f32; self.sweep_len];
        self.read_sweep_into(sweep as usize, &mut out).ok()?;
        Some(out)
    }

    /// Decodes a sweep's samples in physical units directly into `out`,
    /// without allocating.
    ///
    /// Int16 data is scaled by `gain`/`offset`; float32 data is written as
    /// stored (see [`Channel::get_sweep`]). `out`'s length must equal
    /// [`Channel::get_sweep_len`], and `sweep` must be a valid sweep index
    /// for this channel, otherwise this returns `Err` without modifying
    /// `out`.
    pub fn read_sweep_into(&self, sweep: usize, out: &mut [f32]) -> Result<(), AbfError> {
        if sweep >= self.sweeps_count as usize {
            return Err(AbfError::SweepOutOfRange {
                sweep,
                sweeps_count: self.sweeps_count as usize,
            });
        }
        if out.len() != self.sweep_len {
            return Err(AbfError::BufferLengthMismatch {
                expected: self.sweep_len,
                actual: out.len(),
            });
        }
        let start = self.sweep_len * sweep;
        match self.file_kind {
            FileKind::I16 => {
                for (j, o) in out.iter_mut().enumerate() {
                    *o = self.read_i16(start + j) as f32 * self.gain + self.offset;
                }
            }
            FileKind::F32 => {
                for (j, o) in out.iter_mut().enumerate() {
                    *o = self.read_f32(start + j);
                }
            }
        }
        Ok(())
    }

    /// Returns a lazy, scaled iterator over a sweep's samples in physical
    /// units, or `None` if `sweep` is out of range.
    ///
    /// Each sample is decoded on demand as the iterator is advanced, so
    /// consuming it (e.g. via `for`, `.sum()`, or `.zip(..)`) does not
    /// allocate. See [`Channel::get_sweep`] for the scaling rules.
    pub fn sweep_iter(&self, sweep: usize) -> Option<impl ExactSizeIterator<Item = f32> + '_> {
        if sweep >= self.sweeps_count as usize {
            return None;
        }
        let start = self.sweep_len * sweep;
        Some(
            (start..start + self.sweep_len).map(move |j| match self.file_kind {
                FileKind::I16 => self.read_i16(j) as f32 * self.gain + self.offset,
                FileKind::F32 => self.read_f32(j),
            }),
        )
    }

    /// Returns a lazy iterator over a sweep's raw, unscaled int16 samples,
    /// or `None` if `sweep` is out of range or the channel is float32 (see
    /// [`Channel::get_raw_sweep`]).
    ///
    /// Each sample is decoded on demand as the iterator is advanced, so
    /// consuming it does not allocate.
    pub fn raw_sweep_iter(&self, sweep: usize) -> Option<impl ExactSizeIterator<Item = i16> + '_> {
        if sweep >= self.sweeps_count as usize || self.file_kind != FileKind::I16 {
            return None;
        }
        let start = self.sweep_len * sweep;
        Some((start..start + self.sweep_len).map(move |j| self.read_i16(j)))
    }

    pub fn get_sweeps(&self) -> impl Iterator<Item = Option<Vec<f32>>> + '_ {
        (0..self.sweeps_count).map(|s| self.get_sweep(s))
    }

    pub fn get_sweep_len(&self) -> usize {
        self.sweep_len
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
        sweeps_count: u32,
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
            "test".to_string(),
            1.0,
            0.0,
            "test".to_string(),
            sweeps_count,
        );
        (file, channel)
    }

    pub(crate) fn i16_channel(
        values: &[i16],
        sweeps_count: u32,
    ) -> (tempfile::NamedTempFile, Channel) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        build_channel(&bytes, FileKind::I16, sweeps_count)
    }

    pub(crate) fn f32_channel(
        values: &[f32],
        sweeps_count: u32,
    ) -> (tempfile::NamedTempFile, Channel) {
        let bytes: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
        build_channel(&bytes, FileKind::F32, sweeps_count)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn make_channel(values: Vec<i16>, sweeps_count: u32) -> Channel {
        test_support::i16_channel(&values, sweeps_count).1
    }

    fn make_f32_channel(values: Vec<f32>, sweeps_count: u32) -> Channel {
        test_support::f32_channel(&values, sweeps_count).1
    }

    #[test]
    fn get_raw_sweep_returns_none_at_sweeps_count_boundary() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        assert_eq!(ch.get_raw_sweep(3), None);
        assert_eq!(ch.get_raw_sweep(u32::MAX), None);
    }

    #[test]
    fn get_raw_sweep_returns_exact_sweep_len_for_last_sweep() {
        let ch = make_channel(vec![0, 1, 2, 3, 4, 5], 3);
        let last = ch.get_raw_sweep(2).unwrap();
        assert_eq!(last.len(), ch.get_sweep_len());
        assert_eq!(last, vec![4, 5]);
    }

    #[test]
    fn get_raw_sweep_does_not_absorb_remainder_into_last_sweep() {
        // 7 values over 3 sweeps: sweep_len = 7 / 3 = 2 (integer division), so
        // every sweep must be exactly 2 points long and the trailing value
        // that doesn't fit evenly is excluded rather than tacked onto the
        // last sweep.
        let ch = make_channel(vec![10, 20, 30, 40, 50, 60, 70], 3);
        assert_eq!(ch.get_sweep_len(), 2);
        for s in 0..3 {
            assert_eq!(ch.get_raw_sweep(s).unwrap().len(), 2);
        }
        assert_eq!(ch.get_raw_sweep(2).unwrap(), vec![50, 60]);
    }

    #[test]
    fn get_file_kind_reflects_the_stored_representation() {
        assert_eq!(make_channel(vec![0], 1).get_file_kind(), FileKind::I16);
        assert_eq!(
            make_f32_channel(vec![0.0], 1).get_file_kind(),
            FileKind::F32
        );
    }

    #[test]
    fn f32_channel_get_raw_sweep_is_always_none() {
        let ch = make_f32_channel(vec![1.5, -2.25, 3.0, 4.0], 2);
        assert_eq!(ch.get_raw_sweep(0), None);
        assert_eq!(ch.get_raw_sweep(1), None);
    }

    #[test]
    fn f32_channel_get_sweep_returns_values_unscaled() {
        let mut ch = make_f32_channel(vec![1.5, -2.25, 3.0, 4.0], 2);
        // gain/offset must be ignored for float data even if non-default.
        ch.gain = 2.0;
        ch.offset = 100.0;
        assert_eq!(ch.get_sweep(0).unwrap(), vec![1.5, -2.25]);
        assert_eq!(ch.get_sweep(1).unwrap(), vec![3.0, 4.0]);
    }

    #[test]
    fn f32_channel_get_sweep_returns_none_out_of_range() {
        let ch = make_f32_channel(vec![1.5, -2.25], 1);
        assert_eq!(ch.get_sweep(1), None);
    }

    #[test]
    fn i16_channel_get_sweep_applies_gain_and_offset() {
        let mut ch = make_channel(vec![1, 2, 3, 4], 2);
        ch.gain = 2.0;
        ch.offset = 1.0;
        assert_eq!(ch.get_sweep(0).unwrap(), vec![3.0, 5.0]);
        assert_eq!(ch.get_sweep(1).unwrap(), vec![7.0, 9.0]);
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
            "test".to_string(),
            1.0,
            0.0,
            "ch0".to_string(),
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
            "test".to_string(),
            1.0,
            0.0,
            "ch1".to_string(),
            1,
        );
        assert_eq!(ch0.get_raw_sweep(0).unwrap(), vec![1, 2, 3]);
        assert_eq!(ch1.get_raw_sweep(0).unwrap(), vec![-1, -2, -3]);
    }
}
