#![allow(dead_code)]
use crate::data::global::img015::*;
use image::imageops;
use eframe::egui;

pub trait SoftReset {
    fn reset(&mut self);
}

pub const LANGUAGE_PRIORITY: &[&str] = &["en", "ja", "tw", "ko", "es", "de", "fr", "it", "th", ""];

#[derive(Default)]
pub struct DragGuard {
    broken: bool,
}

impl DragGuard {
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let screen_rect = ctx.screen_rect();
        let (pointer_pos, mouse_down) = ctx.input(|i| {
            (i.pointer.interact_pos(), i.pointer.primary_down())
        });
        let in_window = pointer_pos.map_or(false, |p| screen_rect.contains(p));

        if !mouse_down {
            self.broken = false;
        } else if !in_window {
            self.broken = true;
        }

        in_window && !self.broken
    }
}

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

pub const UI_TRAIT_ORDER: &[usize] = &[
    ICON_TRAIT_RED,
    ICON_TRAIT_FLOATING,
    ICON_TRAIT_BLACK,
    ICON_TRAIT_METAL,
    ICON_TRAIT_ANGEL,
    ICON_TRAIT_ALIEN,
    ICON_TRAIT_ZOMBIE,
    ICON_TRAIT_RELIC,
    ICON_TRAIT_AKU,
    ICON_TRAIT_TRAITLESS,
];