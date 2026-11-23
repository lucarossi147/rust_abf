//! Tests for the storage-benchmark helpers in `benches/common/` (issue [34]).
//! The benches themselves only run under `cargo bench`; these keep the
//! helpers (config parsing, synthetic files, case selection, eviction)
//! correct on every OS CI tests.

#[path = "../benches/common/mod.rs"]
mod common;

use common::{
    bench_file, parse_caches, parse_mb_list, run_case, skip_reason, synth_file, Backend, Cache,
    Case, Config, SynthMode,
};
use rust_abf::Abf;
use std::ffi::OsString;

fn config(lookup: &[(&str, &str)]) -> Config {
    Config::from_lookup(|key| {
        lookup
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| OsString::from(v))
    })
    .unwrap()
}

#[test]
fn parses_mb_lists() {
    assert_eq!(parse_mb_list("64").unwrap(), [64]);
    assert_eq!(parse_mb_list(" 64, 1024 ,").unwrap(), [64, 1024]);
    assert_eq!(parse_mb_list("").unwrap(), Vec::<u64>::new());
    assert!(parse_mb_list("1GB").is_err());
}

#[test]
fn parses_cache_states() {
    assert_eq!(parse_caches("warm").unwrap(), [Cache::Warm]);
    assert_eq!(
        parse_caches("cold, warm").unwrap(),
        [Cache::Cold, Cache::Warm]
    );
    assert!(parse_caches("hot").is_err());
}

#[test]
fn config_defaults_and_file_list() {
    let cfg = config(&[]);
    assert!(cfg.files.is_empty());
    assert_eq!(cfg.synth_mb, [64]);
    assert_eq!(cfg.caches, [Cache::Warm]);
    assert_eq!(cfg.owned_max_mb, 2048);
    assert_eq!(cfg.buf_max_mb, 512);

    let joined = std::env::join_paths(["a/one.abf", "b/two.abf"]).unwrap();
    let cfg =
        Config::from_lookup(|key| (key == "RUST_ABF_BENCH_FILES").then(|| joined.clone())).unwrap();
    assert_eq!(cfg.files.len(), 2);
    assert!(cfg.files[1].ends_with("two.abf"));

    assert!(Config::from_lookup(|key| {
        (key == "RUST_ABF_BENCH_CACHE").then(|| OsString::from("lukewarm"))
    })
    .is_err());
}

#[test]
fn synthetic_gap_free_file_opens_as_one_sweep() {
    let dir = tempfile::tempdir().unwrap();
    let path = synth_file(dir.path(), 1, SynthMode::GapFree).unwrap();
    let abf = Abf::from_file(&path).unwrap();
    assert_eq!(abf.channel_count(), 2);
    assert_eq!(abf.sweep_count(), 1);
    let samples = abf.channel(0).unwrap().sweep_len() as u64 * 2;
    let data_bytes = samples * 2;
    assert!(
        data_bytes <= 1 << 20 && data_bytes > (1 << 20) - 4,
        "{data_bytes}"
    );
    // Not sparse: real non-zero data.
    assert!(abf
        .channel(0)
        .unwrap()
        .raw_sweep(0)
        .unwrap()
        .iter()
        .any(|&v| v != 0));

    // Reused, not regenerated.
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let again = synth_file(dir.path(), 1, SynthMode::GapFree).unwrap();
    assert_eq!(again, path);
    assert_eq!(
        std::fs::metadata(&again).unwrap().modified().unwrap(),
        modified
    );
}

#[test]
fn synthetic_episodic_file_has_one_second_sweeps() {
    let dir = tempfile::tempdir().unwrap();
    let path = synth_file(dir.path(), 2, SynthMode::Episodic).unwrap();
    let abf = Abf::from_file(&path).unwrap();
    assert_eq!(abf.channel_count(), 2);
    assert!(abf.sweep_count() > 1, "{}", abf.sweep_count());
    assert_eq!(
        abf.channel(0).unwrap().sweep_len(),
        abf.sampling_rate() as usize
    );
}

#[test]
fn every_case_gives_the_same_checksum_on_every_backend() {
    let dir = tempfile::tempdir().unwrap();
    let path = synth_file(dir.path(), 2, SynthMode::Episodic).unwrap();
    for case in Case::ALL {
        let sums: Vec<f64> = Backend::ALL
            .iter()
            .map(|backend| run_case(*case, *backend, &path).unwrap())
            .collect();
        assert!(sums[0].is_finite(), "{case:?}");
        assert!(sums.iter().all(|s| s == &sums[0]), "{case:?}: {sums:?}");
        if *case != Case::Open {
            assert_ne!(sums[0], 0.0, "{case:?} read no data");
        }
    }
}

#[test]
fn skips_cases_that_do_not_apply() {
    let dir = tempfile::tempdir().unwrap();
    let gap_free = bench_file(synth_file(dir.path(), 1, SynthMode::GapFree).unwrap()).unwrap();
    let episodic = bench_file(synth_file(dir.path(), 2, SynthMode::Episodic).unwrap()).unwrap();
    let cfg = config(&[]);

    let skip = |case, backend, file, cfg| skip_reason(case, backend, file, cfg).is_some();
    assert!(skip(Case::RandomSweeps, Backend::FromFile, &gap_free, &cfg));
    assert!(!skip(
        Case::RandomSweeps,
        Backend::FromFile,
        &episodic,
        &cfg
    ));
    assert!(!skip(Case::ReadAllInto, Backend::FromFile, &gap_free, &cfg));

    let tight = config(&[
        ("RUST_ABF_BENCH_BUF_MAX_MB", "0"),
        ("RUST_ABF_BENCH_OWNED_MAX_MB", "0"),
    ]);
    assert!(skip(
        Case::ReadAllInto,
        Backend::FromFile,
        &gap_free,
        &tight
    ));
    assert!(skip(Case::Open, Backend::FromBytes, &gap_free, &tight));
    assert!(!skip(Case::Open, Backend::FromFile, &gap_free, &tight));
}

#[test]
#[cfg(any(windows, target_os = "linux"))]
fn evicting_a_file_from_the_page_cache_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let path = synth_file(dir.path(), 1, SynthMode::GapFree).unwrap();
    common::evict(&path).unwrap();
    // Still readable afterwards.
    assert!(Abf::from_file(&path).is_ok());
}
