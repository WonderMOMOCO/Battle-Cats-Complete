use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ManifestEntry {
    pub pack: String,
    pub offset: u64, 
    pub size: usize,
    pub checksum: u64,
}

pub fn load(path: &Path) -> HashMap<String, ManifestEntry> {
    if let Ok(file) = File::open(path) {
        if let Ok(manifest) = serde_json::from_reader(BufReader::new(file)) {
            return manifest;
        }
    }
    HashMap::new()
}

pub fn save(path: &Path, manifest: &HashMap<String, ManifestEntry>) {
    if let Some(parent_directory) = path.parent() {
        let _ = fs::create_dir_all(parent_directory);
    }
    if let Ok(file) = File::create(path) {
        let _ = serde_json::to_writer_pretty(BufWriter::new(file), manifest);
    }
}

// Deterministic FNV-1a hash (64-bit)
pub fn hash(data: &[u8]) -> u64 {
    let mut current_hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        current_hash ^= byte as u64;
        current_hash = current_hash.wrapping_mul(0x100000001b3);
    }
    current_hash
}