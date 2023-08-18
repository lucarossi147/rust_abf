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

        // println!("{:?}", data_section.read().into_iter().take(10).collect::<Vec<i16>>());
        let number_of_channels = adc_section.get_channel_count();
        // println!("Channels are {:?}", number_of_channels);
        let data = data_section.read(number_of_channels);

        // println!("I have {:?} data", data.len());
        // for d in &data {
        //     println!("channel: {:?}, {:?}.....{:?}", d.channel, &d.values[0..10], &d.values[d.values.len()-1]);
        // }
        // println!("total data: {:?}", data.iter().map(|d| d.values.len()).sum::<usize>());


        // let dataByteStart = data_section.[0]*BLOCKSIZE;
        // let dataPointCount = _sectionMap.DataSection[2];
        // let channelCount = _sectionMap.ADCSection[2];
        // let dataRate = (1e6 / _protocolSection.fADCSequenceInterval)
        let adc_info = adc_section.get_adc_infos();
        let data_rate = 1e6 / protocol_section.adc_sequence_interval();
        let data_sec_per_point = 1.0 / data_rate;
        let sweep_count = actual_episodes;

        let scaling_factors = (0..number_of_channels).into_iter()
        .map(|_| 1.0 as f32)
        .enumerate()
        .map(|(ch, sf)| (sf, &adc_info[ch]))
        .map(|(sf, ai)| (sf / ai.instrument_scale_factor as f32, ai))
        .map(|(sf, ai)| (sf/ai.signal_gain as f32, ai))
        .map(|(sf, ai)| (sf/ai.adc_programmable_gain as f32, ai))
        .map(|(sf, ai)| if ai.telegraph_enable != 0 {(sf/ai.telegraph_addit_gain as f32, ai)} else {(sf, ai)})
        .map(|(sf, ai)| (sf* protocol_section.adc_range() as f32, ai))
        .map(|(sf, ai)| (sf/ protocol_section.adc_resolution() as f32, ai))
        .map(|(sf, ai)| (sf + ai.instrument_offset as f32, ai))
        .map(|(sf, ai)| (sf - ai.signal_offset as f32, ai))
        .map(|(sf, _)| sf)
        .collect::<Vec<f32>>();

        // for ch in 0..number_of_channels {
        //     let ai =  &adc_info[ch];
        //     println!("{:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?}, {:?},",
        //     ai.instrument_scale_factor as f64,
        //     ai.signal_gain as f64,
        //     ai.adc_programmable_gain as f64,
        //     ai.telegraph_enable,
        //     ai.telegraph_addit_gain as f64,
        //     protocol_section.adc_range() as f64,
        //     protocol_section.adc_resolution() as f64,
        //     ai.instrument_offset as f64,
        //     ai.signal_offset,
        // );
        // }


        println!("{:?},{:?}, ", &scaling_factors, protocol_section.adc_resolution());
        // for i in range(channelCount):
        // scaleFactors[i] /= fInstrumentScaleFactor[i]
        // scaleFactors[i] /= fSignalGain[i]
        // scaleFactors[i] /= fADCProgrammableGain[i]
        // if nTelegraphEnable:
        //     scaleFactors[i] /= fTelegraphAdditGain[i]
        // scaleFactors[i] *= fADCRange
        // scaleFactors[i] /= lADCResolution
        // scaleFactors[i] += fInstrumentOffset[i]
        // scaleFactors[i] -= fSignalOffset[i]

        // protocol_section.print_info();
        // data_section.print_info();

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
        match self.data.get(&channel) {
            Some(values)=> Some(values.into_iter().map(|v| *v as f32 * self.scaling_factors[channel]).collect::<Vec<f32>>()),
            None=> None 
        }
    }

    fn get_file_signature(&self) -> AbfKind {
        self.file_signature
    }
    fn get_channel_count(&self) -> usize {
        self.number_of_channels
    }
}