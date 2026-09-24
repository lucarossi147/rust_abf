mod section;
use super::{Abf, Channel};
use crate::byte_reader::ByteReader;
use crate::error::AbfError;
use crate::AbfKind;
use memmap2::Mmap;
use section::section_producer::SectionProducer;
use std::path::PathBuf;

const HEADER: &str = "header";

impl Abf {
    pub fn from_abf_v2(memmap: Mmap, path: PathBuf) -> Result<Self, AbfError> {
        let abf_kind = AbfKind::AbfV2;
        let reader = ByteReader::new(&memmap);
        let actual_episodes = reader.read_u32(HEADER, 12)?;
        let data_format: u16 = reader.read_u16(HEADER, 30)?;

        // useful sections
        let sec_prod = SectionProducer::new(reader);
        let protocol_section = sec_prod.get_protocol_section()?;
        let adc_section = sec_prod.get_adc_section()?;
        sec_prod.get_dac_section()?;
        let strings_section = sec_prod.get_strings_section()?;
        let data_section = sec_prod.get_data_section()?;

        let number_of_channels = adc_section.get_channel_count();
        let data = data_section.read(number_of_channels, data_format)?;
        let adc_infos = adc_section.get_adc_infos()?;

        let sampling_rate = 1e6 / protocol_section.adc_sequence_interval()?;
        let adc_range = protocol_section.adc_range()?;
        let adc_resolution = protocol_section.adc_resolution()?;
        let sweep_count = actual_episodes;

        let indexed_strings = strings_section.read_indexed_strings()?;
        let sweeps_count = match sweep_count {
            0 | 1 => 1,
            n => n,
        };

        let channels_count =
            u32::try_from(number_of_channels).map_err(|_| AbfError::InvalidSection {
                section: "adc",
                reason: "channel count exceeds u32".to_string(),
            })?;

        let channels: Vec<Channel> = data
            .into_iter()
            .zip(adc_infos.iter())
            .map(|(values, adc_info)| {
                let mut gain = 1.0_f32
                    / adc_info.instrument_scale_factor
                    / adc_info.signal_gain
                    / adc_info.adc_programmable_gain;
                if adc_info.telegraph_enable != 0 {
                    gain /= adc_info.telegraph_addit_gain;
                }
                let gain = gain * adc_range / adc_resolution as f32;
                let offset = adc_info.instrument_offset - adc_info.signal_offset;

                Channel::new(
                    values,
                    indexed_strings
                        .get(adc_info.adc_units_index)
                        .cloned()
                        .unwrap_or_else(|| "nan".to_string()),
                    gain,
                    offset,
                    indexed_strings
                        .get(adc_info.adc_channel_name_index)
                        .cloned()
                        .unwrap_or_else(|| "nan".to_string()),
                    sweeps_count,
                )
            })
            .collect();

        Ok(Self {
            abf_kind,
            channels_count,
            sweeps_count,
            sampling_rate,
            channels,
            path,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn gap_free_mode_always_yields_a_single_sweep() {
        assert_eq!(sweep_count(3, 0), 1);
        assert_eq!(sweep_count(3, 1), 1);
        assert_eq!(sweep_count(3, 7), 1);
    }

    #[test]
    fn other_modes_use_actual_episodes() {
        for op_mode in [1, 2, 4, 5] {
            assert_eq!(sweep_count(op_mode, 0), 1);
            assert_eq!(sweep_count(op_mode, 1), 1);
            assert_eq!(sweep_count(op_mode, 42), 42);
        }
    }
}
