use std::path::Path;
use eframe::egui;
use crate::features::stage::data::stage::{BossType, EnemyAmount};
use crate::global::utils::autocrop;

pub fn format_enemy_amount(spawn_amount: &EnemyAmount) -> String {
    match spawn_amount {
        EnemyAmount::Infinite => "∞".to_string(),
        EnemyAmount::Limit(limited_amount) => limited_amount.to_string(),
    }
}

pub fn format_enemy_respawn(spawn_amount: &EnemyAmount, respawn_min_frames: u32, respawn_max_frames: u32) -> String {
    let is_singular_enemy_spawn = spawn_amount == &EnemyAmount::Limit(1);
    if is_singular_enemy_spawn {
        return "-".to_string();
    }
    
    if respawn_min_frames == respawn_max_frames {
        return format!("{}f", respawn_min_frames);
    }
    
    format!("{}f ~ {}f", respawn_min_frames, respawn_max_frames)
}

pub fn format_boss_type(boss_type: &BossType) -> String {
    match boss_type {
        BossType::None => "-".to_string(),
        BossType::Boss => "Yes".to_string(),
        BossType::ScreenShake => "Yes (Shake)".to_string(),
        BossType::Unknown(_) => "Unknown".to_string(),
    }
}

pub fn format_kill_count(kill_count: u32) -> String {
    if kill_count == 0 {
        return "-".to_string();
    }
    kill_count.to_string()
}

pub fn format_base_hp_percentage(base_hp_percentage: u32) -> String {
    if base_hp_percentage == 100 {
        return "-".to_string();
    }
    format!("{}%", base_hp_percentage)
}

pub fn process_enemy_icon_texture(icon_file_path: &Path) -> Option<egui::ColorImage> {
    let Ok(loaded_raw_image_data) = image::open(icon_file_path) else {
        return None;
    };
    
    let autocropped_rgba_image = autocrop(loaded_raw_image_data.to_rgba8());
    let image_dimensions = [autocropped_rgba_image.width() as usize, autocropped_rgba_image.height() as usize];
    
    Some(egui::ColorImage::from_rgba_unmultiplied(image_dimensions, autocropped_rgba_image.as_flat_samples().as_slice()))
}