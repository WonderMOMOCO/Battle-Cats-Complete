use eframe::egui;
use crate::features::stage::registry::Stage;

fn center_header(ui: &mut egui::Ui, text: &str) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add(egui::Label::new(egui::RichText::new(text).strong()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

fn center_text(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
        ui.add(egui::Label::new(text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn draw(ui: &mut egui::Ui, stage: &Stage) {
    ui.strong("General Information");
    ui.separator();

    egui::Grid::new("stage_meta_grid")
        .striped(true)
        .spacing([15.0, 8.0])
        .show(ui, |ui| {
            // ROW 1: HEADERS
            center_header(ui, "Base HP");
            center_header(ui, "Energy");
            center_header(ui, "XP Base");
            center_header(ui, "Width");
            center_header(ui, "Max Enemy");
            center_header(ui, "No Cont.");
            ui.end_row();

            // ROW 1: VALUES
            center_text(ui, stage.base_hp.to_string());
            center_text(ui, stage.energy.to_string());
            center_text(ui, stage.xp.to_string());
            center_text(ui, stage.width.to_string());
            center_text(ui, stage.max_enemies.to_string());
            center_text(ui, if stage.is_no_continues { "Yes" } else { "No" });
            ui.end_row();

            // ROW 2: HEADERS
            center_header(ui, "Boss Guard");
            
            if stage.anim_base_id != 0 {
                center_header(ui, "Anim Base");
            } else {
                center_header(ui, "Base Img");
            }
            
            center_header(ui, "BG ID");
            center_header(ui, "BGM");
            center_header(ui, "Boss BGM");
            center_header(ui, "");
            ui.end_row();

            // ROW 2: VALUE
            center_text(ui, if stage.is_base_indestructible { "Active" } else { "-" });
            
            if stage.anim_base_id != 0 {
                let actual_enemy_id = if stage.anim_base_id >= 2 { stage.anim_base_id - 2 } else { 0 };
                center_text(ui, format!("E-{:03}", actual_enemy_id));
            } else {
                center_text(ui, stage.base_id.to_string());
            }

            center_text(ui, stage.background_id.to_string());
            center_text(ui, stage.init_track.to_string());
            
            if stage.boss_track == 0 && stage.bgm_change_percent == 0 {
                center_text(ui, "-");
            } else {
                center_text(ui, format!("Trk {} ({}%)", stage.boss_track, stage.bgm_change_percent));
            }
            
            center_text(ui, "");
            ui.end_row();
        });
}