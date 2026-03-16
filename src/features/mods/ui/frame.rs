use eframe::egui;
use crate::features::settings::logic::Settings;
use crate::features::mods::logic::state::ModState;
use super::{details, import};

pub fn show(ctx: &egui::Context, state: &mut ModState, settings: &mut Settings) {
    let mut list = state.list.take().unwrap_or_default();

    egui::SidePanel::left("mod_list_panel")
        .resizable(false)
        .exact_width(160.0) 
        .show(ctx, |ui| {
            list.render(ui, state, settings);
        });

    state.list = Some(list);

    egui::CentralPanel::default().show(ctx, |ui| {
        details::render(ui, state, settings);
    });

    if state.import.is_open {
        import::show(ctx, state, settings);
    }
}