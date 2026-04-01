use eframe::egui;
use crate::features::import::logic::{ImportState, ImportSubTab};
use crate::features::settings::logic::Settings;
use crate::features::import::ui::{adb_view, raw_view, decrypt_view};

pub fn show(ui: &mut egui::Ui, state: &mut ImportState, settings: &mut Settings) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;

        let active_color = egui::Color32::from_rgb(31, 106, 165);
        let inactive_color = egui::Color32::from_gray(60);

        let is_adb = state.import_sub_tab == ImportSubTab::Emulator;
        if ui.add(egui::Button::new(egui::RichText::new("Android").color(egui::Color32::WHITE).size(14.0))
            .fill(if is_adb { active_color } else { inactive_color })
            .min_size(egui::vec2(80.0, 30.0)))
            .clicked() 
        {
            state.import_sub_tab = ImportSubTab::Emulator;
        }

        let is_decrypt = state.import_sub_tab == ImportSubTab::Decrypt;
        if ui.add(egui::Button::new(egui::RichText::new("Pack").color(egui::Color32::WHITE).size(14.0))
            .fill(if is_decrypt { active_color } else { inactive_color })
            .min_size(egui::vec2(80.0, 30.0)))
            .clicked() 
        {
            state.import_sub_tab = ImportSubTab::Decrypt;
        }

        let is_sort = state.import_sub_tab == ImportSubTab::Sort;
        if ui.add(egui::Button::new(egui::RichText::new("Raw").color(egui::Color32::WHITE).size(14.0))
            .fill(if is_sort { active_color } else { inactive_color })
            .min_size(egui::vec2(80.0, 30.0)))
            .clicked() 
        {
            state.import_sub_tab = ImportSubTab::Sort;
        }
    });

    ui.add_space(15.0);

    match state.import_sub_tab {
        ImportSubTab::Emulator => adb_view::show(ui, state, settings),
        ImportSubTab::Decrypt => decrypt_view::show(ui, state),
        ImportSubTab::Sort => raw_view::show(ui, state),
    }
}