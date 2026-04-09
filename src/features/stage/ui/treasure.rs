use eframe::egui;
use crate::features::stage::registry::Stage;
use crate::features::stage::data::mapstagedata::RewardStructure;

fn get_treasure_rule_desc(rule: i32) -> &'static str {
    match rule {
        1 => "Once, Then Unlimited",
        0 => "Unlimited",
        -1 => "Raw Percentages (Unlimited)",
        -3 => "Guaranteed (Once)",
        -4 => "Guaranteed (Unlimited)",
        _ => "Unknown Rule",
    }
}

pub fn center_header(ui: &mut egui::Ui, text: &str) {
    ui.vertical_centered(|ui| {
        ui.add(egui::Label::new(egui::RichText::new(text).strong()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn center_text(ui: &mut egui::Ui, text: impl Into<String>) {
    ui.vertical_centered(|ui| {
        ui.add(egui::Label::new(text.into()).wrap_mode(egui::TextWrapMode::Extend));
    });
}

pub fn draw(ui: &mut egui::Ui, stage: &Stage) {
    match &stage.rewards {
        RewardStructure::Treasure { drop_rule, drops } => {
            let rule_desc = get_treasure_rule_desc(*drop_rule);
            ui.strong(format!("Treasure | {}", rule_desc));
            ui.separator();
            
            if drops.is_empty() {
                ui.label("No drops configured.");
                return;
            }

            egui::Grid::new("reward_treasure_grid").striped(true).spacing([15.0, 4.0]).show(ui, |ui| {
                center_header(ui, "Chance");
                center_header(ui, "Item ID");
                center_header(ui, "Amount");
                ui.end_row();

                for drop in drops {
                    center_text(ui, format!("{}%", drop.chance));
                    center_text(ui, drop.id.to_string());
                    center_text(ui, drop.amount.to_string());
                    ui.end_row();
                }
            });
        }
        RewardStructure::Timed(scores) => {
            ui.strong("Timed Score Rewards");
            ui.separator();
            
            if scores.is_empty() {
                ui.label("No timed rewards configured.");
                return;
            }

            egui::Grid::new("reward_timed_grid").striped(true).spacing([15.0, 4.0]).show(ui, |ui| {
                center_header(ui, "Score Required");
                center_header(ui, "Item ID");
                center_header(ui, "Amount");
                ui.end_row();

                for score in scores {
                    center_text(ui, score.score.to_string());
                    center_text(ui, score.id.to_string());
                    center_text(ui, score.amount.to_string());
                    ui.end_row();
                }
            });
        }
        RewardStructure::None => {
            ui.strong("Rewards");
            ui.separator();
            ui.label("No rewards for this stage.");
        }
    }
}