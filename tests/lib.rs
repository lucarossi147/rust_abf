#[cfg(test)]
mod tests {
    use rust_abf::*;
    use std::time::Instant;

    #[test]
    fn test_abfv2_1(){
        let start_time = Instant::now();
        let abf = Abf::new("tests/test_abf/14o08011_ic_pair.abf");
        let elapsed_time = start_time.elapsed();
        println!("{:?}", elapsed_time);
        assert!(matches!(abf.file_signature, AbfType::AbfV2));
        assert_eq!(abf.actual_episodes, 3);
        assert_eq!(abf.file_info_size, 512);
        assert_eq!(abf.file_start_date, 20141008);
        assert_eq!(abf.file_start_time_ms, 60198203);
        assert_eq!(abf.file_type, 1);
        assert_eq!(abf.data[0], -2147);
        // assert!(elapsed_time.as_millis()<2);
    }

    #[test]
    fn test_abfv2_2(){
        let start_time = Instant::now();
        let abf = Abf::new("tests/test_abf/18425108.abf");
        println!("{:?}", start_time.elapsed());
        assert!(matches!(abf.file_signature, AbfType::AbfV2));
        assert_eq!(abf.actual_episodes, 1);
        assert_eq!(abf.file_info_size, 512);
        assert_eq!(abf.file_start_date, 20180425);
        assert_eq!(abf.file_start_time_ms, 56871040);
        assert_eq!(abf.file_type, 1);
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
