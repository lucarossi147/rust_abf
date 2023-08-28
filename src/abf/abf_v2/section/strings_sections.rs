pub mod indexed_strings;

use self::indexed_strings::IndexedStrings;
use super::{Section, StringsSectionType};

impl Section<'_, StringsSectionType>{
    pub fn read_indexed_strings(&self) -> IndexedStrings{
        let from = self.block_number as usize;
        let to = from + self.byte_count as usize;
        let res:Vec<String> = self.mmap[from..to]
        .split(|c| c == &0b00)
        // .map(|str_i| if str_i.len()>1 { &str_i[1..]} else {&str_i[..]})
        .map(String::from_utf8_lossy)
        .filter_map(|str_i| if !str_i.is_empty()  {Some(str_i)} else {None})
        .map(|str_i| str_i.trim().to_string())
        .collect();
        IndexedStrings::new(res[3..].to_vec())
    }
}