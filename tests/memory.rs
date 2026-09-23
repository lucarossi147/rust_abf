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

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator::new();

// ABF1 fixture is excluded: ABF1 parsing is not implemented yet (`Abf::from_file` panics on it).
const FIXTURES: &[&str] = &[
    "tests/test_abf/14o08011_ic_pair.abf",
    "tests/test_abf/18425108.abf",
];

#[test]
fn prints_peak_bytes_per_fixture() {
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
