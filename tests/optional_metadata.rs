//! Tests for backlog [10] / issue #15: a channel whose ADC unit/name string
//! index is out of range for the file's string table must report
//! `Channel::uom`/`Channel::label` as `None`, instead of the placeholder
//! literal `"nan"`.
//!
//! Byte-patches a copy of `18425108.abf`, per CLAUDE.md's TDD guidance,
//! rather than adding a new fixture.
use rust_abf::Abf;

const FIXTURE: &str = "tests/test_abf/18425108.abf";

// ABF2 header offset of the ADC section header, matching `SectionProducer`
// in `src/abf_v2/section/section_producer.rs`.
const ADC_SECTION_HEADER_OFFSET: usize = 92;
// Byte offsets of `lADCChannelNameIndex`/`lADCUnitsIndex` within one ADC
// section entry, matching `adc_section::get_adc_infos`.
const ADC_CHANNEL_NAME_INDEX_OFFSET: usize = 74;
const ADC_UNITS_INDEX_OFFSET: usize = 78;

fn section_block_offset(bytes: &[u8], section_header_offset: usize) -> usize {
    let raw = u32::from_le_bytes(
        bytes[section_header_offset..section_header_offset + 4]
            .try_into()
            .expect("4 bytes"),
    );
    raw as usize * 512
}

/// Overwrites channel 0's string index field (name or units, per `field_offset`)
/// with a value that is out of range for any real string table.
fn set_channel_0_string_index(bytes: &mut [u8], field_offset: usize, value: i32) {
    let block = section_block_offset(bytes, ADC_SECTION_HEADER_OFFSET);
    let offset = block + field_offset;
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_patched_copy(bytes: &[u8], name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("write patched fixture");
    (dir, path)
}

#[test]
fn out_of_range_units_index_yields_uom_none() {
    let mut bytes = std::fs::read(FIXTURE).expect("read fixture");
    set_channel_0_string_index(&mut bytes, ADC_UNITS_INDEX_OFFSET, i32::MAX);

    let (_dir, path) = write_patched_copy(&bytes, "out_of_range_units.abf");
    let abf = Abf::from_file(&path).expect("patched fixture should still parse");
    let channel = abf.channel(0).expect("channel 0 exists");
    assert_eq!(channel.uom(), None);
}

#[test]
fn out_of_range_name_index_yields_label_none() {
    let mut bytes = std::fs::read(FIXTURE).expect("read fixture");
    set_channel_0_string_index(&mut bytes, ADC_CHANNEL_NAME_INDEX_OFFSET, i32::MAX);

    let (_dir, path) = write_patched_copy(&bytes, "out_of_range_name.abf");
    let abf = Abf::from_file(&path).expect("patched fixture should still parse");
    let channel = abf.channel(0).expect("channel 0 exists");
    assert_eq!(channel.label(), None);
}

#[test]
fn negative_string_index_yields_none_instead_of_panicking() {
    let mut bytes = std::fs::read(FIXTURE).expect("read fixture");
    set_channel_0_string_index(&mut bytes, ADC_UNITS_INDEX_OFFSET, -1);

    let (_dir, path) = write_patched_copy(&bytes, "negative_units.abf");
    let abf = Abf::from_file(&path).expect("patched fixture should still parse");
    let channel = abf.channel(0).expect("channel 0 exists");
    assert_eq!(channel.uom(), None);
}
