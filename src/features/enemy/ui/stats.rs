use eframe::egui;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::global::ui::stat_grid::{grid_cell, grid_cell_custom, render_frames};
use crate::features::enemy::registry::{get_enemy_stat, format_enemy_stat, Magnification};

pub fn render(ui: &mut egui::Ui, enemy: &EnemyEntry, magnification: Magnification) {
    let stats = &enemy.stats;
    let frames = enemy.atk_anim_frames;

    let atk_str = format_enemy_stat("Attack", stats, frames, magnification);
    let dps_str = format_enemy_stat("Dps", stats, frames, magnification);
    let range_str = format_enemy_stat("Range", stats, frames, magnification);
    let cycle = (get_enemy_stat("Atk Cycle").get_value)(stats, frames, magnification);
    let atk_type = format_enemy_stat("Atk Type", stats, frames, magnification);
    
    let hp_str = format_enemy_stat("Hitpoints", stats, frames, magnification);
    let kb_str = format_enemy_stat("Knockbacks", stats, frames, magnification);
    let speed_str = format_enemy_stat("Speed", stats, frames, magnification);
    let endure_str = format_enemy_stat("Endure", stats, frames, magnification);
    let cash_str = format_enemy_stat("Cash Drop", stats, frames, magnification);

    let cell_w = 60.0;

    ui.horizontal_top(|ui| {
        egui::Grid::new("enemy_stats_grid")
            .min_col_width(cell_w)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                grid_cell(ui, get_enemy_stat("Attack").display_name, true);
                grid_cell(ui, get_enemy_stat("Dps").display_name, true);
                grid_cell(ui, get_enemy_stat("Range").display_name, true);
                grid_cell(ui, get_enemy_stat("Atk Cycle").display_name, true);
                grid_cell(ui, get_enemy_stat("Atk Type").display_name, true); 
                ui.end_row();
                
                grid_cell(ui, &atk_str, false); 
                grid_cell(ui, &dps_str, false); 
                grid_cell(ui, &range_str, false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cycle, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cycle, cell_w)
                ); 
                grid_cell(ui, &atk_type, false); 
                ui.end_row();

                grid_cell(ui, get_enemy_stat("Hitpoints").display_name, true);
                grid_cell(ui, get_enemy_stat("Knockbacks").display_name, true);
                grid_cell(ui, get_enemy_stat("Speed").display_name, true);
                grid_cell(ui, get_enemy_stat("Endure").display_name, true); 
                grid_cell(ui, get_enemy_stat("Cash Drop").display_name, true);
                ui.end_row();
                
                grid_cell(ui, &hp_str, false); 
                grid_cell(ui, &kb_str, false); 
                grid_cell(ui, &speed_str, false);
                grid_cell(ui, &endure_str, false); 
                grid_cell(ui, &cash_str, false); 
                ui.end_row();
            });
    });
}