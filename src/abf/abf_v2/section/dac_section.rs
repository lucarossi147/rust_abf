use super::{Section, DacSectionType};
use crate::conversion_util as cu;
pub struct DacSectionInfo{
    channel_name_index: i32,
    channel_units_index: i32,
    file_path_index: i32,
}

impl DacSectionInfo {
    // Getter for channel_name_index
    pub fn get_channel_name_index(&self) -> i32 {
        self.channel_name_index
    }

    // Getter for channel_units_index
    pub fn get_channel_units_index(&self) -> i32 {
        self.channel_units_index
    }

    // Getter for file_path_index
    pub fn get_file_path_index(&self) -> i32 {
        self.file_path_index
    }
}

impl Section<'_, DacSectionType>{
    pub fn get_dacs_info(self) -> Vec<DacSectionInfo> {
        (0..self.item_count)
        .map(|ch|self.block_number + ch * self.byte_count)
        .flat_map(usize::try_from)
        .map(|from|{
            let channel_name_index =  cu::mmap_to_i32(self.mmap, from + 24);
            let channel_units_index =  cu::mmap_to_i32(self.mmap, from + 28);
            let file_path_index =  cu::mmap_to_i32(self.mmap, from + 118);
            
            // let telegraph_instrument_name = get_telegraph_name(telegraph_instrument);
            DacSectionInfo{
                channel_name_index,
                channel_units_index,
                file_path_index,
            }
        }).collect()
    }
}
