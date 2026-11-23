use super::{ProtocolSectionType, Section};
use crate::byte_reader::checked_offset;
use crate::error::AbfError;

const SECTION: &str = "protocol";

impl Section<'_, ProtocolSectionType> {
    /// Byte offset of a field `extra` bytes into the protocol section.
    fn field(&self, extra: usize) -> Result<usize, AbfError> {
        checked_offset(self.start()?, extra, SECTION)
    }

    /// `nOperationMode`: the acquisition mode (1 = event-driven
    /// variable-length, 2 = event-driven fixed-length, 3 = gap-free,
    /// 4 = high-speed oscilloscope, 5 = episodic stimulation).
    pub fn operation_mode(&self) -> Result<i16, AbfError> {
        self.reader.read_i16(SECTION, self.field(0)?)
    }

    pub fn adc_sequence_interval(&self) -> Result<f32, AbfError> {
        self.reader.read_f32(SECTION, self.field(2)?)
    }

    pub fn adc_range(&self) -> Result<f32, AbfError> {
        self.reader.read_f32(SECTION, self.field(110)?)
    }

    pub fn adc_resolution(&self) -> Result<i32, AbfError> {
        self.reader.read_i32(SECTION, self.field(118)?)
    }
}
