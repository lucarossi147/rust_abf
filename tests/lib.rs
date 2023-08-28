#[cfg(test)]
mod tests {
    use std::time::Instant;
    use rust_abf::{AbfBuilder, AbfKind};

    #[test]
    fn test_abfv2_1(){
        let start_time = Instant::now();
        let abf = AbfBuilder::from_file("tests/test_abf/14o08011_ic_pair.abf").unwrap();
        let _elapsed_time = start_time.elapsed();
        // println!("{:?}", elapsed_time);
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        for ch in 0..ch_num {
            let data = abf.get_sweep_in_channel(0, ch).unwrap();
            assert_eq!(&data.len(), &600_000);
            assert_eq!(abf.get_channel(ch).and_then(|ch| Some(ch.get_uom())), Some("mV"));
            assert_eq!(&data.len(), &600_000);
        }
        // assert!(elapsed_time.as_millis()<100);
    }

    #[test]
    fn test_access_abf_by_channel(){
        let start_time = Instant::now();
        let abf = AbfBuilder::from_file("tests/test_abf/14o08011_ic_pair.abf").unwrap();
        let _elapsed_time = start_time.elapsed();
        let ch0 = abf.get_channel(0).unwrap();
        // println!("{:?}", elapsed_time);
        assert!(matches!(ch0.get_label(), "IN 0"));
        assert!(matches!(ch0.get_uom(), "mV"));
        assert!(matches!(abf.get_sweeps_count() , 3));
        for s in 0..abf.get_sweeps_count() {
            assert_eq!(
                ch0.get_sweep(s).and_then(|ch| Some(ch.len())), 
                Some(600_000)
            );
        }
    }

    #[test]
    fn test_abfv2_2(){
        let start_time = Instant::now();
        let abf = AbfBuilder::from_file("tests/test_abf/18425108.abf").unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        for ch in 0..ch_num {
            let data = abf.get_sweep_in_channel(0, ch).unwrap();
            assert_eq!(&data.len(), &250000);
            let ch = abf.get_channel(ch).unwrap();
            println!("Channel {:?} has as uom {:?}", ch.get_label(), ch.get_uom() );
        }
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
    }

    #[test]
    fn iterate_over_sweep_and_channel(){
        let start_time = Instant::now();
        let abf = AbfBuilder::from_file("tests/test_abf/18425108.abf").unwrap();
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        let sw_num = abf.get_sweeps_count();
        assert_eq!(ch_num, 2);
        assert_eq!(sw_num, 1);
        (0..ch_num).for_each(|ch| {
            (0..sw_num).for_each(|s|{
                let data = abf.get_sweep_in_channel( s, ch).unwrap();
                assert_eq!(data.len(), 250_000)
            });
        });
    }

    #[test]
    #[ignore = "This test uses a very large file that is not versioned, and would break the ci"]
    fn test_abfv2_heavy(){
        let start_time = Instant::now();
        let abf = AbfBuilder::from_file("C:\\Users\\lucar\\Desktop\\file_CH001_000.abf").unwrap();
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
