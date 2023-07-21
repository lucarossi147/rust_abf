#[cfg(test)]
mod tests {
    use rust_abf::*;
    use std::time::Instant;

    // #[test]
    // fn test_get_file_signature_abf_v1() {
    //     let start_time = Instant::now();
    //     let result = get_file_signature("tests/test_abf/05210017_vc_abf1.abf");
    //         match result {
    //             Ok(r) =>  assert!(matches!(r, AbfType::AbfV1)),
    //             _ => assert!(false),
    //         }
    //     println!("{:?}", start_time.elapsed());
    // }

    // #[test]
    // fn test_get_file_signature_abf_v2() {
    //     let start_time = Instant::now();
    //     let result = get_file_signature("tests/test_abf/18425108.abf");
    //     match result {
    //         Ok(r) =>  assert!(matches!(r, AbfType::AbfV2)),
    //         _ => assert!(false),
    //     }
    //     println!("{:?}", start_time.elapsed());
    // }

    #[test]
    fn test_get_number_of_sweep_from_example_abf(){
        let start_time = Instant::now();
        let result = get_sweep_number("tests/test_abf/14o08011_ic_pair.abf");
        match result {
            Ok(r) =>  assert_eq!(3, r),
            _ => assert!(false),
        }
        println!("{:?}", start_time.elapsed());
    }

    #[test]
    fn test_get_number_of_sweep_from_abf_v1(){
        let start_time = Instant::now();
        let result = get_sweep_number("tests/test_abf/05210017_vc_abf1.abf");
        match result {
            Ok(r) =>  assert_eq!(10, r),
            _ => assert!(false),
        }
        println!("{:?}", start_time.elapsed());
    }

    #[test]
    fn test_get_number_of_sweep_from_abf_v2(){
        let start_time = Instant::now();
        let result = get_sweep_number("tests/test_abf/18425108.abf");
        match result {
            Ok(r) => assert_eq!(1, r),
            _ => assert!(false),
        }
        println!("{:?}", start_time.elapsed());
    }

    #[test]
    fn test_abfv1(){
        let start_time = Instant::now();
        let abf = Abf::new("tests/test_abf/14o08011_ic_pair.abf");
        println!("{:?}", start_time.elapsed());
        assert!(matches!(abf.file_signature, AbfType::AbfV2));
        println!("episodes {:?}", abf.actual_episodes);
        println!("file_info_size {:?}", abf.file_info_size);
        println!("start date {:?}", abf.file_start_date);
        println!("start time {:?}", abf.file_start_time_ms);
        println!("file type {:?}", abf.file_type);
    }
}
