use crate::AbfKind;
use super::{Abf, FileKind, Channel};
use crate::conversion_util as cu;
mod section;
use memmap::Mmap;
use section::section_producer::SectionProducer; 

impl Abf {
    pub fn from_abf_v2(memmap:Mmap, abf_kind: AbfKind) -> Self {
        // let file_version_number = (4..8).map(|i| memmap[i] as i8).collect();
        let file_info_size = cu::from_byte_array_to_u32(&memmap, 8).unwrap();
        let actual_episodes = cu::from_byte_array_to_u32(&memmap, 12).unwrap();
        let file_start_date = cu::from_byte_array_to_u32(&memmap, 16).unwrap();
        let file_start_time_ms = cu::from_byte_array_to_u32(&memmap, 20).unwrap();
        let stopwatch_time = cu::from_byte_array_to_u32(&memmap, 24).unwrap();
        let file_type = cu::from_byte_array_to_u16(&memmap, 28);
        let data_format: u16 = cu::from_byte_array_to_u16(&memmap, 30);
        let simultaneus_scan: u16 = cu::from_byte_array_to_u16(&memmap, 32);
        let crc_enable: u16 = cu::from_byte_array_to_u16(&memmap, 34);
        let file_crc: u32 = cu::from_byte_array_to_u32(&memmap, 36).unwrap();
        let file_guid: u32= cu::from_byte_array_to_u32(&memmap, 40).unwrap();
        let _unknown1 = cu::from_byte_array_to_u32(&memmap, 44).unwrap();
        let _unknown2 = cu::from_byte_array_to_u32(&memmap, 48).unwrap();
        let _unknown3 = cu::from_byte_array_to_u32(&memmap, 52).unwrap();
        let creator_version = cu::from_byte_array_to_u32(&memmap, 56).unwrap();
        let creator_name_index = cu::from_byte_array_to_u32(&memmap, 60).unwrap();
        let modifier_version = cu::from_byte_array_to_u32(&memmap, 64).unwrap();
        let modifier_name_index = cu::from_byte_array_to_u32(&memmap, 68).unwrap();
        let protocol_path_index = cu::from_byte_array_to_u32(&memmap, 72).unwrap();        

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

        let number_of_channels = adc_section.get_channel_count();
        let data = data_section.read(number_of_channels);

        // let dataRate = (1e6 / _protocolSection.fADCSequenceInterval)
        let adc_infos = adc_section.get_adc_infos();
        let dacs_info = dac_section.get_dacs_info();
        let sampling_rate = 1e6 / protocol_section.adc_sequence_interval();
        let sweep_count = actual_episodes;

        let gains: Vec<f32> = (0..number_of_channels)
        .map(|_| 1.0_f32)
        .enumerate()
        .map(|(ch, sf)| (sf, &adc_infos[ch]))
        .map(|(sf, ai)| (sf / ai.instrument_scale_factor, ai))
        .map(|(sf, ai)| (sf/ai.signal_gain, ai))
        .map(|(sf, ai)| (sf/ai.adc_programmable_gain, ai))
        .map(|(sf, ai)| if ai.telegraph_enable != 0 {(sf/ai.telegraph_addit_gain, ai)} else {(sf, ai)})
        .map(|(sf, ai)| (sf* protocol_section.adc_range(), ai))
        .map(|(sf, ai)| (sf/ protocol_section.adc_resolution() as f32))
        .collect();

        let offsets: Vec<f32> = (0..number_of_channels)
        .map(|_| 0.0_f32)
        .enumerate()
        .map(|(ch, sf)| (sf, &adc_infos[ch]))
        .map(|(sf, ai)| (sf + ai.instrument_offset, ai))
        .map(|(sf, ai)| (sf - ai.signal_offset))
        .collect();
        // let strs = strings_section.read_indexed_strings();
        let strs = strings_section.read_indexed_strings();
        // strs.iter().for_each(|s| println!("{}",s));

        // println!("always blank for me, not sure what it is for: {}", &strs[modifier_name_index as usize]);
        // println!("name of program used to create the ABF: {}", &strs[creator_name_index as usize]);
        // println!("path to the protocol used: {}", &strs[protocol_path_index as usize]);
        // // println!("file commend defined in the waveform editor: {}", &strs[ as usize]);
        // for adc_info in adc_infos{
        //     println!("name/label of the ADC: {}", &strs[adc_info.adc_channel_name_index as usize]);
        //     println!("units of the ADC: {}", &strs[adc_info.adc_units_index as usize]);            
        // }
        // for dac_info in dacs_info{
        //     println!("name/label of the DAC: {}", &strs[dac_info.get_channel_name_index() as usize]);
        //     println!("units of the DAC: {}", &strs[dac_info.get_channel_units_index() as usize]);            
        //     println!("path of custom stimulus waveform: {}", &strs[dac_info.get_file_path_index() as usize]);            
        // }
        Self {
            abf_kind,
            file_kind: if data_format == 0 {FileKind::I16} else {FileKind::F32},
            channels_count: number_of_channels as u32,
            sweeps_count: sweep_count,
            sampling_rate,
            channels: (0..number_of_channels)
            .map(|ch|{
                let data = data.get(&ch).unwrap();
                (ch as u32, Channel::new(data.to_owned(), "uom".to_string(), gains[ch], offsets[ch], "label".to_string()))
            })
            .collect(),
        }
    }
}




// pub struct AbfV2{
//     file_signature: AbfKind,            //  0
//     file_version_number: Vec<i8>,       //  4
//     file_info_size: u32,                //  8
//     actual_episodes: u32,               //  12
//     file_start_date: u32,               //  16
//     file_start_time_ms: u32,            //  20
//     stopwatch_time: u32,                //  24
//     file_type: u16,                     //  28
//     data_format: u16,                   //  30
//     simultaneus_scan: u16,              //  32
//     crc_enable: u16,                    //  34
//     file_crc: u32,                      //  38
//     file_guid: u32,                     //  42
//     data: HashMap<usize, Vec<i16>>,
//     abf_kind: AbfKind,
//     number_of_channels: usize,
//     scaling_factors: Vec<f32>,
// }

// impl AbfV2 {
//     pub fn new(memmap:Mmap, abf_kind: AbfKind) -> Self {
//         let file_version_number = (4..8).map(|i| memmap[i] as i8).collect();
//         let file_info_size = cu::from_byte_array_to_u32(&memmap, 8).unwrap();
//         let actual_episodes = cu::from_byte_array_to_u32(&memmap, 12).unwrap();
//         let file_start_date = cu::from_byte_array_to_u32(&memmap, 16).unwrap();
//         let file_start_time_ms = cu::from_byte_array_to_u32(&memmap, 20).unwrap();
//         let stopwatch_time = cu::from_byte_array_to_u32(&memmap, 24).unwrap();
//         let file_type = cu::from_byte_array_to_u16(&memmap, 28);
//         let data_format: u16 = cu::from_byte_array_to_u16(&memmap, 30);
//         let simultaneus_scan: u16 = cu::from_byte_array_to_u16(&memmap, 32);
//         let crc_enable: u16 = cu::from_byte_array_to_u16(&memmap, 34);
//         let file_crc: u32 = cu::from_byte_array_to_u32(&memmap, 36).unwrap();
//         let file_guid: u32= cu::from_byte_array_to_u32(&memmap, 40).unwrap();
//         let _unknown1 = cu::from_byte_array_to_u32(&memmap, 44).unwrap();
//         let _unknown2 = cu::from_byte_array_to_u32(&memmap, 48).unwrap();
//         let _unknown3 = cu::from_byte_array_to_u32(&memmap, 52).unwrap();
//         let creator_version = cu::from_byte_array_to_u32(&memmap, 56).unwrap();
//         let creator_name_index = cu::from_byte_array_to_u32(&memmap, 60).unwrap();
//         let modifier_version = cu::from_byte_array_to_u32(&memmap, 64).unwrap();
//         let modifier_name_index = cu::from_byte_array_to_u32(&memmap, 68).unwrap();
//         let protocol_path_index = cu::from_byte_array_to_u32(&memmap, 72).unwrap();        

//         // useful sections
//         let sec_prod = SectionProducer::new(&memmap);
//         let protocol_section = sec_prod.get_protocol_section();
//         let adc_section = sec_prod.get_adc_section();
//         let dac_section = sec_prod.get_dac_section();
//         let epoch_section = sec_prod.get_epoch_section();
//         let adc_per_dac_section = sec_prod.get_adc_per_dac_section();
//         let epoch_per_dac_section = sec_prod.get_epoch_per_dac_section();
//         let strings_section = sec_prod.get_strings_section();
//         let data_section = sec_prod.get_data_section();
//         let tag_section = sec_prod.get_tag_section();

//         // TODO, create 2 possible abfs, one faster with only useful sections and one with all the possible sections
//         // not useful sections
//         // let user_list_section = sec_prod.produce_from(172);
//         // let stats_region_section = sec_prod.produce_from(188);
//         // let math_section = sec_prod.produce_from(204);
//         // let scope_section = sec_prod.produce_from(268);
//         // let delta_section = sec_prod.produce_from(284);
//         // let voice_tag_section = sec_prod.produce_from(300);
//         // let synch_array_section = sec_prod.produce_from(316);
//         // let annotation_section = sec_prod.produce_from(332);
//         // let stats_section = sec_prod.produce_from(348);

//         let number_of_channels = adc_section.get_channel_count();
//         let data = data_section.read(number_of_channels);

//         // let dataRate = (1e6 / _protocolSection.fADCSequenceInterval)
//         let adc_infos = adc_section.get_adc_infos();
//         let dacs_info = dac_section.get_dacs_info();
//         let sampling_rate = 1e6 / protocol_section.adc_sequence_interval();
//         let sweep_count = actual_episodes;
//         println!("{:?}",sweep_count);

//         let scaling_factors = (0..number_of_channels)
//         .map(|_| 1.0_f32)
//         .enumerate()
//         .map(|(ch, sf)| (sf, &adc_infos[ch]))
//         .map(|(sf, ai)| (sf / ai.instrument_scale_factor, ai))
//         .map(|(sf, ai)| (sf/ai.signal_gain, ai))
//         .map(|(sf, ai)| (sf/ai.adc_programmable_gain, ai))
//         .map(|(sf, ai)| if ai.telegraph_enable != 0 {(sf/ai.telegraph_addit_gain, ai)} else {(sf, ai)})
//         .map(|(sf, ai)| (sf* protocol_section.adc_range(), ai))
//         .map(|(sf, ai)| (sf/ protocol_section.adc_resolution() as f32, ai))
//         .map(|(sf, ai)| (sf + ai.instrument_offset, ai))
//         .map(|(sf, ai)| (sf - ai.signal_offset, ai))
//         .map(|(sf, _)| sf)
//         .collect();

//         // let strs = strings_section.read_indexed_strings();
//         let strs = strings_section.read_indexed_strings();
//         // strs.iter().for_each(|s| println!("{}",s));

//         // println!("always blank for me, not sure what it is for: {}", &strs[modifier_name_index as usize]);
//         // println!("name of program used to create the ABF: {}", &strs[creator_name_index as usize]);
//         // println!("path to the protocol used: {}", &strs[protocol_path_index as usize]);
//         // // println!("file commend defined in the waveform editor: {}", &strs[ as usize]);
//         // for adc_info in adc_infos{
//         //     println!("name/label of the ADC: {}", &strs[adc_info.adc_channel_name_index as usize]);
//         //     println!("units of the ADC: {}", &strs[adc_info.adc_units_index as usize]);            
//         // }
//         // for dac_info in dacs_info{
//         //     println!("name/label of the DAC: {}", &strs[dac_info.get_channel_name_index() as usize]);
//         //     println!("units of the DAC: {}", &strs[dac_info.get_channel_units_index() as usize]);            
//         //     println!("path of custom stimulus waveform: {}", &strs[dac_info.get_file_path_index() as usize]);            
//         // }
//         Self {
//             file_signature: AbfKind::AbfV2,
//             file_version_number,
//             file_info_size,
//             actual_episodes,
//             file_start_date,
//             file_start_time_ms,
//             stopwatch_time,
//             file_type,
//             data_format,
//             simultaneus_scan,
//             crc_enable,
//             file_crc,
//             file_guid,
//             data,
//             abf_kind,
//             number_of_channels,
//             scaling_factors,
//         }
//     }
// }

// impl Abf for AbfV2{

//     fn get_data(&self, channel: usize) -> Option<Vec<f32>> {
//         let data = self.data
//         .get(&channel)
//         .map(|values| values.iter().map(|v| *v as f32));
//         // data in int, needs to be multiplied for the scaling factors
//         if self.data_format == 0 {
//             data.map(|values| values.map(|v| v * self.scaling_factors[channel]).collect())
//         } else {
//             data.map(|values| values.collect())
//         }
//     }

//     fn get_file_signature(&self) -> AbfKind {
//         self.file_signature
//     }
//     fn get_channel_count(&self) -> usize {
//         self.number_of_channels
//     }
// }