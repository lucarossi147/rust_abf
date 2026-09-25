use crate::storage::Storage;
use rayon::prelude::*;
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
        if sweep >= self.sweeps_count || self.file_kind != FileKind::I16 {
            return None;
        }
        let start = self.sweep_len * sweep as usize;
        Some(
            (start..start + self.sweep_len)
                .into_par_iter()
                .map(|j| self.read_i16(j))
                .collect(),
        )
    }

    /// Returns the sweep in physical units.
    ///
    /// Int16 data is scaled by `gain`/`offset`; float32 data is returned as
    /// stored, since pyABF does not apply gain/offset scaling to it either.
    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        let start = self.sweep_len * sweep as usize;
        let range = start..start + self.sweep_len;
        Some(match self.file_kind {
            FileKind::I16 => range
                .into_par_iter()
                .map(|j| self.read_i16(j) as f32 * self.gain + self.offset)
                .collect(),
            FileKind::F32 => range.into_par_iter().map(|j| self.read_f32(j)).collect(),
        })
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
