pub mod indexed_strings;

use self::indexed_strings::IndexedStrings;
use super::{Section, StringsSectionType};
use crate::error::AbfError;

const SECTION: &str = "strings";

impl Section<'_, StringsSectionType> {
    pub fn read_indexed_strings(&self) -> Result<IndexedStrings, AbfError> {
        let from = self.start()?;
        let len = usize::try_from(self.byte_count).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "byte_count exceeds addressable range".to_string(),
        })?;
        let bytes = self.reader.read_bytes(SECTION, from, len)?;
        Ok(parse_indexed_strings(bytes))
    }
}

/// Splits the first strings-section entry into its indexed strings.
///
/// Index positions follow pyABF exactly (`src/pyabf/abf2/stringsSection.py`,
/// `StringsSection.__init__`, lines 36-42): start at the **last** `\0\0`,
/// split on `\0`, drop the first piece and keep empty entries, so header
/// indices such as `lADCUnitsIndex` address the result directly.
///
/// Decoding deliberately deviates from pyABF: each byte is read as Latin-1
/// (byte value = code point), so `0xB5 0x56` is `"µV"`. pyABF rewrites
/// `0xB5` to `u` and replaces other non-ASCII bytes. Whitespace is trimmed
/// like Python's `str.strip()` on ASCII; Latin-1 NBSP (`0xA0`) is kept.
fn parse_indexed_strings(bytes: &[u8]) -> IndexedStrings {
    // Python `bytes.rfind` returns -1 when absent, and `b[-1:]` is the last
    // byte (or empty for empty input).
    let start = bytes
        .windows(2)
        .rposition(|w| w == b"\0\0")
        .unwrap_or_else(|| bytes.len().saturating_sub(1));
    let strings = bytes
        .get(start..)
        .unwrap_or_default()
        .split(|&b| b == 0)
        .skip(1)
        .map(|s| {
            let latin1: String = s.iter().copied().map(char::from).collect();
            latin1.trim_matches(is_python_ascii_space).to_string()
        })
        .collect();
    IndexedStrings::new(strings)
}

/// ASCII characters for which Python's `str.isspace()` is true.
fn is_python_ascii_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | '\x1c'..='\x1f' | ' ')
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
