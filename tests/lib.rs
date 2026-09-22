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
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        for ch in 0..ch_num {
            let data = abf.get_sweep_in_channel(0, ch).unwrap();
            assert_eq!(&data.len(), &600_000);
            assert_eq!(abf.get_channel(ch).map(|ch| ch.get_uom()), Some("mV"));
            assert_eq!(&data.len(), &600_000);
        }
        // assert!(elapsed_time.as_millis()<100);
    }

    #[test]
    fn test_access_abf_by_channel() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/14o08011_ic_pair.abf")).unwrap();
        let _elapsed_time = start_time.elapsed();
        let ch0 = abf.get_channel(0).unwrap();
        // println!("{:?}", elapsed_time);
        assert!(matches!(ch0.get_label(), "IN 0"));
        assert!(matches!(ch0.get_uom(), "mV"));
        assert!(matches!(abf.get_sweeps_count(), 3));
        for s in 0..abf.get_sweeps_count() {
            assert_eq!(ch0.get_sweep(s).map(|ch| ch.len()), Some(600_000));
        }
    }

    #[test]
    fn test_abfv2_2() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        for ch in 0..ch_num {
            let data = abf.get_sweep_in_channel(0, ch).unwrap();
            assert_eq!(&data.len(), &250000);
            let ch = abf.get_channel(ch).unwrap();
            println!("Channel {:?} has as uom {:?}", ch.get_label(), ch.get_uom());
        }
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
    }

    #[test]
    fn iterate_over_sweep_and_channel() {
        let start_time = Instant::now();
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        let sw_num = abf.get_sweeps_count();
        assert_eq!(ch_num, 2);
        assert_eq!(sw_num, 1);
        (0..ch_num).for_each(|ch| {
            (0..sw_num).for_each(|s| {
                let data = abf.get_sweep_in_channel(s, ch).unwrap();
                assert_eq!(data.len(), 250_000)
            });
        });
    }

    #[test]
    fn test_iterator_over_channels() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        abf.get_channels()
            .for_each(|c| assert_eq!(c.get_sweep(0).unwrap().len(), 250_000));
    }

    #[test]
    fn test_iterator_over_channels_and_sweeps() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        abf.get_channels()
            .flat_map(|c| c.get_sweeps())
            .for_each(|s| assert_eq!(s.unwrap().len(), 250_000));
    }

    #[test]
    fn test_get_path() {
        let path = Path::new("tests/test_abf/18425108.abf");
        let abf = Abf::from_file(path).unwrap();
        assert_eq!(abf.get_path(), path);
    }

    #[test]
    #[ignore = "This test uses a very large file that is not versioned, and would break the ci"]
    fn test_abfv2_heavy() {
        let start_time = Instant::now();
        let abf =
            Abf::from_file(Path::new("C:\\Users\\lucar\\Desktop\\file_CH001_000.abf")).unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert_eq!(abf.get_sweeps_count(), 1);
        assert_eq!(abf.get_channels_count(), 2);
        assert_eq!(abf.get_channel(0).unwrap().get_label(), "I0");
        assert_eq!(abf.get_channel(0).unwrap().get_uom(), "nA");
        assert_eq!(abf.get_channel(1).unwrap().get_label(), "V0");
        assert_eq!(abf.get_channel(1).unwrap().get_uom(), "mV");
        // assert!(elapsed_time.as_millis()<900);
    }

    #[test]
    fn test_sampling_rate() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let sr = abf.get_sampling_rate();
        assert_eq!(sr, 25000.0);
    }

    #[test]
    fn test_channel_gain_and_offset() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let ch0 = abf.get_channel(0).unwrap();
        assert!(ch0.get_gain().is_finite());
        assert!(ch0.get_offset().is_finite());
    }

    #[test]
    fn test_time_axis() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let time_axis = abf.get_time_axis();
        assert!(!time_axis.is_empty());
        assert_eq!(time_axis[0], 0.0);
        let expected_step = 1.0 / abf.get_sampling_rate();
        assert!((time_axis[1] - expected_step).abs() < f32::EPSILON);
    }

    #[test]
    fn test_time_duration() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let duration = abf.get_time_duration().unwrap();
        let expected = abf.get_channel(0).unwrap().get_sweep(0).unwrap().len() as f32
            / abf.get_sweeps_count() as f32
            / abf.get_sampling_rate();
        assert!((duration - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn test_raw_sweep_out_of_bounds_is_none() {
        let abf = Abf::from_file(Path::new("tests/test_abf/18425108.abf")).unwrap();
        let ch0 = abf.get_channel(0).unwrap();
        assert_eq!(ch0.get_raw_sweep(abf.get_sweeps_count() + 1), None);
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
    // #[test]
    // fn test_abfv1(){
    //     let start_time = Instant::now();
    //     let abf = Abf::new("tests/test_abf/05210017_vc_abf1.abf");
    //     println!("{:?}", start_time.elapsed());
    //     assert!(matches!(abf.file_signature, AbfType::AbfV1));
    //     assert_eq!(abf.actual_episodes, 10);
    //     assert_eq!(abf.file_info_size, 4236247045);
    //     assert_eq!(abf.file_start_date, 6);
    //     assert_eq!(abf.file_start_time_ms, 20050210);
    //     assert_eq!(abf.file_type, 11985);
    // }
}
