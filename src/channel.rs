use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::sync::Arc;

/// The on-disk sample representation of a channel, mirroring ABF2's `nDataFormat`
/// header field (`0` => int16, anything else => float32).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    I16,
    F32,
}

/// The decoded, per-channel sample buffer.
///
/// `nDataFormat == 0` files store gain/offset-scaled int16 samples; any other
/// value means samples are already stored as physical-unit float32 values
/// (pyABF does not apply gain/offset to float data either).
#[derive(Clone)]
pub enum ChannelValues {
    I16(Arc<[i16]>),
    F32(Arc<[f32]>),
}

impl ChannelValues {
    pub fn file_kind(&self) -> FileKind {
        match self {
            ChannelValues::I16(_) => FileKind::I16,
            ChannelValues::F32(_) => FileKind::F32,
        }
    }

    fn len(&self) -> usize {
        match self {
            ChannelValues::I16(v) => v.len(),
            ChannelValues::F32(v) => v.len(),
        }
    }
}

#[derive(Clone)]
pub struct Channel {
    values: ChannelValues,
    uom: String,
    gain: f32,
    offset: f32,
    label: String,
    sweeps_count: u32,
}

impl Channel {
    pub fn new(
        values: ChannelValues,
        uom: String,
        gain: f32,
        offset: f32,
        label: String,
        sweeps_count: u32,
    ) -> Self {
        Self {
            values,
            uom,
            gain,
            offset,
            label,
            sweeps_count,
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
        self.values.file_kind()
    }

    /// Returns the raw, unscaled int16 samples for a sweep.
    ///
    /// Only meaningful for [`FileKind::I16`] channels. Float32 channels are
    /// already stored in physical units (see [`Channel::get_sweep`]), so
    /// there is no int16 representation to hand back and this always
    /// returns `None` for them.
    pub fn get_raw_sweep(&self, sweep: u32) -> Option<Vec<i16>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        match &self.values {
            ChannelValues::I16(values) => {
                let len = self.get_sweep_len();
                let start = len * sweep as usize;
                let end = start + len;
                let slice = values.get(start..end)?;
                Some(slice.par_iter().copied().collect())
            }
            ChannelValues::F32(_) => None,
        }
    }

    /// Returns the sweep in physical units.
    ///
    /// Int16 data is scaled by `gain`/`offset`; float32 data is returned as
    /// stored, since pyABF does not apply gain/offset scaling to it either.
    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        let len = self.get_sweep_len();
        let start = len * sweep as usize;
        let end = start + len;
        match &self.values {
            ChannelValues::I16(values) => {
                let slice = values.get(start..end)?;
                Some(
                    slice
                        .par_iter()
                        .map(|v| *v as f32 * self.gain + self.offset)
                        .collect(),
                )
            }
            ChannelValues::F32(values) => Some(values.get(start..end)?.to_vec()),
        }
    }

    pub fn get_sweeps(&self) -> impl Iterator<Item = Option<Vec<f32>>> + '_ {
        (0..self.sweeps_count).map(|s| self.get_sweep(s))
    }

    pub fn get_sweep_len(&self) -> usize {
        self.values.len() / self.sweeps_count as usize
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn make_channel(values: Vec<i16>, sweeps_count: u32) -> Channel {
        Channel::new(
            ChannelValues::I16(values.into()),
            "mV".to_string(),
            1.0,
            0.0,
            "test".to_string(),
            sweeps_count,
        )
    }

    fn make_f32_channel(values: Vec<f32>, sweeps_count: u32) -> Channel {
        Channel::new(
            ChannelValues::F32(values.into()),
            "pA".to_string(),
            1.0,
            0.0,
            "test".to_string(),
            sweeps_count,
        )
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
}
