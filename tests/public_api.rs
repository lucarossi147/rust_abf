//! Compile-level check for backlog [10] / issue #15: exercises every
//! intended 0.5.0 public API item (and nothing deprecated), so an
//! accidental widening of the public surface or a broken rename shows up as
//! a compile failure here even if no other test happens to touch it.
#![deny(deprecated)]

use rust_abf::{Abf, AbfError, AbfKind};
use std::path::Path;

const FIXTURE: &str = "tests/test_abf/18425108.abf";

#[test]
fn public_api_surface_compiles_and_behaves() {
    let abf: Abf = Abf::from_file(Path::new(FIXTURE)).expect("open fixture");

    match abf.kind() {
        AbfKind::AbfV1 | AbfKind::AbfV2 => {}
    }

    let channel_count: usize = abf.channel_count();
    let sweep_count: usize = abf.sweep_count();
    let _sampling_rate: f32 = abf.sampling_rate();
    let _path: &Path = abf.path();
    let _time_axis: Vec<f32> = abf.time_axis();
    let _time_duration: Option<f32> = abf.time_duration();

    for channel_index in 0..channel_count {
        let channel = abf.channel(channel_index).expect("channel exists");
        assert_eq!(channel.index(), channel_index);

        let _uom: Option<&str> = channel.uom();
        let _label: Option<&str> = channel.label();
        let _gain: f32 = channel.gain();
        let _offset: f32 = channel.offset();
        let _file_kind = channel.file_kind();
        let sweep_len: usize = channel.sweep_len();

        for sweep_index in 0..sweep_count {
            let scaled: Option<Vec<f32>> = channel.sweep(sweep_index);
            assert_eq!(scaled.as_ref().map(Vec::len), Some(sweep_len));
            let _raw: Option<Vec<i16>> = channel.raw_sweep(sweep_index);

            let mut buf = vec![0.0f32; sweep_len];
            channel
                .read_sweep_into(sweep_index, &mut buf)
                .expect("valid sweep index and buffer length");

            if let Some(iter) = channel.sweep_iter(sweep_index) {
                assert_eq!(iter.len(), sweep_len);
            }
            let _raw_iter = channel.raw_sweep_iter(sweep_index);

            assert_eq!(abf.sweep(channel_index, sweep_index), scaled);
        }

        let _: Vec<Option<Vec<f32>>> = channel.sweeps().collect();
    }

    let _: Vec<_> = abf.channels().collect();

    let err: AbfError = Abf::from_file(Path::new("tests/test_abf/wrong_signature.abf"))
        .expect_err("wrong signature is an error");
    match err {
        AbfError::Io(_)
        | AbfError::InvalidSignature
        | AbfError::UnsupportedVersion(_)
        | AbfError::Truncated { .. }
        | AbfError::InvalidSection { .. }
        | AbfError::Unsupported(_)
        | AbfError::SweepOutOfRange { .. }
        | AbfError::BufferLengthMismatch { .. } => {}
    }
}
