use std::fs::File;
use std::str;
use memmap::Mmap;
use byteorder::{LittleEndian, ReadBytesExt};
use rayon::prelude::*;

struct SectionProducer {
    mmap: Mmap,
}

impl SectionProducer {
    pub fn produce_from(&self, from: usize) -> Section {
        Section::new(&self.mmap, from)
    } 
}

#[derive(Debug, Clone)]
struct Data {
    pub channel: usize,
    pub values: Vec<i16>
}
impl Data {
    pub fn new(channel: usize, values:Vec<i16>)->Self{
        Self { channel, values }
    }
}

struct Section<'a>{
    mmap: &'a Mmap,
    pub block_number: u32,
    pub byte_count: u32,
    pub item_count: u32, //todo this may be an i32 
}

impl<'a> Section<'a> {
    pub fn new(mmap: &'a Mmap, from: usize) -> Self {
        let block_number = 512 * from_byte_array_to_u32(mmap, from).unwrap();
        let byte_count = from_byte_array_to_u32(mmap, from + std::mem::size_of::<u32>()).unwrap();
        let item_count = from_byte_array_to_u32(&mmap, from + 2*(std::mem::size_of::<u32>())).unwrap();
        Section { 
            mmap,
            block_number,
            byte_count,
            item_count,
        }
    }

    pub fn read(&self, number_of_channels: u32) -> Vec<Data> {
        let from = usize::try_from(self.block_number).unwrap();
        let to = usize::try_from(self.block_number+self.item_count).unwrap();
        let byte_count = usize::try_from(self.byte_count).unwrap();
        let number_of_channels = usize::try_from(number_of_channels).unwrap();
        let partial_res = self.mmap[from..to]
        .par_chunks_exact(byte_count)
        .map(|c|byte_array_to_i16(c))
        .collect::<Vec<i16>>();
        println!("{:?}", &partial_res[partial_res.len()-10..]);
        match number_of_channels {
            1 => vec![Data::new(0, partial_res)],
            n => {
                let result_data: Vec<Data> = partial_res
                .par_iter()
                .enumerate()
                .fold(|| vec![Data { channel: 0, values: Vec::new() }; n], |mut acc, (index, &value)| {
                    let channel_index = index % n;
                    acc[channel_index].channel = channel_index;
                    acc[channel_index].values.push(value);
                    acc
                })
                .reduce_with(|acc1, acc2| {
                    acc1.into_iter()
                    .zip(acc2)
                    .map(|(mut data1,mut data2)|{
                        data1.values.append(&mut data2.values);
                        data1
                    })
                    .collect()
                })
                .unwrap();
                result_data
            }
        }
    } 
}

// TODO this will become an Abf Header
// TODO the Abf Header will be an enum, of either abf_v1 or abf_v2 
pub struct Abf{
    pub file_signature: AbfType,            //  0
    pub file_version_number: Vec<i8>,       //  4
    pub file_info_size: u32,                //  8
    pub actual_episodes: u32,               //  12
    pub file_start_date: u32,               //  16
    pub file_start_time_ms: u32,            //  20
    pub stopwatch_time: u32,                //  24
    pub file_type: u16,                     //  28
    pub data_format: u16,                   //  30
    pub simultaneus_scan: u16,              //  32
    pub crc_enable: u16,                    //  34
    pub file_crc: u32,                      //  38
    pub file_guid: u32,                     //  42
    data: Vec<Data>,
}

impl Abf {
    pub fn new(path: &str) -> Self {
        let memmap = mmap(path).unwrap();
        let file_signature = match from_bytes_array_to_string(&memmap, 0, 4) {
            Ok(v) => get_file_signature(v),
            _ => AbfType::Invalid,
        };    
        let file_version_number = (4..=8).map(|i| memmap[i] as i8).collect();
        let file_info_size = from_byte_array_to_u32(&memmap, 8).unwrap();
        let actual_episodes = from_byte_array_to_u32(&memmap, 12).unwrap();
        let file_start_date = from_byte_array_to_u32(&memmap, 16).unwrap();
        let file_start_time_ms = from_byte_array_to_u32(&memmap, 20).unwrap();
        let stopwatch_time = from_byte_array_to_u32(&memmap, 24).unwrap();
        let file_type = from_byte_array_to_u16(&memmap, 28);
        let data_format: u16 = from_byte_array_to_u16(&memmap, 30);
        let simultaneus_scan: u16 = from_byte_array_to_u16(&memmap, 32);
        let crc_enable: u16 = from_byte_array_to_u16(&memmap, 34);
        let file_crc: u32 = from_byte_array_to_u32(&memmap, 38).unwrap();
        let file_guid: u32= from_byte_array_to_u32(&memmap, 42).unwrap();

        // useful sections
        let sec_prod = SectionProducer{mmap: memmap};
        let protocol_section = sec_prod.produce_from(76);
        let adc_section = sec_prod.produce_from(92);
        let dac_section = sec_prod.produce_from(108);
        let epoch_section = sec_prod.produce_from(124);
        let adc_per_dac_section = sec_prod.produce_from(140);
        let epoch_per_dac_section = sec_prod.produce_from(156);
        let strings_section = sec_prod.produce_from(220);
        let data_section = sec_prod.produce_from(236);
        let tag_section = sec_prod.produce_from(252);

        // TODO, create 2 possible abfs, one faster with only useful sections and one with all the possible sections
        // not useful sections
        let user_list_section = sec_prod.produce_from(172);
        let stats_region_section = sec_prod.produce_from(188);
        let math_section = sec_prod.produce_from(204);
        let scope_section = sec_prod.produce_from(268);
        let delta_section = sec_prod.produce_from(284);
        let voice_tag_section = sec_prod.produce_from(300);
        let synch_array_section = sec_prod.produce_from(316);
        let annotation_section = sec_prod.produce_from(332);
        let stats_section = sec_prod.produce_from(348);

        // println!("{:?}", data_section.read().into_iter().take(10).collect::<Vec<i16>>());
        let channels_num = match file_signature {
            AbfType::AbfV1 => todo!(),
            AbfType::AbfV2 => adc_section.item_count,
            AbfType::Invalid => panic!("An invalid abf does not have a number of channels")
        };
        println!("Channels are {:?}", channels_num);
        let data = data_section.read(channels_num);

        // let scale_factors = (0..channels_num).into_iter()
        // .map(|i| i=1)
        // .map(|i| i/)

        println!("I have {:?} data", data.len());
        for d in &data {
            println!("channel: {:?}, {:?}.....{:?}", d.channel, &d.values[0..10], &d.values[d.values.len()-1]);
        }
        println!("total data: {:?}", data.iter().map(|d| d.values.len()).sum::<usize>());
        Abf {
            file_signature,
            file_version_number,
            file_info_size,
            actual_episodes,
            file_start_date,
            file_start_time_ms,
            stopwatch_time,
            file_type,
            data_format,
            simultaneus_scan,
            crc_enable,
            file_crc,
            file_guid,
            data,
        }
    }
}

pub enum AbfType {
    AbfV1,
    AbfV2,
    Invalid,
}

fn mmap(path: &str) -> Result<Mmap, std::io::Error> {
    Ok( unsafe {
         Mmap::map(&File::open(path)?)? 
        }
    )
}

fn from_bytes_array_to_string(mmap: &Mmap, from: usize, len: usize) -> Result<&str, str::Utf8Error>{
    str::from_utf8(&mmap[from..from+len])
}

fn from_byte_array_to_u32(mmap: &Mmap, from: usize) -> Result<u32, ()> {
    // Extract the relevant bytes (4 bytes) from the mmap slice
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<u32>()];
    // Convert the bytes to a u32 value in little-endian order
    if let Ok(u32) = bytes_slice.read_u32::<LittleEndian>(){
        Ok(u32)
    } else {
        Err(())
    }
}

fn from_byte_array_to_i32(mmap: &Mmap, from: usize) -> Result<i32, ()> {
    // Extract the relevant bytes (4 bytes) from the mmap slice
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<i32>()];
    // Convert the bytes to a u32 value in little-endian order
    if let Ok(u32) = bytes_slice.read_i32::<LittleEndian>(){
        Ok(u32)
    } else {
        Err(())
    }
}

fn from_byte_array_to_u16(mmap: &Mmap, from: usize) -> u16 {
    let mut bytes_slice = &mmap[from..from + std::mem::size_of::<u16>()];
    bytes_slice.read_u16::<LittleEndian>().unwrap()
}

fn byte_array_to_i16(ba: &[u8]) -> i16 {
    let mut ba = ba;
    ba.read_i16::<LittleEndian>().unwrap()
}

fn get_file_signature(file_signature_str: &str) -> AbfType {
    match file_signature_str {
        "ABF " => AbfType::AbfV1,
        "ABF2" => AbfType::AbfV2,
        _ => AbfType::Invalid,
    }
}
