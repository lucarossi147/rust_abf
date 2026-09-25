//! Tests for backlog [6] / issue #11: `Abf::from_file` must respect ABF2's
//! `nOperationMode` header field instead of always trusting `lActualEpisodes`.
//!
//! pyABF (the reference implementation) treats `nOperationMode == 3` as
//! gap-free acquisition (always a single sweep) and `nOperationMode == 1` as
//! event-driven variable-length acquisition, where each sweep may have a
//! different length recorded in the synch array section. rust_abf currently
//! ignores `nOperationMode` entirely and always derives the sweep count from
//! `lActualEpisodes`, silently producing garbage splits for both cases.
//!
//! These tests byte-patch a copy of `14o08011_ic_pair.abf` (which has 3
//! synch-array entries, all of length 1_200_000) rather than adding a new
//! fixture, per CLAUDE.md's TDD guidance for this issue.
use rust_abf::{Abf, AbfError};

const FIXTURE: &str = "tests/test_abf/14o08011_ic_pair.abf";

// ABF2 header offsets, matching `SectionProducer` in
// `src/abf_v2/section/section_producer.rs` and pyABF's `headerV2.py`.
const PROTOCOL_SECTION_HEADER_OFFSET: usize = 76;
const SYNCH_ARRAY_SECTION_HEADER_OFFSET: usize = 316;
// `nOperationMode` is an `i16` at offset 0 of the protocol section's data block.
const OPERATION_MODE_OFFSET_IN_PROTOCOL_BLOCK: usize = 0;
const EVENT_DRIVEN_VARIABLE_LENGTH_MODE: i16 = 1;
const GAP_FREE_MODE: i16 = 3;

/// `Abf` doesn't implement `Debug`, so describe just the error side for
/// assertion messages (mirrors the helper in `tests/errors.rs`).
fn describe_err<T>(result: &Result<T, AbfError>) -> String {
    match result {
        Ok(_) => "Ok(_)".to_string(),
        Err(e) => format!("Err({e:?})"),
    }
}

/// Reads a section header's `uBlockIndex` field and returns the byte offset
/// of the section's data block (`uBlockIndex * 512`).
fn section_block_offset(bytes: &[u8], section_header_offset: usize) -> usize {
    let raw = u32::from_le_bytes(
        bytes[section_header_offset..section_header_offset + 4]
            .try_into()
            .expect("4 bytes"),
    );
    raw as usize * 512
}

/// Reads a section header's `uBytes` field (the byte size of each entry).
fn section_entry_byte_count(bytes: &[u8], section_header_offset: usize) -> usize {
    u32::from_le_bytes(
        bytes[section_header_offset + 4..section_header_offset + 8]
            .try_into()
            .expect("4 bytes"),
    ) as usize
}

fn set_operation_mode(bytes: &mut [u8], mode: i16) {
    let offset = section_block_offset(bytes, PROTOCOL_SECTION_HEADER_OFFSET)
        + OPERATION_MODE_OFFSET_IN_PROTOCOL_BLOCK;
    bytes[offset..offset + 2].copy_from_slice(&mode.to_le_bytes());
}

/// Overwrites the `lLength` field (the second `i32` of the entry) of the
/// `entry_index`-th synch array entry.
fn set_synch_array_entry_length(bytes: &mut [u8], entry_index: usize, length: i32) {
    let block = section_block_offset(bytes, SYNCH_ARRAY_SECTION_HEADER_OFFSET);
    let entry_byte_count = section_entry_byte_count(bytes, SYNCH_ARRAY_SECTION_HEADER_OFFSET);
    let offset = block + entry_index * entry_byte_count + 4;
    bytes[offset..offset + 4].copy_from_slice(&length.to_le_bytes());
}

fn write_patched_copy(bytes: &[u8], name: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("create tempdir");
    let path = dir.path().join(name);
    std::fs::write(&path, bytes).expect("write patched fixture");
    (dir, path)
}

#[test]
fn variable_length_event_driven_sweeps_are_reported_as_unsupported() {
    let mut bytes = std::fs::read(FIXTURE).expect("read fixture");
    set_operation_mode(&mut bytes, EVENT_DRIVEN_VARIABLE_LENGTH_MODE);
    // The fixture's 3 synch-array entries are all length 1_200_000; shrink
    // the middle one so the lengths are no longer uniform.
    set_synch_array_entry_length(&mut bytes, 1, 900_000);

    let (_dir, path) = write_patched_copy(&bytes, "variable_length.abf");
    let result = Abf::from_file(&path);
    assert!(
        matches!(result, Err(AbfError::Unsupported(_))),
        "expected Err(AbfError::Unsupported(_)), got {}",
        describe_err(&result)
    );
}

#[test]
fn event_driven_mode_with_uniform_synch_array_lengths_still_parses() {
    let mut bytes = std::fs::read(FIXTURE).expect("read fixture");
    set_operation_mode(&mut bytes, EVENT_DRIVEN_VARIABLE_LENGTH_MODE);
    // Synch-array entries are left at their original, uniform lengths.

    let (_dir, path) = write_patched_copy(&bytes, "uniform_length.abf");
    let result = Abf::from_file(&path);
    assert!(
        result.is_ok(),
        "expected Ok(_), got {}",
        describe_err(&result)
    );
    assert_eq!(result.expect("checked is_ok above").get_sweeps_count(), 3);
}

#[test]
fn gap_free_mode_forces_a_single_sweep_regardless_of_actual_episodes() {
    let mut bytes = std::fs::read(FIXTURE).expect("read fixture");
    set_operation_mode(&mut bytes, GAP_FREE_MODE);
    // lActualEpisodes is still 3 in the header; gap-free must override it.

    let (_dir, path) = write_patched_copy(&bytes, "gap_free.abf");
    let result = Abf::from_file(&path);
    assert!(
        result.is_ok(),
        "expected Ok(_), got {}",
        describe_err(&result)
    );
    assert_eq!(result.expect("checked is_ok above").get_sweeps_count(), 1);
}
