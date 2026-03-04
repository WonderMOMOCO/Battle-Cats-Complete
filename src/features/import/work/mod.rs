pub mod import;
pub mod export;

use std::sync::mpsc::Sender;

// Import
pub fn import_standard_folder(path: &str, tx: Sender<String>) -> Result<bool, String> {
    import::import_from_folder(path, tx)
}

// Imoport
pub fn import_standard_archive(path: &str, tx: Sender<String>) -> Result<bool, String> {
    import::import_from_archive(path, tx)
}

// Export
pub fn create_game_archive(tx: Sender<String>, level: i32, filename: String) -> Result<(), String> {
    export::create_game_archive(tx, level, filename)
}