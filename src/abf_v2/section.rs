use crate::byte_reader::ByteReader;
use crate::error::AbfError;

pub mod adc_section;
pub mod data_section;
pub mod protocol_section;
pub mod section_producer;
pub mod strings_sections;
pub mod synch_array_section;

pub struct ProtocolSectionType;
pub struct AdcSectionType;
pub struct DacSectionType;
pub struct StringsSectionType;
pub struct DataSectionType;
pub struct SynchArraySectionType;

pub struct Section<'a, SectionType> {
    reader: ByteReader<'a>,
    block_number: u32,
    byte_count: u32,
    item_count: u32,
    section_type: std::marker::PhantomData<SectionType>,
}

impl<'a, T> Section<'a, T> {
    fn new(
        reader: ByteReader<'a>,
        from: usize,
        section_type: std::marker::PhantomData<T>,
    ) -> Result<Section<'a, T>, AbfError> {
        let raw_block_number = reader.read_u32("section_header", from)?;
        let block_number = raw_block_number
            .checked_mul(512)
            .ok_or(AbfError::InvalidSection {
                section: "section_header",
                reason: "block_number * 512 overflows u32".to_string(),
            })?;
        let byte_count = reader.read_u32("section_header", from + 4)?;
        let item_count = reader.read_u32("section_header", from + 8)?;
        Ok(Section {
            reader,
            block_number,
            byte_count,
            item_count,
            section_type,
        })
    }

    /// Byte offset of the `index`-th fixed-size item in this section.
    fn item_offset(&self, index: u32) -> Result<usize, AbfError> {
        let byte_offset = index
            .checked_mul(self.byte_count)
            .and_then(|v| v.checked_add(self.block_number))
            .ok_or(AbfError::InvalidSection {
                section: "section_item",
                reason: "item offset overflows u32".to_string(),
            })?;
        usize::try_from(byte_offset).map_err(|_| AbfError::InvalidSection {
            section: "section_item",
            reason: "item offset exceeds addressable range".to_string(),
        })
    }
}
