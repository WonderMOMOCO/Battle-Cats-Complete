use serde::{Deserialize, Serialize};
use std::sync::mpsc::Receiver;

use super::lang;
use super::upd::UpdateMode;

#[derive(Serialize, Deserialize, Default)]
#[serde(default)] 
pub struct Settings {
    pub general: GeneralSettings,
    pub cat_data: CatDataSettings,
    pub game_data: GameDataSettings,
    pub animation: AnimSettings,
    
    #[serde(skip)] 
    pub runtime: RuntimeState,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralSettings {
    pub game_language: String, 
    pub update_mode: UpdateMode,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            game_language: "".to_string(),
            update_mode: UpdateMode::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CatDataSettings {
    pub preferred_banner_form: usize,
    pub high_banner_quality: bool,
    pub unit_persistence: bool,
    pub show_invalid_units: bool,
    pub expand_spirit_details: bool,
    pub default_level: i32,
    pub auto_level_calculations: bool,
    pub bump_ultra_60: bool,
}

impl Default for CatDataSettings {
    fn default() -> Self {
        Self {
            preferred_banner_form: 0,
            high_banner_quality: false,
            unit_persistence: true,
            show_invalid_units: false,
            expand_spirit_details: false,
            default_level: 50,
            auto_level_calculations: true,
            bump_ultra_60: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GameDataSettings {
    pub manual_ip: String,
    pub app_folder_persistence: bool,
    pub enable_ultra_compression: bool,
    pub last_compression_level: i32,
}

impl Default for GameDataSettings {
    fn default() -> Self {
        Self {
            manual_ip: String::new(),
            app_folder_persistence: false,
            enable_ultra_compression: false,
            last_compression_level: 9,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct AnimSettings {
    pub centering_behavior: usize, 
    pub interpolation: bool,
    pub native_fps: f32,
    pub debug_view: bool,
    pub auto_set_camera_region: bool,
    pub default_showcase_walk: i32,
    pub default_showcase_idle: i32,
    pub default_showcase_kb: i32,
    pub last_export_format: i32,
    pub controls_expanded: bool,
    pub export_popup_open: bool,
}

impl Default for AnimSettings {
    fn default() -> Self {
        Self {
            centering_behavior: 2,
            interpolation: false,
            native_fps: 30.0,
            debug_view: false,
            auto_set_camera_region: false,
            default_showcase_walk: 90,
            default_showcase_idle: 90,
            default_showcase_kb: 60,
            last_export_format: 0,
            controls_expanded: true,
            export_popup_open: false,
        }
    }
}

pub struct RuntimeState {
    pub manual_check_requested: bool,
    pub active_tab: String,
    pub available_languages: Vec<String>,
    pub rx_lang: Option<Receiver<Vec<String>>>,
    pub show_ip_field: bool,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            manual_check_requested: false,
            active_tab: "General".to_string(),
            available_languages: Vec::new(),
            rx_lang: Some(lang::start_scan()), 
            show_ip_field: false,
        }
    }
}

impl Settings {
    pub fn update_language_list(&mut self) {
        lang::handle_update(
            &mut self.runtime.rx_lang, 
            &mut self.runtime.available_languages, 
            &mut self.general.game_language
        );
    }
}

#[derive(Clone, Debug)]
pub struct ScannerConfig {
    pub language: String,
    pub preferred_form: usize,
    pub show_invalid: bool,
}

#[derive(Clone, Debug)]
pub struct EmulatorConfig {
    pub keep_app_folder: bool,
    pub manual_ip: String,
}

impl Settings {
    pub fn scanner_config(&self) -> ScannerConfig {
        ScannerConfig {
            language: self.general.game_language.clone(),
            preferred_form: self.cat_data.preferred_banner_form,
            show_invalid: self.cat_data.show_invalid_units,
        }
    }

    pub fn emulator_config(&self) -> EmulatorConfig {
        EmulatorConfig {
            keep_app_folder: self.game_data.app_folder_persistence,
            manual_ip: self.game_data.manual_ip.clone(),
        }
    }
}