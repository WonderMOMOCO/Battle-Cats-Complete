use eframe::egui;
use crate::core::cat::scanner::CatEntry;
use crate::core::cat::stats::CatRaw;
use crate::ui::components::stat_grid::{grid_cell, grid_cell_custom, render_frames};

pub fn render(ui: &mut egui::Ui, cat: &CatEntry, s: &CatRaw, form: usize, level: i32) {
    let curve = cat.curve.as_ref();
    let hp = curve.map_or(s.hitpoints, |c| c.calculate_stat(s.hitpoints, level));
    let atk_1 = curve.map_or(s.attack_1, |c| c.calculate_stat(s.attack_1, level));
    let atk_2 = curve.map_or(s.attack_2, |c| c.calculate_stat(s.attack_2, level));
    let atk_3 = curve.map_or(s.attack_3, |c| c.calculate_stat(s.attack_3, level));
    
    let total_atk = atk_1 + atk_2 + atk_3;
    let cycle = s.attack_cycle(cat.atk_anim_frames[form]);
    let dps = if cycle > 0 { (total_atk as f32 * 30.0 / cycle as f32) as i32 } else { 0 };
    let atk_type = if s.area_attack == 0 { "Single" } else { "Area" };

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
                grid_cell(ui, &s.standing_range.to_string(), false);
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
                let cd_val = s.effective_cooldown();
                grid_cell(ui, &hp.to_string(), false); 
                grid_cell(ui, &s.knockbacks.to_string(), false); 
                grid_cell(ui, &s.speed.to_string(), false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cd_val, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cd_val, cell_w)
                ); 
                grid_cell(ui, &format!("{}¢", s.eoc1_cost * 3 / 2), false); 
                ui.end_row();
            });
    });
}