#[cfg(test)]
mod tests {
    use std::time::Instant;
    use rust_abf::{AbfBuilder, AbfKind};
    use rust_abf::abf::Abf;
    
    #[test]
    fn test_abfv2_1(){
        let start_time = Instant::now();
        let abf = AbfBuilder::from_file("tests/test_abf/14o08011_ic_pair.abf").unwrap();
        let elapsed_time = start_time.elapsed();
        // println!("{:?}", elapsed_time);
        assert!(matches!(abf.get_file_signature(), AbfKind::AbfV2));
        let ch_num = abf.get_channels_count();
        for ch in 0..ch_num {
            let data = abf.get_sweep_in_channel(0, ch).unwrap();
            assert_eq!(&data.len(), &1_800_000);
            // println!("{:?} ... {:?}", &data[..10], &data[&data.len()-10..], );
        }
        // print!("{:?}, {:?}", elapsed_time, start_time.elapsed().as_millis());
        // assert!(elapsed_time.as_millis()<100);
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
