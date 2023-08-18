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

    pub fn print_info(self){
        println!("section type: {:?}, block number: {:?}, byte count: {:?}, item count: {:?}", self.section_type, self.block_number, self.byte_count, self.item_count);
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

impl <'a, ProtocolSectionType> Section<'a, ProtocolSectionType> {

    pub  fn operation_mode(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 0)
    }
    
    pub  fn adc_sequence_interval(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 2)
    }
    
    pub  fn enable_file_compression(&self) -> u8 {
        self.mmap[6] as u8
    }
    
    // pub fn _s_unused(&self) -> [u8; 3] {
    //     self.read_bytes(7)
    // }

    pub  fn file_compression_ratio(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 10)
    }
    
    pub  fn synch_time_unit(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 14)
    }
    
    pub  fn seconds_per_run(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 18)
    }
    
    pub  fn num_samples_per_episode(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 22)
    }
    
    pub  fn pre_trigger_samples(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 26)
    }
    
    pub  fn episodes_per_run(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 30)
    }
    
    pub  fn runs_per_trial(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 34)
    }
    
    pub  fn number_of_trials(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 38)
    }
    
    pub  fn averaging_mode(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 42)
    }
    
    pub  fn undo_run_count(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 44)
    }
    
    pub  fn first_episode_in_run(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 46)
    }
    
    pub  fn trigger_threshold(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 48)
    }

    pub  fn trigger_source(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 52)
    }
    
    pub  fn trigger_action(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 54)
    }
    
    pub  fn trigger_polarity(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 56)
    }
    
    pub  fn scope_output_interval(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 58)
    }
    
    pub  fn episode_start_to_start(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 62)
    }
    
    pub  fn run_start_to_start(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 66)
    }
    
    pub  fn average_count(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 70)
    }
    
    pub  fn trial_start_to_start(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 74)
    }
    
    pub  fn auto_trigger_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 78)
    }
    
    pub  fn first_run_delay_s(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 80)
    }

    pub  fn channel_stats_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 84)
    }
    
    pub  fn samples_per_trace(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 86)
    }
    
    pub  fn start_display_num(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 90)
    }
    
    pub  fn finish_display_num(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 94)
    }
    
    pub  fn show_pn_raw_data(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 98)
    }
    
    pub  fn statistics_period(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 100)
    }
    
    pub  fn statistics_measurements(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 104)
    }
    
    pub  fn statistics_save_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 108)
    }
    
    pub  fn adc_range(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 110)
    }
    
    pub  fn dac_range(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 114)
    }
    
    pub  fn adc_resolution(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 118)
    }
    
    pub  fn dac_resolution(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 122)
    }
    
    pub  fn experiment_type(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 126)
    }
    
    pub  fn manual_info_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 128)
    }
    
    pub  fn comments_enable(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 130)
    }
    
    pub  fn file_comment_index(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 132)
    }
    
    pub  fn auto_analyse_enable(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 136)
    }
    
    pub  fn signal_type(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 138)
    }
    
    pub  fn digital_enable(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 140)
    }

    pub  fn active_dac_channel(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 142)
    }
    
    pub  fn digital_holding(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 144)
    }
    
    pub  fn digital_inter_episode(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 146)
    }
    
    pub  fn digital_dac_channel(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 148)
    }
    
    pub  fn digital_train_active_logic(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 150)
    }
    
    pub  fn stats_enable(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 152)
    }
    
    pub  fn statistics_clear_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 154)
    }
    
    pub  fn level_hysteresis(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 156)
    }
    
    pub  fn time_hysteresis(&self) -> i32 {
        cu::mmap_to_i32(&self.mmap, 158)
    }
    
    pub  fn allow_external_tags(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 162)
    }
    
    pub  fn average_algorithm(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 164)
    }
    
    pub  fn average_weighting(&self) -> f32 {
        cu::mmap_to_f32(&self.mmap, 166)
    }
    
    pub  fn undo_prompt_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 170)
    }
    
    pub  fn trial_trigger_source(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 172)
    }
    pub  fn statistics_display_strategy(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 174)
    }
    
    pub  fn external_tag_type(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 176)
    }
    
    pub  fn scope_trigger_out(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 178)
    }
    
    pub  fn ltp_type(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 180)
    }
    
    pub  fn alternate_dac_output_state(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 182)
    }
    
    pub  fn alternate_digital_output_state(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 184)
    }
    
    pub  fn cell_id(&self) -> [f32; 3] {
        [
            cu::mmap_to_f32(&self.mmap, 186),
            cu::mmap_to_f32(&self.mmap, 190),
            cu::mmap_to_f32(&self.mmap, 194),
        ]
    }
    
    pub  fn digitizer_adcs(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 198)
    }
    
    pub  fn digitizer_dacs(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 200)
    }
    
    pub  fn digitizer_total_digital_outs(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 202)
    }
    
    pub  fn digitizer_synch_digital_outs(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 204)
    }
    
    pub  fn digitizer_type(&self) -> i16 {
        cu::mmap_to_i16(&self.mmap, 206)
    }
    
    // pub  fn digitizer_type(&self) -> String {
    //     get_digitizer_name(self.n_digitizer_type())
    // }
}