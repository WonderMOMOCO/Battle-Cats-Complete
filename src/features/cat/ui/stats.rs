use eframe::egui;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::stats::CatRaw;
use crate::ui::components::stat_grid::{grid_cell, grid_cell_custom, render_frames};

pub fn render(
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    final_stats: &CatRaw, 
    form: usize
) {
    let total_atk = final_stats.attack_1 + final_stats.attack_2 + final_stats.attack_3;
    let cycle = final_stats.attack_cycle(cat.atk_anim_frames[form]);
    let dps = if cycle > 0 { (total_atk as f32 * 30.0 / cycle as f32) as i32 } else { 0 };
    let atk_type = if final_stats.area_attack == 0 { "Single" } else { "Area" };

    let cell_w = 60.0;

    // Stats Grid
    ui.horizontal_top(|ui| {
        egui::Grid::new("stats_grid_right")
            .min_col_width(cell_w)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                // Row 1 Header
                grid_cell(ui, "Atk", true);
                grid_cell(ui, "Dps", true);
                grid_cell(ui, "Range", true);
                grid_cell(ui, "Atk Cycle", true);
                grid_cell(ui, "Atk Type", true); 
                ui.end_row();
                
                // Row 1 Data
                grid_cell(ui, &total_atk.to_string(), false); 
                grid_cell(ui, &dps.to_string(), false); 
                grid_cell(ui, &final_stats.standing_range.to_string(), false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cycle, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cycle, cell_w)
                ); 
                grid_cell(ui, atk_type, false); 
                ui.end_row();

                // Row 2 Header
                grid_cell(ui, "Hp", true);
                grid_cell(ui, "Kb", true);
                grid_cell(ui, "Speed", true);
                grid_cell(ui, "Cooldown", true);
                grid_cell(ui, "Cost", true); 
                ui.end_row();
                
                // Row 2 Data
                let cd_val = final_stats.effective_cooldown();
                grid_cell(ui, &final_stats.hitpoints.to_string(), false); 
                grid_cell(ui, &final_stats.knockbacks.to_string(), false); 
                grid_cell(ui, &final_stats.speed.to_string(), false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cd_val, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cd_val, cell_w)
                ); 
                grid_cell(ui, &format!("{}¢", final_stats.eoc1_cost * 3 / 2), false); 
                ui.end_row();
            });
    });
}