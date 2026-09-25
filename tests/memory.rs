//! Prints peak memory usage for opening ABF files and reading a sweep.
//!
//! This is a dedicated test binary (separate from `tests/lib.rs`) because it installs
//! a custom `#[global_allocator]` to measure allocations; keeping it separate means the
//! allocator doesn't affect any other test binary.
//!
//! Run with `cargo test --test memory -- --nocapture` to see the printed numbers.
//! There are no thresholds (yet) — this just establishes a baseline (see BENCHMARKS.md).

#[path = "common/alloc_counter.rs"]
mod alloc_counter;

use alloc_counter::{peak_bytes_during, CountingAllocator};
use rust_abf::Abf;
use std::path::Path;
use std::sync::Mutex;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

// The allocator above tracks a single, process-wide byte count: an
// allocation on *any* thread bumps the same counter, regardless of which
// thread's measured section is currently running. So it's not enough to
// serialize the individual `peak_bytes_during` calls (any allocation by the
// *other* test running concurrently in between would still be attributed to
// whichever measurement happens to be in progress) — the two `#[test]`
// functions in this binary must not run concurrently at all, since `cargo
// test` otherwise runs them in parallel by default. Each test holds this
// mutex for its entire body to guarantee that.
static MEASURE_LOCK: Mutex<()> = Mutex::new(());

fn lock_measurements() -> std::sync::MutexGuard<'static, ()> {
    MEASURE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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
    let _guard = lock_measurements();
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
    let _guard = lock_measurements();
    for fixture in FIXTURES {
        let path = Path::new(fixture);
        let abf = Abf::from_file(path).unwrap();

        for channel in abf.get_channels() {
            let mut buf = vec![0.0f32; channel.get_sweep_len()];
            // `peak_bytes_during` reports an absolute high-water mark, which
            // starts at whatever is already allocated (here, `buf` itself).
            // Capture that starting point with a no-op call, so the
            // assertion below checks that the read loop allocates nothing
            // *beyond* the reused buffer, rather than nothing at all.
            let (_, baseline) = peak_bytes_during(&ALLOCATOR, || {});
            let (_, peak_bytes) = peak_bytes_during(&ALLOCATOR, || {
                for sweep in 0..abf.get_sweeps_count() {
                    channel.read_sweep_into(sweep as usize, &mut buf).unwrap();
                }
            });
            assert_eq!(
                peak_bytes, baseline,
                "{fixture}: read_sweep_into peak bytes = {peak_bytes}, expected no more than the {baseline}-byte baseline"
            );
        }
    }
}

#[test]
fn prints_peak_bytes_per_fixture() {
    let _guard = lock_measurements();
    for fixture in FIXTURES {
        let path = Path::new(fixture);

        let (abf, open_peak_bytes) =
            peak_bytes_during(&ALLOCATOR, || Abf::from_file(path).unwrap());
        println!("{fixture}: from_file peak bytes = {open_peak_bytes}");

        let (_, sweep_peak_bytes) =
            peak_bytes_during(&ALLOCATOR, || abf.get_sweep_in_channel(0, 0).unwrap());
        println!("{fixture}: read one sweep peak bytes = {sweep_peak_bytes}");
    }
}
