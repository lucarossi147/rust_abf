//! Prints peak memory usage for opening ABF files and reading a sweep.
//!
//! This is a dedicated test binary (separate from `tests/lib.rs`) because it installs
//! a custom `#[global_allocator]` to measure allocations; keeping it separate means the
//! allocator doesn't affect any other test binary.
//!
//! Run with `cargo test --test memory -- --nocapture` to see the printed numbers.
//! Measurements are per-thread (see `tests/common/alloc_counter.rs`), so the tests can
//! safely run in parallel. Apart from the thresholds below, this just establishes a baseline (see BENCHMARKS.md).

#[path = "common/alloc_counter.rs"]
mod alloc_counter;

use alloc_counter::{peak_bytes_during, CountingAllocator};
use rust_abf::Abf;
use std::path::Path;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

// ABF1 fixture is excluded: ABF1 parsing is not implemented yet (`Abf::from_file` panics on it).
const FIXTURES: &[&str] = &[
    "tests/test_abf/14o08011_ic_pair.abf",
    "tests/test_abf/18425108.abf",
];

/// Issue [7]: `Abf::from_file` must memory-map and lazily decode sample data
/// instead of eagerly copying/de-interleaving it onto the heap, so peak
/// allocation at open time should stay small and independent of file size.
const MAX_OPEN_PEAK_BYTES: usize = 64 * 1024;

#[test]
fn opening_a_file_does_not_copy_sample_data_onto_the_heap() {
    for fixture in FIXTURES {
        let path = Path::new(fixture);
        let (_abf, open_peak_bytes) =
            peak_bytes_during(&ALLOCATOR, || Abf::from_file(path).unwrap());
        assert!(
            open_peak_bytes < MAX_OPEN_PEAK_BYTES,
            "{fixture}: from_file peak bytes = {open_peak_bytes}, expected < {MAX_OPEN_PEAK_BYTES}"
        );
    }
}

/// Issue [8]: `Channel::read_sweep_into` decodes into a caller-supplied
/// buffer, so reading every sweep of every channel with one reused buffer
/// should not allocate at all after the buffer itself is allocated.
#[test]
fn read_sweep_into_does_not_allocate_after_the_buffer_is_reused() {
    for fixture in FIXTURES {
        let path = Path::new(fixture);
        let abf = Abf::from_file(path).unwrap();

        for channel in abf.channels() {
            let mut buf = vec![0.0f32; channel.sweep_len()];
            // `peak_bytes_during` counts only this thread's allocations made
            // inside the closure (baseline 0), so `buf` itself, allocated
            // beforehand, is not included: the read loop must allocate nothing.
            let (_, peak_bytes) = peak_bytes_during(&ALLOCATOR, || {
                for sweep in 0..abf.sweep_count() {
                    channel.read_sweep_into(sweep, &mut buf).unwrap();
                }
            });
            assert_eq!(
                peak_bytes, 0,
                "{fixture}: read_sweep_into peak bytes = {peak_bytes}, expected 0"
            );
        }
    }
}

/// `Abf::read_sweep_all_channels_into` with reused buffers must not allocate
/// on either decode strategy (and never spawns threads, even with `parallel`).
#[test]
fn read_sweep_all_channels_into_does_not_allocate_after_the_buffers_are_reused() {
    for fixture in FIXTURES {
        let path = Path::new(fixture);
        let abf = Abf::from_file(path).unwrap();
        let mut bufs: Vec<Vec<f32>> = abf
            .channels()
            .map(|c| vec![0.0f32; c.sweep_len()])
            .collect();
        let (_, peak_bytes) = peak_bytes_during(&ALLOCATOR, || {
            for sweep in 0..abf.sweep_count() {
                abf.read_sweep_all_channels_into(sweep, &mut bufs).unwrap();
            }
        });
        assert_eq!(
            peak_bytes, 0,
            "{fixture}: read_sweep_all_channels_into peak bytes = {peak_bytes}, expected 0"
        );
    }
}

#[test]
fn prints_peak_bytes_per_fixture() {
    for fixture in FIXTURES {
        let path = Path::new(fixture);

        let (abf, open_peak_bytes) =
            peak_bytes_during(&ALLOCATOR, || Abf::from_file(path).unwrap());
        println!("{fixture}: from_file peak bytes = {open_peak_bytes}");

        let (_, sweep_peak_bytes) = peak_bytes_during(&ALLOCATOR, || abf.sweep(0, 0).unwrap());
        println!("{fixture}: read one sweep peak bytes = {sweep_peak_bytes}");
    }
}
