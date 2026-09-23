# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- CI: added a five-job pipeline (fmt, clippy, multi-OS test matrix, docs, coverage gate) — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Fixed all `cargo fmt` and `cargo clippy -D warnings` violations across the crate (no behavior change).
- Added tests covering `Channel::get_gain`/`get_offset`, `Abf::get_time_axis`, `Abf::get_time_duration`, out-of-bounds sweep access, and invalid file signatures, raising line coverage to ~99%.
- Fixed: `Channel::get_raw_sweep`/`get_sweep` panicked when called with `sweep == sweeps_count` (off-by-one bound check). Every sweep is now exactly `get_sweep_len()` points long; if the sample count isn't evenly divisible by `sweeps_count`, the trailing remainder is no longer silently appended to the last sweep.
