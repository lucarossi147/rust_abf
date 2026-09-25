use super::{DataSectionType, Section};
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

        let total_bytes =
            self.item_count
                .checked_mul(self.byte_count)
                .ok_or(AbfError::InvalidSection {
                    section: SECTION,
                    reason: "section size overflows u32".to_string(),
                })?;
        let to = self
            .block_number
            .checked_add(total_bytes)
            .ok_or(AbfError::InvalidSection {
                section: SECTION,
                reason: "section end overflows u32".to_string(),
            })?;
        let from = usize::try_from(self.block_number).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "section start exceeds addressable range".to_string(),
        })?;
        let to = usize::try_from(to).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "section end exceeds addressable range".to_string(),
        })?;

        // Validates that the section fits within the backing storage without
        // copying any of it: `read_bytes` returns a borrowed slice.
        self.reader.read_bytes(SECTION, from, to - from)?;

        let total_items =
            usize::try_from(self.item_count).map_err(|_| AbfError::InvalidSection {
                section: SECTION,
                reason: "item_count exceeds addressable range".to_string(),
            })?;
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
        // A 12-byte section header: block_number=0, byte_count=2 (matching
        // DATA_FORMAT_INT16's expected width), item_count=0. With
        // number_of_channels=0, `layout` must short circuit to `Ok(None)`
        // before doing any section-size validation.
        let mut bytes = [0u8; 12];
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes());
        let reader = crate::byte_reader::ByteReader::new(&bytes);
        let section = Section::<DataSectionType>::new(reader, 0, std::marker::PhantomData).unwrap();
        assert!(section.layout(0, DATA_FORMAT_INT16).unwrap().is_none());
    }
}
