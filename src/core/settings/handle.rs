use super::Settings;

// Config Bundles
#[derive(Clone, Debug)]
pub struct ScannerConfig {
    pub language: String,
    pub preferred_form: usize,
    pub show_invalid: bool,
}

#[derive(Clone, Debug)]
pub struct EmulatorConfig {
    pub keep_app_folder: bool,
}

impl Settings {
    pub fn scanner_config(&self) -> ScannerConfig {
        ScannerConfig {
            language: self.game_language.clone(),
            preferred_form: self.preferred_banner_form,
            show_invalid: self.show_invalid_units,
        }
    }

    pub fn emulator_config(&self) -> EmulatorConfig {
        EmulatorConfig {
            keep_app_folder: self.app_folder_persistence,
        }
    }
}