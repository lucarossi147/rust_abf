use super::{ProtocolSectionType, Section};
use crate::error::AbfError;

impl Section<'_, ProtocolSectionType> {
    pub fn adc_sequence_interval(&self) -> Result<f32, AbfError> {
        self.reader
            .read_f32("protocol", self.block_number as usize + 2)
    }

    pub fn adc_range(&self) -> Result<f32, AbfError> {
        self.reader
            .read_f32("protocol", self.block_number as usize + 110)
    }

    pub fn adc_resolution(&self) -> Result<i32, AbfError> {
        self.reader
            .read_i32("protocol", self.block_number as usize + 118)
    }
}
