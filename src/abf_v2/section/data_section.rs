use super::{to_usize, DataSectionType, Section};
use crate::channel::FileKind;
use crate::error::AbfError;

/// ABF2 `nDataFormat` value for int16 samples; any other value is float32,
/// matching pyABF's handling of the header field (see issue #8).
const DATA_FORMAT_INT16: u16 = 0;

const SECTION: &str = "data";

/// Describes where a data section's interleaved samples live in the backing
/// storage, without reading or copying any sample bytes: samples are decoded
/// lazily, straight out of the shared storage, by [`crate::channel::Channel`].
pub(crate) struct DataLayout {
    /// Absolute byte offset of the first sample in the backing storage.
    pub(crate) data_offset: usize,
    /// Total number of samples in the section, across every channel.
    pub(crate) total_items: usize,
    pub(crate) file_kind: FileKind,
}

impl Section<'_, DataSectionType> {
    /// Validates the data section's header fields against the backing
    /// storage and, if there is at least one channel, returns a
    /// [`DataLayout`] describing where its samples live. Returns `Ok(None)`
    /// for a zero-channel file, matching the absence of any data to lay out.
    pub(crate) fn layout(
        &self,
        number_of_channels: usize,
        data_format: u16,
    ) -> Result<Option<DataLayout>, AbfError> {
        let expected_width: u32 = if data_format == DATA_FORMAT_INT16 {
            2
        } else {
            4
        };
        if self.byte_count != expected_width {
            return Err(AbfError::InvalidSection {
                section: SECTION,
                reason: format!(
                    "unexpected sample width {} (expected {expected_width})",
                    self.byte_count
                ),
            });
        }
        if number_of_channels == 0 {
            return Ok(None);
        }

        let total_bytes = self
            .item_count
            .checked_mul(u64::from(self.byte_count))
            .ok_or(AbfError::InvalidSection {
                section: SECTION,
                reason: "section size overflows u64".to_string(),
            })?;
        let from = self.start()?;
        let len = to_usize(total_bytes, "data section size")?;

        // Validates that the section fits within the backing storage without
        // copying any of it: `read_bytes` returns a borrowed slice.
        self.reader.read_bytes(SECTION, from, len)?;

        let total_items = to_usize(self.item_count, "data section item count")?;
        let file_kind = if data_format == DATA_FORMAT_INT16 {
            FileKind::I16
        } else {
            FileKind::F32
        };

        Ok(Some(DataLayout {
            data_offset: from,
            total_items,
            file_kind,
        }))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_none_for_zero_channels() {
        // A 16-byte section-map entry: block index 0, byte_count=2 (matching
        // DATA_FORMAT_INT16's expected width), item_count=0. With
        // number_of_channels=0, `layout` must short circuit to `Ok(None)`
        // before doing any section-size validation.
        let mut bytes = [0u8; 16];
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        let reader = crate::byte_reader::ByteReader::new(&bytes);
        let section = Section::<DataSectionType>::new(reader, 0, std::marker::PhantomData).unwrap();
        assert!(section.layout(0, DATA_FORMAT_INT16).unwrap().is_none());
    }

    /// A 16-byte ABF2 section-map entry (`uBlockIndex: u32`, `uBytes: u32`,
    /// `llNumEntries: i64`) followed by `extra` zero bytes.
    fn section_map(block: u32, bytes: u32, entries: i64, extra: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(16 + extra);
        v.extend_from_slice(&block.to_le_bytes());
        v.extend_from_slice(&bytes.to_le_bytes());
        v.extend_from_slice(&entries.to_le_bytes());
        v.resize(16 + extra, 0);
        v
    }

    fn layout_of(bytes: &[u8]) -> Result<Option<DataLayout>, AbfError> {
        let reader = crate::byte_reader::ByteReader::new(bytes);
        Section::<DataSectionType>::new(reader, 0, std::marker::PhantomData)?
            .layout(1, DATA_FORMAT_INT16)
    }

    /// Block index 2^23 starts exactly at 4 GiB: that is a valid offset that
    /// just isn't in this tiny buffer, not an arithmetic overflow.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn section_starting_at_4gib_is_truncated_not_overflow() {
        let bytes = section_map(1 << 23, 2, 1, 0);
        let err = layout_of(&bytes).err().unwrap();
        assert!(
            matches!(err, AbfError::Truncated { section: SECTION, offset } if offset == 1 << 32),
            "{err:?}"
        );
    }

    /// A data section larger than 4 GiB is a size, not an overflow.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn data_section_larger_than_4gib_is_truncated_not_overflow() {
        let bytes = section_map(1, 2, (1 << 31) + 1, 0);
        let err = layout_of(&bytes).err().unwrap();
        assert!(
            matches!(
                err,
                AbfError::Truncated {
                    section: SECTION,
                    ..
                }
            ),
            "{err:?}"
        );
    }

    /// `llNumEntries` is 64-bit: its high half must not be dropped. Here the
    /// low half is 1 and one sample is present, so reading only 32 bits
    /// would silently report a 1-sample recording.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn entry_count_high_bits_are_not_truncated() {
        let bytes = section_map(1, 2, (1 << 32) + 1, 512 - 16 + 2);
        let err = layout_of(&bytes).err().unwrap();
        assert!(
            matches!(
                err,
                AbfError::Truncated {
                    section: SECTION,
                    ..
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn negative_entry_count_is_invalid_section() {
        // Low half is 1, so a 32-bit read would accept it.
        let bytes = section_map(1, 2, -(1 << 32) + 1, 512 - 16 + 2);
        let err = layout_of(&bytes).err().unwrap();
        assert!(matches!(err, AbfError::InvalidSection { .. }), "{err:?}");
    }
}
