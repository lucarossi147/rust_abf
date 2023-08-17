use std::collections::HashMap;

struct AbfV1{
    data: HashMap<usize, Vec<i16>>
}


// impl Abf for AbfV1{
//     fn open<T,E>(filepath: &str) -> Result<T, E>{

//     }

//     fn get_data(self, channel: usize) -> Option<&'static Vec<i16>>{
//        self.data.get(&channel)
//     }

//     fn get_file_signature(self, file_signature_str: &str) -> AbfKind;
// }