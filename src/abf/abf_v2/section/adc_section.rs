use super::{Section, AdcSectionType};
use crate::conversion_util as cu;
pub struct AdcSectionInfo{
    pub adc_num: i16,
    pub telegraph_enable: i16,
    pub telegraph_instrument: i16,
    pub telegraph_addit_gain: f32,
    pub telegraph_filter: f32,
    pub telegraph_membrane_cap: f32,
    pub telegraph_mode: i16,
    pub telegraph_access_resistance: f32,
    pub adc_p_to_l_channel_map: i16,
    pub adc_sampling_seq: i16,
    pub adc_programmable_gain: f32,
    pub adc_display_amplification: f32,
    pub adc_display_offset: f32,
    pub instrument_scale_factor: f32,
    pub instrument_offset: f32,
    pub signal_gain: f32,
    pub signal_offset: f32,
    pub signal_lowpass_filter: f32,
    pub signal_highpass_filter: f32,
    pub lowpass_filter_type: u8,
    pub highpass_filter_type: u8,
    pub post_process_lowpass_filter: f32,
    pub post_process_lowpass_filter_type: i8,
    pub enabled_during_pn: u8,
    pub stats_channel_polarity: i16,
    pub adc_channel_name_index: i32,
    pub adc_units_index: i32,
}

impl<'a> Section<'a, AdcSectionType>{

    pub fn get_adc_infos(self) -> Vec<AdcSectionInfo> {
        (0..self.item_count).into_iter()
        .map(|ch|self.block_number + ch * self.byte_count)
        .flat_map(|from| usize::try_from(from))
        .map(|from|{
            let adc_num = cu::mmap_to_i16(&self.mmap, from);
            let telegraph_enable = cu::mmap_to_i16(&self.mmap, from + 2);
            let telegraph_instrument = cu::mmap_to_i16(&self.mmap, from + 4);
            let telegraph_addit_gain: f32 = cu::mmap_to_f32(&self.mmap, from + 6);
            let telegraph_filter: f32 = cu::mmap_to_f32(&self.mmap, from + 10);
            let telegraph_membrane_cap: f32 = cu::mmap_to_f32(&self.mmap, from + 14);
            let telegraph_mode = cu::mmap_to_i16(&self.mmap, from + 18);
            let telegraph_access_resistance = cu::mmap_to_f32(&self.mmap, from + 20);
            let adc_p_to_l_channel_map = cu::mmap_to_i16(&self.mmap, from + 24);
            let adc_sampling_seq = cu::mmap_to_i16(&self.mmap, from + 26);
            let adc_programmable_gain = cu::mmap_to_f32(&self.mmap, from + 28);
            let adc_display_amplification = cu::mmap_to_f32(&self.mmap, from + 32);
            let adc_display_offset = cu::mmap_to_f32(&self.mmap, from + 36);
            let instrument_scale_factor = cu::mmap_to_f32(&self.mmap, from + 40);
            let instrument_offset = cu::mmap_to_f32(&self.mmap, from + 44);
            let signal_gain = cu::mmap_to_f32(&self.mmap, from + 48);
            let signal_offset = cu::mmap_to_f32(&self.mmap, from + 52);
            let signal_lowpass_filter = cu::mmap_to_f32(&self.mmap, from + 56);
            let signal_highpass_filter = cu::mmap_to_f32(&self.mmap, from + 60);
            let lowpass_filter_type = cu::mmap_to_u8(&self.mmap, from + 64);
            let highpass_filter_type = cu::mmap_to_u8(&self.mmap, from + 65);
            let post_process_lowpass_filter = cu::mmap_to_f32(&self.mmap, from + 66);
            let post_process_lowpass_filter_type = self.mmap[from+70] as i8;
            let enabled_during_pn = cu::mmap_to_u8(&self.mmap, from + 71);
            let stats_channel_polarity = cu::mmap_to_i16(&self.mmap, from + 72);
            let adc_channel_name_index = cu::mmap_to_i32(&self.mmap, from + 74);
            let adc_units_index = cu::mmap_to_i32(&self.mmap, from + 78);
            // let telegraph_instrument_name = get_telegraph_name(telegraph_instrument);
            AdcSectionInfo{
                adc_num,
                telegraph_enable,
                telegraph_instrument,
                telegraph_addit_gain,
                telegraph_filter,
                telegraph_membrane_cap,
                telegraph_mode,
                telegraph_access_resistance,
                adc_p_to_l_channel_map,
                adc_sampling_seq,
                adc_programmable_gain,
                adc_display_amplification,
                adc_display_offset,
                instrument_scale_factor,
                instrument_offset,
                signal_gain,
                signal_offset,
                signal_lowpass_filter,
                signal_highpass_filter,
                lowpass_filter_type,
                highpass_filter_type,
                post_process_lowpass_filter,
                post_process_lowpass_filter_type,
                enabled_during_pn,
                stats_channel_polarity,
                adc_channel_name_index,
                adc_units_index,
            }
        }).collect()
    }


    pub fn get_channel_count(&self)->usize{
        usize::try_from(self.item_count).unwrap()
    }
}
