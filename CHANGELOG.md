# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Fixed `Abf::get_time_axis()` returning a truncated axis (divided sweep length by `sweeps_count` a second time) for multi-sweep files, and removed a panic when channel 0 is missing. Now computed directly from a channel's sweep length in `f64` to avoid drift on long recordings, and returns an empty `Vec` if there are no channels.
- CI: added a five-job pipeline (fmt, clippy, multi-OS test matrix, docs, coverage gate) — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Fixed all `cargo fmt` and `cargo clippy -D warnings` violations across the crate (no behavior change).
- Added tests covering `Channel::get_gain`/`get_offset`, `Abf::get_time_axis`, `Abf::get_time_duration`, out-of-bounds sweep access, and invalid file signatures, raising line coverage to ~99%.
