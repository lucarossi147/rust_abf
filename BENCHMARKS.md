# Benchmarks

Baseline numbers for `rust_abf` 0.4.4, established as part of [F3]. Re-run with
`cargo bench` (timing, via [Criterion](https://github.com/bheisler/criterion.rs)) and
`cargo test --test memory -- --nocapture` (peak memory) whenever a `src/` data path
changes, and compare against these numbers — see the performance guard in `CLAUDE.md`
(no regressions greater than 5% without justification).

Only the two ABF2 fixtures are exercised. `05210017_vc_abf1.abf` is an ABF1 file and
`Abf::from_file` does not support ABF1 yet ([20]), so it would panic.

## Machine

- CI runner equivalent (GitHub Actions `ubuntu-latest`-class VM): x86_64, 4 vCPUs, 15 GiB RAM
- OS: Ubuntu 24.04 (noble), kernel 6.17
- `cargo 1.98.1 (797e8a9bc 2026-08-05)`, release profile (`cargo bench` default)

## Before / after [7] (lazy, mmap-backed data access)

[7] replaced `from_abf_v2`'s eager copy-and-de-interleave of the whole data
section into `Vec<i16>`/`Vec<f32>` buffers with lazy, per-sweep decoding
straight out of the memory-mapped file (`Channel` now holds an `Arc<Storage>`,
a data offset, and a stride, and computes each sample's byte offset on
demand). This is a direct trade: `open` no longer touches the sample data at
all (so it's ~1000x faster and allocates a constant, small amount regardless
of file size), but every sweep read now decodes straight from interleaved,
non-contiguous mmap bytes instead of copying a pre-deinterleaved contiguous
buffer, so per-read time regresses. This is the trade-off the issue asks
for, not an accident: `CLAUDE.md` ranks "low memory" above "speed" in the
project's goals, and this issue is specifically about the low-memory goal —
opening a large file should no longer require enough free memory to hold a
full decoded copy of it.

### Timing (`cargo bench`, `benches/read.rs`)

Criterion reports `[lower bound, estimate, upper bound]` of the mean.

| Benchmark | Fixture | Before (0.4.4) | After ([7]) | Change |
| --- | --- | --- | --- | --- |
| `open` | `14o08011_ic_pair.abf` (7.2 MB) | 23.75 ms | 17.26 µs | ~1376x faster |
| `open` | `18425108.abf` (1.0 MB) | 2.73 ms | 16.51 µs | ~165x faster |
| `read_all_sweeps_f32` | `14o08011_ic_pair.abf` | 1.083 ms | 2.718 ms | ~2.5x slower |
| `read_all_sweeps_f32` | `18425108.abf` | 205.9 µs | 397.6 µs | ~1.9x slower |
| `read_all_sweeps_raw` | `14o08011_ic_pair.abf` | 270.0 µs | 2.580 ms | ~9.6x slower |
| `read_all_sweeps_raw` | `18425108.abf` | 66.8 µs | 377.9 µs | ~5.7x slower |

`open` now only memory-maps the file and parses the (small, fixed-size)
header/section metadata, independent of the data section's size — hence the
roughly constant, sub-20µs time for both fixtures instead of scaling with
file size. `read_all_sweeps_raw` regresses more than `read_all_sweeps_f32`
because its *old* implementation was effectively a contiguous slice copy
(`slice.par_iter().copied().collect()`), the cheapest possible operation;
its new implementation, like `read_all_sweeps_f32`, must compute a strided
byte offset and bounds-checked slice per sample, which also defeats
sequential-access prefetching on the interleaved mmap bytes. Every
`Abf::from_file` call in a full read-then-discard workload still comes out
far ahead: e.g. for `14o08011_ic_pair.abf`, old `open` + `read_all_sweeps_f32`
totalled ~24.8 ms, new totals ~2.74 ms.

### Peak memory (`cargo test --release --test memory -- --nocapture --test-threads=1`, `tests/memory.rs`)

Measured with a custom counting `#[global_allocator]` (`tests/common/alloc_counter.rs`)
in a dedicated test binary; "peak bytes" is the highest total bytes-allocated
watermark reached while the measured closure ran. `--test-threads=1` avoids the
two tests' allocation counts bleeding into each other (the allocator is a single
process-wide counter); the in-binary tests instead serialize via a `Mutex`.

| Fixture | `Abf::from_file` peak (before) | `Abf::from_file` peak (after) | Read one sweep peak (before) | Read one sweep peak (after) |
| --- | --- | --- | --- | --- |
| `14o08011_ic_pair.abf` | 11,442,218 bytes (~10.9 MiB) | 15,791 bytes (~15.4 KiB) | 10,838,070 bytes (~10.3 MiB) | 2,447,456 bytes (~2.3 MiB) |
| `18425108.abf` | 1,577,370 bytes (~1.5 MiB) | 49,253 bytes (~48.1 KiB) | 2,538,064 bytes (~2.4 MiB) | 1,047,450 bytes (~1.0 MiB) |

`Abf::from_file` peak memory dropped from ~1.5-1.6x the raw file size to a small,
file-size-independent amount (header/section metadata plus a handful of small
`String`/`Vec` allocations) — both fixtures now open under the 64 KiB budget
asserted by `tests/memory.rs`'s `opening_a_file_does_not_copy_sample_data_onto_the_heap`
test. Reading one sweep also dropped, since decoding no longer needs the
whole pre-deinterleaved dataset resident to begin with.
