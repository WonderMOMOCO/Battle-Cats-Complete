use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

pub mod lang;

#[derive(Serialize, Deserialize)]
#[serde(default)] 
pub struct Settings {
    pub high_banner_quality: bool,
    pub unit_persistence: bool,
    pub expand_spirit_details: bool,
    pub ability_padding_x: f32,
    pub ability_padding_y: f32,
    pub trait_padding_y: f32,
    pub game_language: String, 
    
    #[serde(skip)] 
    pub active_tab: String,

    #[serde(skip)]
    pub available_languages: Vec<String>,
    #[serde(skip)]
    pub rx_lang: Option<Receiver<Vec<String>>>,
}

impl Default for Settings {
    fn default() -> Self {
        let mut s = Self {
            high_banner_quality: false,
            unit_persistence: true,
            expand_spirit_details: false,
            ability_padding_x: 3.0,
            ability_padding_y: 5.0,
            trait_padding_y: 5.0,
            game_language: "".to_string(), 
            
            active_tab: "General".to_string(),
            
            available_languages: Vec::new(),
            rx_lang: None,
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

    pub fn trigger_language_scan(&mut self) {
        self.rx_lang = Some(lang::start_scan());
    }
}