use crate::conversion_util as cu;
use memmap2::Mmap;

pub mod section_producer;
pub mod adc_section;
pub mod protocol_section;
pub mod data_section;
pub mod strings_sections;
pub mod dac_section;

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

    pub fn print_info(self){
        println!("section type: {:?}, block number: {:?}, byte count: {:?}, item count: {:?}", self.section_type, self.block_number, self.byte_count, self.item_count);
    }
}