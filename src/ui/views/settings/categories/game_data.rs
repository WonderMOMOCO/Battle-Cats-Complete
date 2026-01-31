use eframe::egui;
use crate::core::settings::Settings;

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;

    ui.add_space(5.0);
    ui.heading("Emulator");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        let label_response = ui.label("App Folder Persistence");
        let tooltip_text = "Skip the deletion of the \"game/app\" directory after emulator import";
        label_response.on_hover_text(tooltip_text);

        let toggle_response = crate::ui::views::settings::toggle_ui(ui, &mut settings.app_folder_persistence)
            .on_hover_text(tooltip_text);

        if toggle_response.changed() {
            refresh_needed = true;
        }
    });

    ui.add_space(20.0);

    ui.heading("Export");
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        let label_response = ui.label("Enable Ultra Compression");
        
        let tooltip_text = "Allows compression levels up to 21.\n\
                            WARNING: Levels above 15 require significant RAM and time.";
        
        label_response.on_hover_text(tooltip_text);

        let toggle_response = crate::ui::views::settings::toggle_ui(ui, &mut settings.enable_ultra_compression)
            .on_hover_text(tooltip_text);

        if toggle_response.changed() {
            refresh_needed = true;
            
            if !settings.enable_ultra_compression && settings.last_compression_level > 15 {
                settings.last_compression_level = 15;
            }
        }
    });

    refresh_needed
}