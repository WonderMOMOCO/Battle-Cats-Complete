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

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(12.0)
                        .color(egui::Color32::GRAY)
                );
            });
        });
    });
}