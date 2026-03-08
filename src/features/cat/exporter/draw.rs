use std::collections::HashMap;
use image::{RgbaImage, Rgba};
use ab_glyph::PxScale;
use imageproc::drawing::{draw_filled_rect_mut, draw_text_mut, text_size};
use imageproc::rect::Rect;

use crate::features::cat::logic::abilities::{AbilityItem, CustomIcon};
use crate::global::imgcut::SpriteCut;

pub const SUPERSCRIPT_SCALE: f32 = 0.75;
pub const SUPERSCRIPT_OFFSET_Y: f32 = 0.15;

pub fn draw_rounded_rect_mut(img: &mut RgbaImage, rect: Rect, r: i32, color: Rgba<u8>) {
    if r <= 0 {
        draw_filled_rect_mut(img, rect, color);
        return;
    }
    let w = rect.width() as i32;
    let h = rect.height() as i32;
    let x = rect.left();
    let y = rect.top();

    let r = r.min(w / 2).min(h / 2);

    let w_inner = w - 2 * r;
    let h_inner = h - 2 * r;

    if w_inner > 0 {
        draw_filled_rect_mut(img, Rect::at(x + r, y).of_size(w_inner as u32, h as u32), color);
    }
    if h_inner > 0 {
        draw_filled_rect_mut(img, Rect::at(x, y + r).of_size(w as u32, h_inner as u32), color);
    }

    imageproc::drawing::draw_filled_circle_mut(img, (x + r, y + r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + w - 1 - r, y + r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + r, y + h - 1 - r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + w - 1 - r, y + h - 1 - r), r, color);
}

pub fn draw_bottom_rounded_rect_mut(img: &mut RgbaImage, rect: Rect, r: i32, color: Rgba<u8>) {
    if r <= 0 {
        draw_filled_rect_mut(img, rect, color);
        return;
    }
    let w = rect.width() as i32;
    let h = rect.height() as i32;
    let x = rect.left();
    let y = rect.top();

    let r = r.min(w / 2).min(h);

    let w_inner = w - 2 * r;
    let h_top = h - r;

    if h_top > 0 {
        draw_filled_rect_mut(img, Rect::at(x, y).of_size(w as u32, h_top as u32), color);
    }
    if w_inner > 0 && r > 0 {
        draw_filled_rect_mut(img, Rect::at(x + r, y + h - r).of_size(w_inner as u32, r as u32), color);
    }

    imageproc::drawing::draw_filled_circle_mut(img, (x + r, y + h - 1 - r), r, color);
    imageproc::drawing::draw_filled_circle_mut(img, (x + w - 1 - r, y + h - 1 - r), r, color);
}

pub fn get_icon_image(
    item: &AbilityItem, 
    cuts_map: &HashMap<usize, SpriteCut>,
    img015_base: &RgbaImage,
    multihit_base: &RgbaImage,
    kamikaze_base: &RgbaImage,
    bosswave_base: &RgbaImage,
    export_icon_size: u32,
) -> RgbaImage {
    let mut icon = match item.custom_icon {
        CustomIcon::Multihit => multihit_base.clone(),
        CustomIcon::Kamikaze => kamikaze_base.clone(),
        CustomIcon::BossWave => bosswave_base.clone(),
        CustomIcon::None => {
            if let Some(cut) = cuts_map.get(&item.icon_id) {
                let w = img015_base.width() as f32;
                let h = img015_base.height() as f32;
                
                let px = (cut.uv_coordinates.min.x * w).round() as u32;
                let py = (cut.uv_coordinates.min.y * h).round() as u32;
                let pw = cut.original_size.x.round() as u32;
                let ph = cut.original_size.y.round() as u32;
                
                if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
                    image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image()
                } else {
                    RgbaImage::new(export_icon_size, export_icon_size)
                }
            } else {
                RgbaImage::new(export_icon_size, export_icon_size)
            }
        }
    };

    // Resize the base icon FIRST so it defines the proper bounding box
    if icon.width() != export_icon_size || icon.height() != export_icon_size {
        icon = image::imageops::resize(&icon, export_icon_size, export_icon_size, image::imageops::FilterType::Lanczos3);
    }

    if let Some(border_id) = item.border_id {
        if let Some(cut) = cuts_map.get(&border_id) {
            let w = img015_base.width() as f32;
            let h = img015_base.height() as f32;
            let px = (cut.uv_coordinates.min.x * w).round() as u32;
            let py = (cut.uv_coordinates.min.y * h).round() as u32;
            let pw = cut.original_size.x.round() as u32;
            let ph = cut.original_size.y.round() as u32;
            
            if px + pw <= img015_base.width() && py + ph <= img015_base.height() {
                let mut border = image::imageops::crop_imm(img015_base, px, py, pw, ph).to_image();
                
                // Resize the border to perfectly match the icon's new size
                if border.width() != export_icon_size || border.height() != export_icon_size {
                    border = image::imageops::resize(&border, export_icon_size, export_icon_size, image::imageops::FilterType::Lanczos3);
                }
                
                // Overlay the now-matching border on top of the icon
                image::imageops::overlay(&mut icon, &border, 0, 0);
            }
        }
    }
    
    icon
}

pub fn measure_text_with_superscript(scale: PxScale, font: &impl ab_glyph::Font, text: &str) -> u32 {
    let parts: Vec<&str> = text.split('^').collect();
    let mut total_w = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() { continue; }
        let current_scale = if i % 2 == 0 { scale } else { PxScale::from(scale.y * SUPERSCRIPT_SCALE) };
        let (w, _) = text_size(current_scale, font, part);
        total_w += w;
    }
    total_w
}

pub fn draw_text_with_superscript(
    img: &mut RgbaImage,
    color: Rgba<u8>,
    mut x: i32,
    y: i32,
    base_scale: PxScale,
    font: &impl ab_glyph::Font,
    text: &str,
) {
    let parts: Vec<&str> = text.split('^').collect();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() { continue; }
        let (current_scale, current_y) = if i % 2 == 0 {
            (base_scale, y)
        } else {
            (PxScale::from(base_scale.y * SUPERSCRIPT_SCALE), y - (base_scale.y * SUPERSCRIPT_OFFSET_Y) as i32)
        };
        draw_text_mut(img, color, x, current_y, current_scale, font, part);
        let (w, _) = text_size(current_scale, font, part);
        x += w as i32;
    }
}

pub fn wrap_text(text: &str, font: &impl ab_glyph::Font, scale: PxScale, max_width: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current_line = String::new();
        let mut current_word = String::new();
        
        for c in paragraph.chars() {
            let is_cjk = (c >= '\u{4E00}' && c <= '\u{9FFF}') || 
                         (c >= '\u{3040}' && c <= '\u{30FF}') || 
                         (c >= '\u{AC00}' && c <= '\u{D7AF}');
                         
            if c.is_whitespace() || is_cjk {
                if !current_word.is_empty() {
                    let sep = if current_line.is_empty() { "" } else { " " };
                    let test_line = format!("{}{}{}", current_line, sep, current_word);
                    let w = measure_text_with_superscript(scale, font, &test_line);
                    
                    if w as f32 > max_width {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                            current_line = current_word.clone();
                        } else {
                            lines.push(current_word.clone());
                            current_line.clear();
                        }
                    } else {
                        current_line = test_line;
                    }
                    current_word.clear();
                }
                
                if is_cjk {
                    let test_line = if current_line.is_empty() { c.to_string() } else { format!("{}{}", current_line, c) };
                    let w = measure_text_with_superscript(scale, font, &test_line);
                    if w as f32 > max_width {
                        if !current_line.is_empty() {
                            lines.push(current_line.clone());
                        }
                        current_line = c.to_string();
                    } else {
                        current_line = test_line;
                    }
                }
            } else {
                current_word.push(c);
            }
        }
        
        if !current_word.is_empty() {
            let sep = if current_line.is_empty() { "" } else { " " };
            let test_line = format!("{}{}{}", current_line, sep, current_word);
            let w = measure_text_with_superscript(scale, font, &test_line);
            if w as f32 > max_width {
                if !current_line.is_empty() {
                    lines.push(current_line.clone());
                    current_line = current_word;
                } else {
                    lines.push(current_word);
                    current_line.clear();
                }
            } else {
                current_line = test_line;
            }
        }
        if !current_line.is_empty() { lines.push(current_line); }
    }
    if lines.is_empty() { lines.push(String::new()); }
    lines
}

pub fn draw_centered_text(img: &mut RgbaImage, color: Rgba<u8>, rect: Rect, scale: PxScale, font: &impl ab_glyph::Font, text: &str) {
    let (tw, _) = text_size(scale, font, text);
    let tx = rect.left() + (rect.width() as i32 - tw as i32) / 2;
    let ty = rect.top() + (rect.height() as i32 - scale.y as i32) / 2;
    
    draw_text_mut(img, color, tx.max(rect.left()), ty.max(rect.top()), scale, font, text);
}

pub fn draw_time_cell(img: &mut RgbaImage, bg: Rgba<u8>, rect: Rect, frames: i32, font: &impl ab_glyph::Font, scale_f: f32, scale_i: i32, radius: i32, text_scale: f32) {
    draw_rounded_rect_mut(img, rect, radius, bg);
    
    let sec = frames as f32 / 30.0;
    let sec_str = format!("{:.2}s", sec);
    let f_str = format!(" {}f", frames); 
    
    let scale_sec = PxScale::from(15.0 * text_scale * scale_f);
    let scale_f_text = PxScale::from((15.0 * 0.65) * text_scale * scale_f); 
    
    let (sec_w, _) = text_size(scale_sec, font, &sec_str);
    let (f_w, _) = text_size(scale_f_text, font, &f_str);
    
    let gap = 1 * scale_i as u32;
    let total_w = sec_w + f_w + gap;

    let start_x = rect.left() + (rect.width() as i32 - total_w as i32) / 2;
    let start_y = rect.top() + (rect.height() as i32 - scale_sec.y as i32) / 2;
    
    draw_text_mut(img, Rgba([255, 255, 255, 255]), start_x, start_y, scale_sec, font, &sec_str);
    
    let f_y_offset = (scale_sec.y - scale_f_text.y) * 0.75;
    draw_text_mut(img, Rgba([200, 200, 200, 255]), start_x + sec_w as i32 + gap as i32, start_y + f_y_offset as i32, scale_f_text, font, &f_str);
}