use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;
pub mod lang;
pub mod upd;
pub mod handle;

#[derive(Serialize, Deserialize)]
#[serde(default)] 
pub struct Settings {
    pub high_banner_quality: bool,
    pub unit_persistence: bool,
    pub expand_spirit_details: bool,
    pub show_invalid_units: bool,
    pub animation_interpolation: bool,
    pub animation_debug: bool,
    pub reset_view_on_selection: bool,
    pub centering_behavior: usize, 
    pub ability_padding_x: f32,
    pub ability_padding_y: f32,
    pub trait_padding_y: f32,
    pub game_language: String, 
    pub preferred_banner_form: usize,
    pub update_mode: upd::UpdateMode,
    pub enable_ultra_compression: bool,
    pub last_compression_level: i32,
    pub app_folder_persistence: bool,

    #[serde(skip)] pub manual_check_requested: bool,
    #[serde(skip)] pub active_tab: String,
    #[serde(skip)] pub available_languages: Vec<String>,
    #[serde(skip)] pub rx_lang: Option<Receiver<Vec<String>>>,

    pub native_fps: f32,
}

impl Default for Settings {
    fn default() -> Self {
        let mut s = Self {
            high_banner_quality: false,
            unit_persistence: true,
            expand_spirit_details: false,
            show_invalid_units: false,
            animation_interpolation: false,
            animation_debug: false,
            reset_view_on_selection: true,
            centering_behavior: 2,  
            ability_padding_x: 3.0,
            ability_padding_y: 5.0,
            trait_padding_y: 5.0,
            game_language: "".to_string(), 
            preferred_banner_form: 0,
            update_mode: upd::UpdateMode::default(),
            enable_ultra_compression: false,
            last_compression_level: 9, 
            app_folder_persistence: false,
            active_tab: "General".to_string(),
            manual_check_requested: false,
            available_languages: Vec::new(),
            rx_lang: None,
            
            native_fps: 30.0,
        };
        s.rx_lang = Some(lang::start_scan());
        s
    }
}

impl Settings {
    pub fn update_language_list(&mut self) {
        lang::handle_update(
            &mut self.rx_lang, 
            &mut self.available_languages, 
            &mut self.game_language
        );
    }
}