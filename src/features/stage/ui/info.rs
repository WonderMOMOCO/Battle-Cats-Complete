use eframe::egui;
use crate::features::stage::registry::Stage;
use crate::features::stage::logic::info as info_logic;

fn center_header(ui: &mut egui::Ui, display_text: &str) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(display_text).strong()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

fn center_text(ui: &mut egui::Ui, display_text: impl Into<String>) {
    ui.centered_and_justified(|ui| {
        ui.add(egui::Label::new(display_text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn draw(ui: &mut egui::Ui, stage_data: &Stage) {
    ui.strong("General Information");
    ui.separator();

    let energy_header = if stage_data.category == "B" { "Catamin" } else { "Energy" };
    let formatted_energy_value = info_logic::format_energy_cost(&stage_data.category, stage_data.energy);
    let formatted_difficulty = info_logic::format_difficulty_level(stage_data.difficulty);
    let formatted_crown = info_logic::format_crown_display(stage_data.target_crowns, stage_data.max_crowns);
    let formatted_no_continues = info_logic::format_boolean_status(stage_data.is_no_continues, "Yes", "No");
    let formatted_indestructible = info_logic::format_boolean_status(stage_data.is_base_indestructible, "Active", "-");
    let formatted_boss_track = info_logic::format_boss_track(stage_data.boss_track, stage_data.bgm_change_percent);
    let (base_header, formatted_base_value) = info_logic::format_base_display(stage_data.anim_base_id, stage_data.base_id);

    egui::Grid::new("stage_meta_grid")
        .striped(true)
        .spacing([15.0, 8.0])
        .show(ui, |grid| {
            center_header(grid, "Base HP");
            center_header(grid, energy_header);
            center_header(grid, "XP Base");
            center_header(grid, "Width");
            center_header(grid, "Max Enemy");
            center_header(grid, "Min Respawn");
            center_header(grid, "Difficulty");
            grid.end_row();

            center_text(grid, stage_data.base_hp.to_string());
            center_text(grid, formatted_energy_value);
            center_text(grid, stage_data.xp.to_string());
            center_text(grid, stage_data.width.to_string());
            center_text(grid, stage_data.max_enemies.to_string());
            center_text(grid, format!("{}f", stage_data.min_spawn));
            center_text(grid, formatted_difficulty);
            grid.end_row();

            center_header(grid, "No Cont.");
            center_header(grid, "Boss Guard");
            center_header(grid, &base_header);
            center_header(grid, "BG ID");
            center_header(grid, "BGM");
            center_header(grid, "Boss BGM");
            center_header(grid, "Crowns");
            grid.end_row();

            center_text(grid, formatted_no_continues);
            center_text(grid, formatted_indestructible);
            center_text(grid, formatted_base_value);
            center_text(grid, stage_data.background_id.to_string());
            center_text(grid, stage_data.init_track.to_string());
            center_text(grid, formatted_boss_track);
            center_text(grid, formatted_crown);
            grid.end_row();
        });
}