use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};
use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum FileKind {
    I16,
    //  F32,
}

#[derive(Clone)]
pub struct Channel {
    // channel_kind: ChannelKind,
    values: Arc<[i16]>,
    uom: String,
    gain: f32,
    offset: f32,
    label: String,
    sweeps_count: u32,
    file_kind: FileKind,
}

impl Channel {
    pub fn new(
        // channel_kind: ChannelKind,
        values: Arc<[i16]>,
        uom: String,
        gain: f32,
        offset: f32,
        label: String,
        sweeps_count: u32,
        file_kind: FileKind,
    ) -> Self {
        Self {
            // channel_kind,
            values,
            uom,
            gain,
            offset,
            label,
            sweeps_count,
            file_kind,
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

    pub fn get_raw_sweep(&self, sweep: u32) -> Option<Vec<i16>> {
        if sweep >= self.sweeps_count {
            return None;
        }
        let len = self.get_sweep_len();
        let start = len * sweep as usize;
        let end = start + len;
        Some(self.values[start..end].par_iter().map(|v| *v).collect())
    }

    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        let sweep = self.get_raw_sweep(sweep);
        sweep.map(|s| match self.file_kind {
            // data in int, needs to be multiplied for the scaling factors
            FileKind::I16 => s
                .into_iter()
                .map(|v| v as f32)
                .map(|v| v * self.gain + self.offset)
                .collect(),
            // FileKind::F32 => data.collect(),
        })
    }

    pub fn get_sweeps(&self) -> impl Iterator<Item = Option<Vec<f32>>> + '_ {
        (0..self.sweeps_count).map(|s| self.get_sweep(s))
    }

    pub fn get_sweep_len(&self) -> usize {
        self.values.len() / self.sweeps_count as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_channel(values: Vec<i16>, sweeps_count: u32) -> Channel {
        Channel::new(
            values.into(),
            "mV".to_string(),
            1.0,
            0.0,
            "test".to_string(),
            sweeps_count,
            FileKind::I16,
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
}
