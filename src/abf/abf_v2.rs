use std::collections::HashMap;
use crate::AbfKind;
use super::Abf;
use crate::conversion_util as cu;
mod section;
use memmap::Mmap;
use section::section_producer::SectionProducer; 

pub struct AbfV2{
    file_signature: AbfKind,            //  0
    file_version_number: Vec<i8>,       //  4
    file_info_size: u32,                //  8
    actual_episodes: u32,               //  12
    file_start_date: u32,               //  16
    file_start_time_ms: u32,            //  20
    stopwatch_time: u32,                //  24
    file_type: u16,                     //  28
    data_format: u16,                   //  30
    simultaneus_scan: u16,              //  32
    crc_enable: u16,                    //  34
    file_crc: u32,                      //  38
    file_guid: u32,                     //  42
    data: HashMap<usize, Vec<i16>>,
    abf_kind: AbfKind,
    number_of_channels: usize,
    scaling_factors: Vec<f32>,
}

impl AbfV2 {
    pub fn new(memmap:Mmap, abf_kind: AbfKind) -> Self {
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

        let number_of_channels = adc_section.get_channel_count();
        let data = data_section.read(number_of_channels);

        // let dataByteStart = data_section.[0]*BLOCKSIZE;
        // let dataPointCount = _sectionMap.DataSection[2];
        // let channelCount = _sectionMap.ADCSection[2];
        // let dataRate = (1e6 / _protocolSection.fADCSequenceInterval)
        let adc_info = adc_section.get_adc_infos();
        let data_rate = 1e6 / protocol_section.adc_sequence_interval();
        let data_sec_per_point = 1.0 / data_rate;
        let sweep_count = actual_episodes;

        let scaling_factors = (0..number_of_channels)
        .map(|_| 1.0_f32)
        .enumerate()
        .map(|(ch, sf)| (sf, &adc_info[ch]))
        .map(|(sf, ai)| (sf / ai.instrument_scale_factor, ai))
        .map(|(sf, ai)| (sf/ai.signal_gain, ai))
        .map(|(sf, ai)| (sf/ai.adc_programmable_gain, ai))
        .map(|(sf, ai)| if ai.telegraph_enable != 0 {(sf/ai.telegraph_addit_gain, ai)} else {(sf, ai)})
        .map(|(sf, ai)| (sf* protocol_section.adc_range(), ai))
        .map(|(sf, ai)| (sf/ protocol_section.adc_resolution() as f32, ai))
        .map(|(sf, ai)| (sf + ai.instrument_offset, ai))
        .map(|(sf, ai)| (sf - ai.signal_offset, ai))
        .map(|(sf, _)| sf)
        .collect::<Vec<f32>>();

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
            abf_kind,
            number_of_channels,
            scaling_factors,
        }
    }
}

impl Abf for AbfV2{

    fn get_data(&self, channel: usize) -> Option<Vec<f32>> {
        self.data.get(&channel).map(|values| values.iter().map(|v| *v as f32 * self.scaling_factors[channel]).collect::<Vec<f32>>())
    }

    fn get_file_signature(&self) -> AbfKind {
        self.file_signature
    }
    fn get_channel_count(&self) -> usize {
        self.number_of_channels
    }
}