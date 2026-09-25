use super::{Section, SynchArraySectionType};
use crate::byte_reader::checked_offset;
use crate::error::AbfError;

const SECTION: &str = "synch_array";

impl Section<'_, SynchArraySectionType> {
    /// Returns each entry's `lLength` field (in synchronized time units,
    /// i.e. samples per channel for that sweep), in on-disk order. The
    /// preceding `lStart` field is not read: only the lengths are needed to
    /// detect non-uniform (variable-length) sweeps.
    pub fn lengths(&self) -> Result<Vec<i32>, AbfError> {
        (0..self.item_count)
            .map(|i| {
                let from = self.item_offset(i)?;
                self.reader
                    .read_i32(SECTION, checked_offset(from, 4, SECTION)?)
            })
            .collect()
    }
}
