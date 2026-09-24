use super::{AdcSectionType, Section};
use crate::byte_reader::checked_offset;
use crate::error::AbfError;

// Mirrors the full ABF2 ADC section header (see pyABF's ADCSection); not every
// field is consumed yet, but they are kept for parsing fidelity with the format.
#[allow(dead_code)]
pub struct AdcSectionInfo {
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

const SECTION: &str = "adc";

impl Section<'_, AdcSectionType> {
    pub fn get_adc_infos(&self) -> Result<Vec<AdcSectionInfo>, AbfError> {
        (0..self.item_count)
            .map(|ch| -> Result<AdcSectionInfo, AbfError> {
                let from = self.item_offset(ch)?;
                let field = |extra: usize| checked_offset(from, extra, SECTION);

                let adc_num = self.reader.read_i16(SECTION, field(0)?)?;
                let telegraph_enable = self.reader.read_i16(SECTION, field(2)?)?;
                let telegraph_instrument = self.reader.read_i16(SECTION, field(4)?)?;
                let telegraph_addit_gain = self.reader.read_f32(SECTION, field(6)?)?;
                let telegraph_filter = self.reader.read_f32(SECTION, field(10)?)?;
                let telegraph_membrane_cap = self.reader.read_f32(SECTION, field(14)?)?;
                let telegraph_mode = self.reader.read_i16(SECTION, field(18)?)?;
                let telegraph_access_resistance = self.reader.read_f32(SECTION, field(20)?)?;
                let adc_p_to_l_channel_map = self.reader.read_i16(SECTION, field(24)?)?;
                let adc_sampling_seq = self.reader.read_i16(SECTION, field(26)?)?;
                let adc_programmable_gain = self.reader.read_f32(SECTION, field(28)?)?;
                let adc_display_amplification = self.reader.read_f32(SECTION, field(32)?)?;
                let adc_display_offset = self.reader.read_f32(SECTION, field(36)?)?;
                let instrument_scale_factor = self.reader.read_f32(SECTION, field(40)?)?;
                let instrument_offset = self.reader.read_f32(SECTION, field(44)?)?;
                let signal_gain = self.reader.read_f32(SECTION, field(48)?)?;
                let signal_offset = self.reader.read_f32(SECTION, field(52)?)?;
                let signal_lowpass_filter = self.reader.read_f32(SECTION, field(56)?)?;
                let signal_highpass_filter = self.reader.read_f32(SECTION, field(60)?)?;
                let lowpass_filter_type = self.reader.read_u8(SECTION, field(64)?)?;
                let highpass_filter_type = self.reader.read_u8(SECTION, field(65)?)?;
                let post_process_lowpass_filter = self.reader.read_f32(SECTION, field(66)?)?;
                let post_process_lowpass_filter_type = self.reader.read_i8(SECTION, field(70)?)?;
                let enabled_during_pn = self.reader.read_u8(SECTION, field(71)?)?;
                let stats_channel_polarity = self.reader.read_i16(SECTION, field(72)?)?;
                let adc_channel_name_index = self.reader.read_i32(SECTION, field(74)?)?;
                let adc_units_index = self.reader.read_i32(SECTION, field(78)?)?;

                Ok(AdcSectionInfo {
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
                })
            })
            .collect()
    }

    pub fn get_channel_count(&self) -> usize {
        usize::try_from(self.item_count).unwrap_or(0)
    }
}
