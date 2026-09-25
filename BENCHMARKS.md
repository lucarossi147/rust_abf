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

## Before / after [8] (allocation-free sweep reading API)

[8] adds `Channel::read_sweep_into` (decode a sweep directly into a
caller-supplied `&mut [f32]`), and `Channel::sweep_iter`/`raw_sweep_iter`
(lazy, per-sample iterators), so a tight loop over many sweeps can reuse one
buffer instead of allocating a fresh `Vec` per call. `get_sweep`/`get_raw_sweep`
are reimplemented on top of these instead of keeping their own separate
decode path.

This required dropping the `rayon`-parallel decode that `get_sweep`/
`get_raw_sweep` used internally (and removing the now-unused `rayon`
dependency): `read_sweep_into` must be *provably* allocation-free, but
rayon's global thread pool allocates on its first use per process and can
grow its per-worker deques on subsequent parallel calls, so calling into it
from `read_sweep_into` would make the zero-allocation guarantee this issue
asks for either false or dependent on unstable rayon internals. All three
new APIs, and `get_sweep`/`get_raw_sweep` built on top of them, now decode
sequentially. `CLAUDE.md` ranks "low memory" above "speed", and this issue
is specifically about the low-memory goal, so this trade (like [7]'s) is
accepted rather than avoided.

### Timing (`cargo bench`, `benches/read.rs`)

| Benchmark | Fixture | Before ([7]) | After ([8]) | Change |
| --- | --- | --- | --- | --- |
| `open` | `14o08011_ic_pair.abf` | 21.57 µs | 21.68 µs | ~1% slower (noise) |
| `open` | `18425108.abf` | 20.04 µs | 20.98 µs | ~5% slower (noise) |
| `read_all_sweeps_f32` | `14o08011_ic_pair.abf` | 3.206 ms | 3.679 ms | ~15% slower |
| `read_all_sweeps_f32` | `18425108.abf` | 480.2 µs | 509.5 µs | ~6% slower |
| `read_all_sweeps_raw` | `14o08011_ic_pair.abf` | 3.333 ms | 4.954 ms | ~49% slower |
| `read_all_sweeps_raw` | `18425108.abf` | 489.3 µs | 689.7 µs | ~41% slower |
| `read_all_sweeps_into` (new) | `14o08011_ic_pair.abf` | n/a | 3.504 ms | n/a |
| `read_all_sweeps_into` (new) | `18425108.abf` | n/a | 511.5 µs | n/a |

`open` is unaffected (it never touched sample decoding). `read_all_sweeps_raw`
regresses the most, same as in [7]'s table, because it lost the most from
giving up parallelism: `get_raw_sweep`'s old body was `range.into_par_iter()
.map(read_i16).collect()`, now it's `raw_sweep_iter(..).collect()`, a plain
sequential loop. `read_all_sweeps_into` (the new, allocation-free entry
point) lands close to `read_all_sweeps_f32`, as expected since `get_sweep`
now just allocates a `Vec` and calls the same `read_sweep_into` body.

### Peak memory (`cargo test --release --test memory -- --nocapture --test-threads=1`, `tests/memory.rs`)

| Fixture | Read one sweep peak (before [8]) | Read one sweep peak (after [8]) | `read_sweep_into` over every sweep/channel, reused buffer |
| --- | --- | --- | --- |
| `14o08011_ic_pair.abf` | 2,447,456 bytes | 2,414,296 bytes | 0 bytes beyond the buffer |
| `18425108.abf` | 1,047,450 bytes | 1,014,290 bytes | 0 bytes beyond the buffer |

Dropping rayon also shaved a few KiB off `get_sweep`'s own peak (no more
per-call thread-pool/work-splitting bookkeeping). The new
`read_sweep_into_does_not_allocate_after_the_buffer_is_reused` test in
`tests/memory.rs` asserts the last column directly: reading every sweep of
every channel through one reused buffer allocates no more than the buffer
itself did.
