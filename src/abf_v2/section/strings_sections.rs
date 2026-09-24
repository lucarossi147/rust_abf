pub mod indexed_strings;

use self::indexed_strings::IndexedStrings;
use super::{Section, StringsSectionType};
use crate::error::AbfError;

const SECTION: &str = "strings";

impl Section<'_, StringsSectionType> {
    pub fn read_indexed_strings(&self) -> Result<IndexedStrings, AbfError> {
        let from = usize::try_from(self.block_number).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "section start exceeds addressable range".to_string(),
        })?;
        let len = usize::try_from(self.byte_count).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "byte_count exceeds addressable range".to_string(),
        })?;
        let bytes = self.reader.read_bytes(SECTION, from, len)?;
        let res: Vec<String> = bytes
            .split(|c| *c == 0u8)
            .map(String::from_utf8_lossy)
            .filter(|str_i| !str_i.is_empty())
            .map(|str_i| str_i.trim().to_string())
            .collect();
        // The first 3 strings are reserved (file comment / ADC comment path /
        // protocol path); a malformed file with fewer than 3 just yields no
        // indexed strings instead of panicking.
        let indexed = res.get(3..).unwrap_or(&[]).to_vec();
        Ok(IndexedStrings::new(indexed))
    }
}
