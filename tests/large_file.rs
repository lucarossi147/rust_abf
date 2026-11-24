//! Backlog [22] / issue #44: ABF2 files larger than 4 GiB.
//!
//! Builds a sparse ~12 GiB file from the header and metadata sections of
//! `18425108.abf`, with the data section moved past 4 GiB and holding more
//! than 2^32 samples (so `llNumEntries` has its high half set). Only a few
//! marker samples are written; the rest is a hole, so the file costs almost
//! no disk space on filesystems with sparse-file support (ext4, APFS, NTFS
//! once the sparse flag is set).
#![forbid(unsafe_code)]
#![cfg(target_pointer_width = "64")]

use rust_abf::Abf;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;

const FIXTURE: &str = "tests/test_abf/18425108.abf";
const BLOCK: u64 = 512;
const PROTOCOL_MAP: usize = 76;
const DATA_MAP: usize = 236;
const SYNCH_ARRAY_MAP: usize = 316;
const EPISODIC: i16 = 5;

const CHANNELS: u64 = 2;
const SWEEP_LEN: u64 = 1024;
const SWEEPS: u64 = (1 << 21) + 1;
const ENTRIES: u64 = CHANNELS * SWEEP_LEN * SWEEPS; // 2^32 + 2048
const DATA_BLOCK: u64 = (1 << 23) + 1; // starts at 4 GiB + 512

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

#[cfg(windows)]
fn make_sparse(path: &Path) {
    // Without the sparse flag NTFS would zero-fill up to every write.
    let status = std::process::Command::new("fsutil")
        .args(["sparse", "setflag"])
        .arg(path)
        .status()
        .expect("run fsutil");
    assert!(status.success(), "fsutil sparse setflag failed");
}

#[cfg(not(windows))]
fn make_sparse(_: &Path) {}

fn write_i16_at(file: &mut File, item: u64, value: i16) {
    let offset = DATA_BLOCK * BLOCK + item * 2;
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(&value.to_le_bytes()).unwrap();
}

#[test]
fn opens_file_larger_than_4gib_and_reads_last_sweep() {
    assert!(ENTRIES > u64::from(u32::MAX));
    let mut header = std::fs::read(FIXTURE).unwrap();
    assert_eq!(header[92 + 8], CHANNELS as u8, "fixture has 2 ADC channels");

    // Keep everything before the original data section.
    let old_data_start = u32_at(&header, DATA_MAP) as usize * BLOCK as usize;
    header.truncate(old_data_start);

    // Episodic, SWEEPS sweeps; no synch array.
    header[12..16].copy_from_slice(&(SWEEPS as u32).to_le_bytes());
    let protocol = u32_at(&header, PROTOCOL_MAP) as usize * BLOCK as usize;
    header[protocol..protocol + 2].copy_from_slice(&EPISODIC.to_le_bytes());
    header[SYNCH_ARRAY_MAP..SYNCH_ARRAY_MAP + 16].fill(0);
    // Data section map: block index, 2-byte entries, 64-bit entry count.
    header[DATA_MAP..DATA_MAP + 4].copy_from_slice(&(DATA_BLOCK as u32).to_le_bytes());
    header[DATA_MAP + 8..DATA_MAP + 16].copy_from_slice(&(ENTRIES as i64).to_le_bytes());

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large.abf");
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .unwrap();
    make_sparse(&path);
    file.write_all(&header).unwrap();
    file.set_len(DATA_BLOCK * BLOCK + ENTRIES * 2).unwrap();

    // Markers in the last sweep: first sample of channel 0, last of channel 1.
    let last = SWEEPS - 1;
    write_i16_at(&mut file, last * SWEEP_LEN * CHANNELS, 1234);
    write_i16_at(
        &mut file,
        (last * SWEEP_LEN + SWEEP_LEN - 1) * CHANNELS + 1,
        -4321,
    );
    drop(file);

    let abf = Abf::from_file(&path).unwrap();
    assert_eq!(abf.channel_count(), CHANNELS as usize);
    assert_eq!(abf.sweep_count(), SWEEPS as usize);
    let ch0 = abf.channel(0).unwrap();
    let ch1 = abf.channel(1).unwrap();
    assert_eq!(ch0.sweep_len(), SWEEP_LEN as usize);

    let s0 = ch0.raw_sweep(last as usize).unwrap();
    let s1 = ch1.raw_sweep(last as usize).unwrap();
    assert_eq!(s0.len(), SWEEP_LEN as usize);
    assert_eq!(s0[0], 1234);
    assert!(s0[1..].iter().all(|&v| v == 0));
    assert_eq!(s1[SWEEP_LEN as usize - 1], -4321);
    assert!(s1[..SWEEP_LEN as usize - 1].iter().all(|&v| v == 0));
    assert!(ch0.raw_sweep(SWEEPS as usize).is_none());
}
