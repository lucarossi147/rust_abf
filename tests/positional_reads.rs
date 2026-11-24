//! Backlog [21] / issue #43: `Abf::from_file` reads the file with positional
//! reads instead of a memory map.
#![forbid(unsafe_code)]

use rust_abf::Abf;
use std::path::Path;

const FIXTURES: [&str; 2] = [
    "tests/test_abf/18425108.abf",
    "tests/test_abf/14o08011_ic_pair.abf",
];

fn open_both(path: &str) -> (Abf, Abf) {
    let from_file = Abf::from_file(Path::new(path)).unwrap();
    let from_bytes = Abf::from_bytes(std::fs::read(path).unwrap()).unwrap();
    (from_file, from_bytes)
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

#[test]
fn from_file_reads_exactly_what_from_bytes_reads() {
    for path in FIXTURES {
        let (file, bytes) = open_both(path);
        assert_eq!(file.sweep_count(), bytes.sweep_count());
        for (a, b) in file.channels().zip(bytes.channels()) {
            for sweep in 0..file.sweep_count() {
                let expected = bits(&b.sweep(sweep).unwrap());
                assert_eq!(bits(&a.sweep(sweep).unwrap()), expected, "{path} {sweep}");
                let mut out = vec![0.0; a.sweep_len()];
                a.read_sweep_into(sweep, &mut out).unwrap();
                assert_eq!(bits(&out), expected);
                let iter: Vec<f32> = a.sweep_iter(sweep).unwrap().collect();
                assert_eq!(bits(&iter), expected);
                assert_eq!(a.raw_sweep(sweep), b.raw_sweep(sweep));
                let raw_iter: Option<Vec<i16>> = a.raw_sweep_iter(sweep).map(Iterator::collect);
                assert_eq!(raw_iter, b.raw_sweep(sweep));
            }
        }
        for sweep in 0..file.sweep_count() {
            let expected: Vec<Vec<u32>> = bytes
                .sweep_all_channels(sweep)
                .unwrap()
                .iter()
                .map(|c| bits(c))
                .collect();
            let all: Vec<Vec<u32>> = file
                .sweep_all_channels(sweep)
                .unwrap()
                .iter()
                .map(|c| bits(c))
                .collect();
            assert_eq!(all, expected, "{path} {sweep}");
            let mut outs: Vec<Vec<f32>> =
                file.channels().map(|c| vec![0.0; c.sweep_len()]).collect();
            file.read_sweep_all_channels_into(sweep, &mut outs).unwrap();
            let into: Vec<Vec<u32>> = outs.iter().map(|c| bits(c)).collect();
            assert_eq!(into, expected);
        }
    }
}

/// Shrinking the file after it was opened must not panic (with a memory map
/// this is a SIGBUS on Linux and a sharing violation on Windows). Until [26]
/// turns it into an error, samples that can no longer be read come back as
/// zeros.
#[test]
fn a_file_truncated_after_open_reads_zeros_without_panicking() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("shrinks.abf");
    std::fs::copy(FIXTURES[1], &path).unwrap();
    let abf = Abf::from_file(&path).unwrap();
    let channel = abf.channel(0).unwrap();
    let last = abf.sweep_count() - 1;
    assert!(channel.sweep(last).unwrap().iter().any(|&v| v != 0.0));

    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(4096).unwrap();
    drop(file);

    assert!(channel.sweep(last).unwrap().iter().all(|&v| v == 0.0));
    assert!(channel.sweep_iter(last).unwrap().all(|v| v == 0.0));
    assert!(channel.raw_sweep(last).unwrap().iter().all(|&v| v == 0));
    let mut outs: Vec<Vec<f32>> = abf.channels().map(|c| vec![1.0; c.sweep_len()]).collect();
    abf.read_sweep_all_channels_into(last, &mut outs).unwrap();
    assert!(outs.iter().flatten().all(|&v| v == 0.0));
}

#[test]
fn abf_is_still_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Abf>();
}
