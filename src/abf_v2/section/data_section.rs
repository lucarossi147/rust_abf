use super::{DataSectionType, Section};
use crate::channel::ChannelValues;
use crate::error::AbfError;
use rayon::prelude::*;
use std::sync::Arc;

/// ABF2 `nDataFormat` value for int16 samples; any other value is float32,
/// matching pyABF's handling of the header field (see issue #8).
const DATA_FORMAT_INT16: u16 = 0;

const SECTION: &str = "data";

fn byte_array_to_i16(ba: &[u8]) -> i16 {
    match ba.try_into() {
        Ok(arr) => i16::from_le_bytes(arr),
        Err(_) => 0,
    }
}

fn byte_array_to_f32(ba: &[u8]) -> f32 {
    match ba.try_into() {
        Ok(arr) => f32::from_le_bytes(arr),
        Err(_) => 0.0,
    }
}

impl Section<'_, DataSectionType> {
    pub fn read(
        &self,
        number_of_channels: usize,
        data_format: u16,
    ) -> Result<Vec<ChannelValues>, AbfError> {
        let expected_width: u32 = if data_format == DATA_FORMAT_INT16 {
            2
        } else {
            4
        };
        if self.byte_count != expected_width {
            return Err(AbfError::InvalidSection {
                section: SECTION,
                reason: format!(
                    "unexpected sample width {} (expected {expected_width})",
                    self.byte_count
                ),
            });
        }
        if number_of_channels == 0 {
            return Ok(Vec::new());
        }

        let total_bytes =
            self.item_count
                .checked_mul(self.byte_count)
                .ok_or(AbfError::InvalidSection {
                    section: SECTION,
                    reason: "section size overflows u32".to_string(),
                })?;
        let to = self
            .block_number
            .checked_add(total_bytes)
            .ok_or(AbfError::InvalidSection {
                section: SECTION,
                reason: "section end overflows u32".to_string(),
            })?;
        let from = usize::try_from(self.block_number).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "section start exceeds addressable range".to_string(),
        })?;
        let to = usize::try_from(to).map_err(|_| AbfError::InvalidSection {
            section: SECTION,
            reason: "section end exceeds addressable range".to_string(),
        })?;
        let byte_count =
            usize::try_from(self.byte_count).map_err(|_| AbfError::InvalidSection {
                section: SECTION,
                reason: "byte_count exceeds addressable range".to_string(),
            })?;

        let bytes = self.reader.read_bytes(SECTION, from, to - from)?;
        let chunks = bytes.par_chunks_exact(byte_count);
        Ok(if data_format == DATA_FORMAT_INT16 {
            split_by_channel(chunks.map(byte_array_to_i16), number_of_channels)
                .into_iter()
                .map(ChannelValues::I16)
                .collect()
        } else {
            split_by_channel(chunks.map(byte_array_to_f32), number_of_channels)
                .into_iter()
                .map(ChannelValues::F32)
                .collect()
        })
    }
}

fn split_by_channel<I, T>(partial_res: I, number_of_channels: usize) -> Vec<Arc<[T]>>
where
    I: IndexedParallelIterator<Item = T> + Clone,
    T: Send,
{
    match number_of_channels {
        1 => vec![partial_res.collect::<Arc<[T]>>()],
        n => {
            let partial_res_with_idxs = partial_res.enumerate().map(|(i, e)| (i % n, e));
            // TODO, the last thing that comes to my mind to speedup even more the program is making the partial_res_with_idxs mutable and remove at every iteration
            // the entries that have been used (if channel 0 is been used, then we can remove every element of that channel and the next iteration will be 1/n faster)
            (0..n)
                .map(|c| {
                    partial_res_with_idxs
                        .clone()
                        .filter_map(|(idx, e)| if idx == c { Some(e) } else { None })
                        .collect()
                })
                .collect()
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn split_by_channel_deinterleaves_a_single_channel() {
        let data = vec![10_i16, 20, 30];
        let result = split_by_channel(data.into_par_iter(), 1);
        assert_eq!(result.len(), 1);
        assert_eq!(&*result[0], &[10, 20, 30]);
    }

    #[test]
    fn split_by_channel_deinterleaves_multiple_channels() {
        // interleaved as ch0, ch1, ch0, ch1, ch0, ch1
        let data = vec![1.0_f32, -1.0, 2.0, -2.0, 3.0, -3.0];
        let result = split_by_channel(data.into_par_iter(), 2);
        assert_eq!(result.len(), 2);
        assert_eq!(&*result[0], &[1.0, 2.0, 3.0]);
        assert_eq!(&*result[1], &[-1.0, -2.0, -3.0]);
    }

    #[test]
    fn byte_array_to_i16_returns_zero_for_short_input_instead_of_panicking() {
        assert_eq!(byte_array_to_i16(&[0x01]), 0);
    }

    #[test]
    fn byte_array_to_f32_returns_zero_for_short_input_instead_of_panicking() {
        assert_eq!(byte_array_to_f32(&[0x01, 0x02]), 0.0);
    }
}
