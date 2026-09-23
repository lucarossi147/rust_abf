# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

- Added `scripts/gen_golden.py`, generating golden reference JSON from pyABF (the format's reference implementation) for every fixture in `tests/test_abf/` it can open.
- Added `tests/golden.rs`, a numeric-oracle test suite comparing rust_abf's parsed output against the golden JSON (f32 tolerance `abs ≤ 1e-4 * max(1, |expected|)`, sums at relative `1e-5`). Includes `#[ignore = "fixed by #<issue>"]` tests documenting known-wrong behaviour (ABF1 support, `get_time_axis()` on multi-sweep files, `get_channels()` ordering) so those issues just need to remove the ignore.
- CI: added a `golden` job that regenerates the golden JSON with pyABF and fails on `git diff --exit-code tests/golden` — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- CI: added a five-job pipeline (fmt, clippy, multi-OS test matrix, docs, coverage gate) — see PR description for the workflow content, which could not be committed directly due to GitHub App permissions on `.github/workflows/`.
- Fixed all `cargo fmt` and `cargo clippy -D warnings` violations across the crate (no behavior change).
- Added tests covering `Channel::get_gain`/`get_offset`, `Abf::get_time_axis`, `Abf::get_time_duration`, out-of-bounds sweep access, and invalid file signatures, raising line coverage to ~99%.
