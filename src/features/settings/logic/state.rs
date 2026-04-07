use serde::{Deserialize, Serialize};
use super::lang;
use super::upd::UpdateMode;

#[derive(Serialize, Deserialize, Default)]
#[serde(default)] 
pub struct Settings {
    pub general: GeneralSettings,
    pub cat_data: CatDataSettings,
    pub enemy_data: EnemyDataSettings,
    pub game_data: GameDataSettings,
    pub animation: AnimSettings,
    
    #[serde(skip)] 
    pub runtime: RuntimeState,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct GeneralSettings {
    #[serde(default = "crate::features::settings::logic::lang::default_priority")]
    pub language_priority: Vec<String>, 
    pub update_mode: UpdateMode,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            language_priority: lang::default_priority(),
            update_mode: UpdateMode::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct CatDataSettings {
    pub preferred_banner_form: usize,
    pub high_banner_quality: bool,
    pub show_invalid_cats: bool,
    pub expand_spirit_details: bool,
    pub default_level: i32,
    pub auto_level_calculations: bool,
    pub bump_ultra_60: bool,
}

impl Default for CatDataSettings {
    fn default() -> Self {
        Self {
            preferred_banner_form: 3,
            high_banner_quality: true,
            show_invalid_cats: false,
            expand_spirit_details: false,
            default_level: 50,
            auto_level_calculations: true,
            bump_ultra_60: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct EnemyDataSettings {
    pub show_invalid_enemies: bool,
}

impl Default for EnemyDataSettings {
    fn default() -> Self {
        Self {
            show_invalid_enemies: false,
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
    pub adb_import_type_idx: usize,
    pub adb_region_idx: usize,
}

impl Default for GameDataSettings {
    fn default() -> Self {
        Self {
            manual_ip: String::new(),
            app_folder_persistence: false,
            enable_ultra_compression: false,
            last_compression_level: 9,
            adb_import_type_idx: 0,
            adb_region_idx: 4,
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
    pub use_tight_bounds: bool,
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
            use_tight_bounds: true,
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
    pub show_ip_field: bool,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            manual_check_requested: false,
            active_tab: "General".to_string(),
            show_ip_field: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScannerConfig {
    pub language_priority: Vec<String>,
    pub preferred_form: usize,
    pub show_invalid_cats: bool,
    pub show_invalid_enemies: bool,
}

#[derive(Clone, Debug)]
pub struct EmulatorConfig {
    pub keep_app_folder: bool,
    pub manual_ip: String,
}

impl Settings {
    pub fn scanner_config(&self) -> ScannerConfig {
        ScannerConfig {
            language_priority: self.general.language_priority.clone(),
            preferred_form: self.cat_data.preferred_banner_form,
            show_invalid_cats: self.cat_data.show_invalid_cats,
            show_invalid_enemies: self.enemy_data.show_invalid_enemies,
        }
    }

    pub fn emulator_config(&self) -> EmulatorConfig {
        EmulatorConfig {
            keep_app_folder: self.game_data.app_folder_persistence,
            manual_ip: self.game_data.manual_ip.clone(),
        }
    }
}