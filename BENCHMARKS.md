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

## Timing (`cargo bench`, `benches/read.rs`)

Criterion reports `[lower bound, estimate, upper bound]` of the mean.

| Benchmark | Fixture | Time |
| --- | --- | --- |
| `open` | `14o08011_ic_pair.abf` (7.2 MB, 2 ch × 3 sweeps × 600,000 samples) | 23.75 ms |
| `open` | `18425108.abf` (1.0 MB, 2 ch × 1 sweep × 250,000 samples) | 2.73 ms |
| `read_all_sweeps_f32` | `14o08011_ic_pair.abf` | 1.083 ms |
| `read_all_sweeps_f32` | `18425108.abf` | 205.9 µs |
| `read_all_sweeps_raw` | `14o08011_ic_pair.abf` | 270.0 µs |
| `read_all_sweeps_raw` | `18425108.abf` | 66.8 µs |

`open` dominates and scales with file size — it memory-maps the file and eagerly
decodes/scales every channel's samples into `Vec<i16>`/derived structures.
`read_all_sweeps_f32` (gain/offset-scaled `f32`, via `Channel::get_sweep`) costs
roughly 3-4x `read_all_sweeps_raw` (`i16`, via `Channel::get_raw_sweep`), consistent
with allocating and writing a `Vec<f32>` on top of the raw `i16` data plus the
scaling multiply-add per sample.

## Peak memory (`cargo test --test memory -- --nocapture`, `tests/memory.rs`)

Measured with a custom counting `#[global_allocator]` (`tests/common/alloc_counter.rs`)
in a dedicated test binary; "peak bytes" is the highest total bytes-allocated
watermark reached while the measured closure ran.

| Fixture | `Abf::from_file` peak | Read one sweep peak |
| --- | --- | --- |
| `14o08011_ic_pair.abf` | 11,442,218 bytes (~10.9 MiB) | 10,838,070 bytes (~10.3 MiB) |
| `18425108.abf` | 1,577,370 bytes (~1.5 MiB) | 2,538,064 bytes (~2.4 MiB) |

`Abf::from_file` peak memory is roughly 1.5-1.6x the raw file size, from holding the
memory-mapped file plus the eagerly-decoded per-channel `i16` buffers at once.
No thresholds are asserted yet — the test only prints the numbers so they can be
tracked over time; failure thresholds are left to a follow-up issue.
