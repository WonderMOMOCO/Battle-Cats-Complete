use eframe::egui;
use std::collections::HashMap;
use crate::features::stage::registry::Stage;
use crate::features::stage::data::stage::{BossType, EnemyAmount};
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::global::utils::autocrop;
use super::treasure::center_header;

fn center_enemy_text(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add_space(8.0);
        ui.add(egui::Label::new(text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn draw(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    stage: &Stage,
    enemy_registry: &HashMap<u32, EnemyEntry>,
    texture_cache: &mut HashMap<u32, egui::TextureHandle>
) {
    ui.strong("Enemy Layout");
    ui.separator();

    if stage.enemies.is_empty() {
        ui.label("No enemies defined for this stage.");
        return;
    }

    egui::Grid::new("enemy_grid")
        .striped(true)
        .spacing([15.0, 4.0])
        .min_row_height(32.0) 
        .show(ui, |ui| {
            center_header(ui, "Enemy");
            center_header(ui, "Count");
            center_header(ui, "HP %");
            center_header(ui, "Atk %");
            center_header(ui, "Base %");
            center_header(ui, "Spawn");
            center_header(ui, "Respawn");
            center_header(ui, "Boss");
            center_header(ui, "Kills"); 
            ui.end_row();

            for enemy in &stage.enemies {
                ui.allocate_ui_with_layout(
                    egui::vec2(32.0, 32.0),
                    egui::Layout::bottom_up(egui::Align::Center), 
                    |ui| {
                        let mut rendered_icon = false;
                        if let Some(entry) = enemy_registry.get(&enemy.id) {
                            if let Some(icon_path) = &entry.icon_path {
                                
                                if !texture_cache.contains_key(&enemy.id) {
                                    if let Ok(img) = image::open(icon_path) {
                                        let rgba = autocrop(img.to_rgba8());
                                        let size = [rgba.width() as usize, rgba.height() as usize];
                                        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_flat_samples().as_slice());
                                        let tex = ctx.load_texture(format!("stage_enemy_icon_{}", enemy.id), color_image, egui::TextureOptions::LINEAR);
                                        texture_cache.insert(enemy.id, tex);
                                    }
                                }

                                if let Some(tex) = texture_cache.get(&enemy.id) {
                                    ui.add(egui::Image::new(tex).max_size(egui::vec2(32.0, 32.0)));
                                    rendered_icon = true;
                                }
                            }
                        }

                        if !rendered_icon {
                            ui.add_space(6.0);
                            ui.add(egui::Label::new(format!("{:03}", enemy.id)).wrap_mode(egui::TextWrapMode::Extend));
                        }
                    }
                );

                let amount_str = match enemy.amount {
                    EnemyAmount::Infinite => "∞".to_string(),
                    EnemyAmount::Limit(n) => n.to_string(),
                };
                center_enemy_text(ui, amount_str);
                
                center_enemy_text(ui, format!("{}%", enemy.magnification));
                center_enemy_text(ui, format!("{}%", enemy.atk_magnification));

                let base_hp_str = if enemy.base_hp_perc == 100 { "-".to_string() } else { format!("{}%", enemy.base_hp_perc) };
                center_enemy_text(ui, base_hp_str);

                center_enemy_text(ui, format!("{}f", enemy.start_frame));
                
                let respawn_str = if enemy.amount == EnemyAmount::Limit(1) {
                    "-".to_string() 
                } else if enemy.respawn_min == enemy.respawn_max {
                    format!("{}f", enemy.respawn_min)
                } else {
                    format!("{}f ~ {}f", enemy.respawn_min, enemy.respawn_max)
                };
                center_enemy_text(ui, respawn_str);

                let boss_str = if enemy.is_base {
                    "Base".to_string()
                } else {
                    match enemy.boss_type {
                        BossType::None => "-".to_string(),
                        BossType::Boss => "Yes".to_string(),
                        BossType::ScreenShake => "Yes (Shake)".to_string(),
                        BossType::Unknown(_) => "Unknown".to_string(),
                    }
                };
                center_enemy_text(ui, boss_str);

                center_enemy_text(ui, if enemy.kill_count == 0 { "-".to_string() } else { enemy.kill_count.to_string() });

                ui.end_row();
            }
        });
}