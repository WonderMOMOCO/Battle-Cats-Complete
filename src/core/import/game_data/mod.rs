pub mod import;
pub mod export;

use std::sync::mpsc::Sender;

// Import
pub fn import_standard_folder(path: &str, tx: Sender<String>) -> Result<bool, String> {
    import::import_from_folder(path, tx)
}

pub fn import_standard_zip(path: &str, tx: Sender<String>) -> Result<bool, String> {
    import::import_from_zip(path, tx)
}

// Dev
#[cfg(feature = "dev")]
pub fn run_dev_decryption(path: &str, region: &str, tx: Sender<String>) -> Result<String, String> {
    crate::dev::decrypt::run_decryption(path, region, tx)
        .map(|_| "Success! Decryption complete.".to_string())
}

// Export
pub fn create_game_zip(tx: Sender<String>, level: i32, filename: String) -> Result<(), String> {
    export::create_game_zip(tx, level, filename)
}