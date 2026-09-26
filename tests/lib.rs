#[cfg(test)]
mod tests {
    use rust_abf::{self, Abf, AbfKind};
    use std::{path::Path, time::Instant};

    #[test]
    fn test_abfv2_1() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
        let _elapsed_time = start_time.elapsed();
        // println!("{:?}", elapsed_time);
        assert!(matches!(abf.kind(), AbfKind::AbfV2));
        let ch_num = abf.channel_count();
        for ch in 0..ch_num {
            let data = abf.sweep(ch, 0).unwrap();
            assert_eq!(&data.len(), &600_000);
            assert_eq!(abf.channel(ch).map(|ch| ch.uom()), Some(Some("mV")));
            assert_eq!(&data.len(), &600_000);
        }
        // assert!(elapsed_time.as_millis()<100);
    }

    #[test]
    fn test_access_abf_by_channel() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
        let _elapsed_time = start_time.elapsed();
        let ch0 = abf.channel(0).unwrap();
        // println!("{:?}", elapsed_time);
        assert!(matches!(ch0.label(), Some("IN 0")));
        assert!(matches!(ch0.uom(), Some("mV")));
        assert!(matches!(abf.sweep_count(), 3));
        for s in 0..abf.sweep_count() {
            assert_eq!(ch0.sweep(s).map(|ch| ch.len()), Some(600_000));
        }
    }

    #[test]
    fn test_abfv2_2() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.kind(), AbfKind::AbfV2));
        let ch_num = abf.channel_count();
        for ch in 0..ch_num {
            let data = abf.sweep(ch, 0).unwrap();
            assert_eq!(&data.len(), &250000);
            let ch = abf.channel(ch).unwrap();
            println!("Channel {:?} has as uom {:?}", ch.label(), ch.uom());
        }
        assert!(matches!(abf.kind(), AbfKind::AbfV2));
    }

    #[test]
    fn iterate_over_sweep_and_channel() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.kind(), AbfKind::AbfV2));
        let ch_num = abf.channel_count();
        let sw_num = abf.sweep_count();
        assert_eq!(ch_num, 2);
        assert_eq!(sw_num, 1);
        (0..ch_num).for_each(|ch| {
            (0..sw_num).for_each(|s| {
                let data = abf.sweep(ch, s).unwrap();
                assert_eq!(data.len(), 250_000)
            });
        });
    }

    #[test]
    fn test_iterator_over_channels() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        abf.channels()
            .for_each(|c| assert_eq!(c.sweep(0).unwrap().len(), 250_000));
    }

    #[test]
    fn test_iterator_over_channels_and_sweeps() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        abf.channels()
            .flat_map(|c| c.sweeps())
            .for_each(|s| assert_eq!(s.unwrap().len(), 250_000));
    }

    #[test]
    fn test_get_path() {
        let path = Path::new("tests/test_abf/18425108.abf");
        let abf = Abf::from_file(path).unwrap();
        assert_eq!(abf.path(), path);
    }

    #[test]
    fn test_abfv2_heavy() {
        let Ok(path) = std::env::var("ABF_HEAVY_FILE") else {
            eprintln!("skipping: ABF_HEAVY_FILE not set");
            return;
        };
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new(&path)).unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert_eq!(abf.sweep_count(), 1);
        assert_eq!(abf.channel_count(), 2);
        assert_eq!(abf.channel(0).unwrap().label(), Some("I0"));
        assert_eq!(abf.channel(0).unwrap().uom(), Some("nA"));
        assert_eq!(abf.channel(1).unwrap().label(), Some("V0"));
        assert_eq!(abf.channel(1).unwrap().uom(), Some("mV"));
        // assert!(elapsed_time.as_millis()<900);
    }

    #[test]
    fn test_sampling_rate() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let sr = abf.sampling_rate();
        assert_eq!(sr, 25000.0);
    }

    #[test]
    fn test_channel_gain_and_offset() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let ch0 = abf.channel(0).unwrap();
        assert!(ch0.gain().is_finite());
        assert!(ch0.offset().is_finite());
    }

    #[test]
    fn test_time_axis() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let time_axis = abf.time_axis();
        assert!(!time_axis.is_empty());
        assert_eq!(time_axis[0], 0.0);
        let expected_step = 1.0 / abf.sampling_rate();
        assert!((time_axis[1] - expected_step).abs() < f32::EPSILON);
    }

    #[test]
    fn test_time_axis_len_matches_sweep_len_for_every_fixture() {
        for path in [
            "tests/test_abf/14o08011_ic_pair.abf",
            "tests/test_abf/18425108.abf",
        ] {
            let abf = Abf::from_file(Path::new(path)).unwrap();
            let expected_len = abf.channel(0).unwrap().sweep_len();
            assert_eq!(
                abf.time_axis().len(),
                expected_len,
                "get_time_axis() length mismatch for {path}"
            );
        }
    }

    #[test]
    fn test_time_axis_values_for_multi_sweep_file() {
        // 14o08011_ic_pair.abf has 3 sweeps of 600_000 points each; get_time_axis()
        // must return one time point per sample of a single sweep, not divide by
        // sweeps_count a second time.
        let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
        let sampling_rate = abf.sampling_rate() as f64;
        let time_axis = abf.time_axis();
        assert_eq!(time_axis.len(), 600_000);
        let last = time_axis.len() - 1;
        for k in [0usize, 1, last] {
            let expected = (k as f64 / sampling_rate) as f32;
            assert!(
                (time_axis[k] - expected).abs() < 1e-4,
                "index {k}: got {}, expected {expected}",
                time_axis[k]
            );
        }
    }

    #[test]
    fn test_time_duration() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let duration = abf.time_duration().unwrap();
        let expected = abf.channel(0).unwrap().sweep(0).unwrap().len() as f32
            / abf.sweep_count() as f32
            / abf.sampling_rate();
        assert!((duration - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn test_raw_sweep_out_of_bounds_is_none() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let ch0 = abf.channel(0).unwrap();
        assert_eq!(ch0.raw_sweep(abf.sweep_count() + 1), None);
    }

    #[test]
    fn test_raw_sweep_at_sweeps_count_boundary_is_none() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let ch0 = abf.channel(0).unwrap();
        let sweeps_count = abf.sweep_count();
        assert_eq!(ch0.raw_sweep(sweeps_count), None);
        assert_eq!(ch0.sweep(sweeps_count), None);
    }

    #[test]
    fn test_sweep_at_usize_max_is_none() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let ch0 = abf.channel(0).unwrap();
        assert_eq!(ch0.raw_sweep(usize::MAX), None);
        assert_eq!(ch0.sweep(usize::MAX), None);
    }

    #[test]
    fn test_last_valid_sweep_is_some_with_expected_len() {
        let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
        let ch0 = abf.channel(0).unwrap();
        let last = abf.sweep_count() - 1;
        let sweep = ch0.raw_sweep(last).unwrap();
        assert_eq!(sweep.len(), ch0.sweep_len());
    }

    #[test]
    fn test_get_channels_order_is_stable_across_opens() {
        for _ in 0..20 {
            let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
            let ch_num = abf.channel_count();
            let expected: Vec<Option<String>> = (0..ch_num)
                .map(|i| abf.channel(i).unwrap().label().map(str::to_string))
                .collect();
            let actual: Vec<Option<String>> = abf
                .channels()
                .map(|c| c.label().map(str::to_string))
                .collect();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn test_from_file_wrong_signature_is_err() {
        let result = Abf::from_file(Path::new("tests/test_abf/wrong_signature.abf"));
        assert!(result.is_err());
    }

    #[test]
    fn test_from_file_invalid_utf8_signature_is_err() {
        let result = Abf::from_file(Path::new("tests/test_abf/invalid_utf8_signature.abf"));
        assert!(result.is_err());
    }
}
