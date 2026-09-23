# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Fixed `Abf::get_channels()`/`get_channel()` returning channels in non-deterministic order: `channels` is now a `Vec<Channel>` indexed by ADC section order instead of a `HashMap<u32, Channel>`; `data_section::read` now returns a `Vec<Arc<[i16]>>` indexed by channel instead of a `HashMap`.
- CI: added a five-job pipeline (fmt, clippy, multi-OS test matrix, docs, coverage gate) — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Fixed all `cargo fmt` and `cargo clippy -D warnings` violations across the crate (no behavior change).
- Added tests covering `Channel::get_gain`/`get_offset`, `Abf::get_time_axis`, `Abf::get_time_duration`, out-of-bounds sweep access, and invalid file signatures, raising line coverage to ~99%.
