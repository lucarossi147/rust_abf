//! One test per `#[deprecated]` alias kept for issue [10] / issue #15's
//! 0.5.0 API rename, asserting each old name still behaves exactly like its
//! new replacement. These aliases (and this file) are removed once the
//! deprecation period ends.
#![allow(deprecated)]

use rust_abf::{Abf, AbfKind};
use std::path::Path;

const FIXTURE: &str = "tests/test_abf/18425108.abf";

#[test]
fn get_channels_count_matches_channel_count() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_channels_count(), abf.channel_count() as u32);
}

#[test]
fn get_sweeps_count_matches_sweep_count() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_sweeps_count(), abf.sweep_count() as u32);
}

#[test]
fn get_sweep_in_channel_matches_sweep_with_swapped_argument_order() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_sweep_in_channel(0, 1), abf.sweep(1, 0));
}

#[test]
fn get_file_signature_matches_kind() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
    assert!(matches!(abf.kind(), AbfKind::AbfV2));
}

#[test]
fn get_channel_matches_channel() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(
        abf.get_channel(0).unwrap().label(),
        abf.channel(0).unwrap().label()
    );
}

#[test]
fn get_channels_matches_channels() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let old: Vec<Option<&str>> = abf.get_channels().map(|c| c.label()).collect();
    let new: Vec<Option<&str>> = abf.channels().map(|c| c.label()).collect();
    assert_eq!(old, new);
}

#[test]
fn get_sampling_rate_matches_sampling_rate() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_sampling_rate(), abf.sampling_rate());
}

#[test]
fn get_path_matches_path() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_path(), abf.path());
}

#[test]
fn get_time_duration_matches_time_duration() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_time_duration(), abf.time_duration());
}

#[test]
fn get_time_axis_matches_time_axis() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    assert_eq!(abf.get_time_axis(), abf.time_axis());
}

#[test]
fn get_uom_falls_back_to_nan_string_like_before() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_uom(), channel.uom().unwrap_or("nan"));
}

#[test]
fn get_label_falls_back_to_nan_string_like_before() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_label(), channel.label().unwrap_or("nan"));
}

#[test]
fn get_gain_matches_gain() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_gain(), channel.gain());
}

#[test]
fn get_offset_matches_offset() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_offset(), channel.offset());
}

#[test]
fn get_file_kind_matches_file_kind() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_file_kind(), channel.file_kind());
}

#[test]
fn get_raw_sweep_matches_raw_sweep() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_raw_sweep(0), channel.raw_sweep(0));
}

#[test]
fn get_sweep_matches_sweep() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_sweep(0), channel.sweep(0));
}

#[test]
fn get_sweeps_matches_sweeps() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    let old: Vec<_> = channel.get_sweeps().collect();
    let new: Vec<_> = channel.sweeps().collect();
    assert_eq!(old, new);
}

#[test]
fn get_sweep_len_matches_sweep_len() {
    let abf = Abf::from_file(Path::new(FIXTURE)).unwrap();
    let channel = abf.channel(0).unwrap();
    assert_eq!(channel.get_sweep_len(), channel.sweep_len());
}
