use crate::byte_reader::{checked_offset, ByteReader};
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

/// ABF2 section blocks are 512 bytes: `uBlockIndex * 512` is the section's
/// byte offset.
const BLOCK_SIZE: u64 = 512;

/// One entry of the ABF2 section map (Axon `ProtocolStructs.h`
/// `ABF_Section`: `uBlockIndex: u32`, `uBytes: u32`, `llNumEntries: i64`).
///
/// Offsets, sizes and counts are kept as `u64` so files larger than 4 GiB
/// work; they are converted to `usize` with `try_from` where used, so a
/// 32-bit target gets an error instead of a truncated value.
pub struct Section<'a, SectionType> {
    reader: ByteReader<'a>,
    /// Byte offset of the section (`uBlockIndex * 512`).
    start: u64,
    /// `uBytes`: size of one entry.
    byte_count: u32,
    /// `llNumEntries`, rejected at parse time if negative.
    item_count: u64,
    section_type: std::marker::PhantomData<SectionType>,
}

impl<'a, T> Section<'a, T> {
    fn new(
        reader: ByteReader<'a>,
        from: usize,
        section_type: std::marker::PhantomData<T>,
    ) -> Result<Section<'a, T>, AbfError> {
        const MAP: &str = "section_header";
        let block_index = reader.read_u32(MAP, from)?;
        let byte_count = reader.read_u32(MAP, checked_offset(from, 4, MAP)?)?;
        let entries = reader.read_i64(MAP, checked_offset(from, 8, MAP)?)?;
        let item_count = u64::try_from(entries).map_err(|_| AbfError::InvalidSection {
            section: MAP,
            reason: format!("negative llNumEntries {entries}"),
        })?;
        Ok(Section {
            reader,
            start: u64::from(block_index) * BLOCK_SIZE,
            byte_count,
            item_count,
            section_type,
        })
    }

    /// Byte offset of the section in the backing storage.
    fn start(&self) -> Result<usize, AbfError> {
        to_usize(self.start, "section start")
    }

    /// Byte offset of the `index`-th fixed-size item in this section.
    fn item_offset(&self, index: u64) -> Result<usize, AbfError> {
        let byte_offset = index
            .checked_mul(u64::from(self.byte_count))
            .and_then(|v| v.checked_add(self.start))
            .ok_or(AbfError::InvalidSection {
                section: "section_item",
                reason: "item offset overflows u64".to_string(),
            })?;
        to_usize(byte_offset, "item offset")
    }
}

/// Converts a file offset, size or count to `usize`, erroring (never
/// truncating) when it doesn't fit, e.g. past 4 GiB on a 32-bit target.
fn to_usize(value: u64, what: &str) -> Result<usize, AbfError> {
    usize::try_from(value).map_err(|_| AbfError::InvalidSection {
        section: "section_header",
        reason: format!("{what} {value} exceeds addressable range"),
    })
}
