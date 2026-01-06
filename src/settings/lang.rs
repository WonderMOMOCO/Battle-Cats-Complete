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

pub fn handle_update(
    rx_opt: &mut Option<Receiver<Vec<String>>>, 
    available: &mut Vec<String>, 
    current_selection: &mut String
) {
    let Some(rx) = rx_opt else { 
        return; 
    };
    
    let Ok(langs) = rx.try_recv() else { 
        return; 
    };

    *available = langs;
    *rx_opt = None; 
    
    validate_selection(current_selection, available);
}

pub fn validate_selection(current: &mut String, available: &[String]) {
    if !current.is_empty() && available.contains(current) {
        return;
    }
    
    for (code, _) in LANGUAGE_PRIORITY {
        if available.contains(&code.to_string()) {
            *current = code.to_string();
            return;
        }
    }
    
    *current = "".to_string();
}

pub fn start_scan() -> Receiver<Vec<String>> {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let base_path = Path::new("game/assets/img015");
        let mut found = Vec::new();

        if base_path.exists() {
            found = LANGUAGE_PRIORITY
                .iter()
                .map(|(code, _)| code.to_string())
                .filter(|code| is_valid_pair(base_path, code))
                .collect();
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