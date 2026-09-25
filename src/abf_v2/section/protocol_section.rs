use super::{ProtocolSectionType, Section};
use crate::error::AbfError;

impl Section<'_, ProtocolSectionType> {
    /// `nOperationMode`: the acquisition mode (1 = event-driven
    /// variable-length, 2 = event-driven fixed-length, 3 = gap-free,
    /// 4 = high-speed oscilloscope, 5 = episodic stimulation).
    pub fn operation_mode(&self) -> Result<i16, AbfError> {
        self.reader.read_i16("protocol", self.block_number as usize)
    }

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
