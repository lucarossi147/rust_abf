//! `Abf::sweep_all_channels` and `Abf::read_sweep_all_channels_into` must
//! agree exactly with the per-channel read APIs on the real fixtures.

use rust_abf::Abf;
use std::path::Path;

const FIXTURES: &[&str] = &[
    "tests/test_abf/14o08011_ic_pair.abf",
    "tests/test_abf/18425108.abf",
];

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

fn check(abf: &Abf, label: &str) {
    assert!(abf.channel_count() >= 1, "{label}");
    assert!(abf.sweep_count() >= 1, "{label}");
    // Buffers are reused across every sweep, pre-filled with junk.
    let mut bufs: Vec<Vec<f32>> = abf
        .channels()
        .map(|c| vec![f32::NAN; c.sweep_len()])
        .collect();
    for s in 0..abf.sweep_count() {
        let all = abf.sweep_all_channels(s).unwrap();
        assert_eq!(all.len(), abf.channel_count(), "{label} sweep {s}");
        abf.read_sweep_all_channels_into(s, &mut bufs).unwrap();
        for (c, channel) in abf.channels().enumerate() {
            let expected = channel.sweep(s).unwrap();
            assert_eq!(bits(&all[c]), bits(&expected), "{label} all c{c} s{s}");
            assert_eq!(bits(&bufs[c]), bits(&expected), "{label} into c{c} s{s}");
            assert_eq!(
                bits(&expected),
                bits(&abf.sweep(c, s).unwrap()),
                "{label} abf.sweep c{c} s{s}"
            );
        }
    }
    assert!(abf.sweep_all_channels(abf.sweep_count()).is_none());
}

#[test]
fn all_channels_match_per_channel_reads_from_file() {
    for fixture in FIXTURES {
        let abf = Abf::from_file(Path::new(fixture)).unwrap();
        check(&abf, fixture);
    }
}

#[test]
fn all_channels_match_per_channel_reads_from_bytes() {
    for fixture in FIXTURES {
        let abf = Abf::from_bytes(std::fs::read(fixture).unwrap()).unwrap();
        check(&abf, fixture);
    }
}
