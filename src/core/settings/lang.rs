use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::fs;

pub const LANGUAGE_LIST: &[(&str, &str)] = &[
    ("", "Automatic"),
    ("en", "English"),
    ("ja", "Japanese"), 
    ("tw", "Taiwanese"),
    ("ko", "Korean"),   
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
    ("--", "None"),
];

pub fn get_label_for_code(target_code: &str) -> String {
    for (code, label) in LANGUAGE_LIST {
        if *code == target_code { return label.to_string(); }
    }
    format!("Unknown ({})", target_code)
}

pub fn handle_update(
    rx_opt: &mut Option<Receiver<Vec<String>>>, 
    available_languages: &mut Vec<String>, 
    current_selection: &mut String
) {
    let Some(rx) = rx_opt else { return; };
    if let Ok(found_languages) = rx.try_recv() {
        *available_languages = found_languages;
        *rx_opt = None;
        validate_selection(current_selection, available_languages);
    }
}

pub fn validate_selection(current_selection: &mut String, available_languages: &[String]) {
    if available_languages.contains(current_selection) { return; }

    // Try to find a valid default from our known list
    for (code, _) in LANGUAGE_LIST {
        if *code == "--" { continue; }
        if available_languages.contains(&code.to_string()) {
            *current_selection = code.to_string();
            return;
        }
    }
    // Fallback to Automatic if nothing else matches
    *current_selection = "".to_string();
    // If Automatic fails, we load nothing and rely on
    // fallbacks defined elsewhere
}

pub fn start_scan() -> Receiver<Vec<String>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let base_path = Path::new("game/assets/img015");
        let mut found_languages = Vec::new();

        // Scan for specific language files (img015_XX.png)
        if base_path.exists() && base_path.is_dir() {
            if let Ok(entries) = fs::read_dir(base_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    
                    // We only care about .png files to identify the language
                    if path.extension().and_then(|ext| ext.to_str()) == Some("png") {
                        let filename = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
                        
                        if filename.starts_with("img015_") {
                            let extracted_code = filename.trim_start_matches("img015_");
                            if !extracted_code.is_empty() {
                                // Verify corresponding .imgcut exists
                                let cut_name = format!("img015_{}.imgcut", extracted_code);
                                if base_path.join(cut_name).exists() {
                                    found_languages.push(extracted_code.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Always ensure "Automatic" ("") is an option
        if !found_languages.contains(&"".to_string()) {
             found_languages.push("".to_string());
        }

        // Always ensure "None" ("--") is an option
        if !found_languages.contains(&"--".to_string()) {
             found_languages.push("--".to_string());
        }

        // Dropdown: Automatic -> Specific -> Unknown -> None
        found_languages.sort_by(|lang_a, lang_b| {
            if lang_a == "" { return std::cmp::Ordering::Less; }
            if lang_b == "" { return std::cmp::Ordering::Greater; }
            if lang_a == "--" { return std::cmp::Ordering::Greater; }
            if lang_b == "--" { return std::cmp::Ordering::Less; }
            lang_a.cmp(lang_b)
        });
        found_languages.dedup();

        let _ = tx.send(found_languages);
    });
    rx
}