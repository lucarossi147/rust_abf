# rust_abf

A Rust crate for reading data from Axon Binary Format (ABF) files, the format produced by
Molecular Devices' pCLAMP/AxoScope software and commonly used in electrophysiology.

- **Correct data**: format semantics are matched against [pyABF](https://github.com/swharden/pyABF), the reference implementation.
- **No panics on any input**: malformed or truncated files return a typed [`AbfError`](https://docs.rs/rust_abf/latest/rust_abf/enum.AbfError.html) instead of panicking.
- **Low memory**: sample data is read from disk and decoded on demand; opening a file reads only its header and metadata, and memory use does not grow with the file's size.
- **No `unsafe`**: the crate is `#![forbid(unsafe_code)]`.
- **Minimal dependencies**: no runtime dependencies by default; `rayon` is optional, behind the `parallel` feature.

Currently only ABF2 files are supported; ABF1 files are recognized but return
`Err(AbfError::UnsupportedVersion(_))`.

## Installation

```toml
[dependencies]
rust_abf = "0.6"
```

or `cargo add rust_abf`.

## Quick start

```rust
use rust_abf::Abf;
use std::path::Path;

let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
assert_eq!(abf.channel_count(), 2);
assert_eq!(abf.sweep_count(), 1);
assert_eq!(abf.sampling_rate(), 25_000.0);

for channel in abf.channels() {
    let sweep = channel.sweep(0).unwrap();
    println!(
        "channel {:?} ({:?}): {} points",
        channel.label(),
        channel.uom(),
        sweep.len()
    );
}
```

## Working with a single channel

```rust
use rust_abf::Abf;
use std::path::Path;

let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
let channel = abf.channel(0).unwrap();
println!("unit: {:?}, label: {:?}", channel.uom(), channel.label());

for sweep_index in 0..abf.sweep_count() {
    let sweep = channel.sweep(sweep_index).unwrap();
    println!("sweep {sweep_index} has {} points", sweep.len());
}
```

A functional style works too, using iterators end to end:

```rust
use rust_abf::Abf;
use std::path::Path;

let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
let total_points: usize = abf
    .channels()
    .flat_map(rust_abf::Channel::sweeps)
    .flatten()
    .map(|sweep| sweep.len())
    .sum();
assert_eq!(total_points, 500_000);
```

## Handling errors

`Abf::from_file` never panics: any I/O failure or malformed input comes back as a typed
[`AbfError`](https://docs.rs/rust_abf/latest/rust_abf/enum.AbfError.html) instead.

```rust
use rust_abf::{Abf, AbfError};
use std::path::Path;

match Abf::from_file(Path::new("tests/test_abf/does_not_exist.abf")) {
    Ok(abf) => println!("opened {} channel(s)", abf.channel_count()),
    Err(AbfError::Io(e)) => eprintln!("could not open the file: {e}"),
    Err(e) => eprintln!("could not parse the file: {e}"),
}
```

## Memory model

`Abf::from_file` reads only the header and metadata sections at open time. Sample data
stays on disk: every read fetches just the bytes it needs, in chunks of up to 256 KiB,
and decodes them. Opening a file is cheap and memory use does not grow with its size,
even when every sample of a multi-GB recording is read. This also means:

- The file stays open for as long as the `Abf` (or any `Channel` obtained from it) is
  alive. Modifying or truncating it in the meantime is safe but gives inconsistent data;
  samples that can no longer be read come back as zeros.
- For small files read many times, `Abf::from_bytes(std::fs::read(path)?)` keeps the whole
  file in memory and decodes straight from it, without a system call per read.
- [`Channel::sweep`] and [`Channel::raw_sweep`] allocate a fresh `Vec` per call;
  [`Channel::sweep_iter`] and [`Channel::raw_sweep_iter`] allocate one 64 KiB buffer when
  created and nothing while iterating; [`Channel::read_sweep_into`] decodes into a
  caller-supplied buffer without allocating, so it can be reused across repeated reads:

  ```rust
  use rust_abf::Abf;
  use std::path::Path;

  let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
  let channel = abf.channel(0).unwrap();
  let mut buffer = vec![0.0f32; channel.sweep_len()];
  for sweep_index in 0..abf.sweep_count() {
      channel.read_sweep_into(sweep_index, &mut buffer).unwrap();
      // `buffer` is reused for every sweep instead of allocating a new `Vec` each time.
  }
  ```

## Reading every channel at once

[`Abf::read_sweep_all_channels_into`] decodes all channels of a sweep into caller-supplied
buffers, one per channel, without allocating. Reuse the buffers across sweeps:

```rust
use rust_abf::Abf;
use std::path::Path;

let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
let mut buffers: Vec<Vec<f32>> = abf
    .channels()
    .map(|channel| vec![0.0f32; channel.sweep_len()])
    .collect();
for sweep_index in 0..abf.sweep_count() {
    abf.read_sweep_all_channels_into(sweep_index, &mut buffers).unwrap();
    // `buffers[c]` now holds channel `c`'s samples for this sweep.
}
```

When at least two channels share a sweep length and a sweep is at least
[`Abf::SINGLE_PASS_MIN_BYTES`] (256 KiB), the interleaved data is read once in a
cache-blocked pass instead of once per channel; otherwise each channel is read separately.
The choice is automatic and the results are bit-identical. [`Abf::sweep_all_channels`] is the
allocating equivalent and returns `Option<Vec<Vec<f32>>>`.

## Optional parallel decoding

```toml
[dependencies]
rust_abf = { version = "0.6", features = ["parallel"] }
```

With the `parallel` feature, the allocating APIs ([`Channel::sweep`], [`Channel::raw_sweep`]
and [`Abf::sweep_all_channels`]) decode sweeps of at least 1,048,576 samples on
[rayon](https://crates.io/crates/rayon)'s thread pool. It does not affect the `*_into`
APIs, which stay single-threaded and allocation-free, nor the iterators, nor smaller
sweeps. rayon 1.11+ needs a newer rustc than this crate's MSRV (1.66); on an old toolchain,
pin `rayon` to 1.10.

## Float vs. int16 samples

ABF files store each channel's samples either as `i16` or as `f32`, reported by
[`Channel::file_kind`] as [`FileKind::I16`] or [`FileKind::F32`]. This crate always hands
back physical units as `f32` from [`Channel::sweep`] and friends, but the two on-disk
representations are *not* treated the same way:

- `i16` samples are scaled with the channel's `gain`/`offset` (`raw as f32 * gain + offset`)
  to produce physical units. The unscaled integers are still available via
  [`Channel::raw_sweep`]/[`Channel::raw_sweep_iter`], which return `None` for `f32` channels.
- `f32` samples are already stored in physical units and are returned unscaled, ignoring
  `gain`/`offset` — matching pyABF, which does not apply scaling to `f32` (`"float"`) data
  either.

## Contributing

Contributions are welcome! If you encounter issues or have suggestions, please open an
issue or submit a pull request.

## License

This project is licensed under the MIT License. See the `LICENSE` file for details.
