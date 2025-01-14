use std::{collections::HashMap, sync::Arc};
use rayon::prelude::*;
use super::{Section, DataSectionType};
use crate::conversion_util as cu;

impl Section<'_, DataSectionType>{
    pub fn read(&self, number_of_channels: usize) -> HashMap<usize, Arc<[i16]>> {
        let from = usize::try_from(self.block_number).unwrap();
        let to = usize::try_from(self.block_number+(self.item_count*self.byte_count)).unwrap();
        let byte_count = usize::try_from(self.byte_count).unwrap();
        let partial_res = self.mmap[from..to]
        .par_chunks_exact(byte_count)
        .map(cu::byte_array_to_i16);
        match number_of_channels {
            1 => HashMap::from([(0, partial_res.collect::<Arc<[i16]>>())]),
            n => {
                let partial_res_with_idxs = partial_res.enumerate().map(|(i, e)| (i%n, e));
                // TODO, the last thing that comes to my mind to speedup even more the program is making the partial_res_with_idxs mutable and remove at every iteration 
                // the entries that have been used (if channel 0 is been used, then we can remove every element of that channel and the next iteration will be 1/n faster)
                let tuples_to_feed = (0..n)
                .map(|c| {
                    (c, partial_res_with_idxs
                        .clone()
                        .filter_map(|(idx, e)| if idx == c {Some(e)} else {None})
                        .collect())
                });
                HashMap::from_iter(tuples_to_feed)
            }
        }
    } 
}