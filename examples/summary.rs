//! Prints a summary of channels, units, sweeps and sampling rate for an ABF file.
//!
//! ```sh
//! cargo run --example summary tests/test_abf/18425108.abf
//! ```

use rust_abf::Abf;
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: summary <path-to-abf-file>");
        return ExitCode::FAILURE;
    };

    let abf = match Abf::from_file(Path::new(&path)) {
        Ok(abf) => abf,
        Err(e) => {
            eprintln!("failed to open {path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!("file: {}", abf.path().display());
    println!("sampling rate: {} Hz", abf.sampling_rate());
    println!("sweeps: {}", abf.sweep_count());
    println!("channels: {}", abf.channel_count());
    for channel in abf.channels() {
        println!(
            "  channel {}: label={:?} unit={:?}",
            channel.index(),
            channel.label(),
            channel.uom()
        );
    }

    ExitCode::SUCCESS
}
