use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::thread;

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

pub fn handle_update(
    rx_opt: &mut Option<Receiver<Vec<String>>>, 
    available: &mut Vec<String>, 
    current_selection: &mut String
) {
    let Some(rx) = rx_opt else { return; };
    let Ok(langs) = rx.try_recv() else { return; };

    *available = langs;
    *rx_opt = None;
    
    validate_selection(current_selection, available);
}

pub fn validate_selection(current: &mut String, available: &[String]) {
    if available.contains(current) {
        return;
    }

    for (code, _) in LANGUAGE_LIST {
        if *code == "--" { continue; }
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
            // Detect all specific language pairs
            let mut langs: Vec<String> = LANGUAGE_LIST
                .iter()
                .filter(|(code, _)| !code.is_empty() && *code != "--") 
                .map(|(code, _)| code.to_string())
                .filter(|code| is_valid_pair(base_path, code))
                .collect();

            // Check if the generic base file exists (img015.png)
            let base_exists = is_valid_pair(base_path, "");

            // If ANY valid file exists, show "Automatic" at the top
            if base_exists || !langs.is_empty() {
                found.push("".to_string());
            }

            // Add the specific languages in the middle
            found.append(&mut langs);

            // Always allow "None" at the bottom
            found.push("--".to_string()); 
        }
        let _ = tx.send(found);
    });
    rx
}

fn is_valid_pair(base: &Path, code: &str) -> bool {
    let (png_name, cut_name) = if code.is_empty() {
        ("img015.png".to_string(), "img015.imgcut".to_string())
    } else {
        (format!("img015_{}.png", code), format!("img015_{}.imgcut", code))
    };

    base.join(png_name).exists() && base.join(cut_name).exists()
}

pub fn get_label_for_code(code: &str) -> String {
    for (c, label) in LANGUAGE_LIST {
        if *c == code { return label.to_string(); }
    }
    "Unknown".to_string()
}