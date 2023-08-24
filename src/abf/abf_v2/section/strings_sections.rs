use super::{Section, StringsSectionType};

impl Section<'_, StringsSectionType>{

    pub fn read_strings_raw(&self)->Vec<Vec<u8>>{
        (0_usize..self.item_count as usize)
        .map(|str_i| self.mmap[(self.block_number as usize + (str_i*self.byte_count as usize))..self.byte_count as usize].to_vec())
        .collect()
    }

    pub fn read_indexed_strings(&self)->Vec<String>{
        let from = self.block_number as usize;
        let to = from + self.byte_count as usize;
        let res:Vec<String> = self.mmap[from..to]
        .split(|c| c == &0b00)
        // .map(|str_i| if str_i.len()>1 { &str_i[1..]} else {&str_i[..]})
        .map(String::from_utf8_lossy)
        .filter_map(|str_i| if !str_i.is_empty()  {Some(str_i)} else {None})
        .map(|str_i| str_i.trim().to_string())
        .collect();
        res[3..].to_vec()
    }

    pub fn read_strings(&self) -> Vec<String> {
        (0_usize..self.item_count as usize)
        .map(|str_i| {
            let from = self.block_number as usize + (str_i*self.byte_count as usize);
            let to = from + self.byte_count as usize;
            let s = String::from_utf8_lossy(&self.mmap[from..to]);
            s.to_string()
        })
        .collect()
    }
}