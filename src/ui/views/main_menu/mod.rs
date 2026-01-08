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
}