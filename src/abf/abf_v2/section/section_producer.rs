use memmap::Mmap;
use super::*;

pub struct SectionProducer<'a> {
    mmap: &'a Mmap,
}

impl<'a> SectionProducer<'a> {
    pub fn new(mmap: &'a Mmap)-> Self {
        Self { mmap: &mmap}
    }
    pub fn get_protocol_section(&self) -> Section<ProtocolSectionType> {
        Section::new(&self.mmap, 76, std::marker::PhantomData::<ProtocolSectionType>)
    } 
    pub fn get_adc_section(&self) -> Section<AdcSectionType> {
        Section::new(&self.mmap, 92, std::marker::PhantomData::<AdcSectionType>)
    } 

    pub fn get_dac_section(&self) -> Section<DacSectionType> {
        Section::new(&self.mmap, 108, std::marker::PhantomData::<DacSectionType>)
    } 
    pub fn get_epoch_section(&self) -> Section<EpochSectionType> {
        Section::new(&self.mmap, 124, std::marker::PhantomData::<EpochSectionType>)
    } 
    pub fn get_adc_per_dac_section(&self) -> Section<AdcPerDacSectionType> {
        Section::new(&self.mmap, 140, std::marker::PhantomData::<AdcPerDacSectionType>)
    } 
    pub fn get_epoch_per_dac_section(&self) -> Section<EpochPerDacSectionType> {
        Section::new(&self.mmap, 156, std::marker::PhantomData::<EpochPerDacSectionType>)
    } 
    pub fn get_strings_section(&self) -> Section<StringsSectionType> {
        Section::new(&self.mmap, 220, std::marker::PhantomData::<StringsSectionType>)
    } 
    pub fn get_data_section(&self) -> Section<DataSectionType> {
        Section::new(&self.mmap, 236, std::marker::PhantomData::<DataSectionType>)
    } 
    pub fn get_tag_section(&self) -> Section<TagSectionType> {
        Section::new(&self.mmap, 252, std::marker::PhantomData::<TagSectionType>)
    } 


}
