use std::path::{Path, PathBuf};

pub fn calculate(path: &Path, temp_apk_dirs: &[PathBuf]) -> u64 {
    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let mut score = 5_000; 
    let parts: Vec<&str> = name.split('_').collect();
    
    if parts.len() >= 3 {
        if let (Ok(version_major), Ok(version_minor)) = (parts[1].parse::<u64>(), parts[2].parse::<u64>()) {
            score = 100_000_000 + (version_major * 100) + version_minor;
        }
    }
    
    if score == 5_000 && name.ends_with("Server") {
        let chars: Vec<char> = name.chars().collect();
        score = if chars.len() > 1 && chars[0].is_ascii_uppercase() && chars[1].is_ascii_uppercase() { 
            20_000 + (chars[0] as u64) 
        } else { 
            10_000 
        };
    }
    
    if score == 5_000 && (name == "DataLocal" || name == "UpdateLocal" || name.ends_with("Local")) { score = 0; }
    if temp_apk_dirs.iter().any(|dir| path.starts_with(dir)) { score += 500_000_000; }
    score
}