//! Golden reference tests. Compares rust_abf's parsed output against numeric
//! oracles generated from pyABF (see `scripts/gen_golden.py` and
//! `tests/golden/README.md`).
use rust_abf::{Abf, AbfKind};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
struct GoldenAbfVersion {
    major: u32,
}

#[derive(Deserialize)]
struct GoldenYSeries {
    first32: Vec<f64>,
    last32: Vec<f64>,
    sum: f64,
    min: f64,
    max: f64,
}

#[derive(Deserialize)]
struct GoldenXSeries {
    first32: Vec<f64>,
    last32: Vec<f64>,
}

#[derive(Deserialize)]
struct GoldenSweep {
    sweep: u32,
    #[serde(rename = "sweepY")]
    sweep_y: GoldenYSeries,
    #[serde(rename = "sweepX")]
    sweep_x: GoldenXSeries,
}

#[derive(Deserialize)]
struct GoldenChannel {
    #[serde(rename = "adcName")]
    adc_name: String,
    #[serde(rename = "adcUnit")]
    adc_unit: String,
    sweeps: Vec<GoldenSweep>,
}

#[derive(Deserialize)]
struct Golden {
    #[serde(rename = "abfVersion")]
    abf_version: GoldenAbfVersion,
    #[serde(rename = "sweepCount")]
    sweep_count: u32,
    #[serde(rename = "channelCount")]
    channel_count: u32,
    #[serde(rename = "dataRate")]
    data_rate: f64,
    #[serde(rename = "sweepPointCount")]
    sweep_point_count: usize,
    channels: Vec<GoldenChannel>,
}

fn load_golden(stem: &str) -> Golden {
    let path = format!("tests/golden/{stem}.json");
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    serde_json::from_slice(&bytes).unwrap_or_else(|e| panic!("parse {path}: {e}"))
}

/// abs(actual - expected) <= 1e-4 * max(1, |expected|)
fn approx_eq(actual: f32, expected: f64) -> bool {
    let tol = 1e-4 * f64::max(1.0, expected.abs());
    ((actual as f64) - expected).abs() <= tol
}

/// relative tolerance of 1e-5, for sums
fn approx_eq_rel(actual: f64, expected: f64) -> bool {
    let tol = 1e-5 * f64::max(1.0, expected.abs());
    (actual - expected).abs() <= tol
}

fn check_against_golden(abf_path: &str, golden_stem: &str) {
    check_abf_against_golden(&Abf::from_file(Path::new(abf_path)).unwrap(), golden_stem);
}

/// Same checks as [`check_against_golden`], but parses `abf_path` through
/// `Abf::from_bytes` instead of `Abf::from_file`, so a passing pair of tests
/// proves the two constructors agree on every golden value, not just on
/// each other in isolation.
fn check_bytes_against_golden(abf_path: &str, golden_stem: &str) {
    let bytes = std::fs::read(abf_path).unwrap();
    check_abf_against_golden(&Abf::from_bytes(bytes).unwrap(), golden_stem);
}

fn check_abf_against_golden(abf: &Abf, golden_stem: &str) {
    let golden = load_golden(golden_stem);

    assert!(matches!(abf.kind(), AbfKind::AbfV2));
    assert_eq!(golden.abf_version.major, 2);
    assert_eq!(abf.channel_count() as u32, golden.channel_count);
    assert_eq!(abf.sweep_count() as u32, golden.sweep_count);
    assert!(
        approx_eq(abf.sampling_rate(), golden.data_rate),
        "sampling_rate: got {}, expected {}",
        abf.sampling_rate(),
        golden.data_rate
    );

    for (ch_idx, golden_channel) in golden.channels.iter().enumerate() {
        let channel = abf
            .channel(ch_idx)
            .unwrap_or_else(|| panic!("missing channel {ch_idx}"));
        assert_eq!(channel.label(), Some(golden_channel.adc_name.as_str()));
        assert_eq!(channel.uom(), Some(golden_channel.adc_unit.as_str()));

        for golden_sweep in &golden_channel.sweeps {
            let sweep_y = abf
                .sweep(ch_idx, golden_sweep.sweep as usize)
                .unwrap_or_else(|| {
                    panic!("missing sweep {} in channel {ch_idx}", golden_sweep.sweep)
                });
            assert_eq!(sweep_y.len(), golden.sweep_point_count);

            for (actual, expected) in sweep_y.iter().take(32).zip(&golden_sweep.sweep_y.first32) {
                assert!(
                    approx_eq(*actual, *expected),
                    "first32 mismatch ch {ch_idx} sweep {}: got {actual}, expected {expected}",
                    golden_sweep.sweep
                );
            }
            for (actual, expected) in sweep_y
                .iter()
                .rev()
                .take(32)
                .rev()
                .zip(&golden_sweep.sweep_y.last32)
            {
                assert!(
                    approx_eq(*actual, *expected),
                    "last32 mismatch ch {ch_idx} sweep {}: got {actual}, expected {expected}",
                    golden_sweep.sweep
                );
            }

            let sum: f64 = sweep_y.iter().map(|v| *v as f64).sum();
            assert!(
                approx_eq_rel(sum, golden_sweep.sweep_y.sum),
                "sum mismatch ch {ch_idx} sweep {}: got {sum}, expected {}",
                golden_sweep.sweep,
                golden_sweep.sweep_y.sum
            );

            let min = sweep_y.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = sweep_y.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            assert!(
                approx_eq(min, golden_sweep.sweep_y.min),
                "min mismatch ch {ch_idx} sweep {}: got {min}, expected {}",
                golden_sweep.sweep,
                golden_sweep.sweep_y.min
            );
            assert!(
                approx_eq(max, golden_sweep.sweep_y.max),
                "max mismatch ch {ch_idx} sweep {}: got {max}, expected {}",
                golden_sweep.sweep,
                golden_sweep.sweep_y.max
            );
        }
    }
}

#[test]
fn golden_18425108() {
    check_against_golden("tests/test_abf/18425108.abf", "18425108");
}

#[test]
fn golden_14o08011_ic_pair() {
    check_against_golden("tests/test_abf/14o08011_ic_pair.abf", "14o08011_ic_pair");
}

#[test]
fn golden_bytes_18425108() {
    check_bytes_against_golden("tests/test_abf/18425108.abf", "18425108");
}

#[test]
fn golden_bytes_14o08011_ic_pair() {
    check_bytes_against_golden("tests/test_abf/14o08011_ic_pair.abf", "14o08011_ic_pair");
}

#[test]
#[ignore = "fixed by #25 ([20] ABF1 support): Abf::from_file panics (todo!()) on the \"ABF \" signature"]
fn golden_abf1_05210017() {
    check_against_golden("tests/test_abf/05210017_vc_abf1.abf", "05210017_vc_abf1");
}

#[test]
fn golden_time_axis_matches_pyabf_for_multi_sweep_file() {
    let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
    let golden = load_golden("14o08011_ic_pair");
    let time_axis = abf.time_axis();
    assert_eq!(time_axis.len(), golden.sweep_point_count);

    let expected_first32 = &golden.channels[0].sweeps[0].sweep_x.first32;
    for (actual, expected) in time_axis.iter().take(32).zip(expected_first32) {
        assert!(approx_eq(*actual, *expected));
    }
    let expected_last32 = &golden.channels[0].sweeps[0].sweep_x.last32;
    for (actual, expected) in time_axis.iter().rev().take(32).rev().zip(expected_last32) {
        assert!(approx_eq(*actual, *expected));
    }
}

#[test]
fn golden_get_channels_preserves_adc_order() {
    let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
    let golden = load_golden("14o08011_ic_pair");
    let labels: Vec<Option<&str>> = abf.channels().map(|c| c.label()).collect();
    let expected: Vec<Option<&str>> = golden
        .channels
        .iter()
        .map(|c| Some(c.adc_name.as_str()))
        .collect();
    assert_eq!(labels, expected);
}
