use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub const LANGUAGE_PRIORITY: &[(&str, &str)] = &[
    ("au", "Automatic"),
    ("en", "English"),
    ("ja", "Japanese"),
    ("tw", "Taiwanese"),
    ("ko", "Korean"),
    ("es", "Spanish"),
    ("de", "German"),
    ("fr", "French"),
    ("it", "Italian"),
    ("th", "Thai"),
];

pub fn refresh_available_languages() -> Receiver<Vec<String>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let base_path = Path::new("game/assets/img015");
        let mut found = Vec::new();

        if base_path.exists() {
            for (code, _) in LANGUAGE_PRIORITY {
                if is_valid_pair(base_path, code) {
                    found.push(code.to_string());
                }
            }
        }
        let _ = tx.send(found);
    });
    rx
}

fn is_valid_pair(base: &Path, code: &str) -> bool {
    let png = base.join(format!("img015_{}.png", code));
    let cut = base.join(format!("img015_{}.imgcut", code));
    png.exists() && cut.exists()
}

pub fn get_label_for_code(code: &str) -> String {
    if code.is_empty() { 
        return "None".to_string(); 
    }
    
    for (c, label) in LANGUAGE_PRIORITY {
        if *c == code { 
            return label.to_string(); 
        }
    }
    "Unknown".to_string()
}