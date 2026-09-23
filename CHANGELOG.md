# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Fixed `Abf::get_time_axis()` returning a truncated axis (divided sweep length by `sweeps_count` a second time) for multi-sweep files, and removed a panic when channel 0 is missing. Now computed directly from a channel's sweep length in `f64` to avoid drift on long recordings, and returns an empty `Vec` if there are no channels.
- Fixed `Abf::get_channels()`/`get_channel()` returning channels in non-deterministic order: `channels` is now a `Vec<Channel>` indexed by ADC section order instead of a `HashMap<u32, Channel>`; `data_section::read` now returns a `Vec<Arc<[i16]>>` indexed by channel instead of a `HashMap`.
- Added `criterion` benchmarks (`benches/read.rs`) covering `open`, `read_all_sweeps_f32`, and `read_all_sweeps_raw` for each ABF2 test fixture. Run with `cargo bench`.
- Added `tests/memory.rs`, a dedicated test binary that measures peak allocator bytes for `Abf::from_file` and for reading one sweep, per fixture, via a custom counting `#[global_allocator]` (`tests/common/alloc_counter.rs`). Run with `cargo test --test memory -- --nocapture`.
- Added `BENCHMARKS.md` with baseline timing and peak-memory numbers and the machine used, to guard against the performance regressions called out in `CLAUDE.md`.
- CI: added a non-blocking `bench` job (`cargo bench -- --quick`) that runs on `workflow_dispatch` only — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Added `scripts/gen_golden.py`, generating golden reference JSON from pyABF (the format's reference implementation) for every fixture in `tests/test_abf/` it can open.
- Added `tests/golden.rs`, a numeric-oracle test suite comparing rust_abf's parsed output against the golden JSON (f32 tolerance `abs ≤ 1e-4 * max(1, |expected|)`, sums at relative `1e-5`). Includes `#[ignore = "fixed by #<issue>"]` tests documenting known-wrong behaviour (ABF1 support, `get_time_axis()` on multi-sweep files, `get_channels()` ordering) so those issues just need to remove the ignore.
- CI: added a `golden` job that regenerates the golden JSON with pyABF and fails on `git diff --exit-code tests/golden` — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- CI: added a five-job pipeline (fmt, clippy, multi-OS test matrix, docs, coverage gate) — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Fixed all `cargo fmt` and `cargo clippy -D warnings` violations across the crate (no behavior change).
- Added tests covering `Channel::get_gain`/`get_offset`, `Abf::get_time_axis`, `Abf::get_time_duration`, out-of-bounds sweep access, and invalid file signatures, raising line coverage to ~99%.
- Fixed: `Channel::get_raw_sweep`/`get_sweep` panicked when called with `sweep == sweeps_count` (off-by-one bound check). Every sweep is now exactly `get_sweep_len()` points long; if the sample count isn't evenly divisible by `sweeps_count`, the trailing remainder is no longer silently appended to the last sweep.
