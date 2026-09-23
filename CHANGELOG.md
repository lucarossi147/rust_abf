# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Added `criterion` benchmarks (`benches/read.rs`) covering `open`, `read_all_sweeps_f32`, and `read_all_sweeps_raw` for each ABF2 test fixture. Run with `cargo bench`.
- Added `tests/memory.rs`, a dedicated test binary that measures peak allocator bytes for `Abf::from_file` and for reading one sweep, per fixture, via a custom counting `#[global_allocator]` (`tests/common/alloc_counter.rs`). Run with `cargo test --test memory -- --nocapture`.
- Added `BENCHMARKS.md` with baseline timing and peak-memory numbers and the machine used, to guard against the performance regressions called out in `CLAUDE.md`.
- CI: added a non-blocking `bench` job (`cargo bench -- --quick`) that runs on `workflow_dispatch` only — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- CI: added a five-job pipeline (fmt, clippy, multi-OS test matrix, docs, coverage gate) — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Fixed all `cargo fmt` and `cargo clippy -D warnings` violations across the crate (no behavior change).
- Added tests covering `Channel::get_gain`/`get_offset`, `Abf::get_time_axis`, `Abf::get_time_duration`, out-of-bounds sweep access, and invalid file signatures, raising line coverage to ~99%.
