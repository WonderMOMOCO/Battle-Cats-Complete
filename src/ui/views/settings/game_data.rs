use eframe::egui;
use crate::core::settings::Settings;
use super::tabs::toggle_ui;

pub fn show(ui: &mut egui::Ui, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;
    egui::ScrollArea::vertical()
        .id_salt("game_data_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {

            ui.heading("Android");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let tooltip = "Attempt to connect to this IP Address Wirelessly if not automatically found when using Android import method\nMake sure you have \"Wireless USB Debugging\" enabled in your devices developer settings\nRequires ABD OEM Drivers Add-On to function";
                
                ui.label("Fallback IP Address").on_hover_text(tooltip);
                
                ui.spacing_mut().item_spacing.x = 4.0; 

                ui.allocate_ui(egui::vec2(100.0, 20.0), |ui| {
                    ui.centered_and_justified(|ui| {
                        if settings.show_ip_field {
                            let hint = egui::RichText::new("192.168.X.X").color(egui::Color32::GRAY);
                            
                            ui.add(egui::TextEdit::singleline(&mut settings.manual_ip)
                                .hint_text(hint)
                                .vertical_align(egui::Align::Center))
                                .on_hover_text(tooltip); 
                        } else {
                            if ui.button("Click to Reveal")
                                .on_hover_text(tooltip)
                                .clicked() 
                            {
                                settings.show_ip_field = true;
                            }
                        }
                    });
                });

                ui.add_space(2.0);

                if ui.button("👁").on_hover_text("Toggle Visibility").clicked() {
                    settings.show_ip_field = !settings.show_ip_field;
                }
            });
            ui.add_space(5.0);
            // --------------------------

            ui.horizontal(|ui| {
                let label_response = ui.label("App Folder Persistence");
                let tooltip_text = "Skip the deletion of the \"game/app\" directory after android import";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(ui, &mut settings.app_folder_persistence)
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
                
                let tooltip_text = "Allows compression levels up to 21\n\
                                    WARNING: Levels above 15 require significant RAM and time";
                
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(ui, &mut settings.enable_ultra_compression)
                    .on_hover_text(tooltip_text);

                if toggle_response.changed() {
                    refresh_needed = true;
                    
                    if !settings.enable_ultra_compression && settings.last_compression_level > 15 {
                        settings.last_compression_level = 15;
                    }
                }
            });
    });

    refresh_needed
}