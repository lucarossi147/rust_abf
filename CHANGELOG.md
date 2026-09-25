# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

- Packaging metadata (issue [12]): added `repository`, `homepage`, `documentation`,
  `readme`, `keywords`, `categories`, and `rust-version = "1.66"` to `Cargo.toml` (MSRV
  determined by `clippy::incompatible_msrv`: `memmap2`'s own MSRV is 1.65, but
  `std::hint::black_box`, used in `benches/read.rs`, requires 1.66). Added
  `exclude = ["tests/test_abf/*", "tests/golden/*", "scripts/*", ".github/*", "benches/*"]`
  so fixtures, golden data, CI scripts, and benchmarks no longer ship in the published
  crate; `cargo package` is now 44.7 KiB compressed (167.9 KiB uncompressed) with no `.abf`
  files, down from 3.5+ MB. Back-filled `CHANGELOG.md` with brief `[0.4.0]`–`[0.4.4]`
  entries. Replaced the hard-coded `C:\Users\lucar\Desktop\...` path in the ignored
  `test_abfv2_heavy` test with an `ABF_HEAVY_FILE` environment variable; the test now
  skips (instead of requiring `--ignored`) when the variable is unset.
- Working docs and doc-tested README (issue [11]): `Channel` and `FileKind` are now
  re-exported from the crate root (`pub use channel::{Channel, FileKind};` in `src/lib.rs`)
  so they actually appear in `cargo doc` output and can be named by callers — previously
  `channel` was a private module with no public re-export, so neither type had a rustdoc
  page even though `Abf::channel`/`Channel::file_kind` returned them. `src/lib.rs` now uses
  `#![doc = include_str!("../README.md")]` (replacing the old `// !` plain comments, which
  docs.rs rendered as an empty crate page) so the README's examples are run as doctests, and
  `#![warn(missing_docs)]` is on, with every public item documented (including an `# Errors`
  section on `Abf::from_file`/`Channel::read_sweep_into` and at least one example per main
  type: `Abf`, `Channel`, `AbfKind`, `FileKind`, `AbfError`). Rewrote the README (its old
  example didn't compile: `.unwrap()` immediately followed by `match Ok(..)` on the
  now-unwrapped value) with compiling examples, and added sections on the lazy-mmap memory
  model and the float32-vs-int16 scaling behavior. Added `examples/summary.rs`
  (`cargo run --example summary <path>`), printing a file's channels, units, sweeps and
  sampling rate.
- **Breaking:** Idiomatic, minimal public API cleanup (issue [10] / #15):
  - `pub mod abf_v2` is now a private module: it was only reachable because of the `pub`, but every item nested under it was already declared with default (crate-private) visibility, so nothing in it was actually part of the public API before this change either — this just makes that explicit. `Abf::from_abf_v2` and `Channel::new` were already `pub(crate)`.
  - Counts and indices are now `usize` instead of `u32`, and every getter drops its `get_` prefix. Renamed (old name deprecated with `#[deprecated(since = "0.5.0", ...)]`, one dedicated test per alias in `tests/deprecated_api.rs`):
    - `Abf::get_channels_count` → `Abf::channel_count`
    - `Abf::get_sweeps_count` → `Abf::sweep_count`
    - `Abf::get_sweep_in_channel(sweep, channel)` → `Abf::sweep(channel, sweep)` (**argument order swapped** to `(channel, sweep)`, matching `Abf::channel`/`Channel::sweep`)
    - `Abf::get_file_signature` → `Abf::kind`
    - `Abf::get_channel` → `Abf::channel`
    - `Abf::get_channels` → `Abf::channels`
    - `Abf::get_sampling_rate` → `Abf::sampling_rate`
    - `Abf::get_path` → `Abf::path`
    - `Abf::get_time_duration` → `Abf::time_duration`
    - `Abf::get_time_axis` → `Abf::time_axis`
    - `Channel::get_uom` → `Channel::uom`
    - `Channel::get_label` → `Channel::label`
    - `Channel::get_gain` → `Channel::gain`
    - `Channel::get_offset` → `Channel::offset`
    - `Channel::get_file_kind` → `Channel::file_kind`
    - `Channel::get_raw_sweep(sweep: u32)` → `Channel::raw_sweep(sweep: usize)`
    - `Channel::get_sweep(sweep: u32)` → `Channel::sweep(sweep: usize)`
    - `Channel::get_sweeps` → `Channel::sweeps`
    - `Channel::get_sweep_len` → `Channel::sweep_len`
  - `Channel::uom`/`Channel::label` now return `Option<&str>` (`None` when the ABF file's string table has no entry for the channel), instead of the literal string `"nan"`. The deprecated `get_uom`/`get_label` aliases preserve the old `"nan"`-fallback behavior. Added `tests/optional_metadata.rs`, byte-patching a fixture's ADC section string indices out of range to cover this.
  - Added `Channel::index() -> usize`, the channel's position in `Abf::channels()`/`Abf::channel(i)`.
  - Added a manual `Debug` impl for `Abf` and `Channel` (both hold an `Arc` over the memory-mapped file, which isn't `Debug`; the impls print the metadata fields only, not the mapped bytes).
  - Added `#[must_use]` to getters.
  - Added `tests/public_api.rs`, a `#![deny(deprecated)]` compile-level check that exercises only the new API surface.
- Removed the `byteorder` runtime dependency: `ByteReader` (`src/byte_reader.rs`) now decodes little-endian values with `u16/i16/u32/i32/f32::from_le_bytes` instead of `byteorder::ReadBytesExt`, with no behavior change. `rayon` had already been fully removed from `src/` (and from `Cargo.toml`) in a prior change; `cargo tree -e normal` now shows only `memmap2` as a normal dependency. See `BENCHMARKS.md` for before/after numbers (no measurable change, since `ByteReader` was never on the per-sample decode path).
- Added `Channel::read_sweep_into(&self, sweep: usize, out: &mut [f32]) -> Result<(), AbfError>`, decoding a sweep's samples in physical units directly into a caller-supplied buffer instead of allocating a new `Vec` per call; returns `Err(AbfError::SweepOutOfRange)`/`Err(AbfError::BufferLengthMismatch)` instead of modifying `out` on a bad sweep index or buffer length. Added `Channel::sweep_iter`/`Channel::raw_sweep_iter`, lazy `ExactSizeIterator`s over a sweep's scaled/raw samples that decode on demand instead of collecting into a `Vec` (`raw_sweep_iter` returns `None` for float32 channels, matching `get_raw_sweep`). `Channel::get_sweep`/`get_raw_sweep` are now implemented on top of these instead of their own separate decode path, and are unchanged from the caller's perspective. Added `tests/memory.rs::read_sweep_into_does_not_allocate_after_the_buffer_is_reused`, asserting that reading every sweep of every channel through one reused buffer allocates no more than the buffer itself. Added `read_all_sweeps_into` to `benches/read.rs`. **Breaking:** dropped the `rayon`-parallel decode `get_sweep`/`get_raw_sweep` used internally (and the now-unused `rayon` dependency), since `read_sweep_into` needs a provable zero-allocation guarantee that calling into rayon's global thread pool cannot give; see `BENCHMARKS.md` for the resulting timing regression (up to ~49% slower on `read_all_sweeps_raw`) and the reasoning for accepting it.
- Changed: `Abf::from_file` no longer copies or de-interleaves sample data into `Vec<i16>`/`Vec<f32>` buffers at open time. `Channel` now decodes each sweep lazily, straight out of the memory-mapped file, via a new internal `Storage` enum (`Arc<Storage>`, shared with `Abf`) that wraps the `Mmap` (an `Owned` variant is planned for issue 14's `Abf::from_bytes`). This drops `Abf::from_file`'s peak allocation from ~1.5-1.6x the file size to a small, file-size-independent amount (both fixtures now open in under 64 KiB — see `tests/memory.rs`) and makes `open` itself ~150-1400x faster, at the cost of slower per-sweep reads (straight-line reads of interleaved mmap bytes instead of a contiguous pre-deinterleaved copy); see `BENCHMARKS.md` for full before/after numbers and the reasoning for accepting that regression. **Breaking:** `Channel::new` and `Abf::from_abf_v2` are no longer public (they took/returned internal representations that no longer exist in public form); `Channel::get_raw_sweep`/`get_sweep`/`get_sweep_len` and all other existing public accessors are unchanged.
- Documented the mmap safety caveat on `Abf::from_file`: modifying or truncating the underlying file while an `Abf` (or any `Channel` obtained from it) is alive is undefined behavior.
- Added a compile-time-checked test asserting `Abf` and `Channel` are `Send + Sync`.
- Fixed: `Abf::from_file` ignored ABF2's `nOperationMode` and always derived the sweep count from `lActualEpisodes`, silently mis-splitting gap-free recordings (mode 3, which is always a single sweep) and variable-length event-driven recordings (mode 1, where sweeps can have different lengths recorded in the synch array section). Gap-free files now always report `sweeps_count == 1`. Event-driven variable-length files with non-uniform synch-array entry lengths now return `Err(AbfError::Unsupported(_))` instead of a garbage split (full support is tracked separately); uniform-length ones parse as before. Added `AbfError::Unsupported(String)`.
- **Breaking:** `Abf::from_file` now returns `Result<Abf, AbfError>` instead of `Result<Abf, std::io::Error>`. `AbfError` is a new public enum (`Io`, `InvalidSignature`, `UnsupportedVersion`, `Truncated`, `InvalidSection`, `Unsupported`) implementing `Display`/`std::error::Error`/`From<std::io::Error>`, giving callers a typed reason instead of a generic I/O error message.
- Fixed: `Abf::from_file` panicked instead of returning an error on malformed or truncated input — an ABF1 file previously hit `todo!()`, and any file too short for a given header/section read (or with a corrupted section offset/count that overflows `u32` or points past the end of the file) panicked via direct slice indexing or `.unwrap()`. All header and section parsing now goes through a bounds-checked `ByteReader` (replacing `conversion_util`) and returns `Result` end-to-end; ABF1 now yields `Err(AbfError::UnsupportedVersion(_))` instead of panicking.
- Fixed: `strings_sections::read_indexed_strings` panicked on a strings section with fewer than 3 entries (`res[3..]`); it now yields no indexed strings instead.
- Added `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` to `src/` (tests exempted) to keep parsing panic-free going forward.
- Added `tests/errors.rs`: fuzz tests truncating `18425108.abf` at every offset in `0..8192` plus 100 deterministic random offsets, and 1000 deterministic single-byte corruptions of the first 4 KB, asserting `Abf::from_file` never panics; plus assertions that ABF1/non-ABF/missing-path inputs produce the expected `AbfError` variant. Uses a hand-rolled fixed-seed xorshift64 PRNG instead of a new `rand` dependency.
- Added `tempfile` dev-dependency, used by the new fuzz tests to write truncated/corrupted fixture copies.

- Fixed float32 ABF2 files (`nDataFormat != 0`) being silently decoded as int16, corrupting sample values: `data_section::read` now decodes samples according to the header's `nDataFormat` field, and `Channel` stores samples as either `i16` or `f32` (`channel::ChannelValues`). Added `Channel::get_file_kind()`. `Channel::get_raw_sweep()` now returns `None` for float32 channels (there is no meaningful raw-integer representation); `Channel::get_sweep()` returns float32 samples as stored, unscaled, matching pyABF's behavior of not applying gain/offset to float32 data (see issue #8). No float32 fixture could be found/added in this change — see PR/issue discussion.
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

## [0.4.4] - 2025-01-14

- Changed: `Channel` stores its sample buffers behind `Arc` instead of `Vec`, so cloning a
  `Channel` no longer copies the underlying sample data.

## [0.4.3] - 2024-10-18

- Added `Channel::get_gain`, `Channel::get_offset`, `Channel::get_raw_sweep`,
  `Channel::get_sweep`, `Channel::get_sweep_len`, and `Abf::get_time_duration`.

## [0.4.2] - 2024-03-06

- Fixed a `Cargo.toml`/README inconsistency from the 0.4.1 release.

## [0.4.1] - 2024-03-06

- Changed: internal module layout refactor of the ABF parsing path (no public API change).

## [0.4.0] - 2024-01-11

- Added `Abf::get_path` and an example demonstrating basic usage.
