use super::*;
use crate::byte_reader::ByteReader;
use crate::error::AbfError;

pub struct SectionProducer<'a> {
    reader: ByteReader<'a>,
}

impl<'a> SectionProducer<'a> {
    pub fn new(reader: ByteReader<'a>) -> Self {
        Self { reader }
    }
    pub fn get_protocol_section(&self) -> Result<Section<'a, ProtocolSectionType>, AbfError> {
        Section::new(
            self.reader,
            76,
            std::marker::PhantomData::<ProtocolSectionType>,
        )
    }
    pub fn get_adc_section(&self) -> Result<Section<'a, AdcSectionType>, AbfError> {
        Section::new(self.reader, 92, std::marker::PhantomData::<AdcSectionType>)
    }

    pub fn get_dac_section(&self) -> Result<Section<'a, DacSectionType>, AbfError> {
        Section::new(self.reader, 108, std::marker::PhantomData::<DacSectionType>)
    }
    pub fn get_strings_section(&self) -> Result<Section<'a, StringsSectionType>, AbfError> {
        Section::new(
            self.reader,
            220,
            std::marker::PhantomData::<StringsSectionType>,
        )
    }
    pub fn get_data_section(&self) -> Result<Section<'a, DataSectionType>, AbfError> {
        Section::new(
            self.reader,
            236,
            std::marker::PhantomData::<DataSectionType>,
        )
    }
}
