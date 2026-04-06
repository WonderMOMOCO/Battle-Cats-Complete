use directories::BaseDirs;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use rustc_hash::FxHasher;


pub fn get_cache_dir() -> Option<PathBuf> {
    if let Some(base_dirs) = BaseDirs::new() {
        let cache_directory = base_dirs.data_local_dir().join("battle_cats_complete").join("cache");
        if !cache_directory.exists() {
            let _ = fs::create_dir_all(&cache_directory);
        }
        Some(cache_directory)
    } else {
        None
    }
}

fn hash_directory_parallel(directory_path: &Path) -> u64 {
    if !directory_path.exists() { return 0; }
    
    let mut file_entries = Vec::new();
    if let Ok(read_directory) = fs::read_dir(directory_path) {
        for directory_entry in read_directory.flatten() {
            file_entries.push(directory_entry.path());
        }
    }

    let child_hashes: Vec<u64> = file_entries.par_iter().map(|child_path| {
        let mut local_hasher = FxHasher::default(); 
        if child_path.is_dir() {
            let subdirectory_hash = hash_directory_parallel(child_path);
            subdirectory_hash.hash(&mut local_hasher);
        } else if let Ok(file_metadata) = child_path.metadata() {
            if let Ok(modified_time) = file_metadata.modified() {
                modified_time.hash(&mut local_hasher);
            }
        }
        local_hasher.finish()
    }).collect();

    let mut final_hasher = FxHasher::default();
    for child_hash in child_hashes {
        child_hash.hash(&mut final_hasher);
    }
    file_entries.len().hash(&mut final_hasher);
    final_hasher.finish()
}

pub fn get_game_hash(active_mod: Option<&str>) -> u64 {
    let mut final_game_hasher = FxHasher::default();
    
    let target_paths = ["game/tables", "game/cats", "game/enemies", "mods"];
    for path_string in target_paths {
        let directory_hash = hash_directory_parallel(Path::new(path_string));
        directory_hash.hash(&mut final_game_hasher);
    }
    
    if let Some(mod_name) = active_mod {
        mod_name.hash(&mut final_game_hasher);
    } else {
        "vanilla_base_game".hash(&mut final_game_hasher);
    }

    final_game_hasher.finish()
}

#[derive(Serialize, Deserialize)]
struct CachePayload<T> {
    hash: u64,
    data: T,
}

pub fn load_with_hash<T: DeserializeOwned>(filename: &str) -> Option<(u64, T)> {
    let cache_path = get_cache_dir()?.join(filename);
    
    let cache_file = match File::open(&cache_path) {
        Ok(f) => f,
        Err(_) => return None,
    };

    match bincode::deserialize_from(cache_file) {
        Ok(payload) => {
            let payload: CachePayload<T> = payload;
            Some((payload.hash, payload.data))
        },
        Err(_) => {
            let _ = fs::remove_file(&cache_path);
            None
        }
    }
}

pub fn save<T: Serialize>(filename: &str, hash: u64, data: &T) {
    if let Some(cache_directory) = get_cache_dir() {
        let payload = CachePayload { hash, data };
        if let Ok(cache_file) = File::create(cache_directory.join(filename)) {
            let _ = bincode::serialize_into(cache_file, &payload);
        }
    }
}