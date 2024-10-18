use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

#[derive(Clone, Copy)]
pub enum FileKind {
     I16,
    //  F32,  
}
pub struct Channel {
    // channel_kind: ChannelKind,
    values: Vec<i16>,
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
        values: Vec<i16>,
        uom: String,
        gain: f32,
        offset: f32,
        label: String,
        sweeps_count: u32,
        file_kind: FileKind,
    ) -> Self {
        Self{
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
        let len = self.get_sweep_len();
        if sweep > self.sweeps_count {
            return None;
        }
        Some(match sweep {
            0 => &self.values[0..len],
            n =>{
                let usize_n = n as usize;
                if n == self.sweeps_count - 1 {
                    &self.values[ len * usize_n .. ]
                } else {
                    &self.values[ len * usize_n .. len * ( usize_n + 1 ) ]
                }
            } 
        }
        .par_iter()
        .map(|v| *v as i16)
        .collect())
    }

    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        let sweep = self.get_raw_sweep(sweep);
        match sweep {
            Some(s) => {
                Some(match self.file_kind {
                    // data in int, needs to be multiplied for the scaling factors
                    FileKind::I16 => s.into_iter().map(|v|v as f32).map(|v| v * self.gain + self.offset).collect(),
                    // FileKind::F32 => data.collect(),
                })
            },
            None => None
        }
    }

    pub fn get_sweeps(&self) -> impl Iterator<Item = Option<Vec<f32>>> +'_ {
        (0..self.sweeps_count).map(|s| self.get_sweep(s))
    }

    pub fn get_sweep_len(&self) -> usize {
        self.values.len() / self.sweeps_count as usize
    }
}