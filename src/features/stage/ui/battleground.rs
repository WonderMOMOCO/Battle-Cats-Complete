use eframe::egui;
use std::collections::HashMap;
use crate::features::stage::registry::Stage;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::stage::logic::battleground as bg_logic;
use super::treasure::center_header;

fn center_enemy_text(ui: &mut egui::Ui, display_text: impl Into<String>) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(display_text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn draw(
    egui_context: &egui::Context, 
    ui: &mut egui::Ui, 
    stage_data: &Stage,
    enemy_registry: &HashMap<u32, EnemyEntry>,
    texture_cache: &mut HashMap<u32, egui::TextureHandle>
) {
    ui.strong("Enemy Layout");
    ui.separator();

    if stage_data.enemies.is_empty() {
        ui.label("No enemies defined for this stage.");
        return;
    }

    egui::Grid::new("enemy_grid")
        .striped(true)
        .spacing([15.0, 4.0])
        .min_row_height(32.0) 
        .show(ui, |grid| {
            center_header(grid, "Enemy");
            center_header(grid, "Count");
            center_header(grid, "HP %");
            center_header(grid, "Atk %");
            center_header(grid, "Base %");
            center_header(grid, "Spawn");
            center_header(grid, "Respawn");
            center_header(grid, "Boss");
            center_header(grid, "Kills"); 
            grid.end_row();

            for enemy_data in &stage_data.enemies {
                grid.with_layout(egui::Layout::bottom_up(egui::Align::Center), |icon_layout| {
                    let mut has_rendered_icon = false;
                    
                    if let Some(located_enemy_entry) = enemy_registry.get(&enemy_data.id) {
                        if let Some(enemy_icon_path) = &located_enemy_entry.icon_path {
                            
                            if !texture_cache.contains_key(&enemy_data.id) {
                                if let Some(processed_color_image) = bg_logic::process_enemy_icon_texture(enemy_icon_path) {
                                    let generated_texture_handle = egui_context.load_texture(format!("stage_enemy_icon_{}", enemy_data.id), processed_color_image, egui::TextureOptions::LINEAR);
                                    texture_cache.insert(enemy_data.id, generated_texture_handle);
                                }
                            }

                            if let Some(cached_texture_handle) = texture_cache.get(&enemy_data.id) {
                                icon_layout.add(egui::Image::new(cached_texture_handle).max_size(egui::vec2(32.0, 32.0)));
                                has_rendered_icon = true;
                            }
                        }
                    }

                    if !has_rendered_icon {
                        icon_layout.add_space(6.0);
                        icon_layout.add(egui::Label::new(format!("{:03}", enemy_data.id)).wrap_mode(egui::TextWrapMode::Extend));
                    }
                });

                let formatted_amount = bg_logic::format_enemy_amount(&enemy_data.amount);
                let formatted_base_hp = bg_logic::format_base_hp_percentage(enemy_data.base_hp_perc);
                let formatted_respawn = bg_logic::format_enemy_respawn(&enemy_data.amount, enemy_data.respawn_min, enemy_data.respawn_max);
                let formatted_boss_type = bg_logic::format_boss_type(&enemy_data.boss_type);
                let formatted_kill_count = bg_logic::format_kill_count(enemy_data.kill_count);

                center_enemy_text(grid, formatted_amount);
                center_enemy_text(grid, format!("{}%", enemy_data.magnification));
                center_enemy_text(grid, format!("{}%", enemy_data.atk_magnification));
                center_enemy_text(grid, formatted_base_hp);
                center_enemy_text(grid, format!("{}f", enemy_data.start_frame));
                center_enemy_text(grid, formatted_respawn);
                center_enemy_text(grid, formatted_boss_type);
                center_enemy_text(grid, formatted_kill_count);

                grid.end_row();
            }
        });
}