# rust_abf

A Rust crate for reading data from Axon Binary Format (ABF) files, the format produced by
Molecular Devices' pCLAMP/AxoScope software and commonly used in electrophysiology.

- **Correct data**: format semantics are matched against [pyABF](https://github.com/swharden/pyABF), the reference implementation.
- **No panics on any input**: malformed or truncated files return a typed [`AbfError`](https://docs.rs/rust_abf/latest/rust_abf/enum.AbfError.html) instead of panicking.
- **Low memory**: files are memory-mapped and decoded lazily; opening a file allocates a small, file-size-independent amount instead of copying the whole file.
- **Minimal dependencies**: `memmap2` is the crate's only runtime dependency.

Currently only ABF2 files are supported; ABF1 files are recognized but return
`Err(AbfError::UnsupportedVersion(_))`.

## Installation

```toml
[dependencies]
rust_abf = "0.5"
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

`Abf::from_file` memory-maps the file rather than reading it into a buffer, and does not
copy or de-interleave sample data at open time. That makes opening a file cheap and its
peak memory use independent of the file's size, but it also means:

- The mapping must stay valid for as long as the `Abf` (or any `Channel` obtained from it,
  since every `Channel` holds its own reference to the same mapping) is alive. Modifying or
  truncating the file on disk while it is mapped is undefined behavior — see the safety
  caveat on [`Abf::from_file`](https://docs.rs/rust_abf/latest/rust_abf/struct.Abf.html#method.from_file).
- Sample data is decoded on demand. [`Channel::sweep`] and [`Channel::raw_sweep`] allocate a
  fresh `Vec` per call; [`Channel::sweep_iter`] and [`Channel::raw_sweep_iter`] decode lazily
  without allocating at all, and [`Channel::read_sweep_into`] decodes into a caller-supplied
  buffer so it can be reused across repeated reads:

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
