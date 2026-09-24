use super::{DataSectionType, Section};
use crate::channel::ChannelValues;
use crate::conversion_util as cu;
use rayon::prelude::*;
use std::sync::Arc;

/// ABF2 `nDataFormat` value for int16 samples; any other value is float32,
/// matching pyABF's handling of the header field (see issue #8).
const DATA_FORMAT_INT16: u16 = 0;

impl Section<'_, DataSectionType> {
    pub fn read(&self, number_of_channels: usize, data_format: u16) -> Vec<ChannelValues> {
        let from = usize::try_from(self.block_number).unwrap();
        let to = usize::try_from(self.block_number + (self.item_count * self.byte_count)).unwrap();
        let byte_count = usize::try_from(self.byte_count).unwrap();
        let chunks = self.mmap[from..to].par_chunks_exact(byte_count);
        if data_format == DATA_FORMAT_INT16 {
            split_by_channel(chunks.map(cu::byte_array_to_i16), number_of_channels)
                .into_iter()
                .map(ChannelValues::I16)
                .collect()
        } else {
            split_by_channel(chunks.map(cu::byte_array_to_f32), number_of_channels)
                .into_iter()
                .map(ChannelValues::F32)
                .collect()
        }
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
}
