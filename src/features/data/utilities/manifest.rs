use std::fs::{self, File};
use std::io::{Read, BufReader, BufWriter};
use std::path::Path;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

#[derive(Serialize, Deserialize, Clone)]
pub struct ManifestEntry {
    pub winner: String,
    pub weight: usize, 
    pub size: usize,
    pub encrypted: usize,
    pub checksum: u64,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PackRecord {
    pub checksum: u64,
}

pub fn load<T: DeserializeOwned + Default>(path: &Path) -> T {
    if let Ok(file) = File::open(path) {
        if let Ok(manifest) = serde_json::from_reader(BufReader::new(file)) {
            return manifest;
        }
    }
    T::default()
}

pub fn save<T: Serialize>(path: &Path, data: &T) {
    if let Some(parent_directory) = path.parent() {
        let _ = fs::create_dir_all(parent_directory);
    }
    if let Ok(file) = File::create(path) {
        let _ = serde_json::to_writer_pretty(BufWriter::new(file), data);
    }
}

pub fn hash(data: &[u8]) -> u64 {
    let mut current_hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        current_hash ^= byte as u64;
        current_hash = current_hash.wrapping_mul(0x100000001b3);
    }
    current_hash
}

pub fn hash_file(path: &Path) -> std::io::Result<u64> {
    let mut file = File::open(path)?;
    let mut current_hash: u64 = 0xcbf29ce484222325;
    let mut buffer = vec![0u8; 65536]; 
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 { break; }
        for &byte in &buffer[..bytes_read] {
            current_hash ^= byte as u64;
            current_hash = current_hash.wrapping_mul(0x100000001b3);
        }
    }
    Ok(current_hash)
}