use std::collections::HashMap;
use rayon::prelude::*;
use crate::conversion_util as cu;
use memmap::Mmap;

pub mod section_producer;
pub struct ProtocolSectionType;
pub struct AdcSectionType;
pub struct DacSectionType;
pub struct EpochSectionType;
pub struct AdcPerDacSectionType;
pub struct EpochPerDacSectionType;
pub struct StringsSectionType;
pub struct DataSectionType;
pub struct TagSectionType;

pub struct Section<'a, SectionType>{
    mmap: &'a Mmap,
    block_number: u32,
    byte_count: u32,
    item_count: u32, //todo this may be an i32 
    section_type: std::marker::PhantomData<SectionType>,
}



impl<'a, T> Section<'a, T> {
    fn new(mmap: &'a Mmap, from: usize, section_type:std::marker::PhantomData<T>) -> Section<T> {
        let block_number = 512 * cu::from_byte_array_to_u32(mmap, from).unwrap();
        let byte_count = cu::from_byte_array_to_u32(mmap, from + std::mem::size_of::<u32>()).unwrap();
        let item_count = cu::from_byte_array_to_u32(mmap, from + 2*(std::mem::size_of::<u32>())).unwrap();
        Section { 
            mmap,
            block_number,
            byte_count,
            item_count,
            section_type,
        }
    }

    
}

impl<'a, DataSectionType> Section<'a, DataSectionType>{
    pub fn read(&self, number_of_channels: usize) -> HashMap<usize, Vec<i16>> {
        let from = usize::try_from(self.block_number).unwrap();
        let to = usize::try_from(self.block_number+self.item_count).unwrap();
        let byte_count = usize::try_from(self.byte_count).unwrap();
        let number_of_channels = usize::try_from(number_of_channels).unwrap();
        let partial_res = self.mmap[from..to]
        .par_chunks_exact(byte_count)
        .map(|c|cu::byte_array_to_i16(c));
        match number_of_channels {
            1 => HashMap::from([(0, partial_res.collect::<Vec<i16>>())]),
            n => {
                let partial_res_with_idxs = partial_res.enumerate().map(|(i, e)| (i%n, e));
                // TODO, the last thing that comes to my mind to speedup even more the program is making the partial_res_with_idxs mutable and remove at every iteration 
                // the entries that have been used (if channel 0 is been used, then we can remove every element of that channel and the next iteration will be 1/n faster)
                let tuples_to_feed = (0..n).into_iter().map(|c| {
                    (c, partial_res_with_idxs.clone().filter_map(|(idx, e)| if idx == c {Some(e)} else {None}).collect())
                });
                HashMap::from_iter(tuples_to_feed)
            }
        }
    } 
}

impl<'a, AdcSectionType> Section<'a, AdcSectionType>{
    pub fn get_channel_count(self)->u32{
        self.item_count
    }
}
