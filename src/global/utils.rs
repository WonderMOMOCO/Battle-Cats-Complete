#![allow(dead_code)]
use image::imageops;
use regex::Regex;
use eframe::egui;

pub trait SoftReset {
    fn reset(&mut self);
}

pub const LANGUAGE_PRIORITY: &[&str] = &["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th", ""];

pub fn autocrop(img: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;

    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 { 
            if x < min_x { min_x = x; }
            if x > max_x { max_x = x; }
            if y < min_y { min_y = y; }
            if y > max_y { max_y = y; }
            found = true;
        }
    }
    if !found { return img; }
    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}

pub fn detect_csv_separator(content: &str) -> char {
    let mut lines_checked = 0;
    
    for line in content.lines() {
        if line.trim().is_empty() { continue; }
        
        if line.contains('|') {
            return '|';
        }

        lines_checked += 1;
        if lines_checked >= 3 { break; }
    }
    
    ','
}

pub fn strip_markdown(text: &str) -> String {
    let mut text = text.to_string();

    if let Ok(re_link) = Regex::new(r"\[([^\]]+)\]\([^\)]+\)") {
        text = re_link.replace_all(&text, "$1").to_string();
    }

    if let Ok(re_list) = Regex::new(r"(?m)^(\s*)[\*\-]\s+") {
        text = re_list.replace_all(&text, "${1}• ").to_string();
    }

    text = text.replace("**", "");
    text = text.replace("__", "");
    text = text.replace("*", ""); 
    text = text.replace("_", "");
    text = text.replace("`", "");

    text
}

pub fn process_markdown(ui: &mut egui::Ui, raw_text: &str) {
    let content = strip_markdown(raw_text);

    for line in content.lines() {
        let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();
        let trimmed = line.trim();

        if trimmed.is_empty() {
            ui.add_space(10.0);
            continue;
        }

        ui.horizontal_top(|ui| {
            if leading_spaces > 0 {
                ui.add_space(leading_spaces as f32 * 6.0); 
            }

            if trimmed.starts_with('•') || trimmed.starts_with('-') || trimmed.starts_with('*') {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.label("•");    
                let text = trimmed.trim_start_matches(|c| c == '•' || c == '-' || c == '*').trim();
                ui.add(egui::Label::new(text).wrap());
            } else if trimmed.starts_with('#') {
                let text = trimmed.trim_start_matches('#').trim();
                ui.add(egui::Label::new(
                    egui::RichText::new(text).heading().strong()
                ).wrap());
            } else {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.add(egui::Label::new(trimmed).wrap());
            }
        });
    }
}