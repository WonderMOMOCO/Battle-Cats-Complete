use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, atomic::AtomicBool};
use crate::core::anim::export::encoding::{ExportFormat, EncoderMessage};
use crate::core::utils::DragGuard;

pub const DEFAULT_WALK_LEN: i32 = 90;
pub const DEFAULT_IDLE_LEN: i32 = 90;
pub const DEFAULT_KB_LEN: i32 = 60;

#[derive(Clone, PartialEq, Debug)]
pub enum ExportMode {
    Manual,
    Loop,
    Showcase,
}

#[derive(Clone, Debug)]
#[allow(dead_code)] 
pub enum LoopStatus {
    Searching(usize),
    Found(i32, i32),
    NotFound,
    Error(String),
}

pub struct ExporterState {
    // Input
    pub frame_start: i32,
    pub frame_end: i32,
    pub max_frame: i32,
    pub frame_start_str: String,
    pub frame_end_str: String,

    // Modes
    pub export_mode: ExportMode,
    pub loop_supported: bool,

    // Loop Mode Inputs
    pub loop_tolerance: i32,
    pub loop_tolerance_str: String,
    pub loop_min: i32,
    pub loop_min_str: String,
    pub loop_max: Option<i32>,    
    pub loop_max_str: String,     

    // Showcase Inputs
    pub showcase_walk_str: String,
    pub showcase_idle_str: String,
    pub showcase_attack_str: String,
    pub showcase_kb_str: String,
    
    // Showcase Data (Parsed/Calculated)
    pub showcase_walk_len: i32,
    pub showcase_idle_len: i32,
    pub detected_attack_len: i32,
    pub showcase_attack_len: i32,
    pub showcase_kb_len: i32,
    
    // Dynamic Defaults
    pub detected_walk_len: i32,
    pub detected_idle_len: i32,
    
    // State Tracking for Settings Updates
    pub last_known_walk_default: i32,
    pub last_known_idle_default: i32,
    pub last_known_kb_default: i32,

    pub fps: i32,
    pub zoom: f32,
    
    // Camera / Crop
    pub region_x: f32,
    pub region_y: f32,
    pub region_w: f32,
    pub region_h: f32,
    
    // Output
    pub file_name: String,
    pub name_prefix: String,
    pub format: ExportFormat,
    
    // New Percentage Settings
    pub quality_percent: i32,
    pub quality_percent_str: String,
    pub compression_percent: i32,      
    pub compression_percent_str: String,

    // Options
    pub background: bool,
    pub user_bg_preference: bool,
    pub interpolation: bool,
    
    // Runtime
    pub is_processing: bool,
    pub current_progress: i32, 
    pub encoded_frames: i32,   
    pub tx: Option<Sender<EncoderMessage>>,
    pub abort: Option<Arc<AtomicBool>>, 
    pub export_result_msg: Option<String>, // Separate msg for Export
    
    // Loop Finding Runtime
    pub is_loop_searching: bool,
    pub loop_frames_searched: usize,
    pub loop_rx: Option<Receiver<LoopStatus>>,
    pub loop_abort: Option<Arc<AtomicBool>>,
    pub loop_search_start_time: Option<f64>,
    pub loop_result_msg: Option<String>, // Separate msg for Loop

    // UI Helpers
    pub drag_guard: DragGuard,
    pub anim_name: String,
    pub completion_time: Option<f64>, 
}

impl Default for ExporterState {
    fn default() -> Self {
        Self {
            frame_start: 0,
            frame_end: 0,
            max_frame: 100,
            frame_start_str: String::new(),
            frame_end_str: String::new(),

            export_mode: ExportMode::Manual,
            loop_supported: false,

            loop_tolerance: 30,
            loop_tolerance_str: String::new(),
            loop_min: 15,
            loop_min_str: String::new(),
            loop_max: None,              
            loop_max_str: String::new(), 

            showcase_walk_str: String::new(),
            showcase_idle_str: String::new(),
            showcase_attack_str: String::new(), 
            showcase_kb_str: String::new(),
            
            showcase_walk_len: DEFAULT_WALK_LEN,
            showcase_idle_len: DEFAULT_IDLE_LEN,
            detected_attack_len: 0, 
            showcase_attack_len: 0, 
            showcase_kb_len: DEFAULT_KB_LEN,
            
            detected_walk_len: DEFAULT_WALK_LEN,
            detected_idle_len: DEFAULT_IDLE_LEN,
            
            // Initialize with hardcoded constants first
            last_known_walk_default: DEFAULT_WALK_LEN,
            last_known_idle_default: DEFAULT_IDLE_LEN,
            last_known_kb_default: DEFAULT_KB_LEN,

            fps: 30,
            zoom: 1.0,
            
            region_x: 0.0,
            region_y: 0.0,
            region_w: 0.0,
            region_h: 0.0,
            
            file_name: String::new(),
            name_prefix: String::new(),
            format: ExportFormat::Gif,
            
            quality_percent: 100,
            quality_percent_str: String::new(),
            compression_percent: 0,
            compression_percent_str: String::new(),

            background: false,
            user_bg_preference: false,
            interpolation: false,
            
            is_processing: false,
            current_progress: 0,
            encoded_frames: 0,
            tx: None,
            abort: None, 
            export_result_msg: None,

            is_loop_searching: false,
            loop_frames_searched: 0,
            loop_rx: None,
            loop_abort: None,
            loop_search_start_time: None,
            loop_result_msg: None,
            
            drag_guard: DragGuard::default(),
            anim_name: String::new(),
            completion_time: None,
        }
    }
}