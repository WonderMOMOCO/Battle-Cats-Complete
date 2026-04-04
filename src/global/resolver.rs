use std::path::{Path, PathBuf};
use std::sync::RwLock;

// Pointer to the currently enabled mod
static ACTIVE_MOD: RwLock<Option<String>> = RwLock::new(None);

pub fn set_active_mod(mod_name: Option<String>) {
    if let Ok(mut active) = ACTIVE_MOD.write() {
        *active = mod_name;
    }
}

// Finds every valid version of the provided filenames in priority order
// Accepts a single string, an array of strings, a Vec of strings, etc.
pub fn get<I, S>(dir: &Path, filenames: I, priority: &[String]) -> Vec<PathBuf> 
where 
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    // Collect the generic iterator into a Vec so we can loop over it multiple times safely
    let names: Vec<String> = filenames.into_iter().map(|s| s.as_ref().to_string()).collect();
    
    let mut targets = Vec::new();
    for code in priority {
        if code == "--" { break; }
        
        for filename in &names {
            if code.is_empty() {
                targets.push(filename.clone());
            } else if let Some(name) = build_regional_name(filename, code) {
                targets.push(name);
            }
        }
    }

    let mut paths = Vec::new();

    // Check ALL Mod variants in priority order BEFORE touching base files
    for target in &targets {
        if let Some(p) = check_mod_override(target) {
            paths.push(p);
        }
    }

    // Check ALL Game variants in priority order
    for target in &targets {
        let local_path = dir.join(target);
        if local_path.exists() {
            paths.push(local_path);
        }
    }

    paths.dedup();
    paths
}

fn check_mod_override(filename: &str) -> Option<PathBuf> {
    // Check if a mod is actually enabled
    let active_mod = {
        let guard = ACTIVE_MOD.read().ok()?;
        guard.as_ref().cloned()?
    };
    
    let mod_dir = Path::new("mods").join(active_mod);
    
    // Check flat path
    let flat_path = mod_dir.join(filename);
    if flat_path.exists() {
        return Some(flat_path);
    }
    
    None
}

fn build_regional_name(base_filename: &str, lang_code: &str) -> Option<String> {
    if lang_code.is_empty() { return None; }
    let path_obj = Path::new(base_filename);
    let stem = path_obj.file_stem()?.to_str()?;
    let ext = path_obj.extension().unwrap_or_default().to_str().unwrap_or("");
    let ext_str = if ext.is_empty() { String::new() } else { format!(".{}", ext) };
    
    Some(format!("{}_{}{}", stem, lang_code, ext_str))
}