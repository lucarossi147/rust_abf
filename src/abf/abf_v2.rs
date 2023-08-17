use std::collections::HashMap;
use crate::mmap;
use super::abf_kind::AbfKind;
use super::Abf;
use crate::conversion_util as cu;
mod section;

use section::section_producer::SectionProducer; 
pub struct AbfV2{
    pub file_signature: AbfKind,            //  0
    pub file_version_number: Vec<i8>,       //  4
    pub file_info_size: u32,                //  8
    pub actual_episodes: u32,               //  12
    pub file_start_date: u32,               //  16
    pub file_start_time_ms: u32,            //  20
    pub stopwatch_time: u32,                //  24
    pub file_type: u16,                     //  28
    pub data_format: u16,                   //  30
    pub simultaneus_scan: u16,              //  32
    pub crc_enable: u16,                    //  34
    pub file_crc: u32,                      //  38
    pub file_guid: u32,                     //  42
    data: HashMap<usize, Vec<i16>>,
}


// impl Abf for AbfV2{
impl AbfV2{
    fn open(filepath: &str) -> Self {
        let memmap = mmap(filepath).unwrap();
        // let file_signature = match cu::from_bytes_array_to_string(&memmap, 0, 4) {
        //     Ok(v) => get_file_signature(v),
        //     _ => return Err("File is not an abf".to_string()),
        // };    
        let file_version_number = (4..=8).map(|i| memmap[i] as i8).collect();
        let file_info_size = cu::from_byte_array_to_u32(&memmap, 8).unwrap();
        let actual_episodes = cu::from_byte_array_to_u32(&memmap, 12).unwrap();
        let file_start_date = cu::from_byte_array_to_u32(&memmap, 16).unwrap();
        let file_start_time_ms = cu::from_byte_array_to_u32(&memmap, 20).unwrap();
        let stopwatch_time = cu::from_byte_array_to_u32(&memmap, 24).unwrap();
        let file_type = cu::from_byte_array_to_u16(&memmap, 28);
        let data_format: u16 = cu::from_byte_array_to_u16(&memmap, 30);
        let simultaneus_scan: u16 = cu::from_byte_array_to_u16(&memmap, 32);
        let crc_enable: u16 = cu::from_byte_array_to_u16(&memmap, 34);
        let file_crc: u32 = cu::from_byte_array_to_u32(&memmap, 38).unwrap();
        let file_guid: u32= cu::from_byte_array_to_u32(&memmap, 42).unwrap();

        // useful sections
        let sec_prod = SectionProducer::new(&memmap);
        let protocol_section = sec_prod.get_protocol_section();
        let adc_section = sec_prod.get_adc_section();
        let dac_section = sec_prod.get_dac_section();
        let epoch_section = sec_prod.get_epoch_section();
        let adc_per_dac_section = sec_prod.get_adc_per_dac_section();
        let epoch_per_dac_section = sec_prod.get_epoch_per_dac_section();
        let strings_section = sec_prod.get_strings_section();
        let data_section = sec_prod.get_data_section();
        let tag_section = sec_prod.get_tag_section();

        // TODO, create 2 possible abfs, one faster with only useful sections and one with all the possible sections
        // not useful sections
        // let user_list_section = sec_prod.produce_from(172);
        // let stats_region_section = sec_prod.produce_from(188);
        // let math_section = sec_prod.produce_from(204);
        // let scope_section = sec_prod.produce_from(268);
        // let delta_section = sec_prod.produce_from(284);
        // let voice_tag_section = sec_prod.produce_from(300);
        // let synch_array_section = sec_prod.produce_from(316);
        // let annotation_section = sec_prod.produce_from(332);
        // let stats_section = sec_prod.produce_from(348);

        // println!("{:?}", data_section.read().into_iter().take(10).collect::<Vec<i16>>());
        let channels_num = adc_section.get_channel_count();
        // println!("Channels are {:?}", channels_num);
        let data = data_section.read(channels_num);

        // let scale_factors = (0..channels_num).into_iter()
        // .map(|i| i=1)
        // .map(|i| i/)

        // println!("I have {:?} data", data.len());
        // for d in &data {
        //     println!("channel: {:?}, {:?}.....{:?}", d.channel, &d.values[0..10], &d.values[d.values.len()-1]);
        // }
        // println!("total data: {:?}", data.iter().map(|d| d.values.len()).sum::<usize>());
        Self {
            file_signature: AbfKind::AbfV2,
            file_version_number,
            file_info_size,
            actual_episodes,
            file_start_date,
            file_start_time_ms,
            stopwatch_time,
            file_type,
            data_format,
            simultaneus_scan,
            crc_enable,
            file_crc,
            file_guid,
            data,
        }
    }

    // fn get_data(self, channel: usize) -> Option<&'static Vec<i16>>{
    //    self.data.get(&channel)
    // }

    // fn get_file_signature(self, file_signature_str: &str) -> AbfKind;
}