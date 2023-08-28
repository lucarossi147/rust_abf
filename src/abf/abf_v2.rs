mod section;

use crate::AbfKind;
use super::{Abf, Channel};
use super::channel::FileKind;
use crate::conversion_util as cu;
use memmap2::Mmap;
use section::section_producer::SectionProducer; 

impl Abf {
    pub fn from_abf_v2(memmap:Mmap, abf_kind: AbfKind) -> Self {
        // let file_version_number = (4..8).map(|i| memmap[i] as i8).collect();
        // let file_info_size = cu::from_byte_array_to_u32(&memmap, 8).unwrap();
        let actual_episodes = cu::from_byte_array_to_u32(&memmap, 12).unwrap();
        // let file_start_date = cu::from_byte_array_to_u32(&memmap, 16).unwrap();
        // let file_start_time_ms = cu::from_byte_array_to_u32(&memmap, 20).unwrap();
        // let stopwatch_time = cu::from_byte_array_to_u32(&memmap, 24).unwrap();
        // let file_type = cu::from_byte_array_to_u16(&memmap, 28);
        let data_format: u16 = cu::from_byte_array_to_u16(&memmap, 30);
        // let simultaneus_scan: u16 = cu::from_byte_array_to_u16(&memmap, 32);
        // let crc_enable: u16 = cu::from_byte_array_to_u16(&memmap, 34);
        // let file_crc: u32 = cu::from_byte_array_to_u32(&memmap, 36).unwrap();
        // let file_guid: u32= cu::from_byte_array_to_u32(&memmap, 40).unwrap();
        let _unknown1 = cu::from_byte_array_to_u32(&memmap, 44).unwrap();
        let _unknown2 = cu::from_byte_array_to_u32(&memmap, 48).unwrap();
        let _unknown3 = cu::from_byte_array_to_u32(&memmap, 52).unwrap();
        // let creator_version = cu::from_byte_array_to_u32(&memmap, 56).unwrap();
        // let creator_name_index = cu::from_byte_array_to_u32(&memmap, 60).unwrap();
        // let modifier_version = cu::from_byte_array_to_u32(&memmap, 64).unwrap();
        // let modifier_name_index = cu::from_byte_array_to_u32(&memmap, 68).unwrap();
        // let protocol_path_index = cu::from_byte_array_to_u32(&memmap, 72).unwrap();        

        // useful sections
        let sec_prod = SectionProducer::new(&memmap);
        let protocol_section = sec_prod.get_protocol_section();
        let adc_section = sec_prod.get_adc_section();
        let _dac_section = sec_prod.get_dac_section();
        // let epoch_section = sec_prod.get_epoch_section();
        // let adc_per_dac_section = sec_prod.get_adc_per_dac_section();
        // let epoch_per_dac_section = sec_prod.get_epoch_per_dac_section();
        let strings_section = sec_prod.get_strings_section();
        let data_section = sec_prod.get_data_section();
        // let tag_section = sec_prod.get_tag_section();

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
        // let dacs_info = dac_section.get_dacs_info();
        let sampling_rate = 1e6 / protocol_section.adc_sequence_interval();
        let sweep_count = actual_episodes;

        let gains: Vec<f32> = (0..number_of_channels)
        .map(|_| 1.0_f32)
        .enumerate()
        .map(|(ch, sf)| (sf, &adc_infos[ch]))
        .map(|(sf, ai)| (sf / ai.instrument_scale_factor, ai))
        .map(|(sf, ai)| (sf/ai.signal_gain, ai))
        .map(|(sf, ai)| (sf/ai.adc_programmable_gain, ai))
        .map(|(sf, ai)| if ai.telegraph_enable != 0 {sf/ai.telegraph_addit_gain} else {sf})
        .map(|sf| (sf* protocol_section.adc_range()))
        .map(|sf| (sf/ protocol_section.adc_resolution() as f32))
        .collect();

        let offsets: Vec<f32> = (0..number_of_channels)
        .map(|_| 0.0_f32)
        .enumerate()
        .map(|(ch, sf)| (sf, &adc_infos[ch]))
        .map(|(sf, ai)| (sf + ai.instrument_offset, ai))
        .map(|(sf, ai)| (sf - ai.signal_offset))
        .collect();
        let indexed_strings = strings_section.read_indexed_strings();
        let sweeps_count = match sweep_count {
            0 | 1 => 1,
            n => n,
        };
        let file_kind = if data_format == 0 {FileKind::I16} else {FileKind::F32};
        Self {
            abf_kind,
            channels_count: number_of_channels as u32,
            sweeps_count,
            sampling_rate,
            channels: (0..number_of_channels)
            .map(|ch|{
                let data = data.get(&ch).unwrap();
                (
                    ch as u32, 
                    Channel::new(
                        data.to_owned(), 
                        indexed_strings.get(adc_infos[ch].adc_units_index).unwrap_or(&"nan".to_string()).clone(), 
                        gains[ch], 
                        offsets[ch],
                        indexed_strings.get(adc_infos[ch].adc_channel_name_index).unwrap_or(&"nan".to_string()).clone(),
                        sweeps_count,
                        file_kind,
                    )
                )
            })
            .collect(),
        }
    }
}