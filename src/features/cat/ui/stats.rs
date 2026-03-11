use eframe::egui;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::stats::CatRaw;
use crate::ui::components::stat_grid::{grid_cell, grid_cell_custom, render_frames};
use crate::features::cat::registry::{get_cat_stat, format_cat_stat};

pub fn render(
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    final_stats: &CatRaw, 
    form: usize
) {
    let anim_frames = cat.atk_anim_frames[form];

    let atk_str = format_cat_stat("Attack", final_stats, anim_frames);
    let dps_str = format_cat_stat("Dps", final_stats, anim_frames);
    let range_str = format_cat_stat("Range", final_stats, anim_frames);
    let atk_type = format_cat_stat("Atk Type", final_stats, anim_frames);
    
    let hp_str = format_cat_stat("Hitpoints", final_stats, anim_frames);
    let kb_str = format_cat_stat("Knockbacks", final_stats, anim_frames);
    let speed_str = format_cat_stat("Speed", final_stats, anim_frames);
    let cost_str = format_cat_stat("Cost", final_stats, anim_frames);

    let cycle = (get_cat_stat("Atk Cycle").get_value)(final_stats, anim_frames);
    let cd_val = (get_cat_stat("Cooldown").get_value)(final_stats, anim_frames);

    let cell_w = 60.0;

    // Stats Grid
    ui.horizontal_top(|ui| {
        egui::Grid::new("stats_grid_right")
            .min_col_width(cell_w)
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                // Row 1 Header
                grid_cell(ui, get_cat_stat("Attack").display_name, true);
                grid_cell(ui, get_cat_stat("Dps").display_name, true);
                grid_cell(ui, get_cat_stat("Range").display_name, true);
                grid_cell(ui, get_cat_stat("Atk Cycle").display_name, true);
                grid_cell(ui, get_cat_stat("Atk Type").display_name, true); 
                ui.end_row();
                
                // Row 1 Data
                grid_cell(ui, &atk_str, false); 
                grid_cell(ui, &dps_str, false); 
                grid_cell(ui, &range_str, false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cycle, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cycle, cell_w)
                ); 
                grid_cell(ui, &atk_type, false); 
                ui.end_row();

                // Row 2 Header
                grid_cell(ui, get_cat_stat("Hitpoints").display_name, true);
                grid_cell(ui, get_cat_stat("Knockbacks").display_name, true);
                grid_cell(ui, get_cat_stat("Speed").display_name, true);
                grid_cell(ui, get_cat_stat("Cooldown").display_name, true);
                grid_cell(ui, get_cat_stat("Cost").display_name, true); 
                ui.end_row();
                
                // Row 2 Data
                grid_cell(ui, &hp_str, false); 
                grid_cell(ui, &kb_str, false); 
                grid_cell(ui, &speed_str, false);
                grid_cell_custom(ui, false, 
                    Some(Box::new(move |ui| { ui.vertical_centered(|ui| render_frames(ui, cd_val, f32::INFINITY)); })), 
                    |ui| render_frames(ui, cd_val, cell_w)
                ); 
                grid_cell(ui, &cost_str, false); 
                ui.end_row();
            });
    });
}