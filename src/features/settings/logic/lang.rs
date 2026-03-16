pub const LANGUAGE_LIST: &[(&str, &str)] = &[
    ("", "Base"), 
    ("en", "English"),
    ("ja", "Japanese"), 
    ("tw", "Taiwanese"),
    ("ko", "Korean"),
    ("--", "None"), 
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
];

pub fn get_label_for_code(target_code: &str) -> String {
    for (code, label) in LANGUAGE_LIST {
        if *code == target_code { return label.to_string(); }
    }
    format!("Unknown ({})", target_code)
}

/// Returns the default initialization array
pub fn default_priority() -> Vec<String> {
    vec![
        "".to_string(), "en".to_string(), "ja".to_string(), "tw".to_string(), 
        "ko".to_string(), "--".to_string(), "es".to_string(), "de".to_string(), 
        "fr".to_string(), "it".to_string(), "th".to_string()
    ]
}

/// Run this when loading Settings to ensure the list has all languages and the separator
pub fn ensure_complete_list(priority_list: &mut Vec<String>) {
    if priority_list.is_empty() {
        *priority_list = default_priority();
        return;
    }
    
    // Ensure "--" exists so the user can always disable things
    if !priority_list.contains(&"--".to_string()) {
        priority_list.push("--".to_string());
    }

    // Append any newly added languages to the very bottom
    for &(code, _) in LANGUAGE_LIST {
        let c = code.to_string();
        if !priority_list.contains(&c) {
            priority_list.push(c);
        }
    }
    
    // Clean up any deprecated language codes
    priority_list.retain(|code| LANGUAGE_LIST.iter().any(|(c, _)| *c == code));
}