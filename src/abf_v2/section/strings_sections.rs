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
        Ok(parse_indexed_strings(bytes))
    }
}

/// Splits the first strings-section entry into its indexed strings.
fn parse_indexed_strings(bytes: &[u8]) -> IndexedStrings {
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
    IndexedStrings::new(indexed)
}

#[cfg(test)]
mod tests {
    use super::parse_indexed_strings;

    fn all(bytes: &[u8]) -> Vec<String> {
        let strings = parse_indexed_strings(bytes);
        (0..).map_while(|i| strings.get(i).cloned()).collect()
    }

    /// Layout of a real entry 0 (`14o08011_ic_pair.abf`): a binary header,
    /// a run of NULs, then the indexed strings. pyABF index 0 is the empty
    /// string before the first name.
    #[test]
    fn indexes_like_pyabf_on_real_layout() {
        let bytes = b"SSCH\x01\0\0\0\x0f\0\0\0\x32\0\0\0\0Clampex\0proto.pro\0comment\0IN 0\0mV\0";
        assert_eq!(
            all(bytes),
            ["", "Clampex", "proto.pro", "comment", "IN 0", "mV", ""]
        );
    }

    /// A whitespace-only entry strips to "" but keeps its slot.
    #[test]
    fn empty_entry_keeps_later_indices_aligned() {
        let bytes = b"a\0b\0c\0\0Clampex\0 \0IN 0\0mV\0";
        let strings = parse_indexed_strings(bytes);
        assert_eq!(strings.get(1).map(String::as_str), Some("Clampex"));
        assert_eq!(strings.get(2).map(String::as_str), Some(""));
        assert_eq!(strings.get(3).map(String::as_str), Some("IN 0"));
        assert_eq!(strings.get(4).map(String::as_str), Some("mV"));
    }

    /// pyABF starts at the *last* `\0\0`, regardless of how many strings
    /// precede it.
    #[test]
    fn header_part_with_any_string_count_indexes_from_last_double_nul() {
        let bytes = b"hdr\0\0\0X\0Y\0Z\0W\0\0IN 0\0mV\0";
        assert_eq!(all(bytes), ["", "IN 0", "mV", ""]);
    }

    /// pyABF: `rfind` returns -1 without a `\0\0`, so only the last byte
    /// is split.
    #[test]
    fn no_double_nul_matches_pyabf() {
        assert_eq!(all(b"abc\0def"), Vec::<String>::new());
        assert_eq!(all(b"abc\0def\0"), [""]);
        assert_eq!(all(b""), Vec::<String>::new());
    }

    #[test]
    fn micro_sign_decodes_as_latin1() {
        let bytes = b"hdr\0\0IN 0\0\xb5V\0";
        assert_eq!(all(bytes), ["", "IN 0", "\u{b5}V", ""]);
    }

    #[test]
    fn high_bytes_map_to_latin1_code_points() {
        let bytes = b"\0\0\xe9t\xe9\0\xff\x80\xa0\0";
        assert_eq!(all(bytes), ["", "\u{e9}t\u{e9}", "\u{ff}\u{80}\u{a0}", ""]);
    }

    /// Python's `str.strip()` on ASCII: removes `\t\n\v\f\r`, `\x1c-\x1f`
    /// and space, but not Latin-1 NBSP (`0xA0`).
    #[test]
    fn strips_python_ascii_whitespace_only() {
        let bytes = b"\0\0 \t\x0b\x0c\x1cmV\r\n\x1f\0\xa0x\xa0\0";
        assert_eq!(all(bytes), ["", "mV", "\u{a0}x\u{a0}", ""]);
    }

    #[test]
    fn negative_or_out_of_range_index_is_none() {
        let strings = parse_indexed_strings(b"\0\0a\0");
        assert_eq!(strings.get(-1), None);
        assert_eq!(strings.get(i32::MIN), None);
        assert_eq!(strings.get(3), None);
        assert_eq!(strings.get(i32::MAX), None);
    }
}
