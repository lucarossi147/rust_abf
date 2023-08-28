use rayon::prelude::{IntoParallelRefIterator, ParallelIterator};

#[derive(Clone, Copy)]
pub enum FileKind {
     I16,
     F32,  
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

    pub fn get_sweep(&self, sweep: u32) -> Option<Vec<f32>> {
        let len = self.values.len() / self.sweeps_count as usize;
        let data = match sweep {
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
        .map(|v| *v as f32);
        Some(match self.file_kind {
            // data in int, needs to be multiplied for the scaling factors
            FileKind::I16 => data.map(|v| v * self.gain + self.offset).collect(),
            FileKind::F32 => data.collect(),
        })
    }
}