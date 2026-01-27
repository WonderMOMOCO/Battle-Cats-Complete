use eframe::egui;

pub fn show(ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);

            ui.heading(
                egui::RichText::new("Battle Cats Complete")
                    .size(40.0)
                    .color(egui::Color32::WHITE)
                    .strong()
            );

            ui.add_space(20.0);
            ui.label(egui::RichText::new("User-Handled Battle Cats Database").size(16.0));
        });
    });

    egui::Area::new("version_area".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]) 
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );
            ui.label(format!("v{}", env!("CARGO_PKG_VERSION")));
        });

    egui::Area::new("social_links_area".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0]) 
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.style_mut().text_styles.insert(
                    egui::TextStyle::Body, 
                    egui::FontId::new(13.0, egui::FontFamily::Proportional),
                );
                
                if ui.hyperlink_to("Discord", "https://discord.com/invite/SNSE8HNhmP").clicked() { }
                ui.label("|");
                ui.hyperlink_to("GitHub", "https://github.com/WonderMOMOCO/Battle-Cats-Complete");
            });
        });
}