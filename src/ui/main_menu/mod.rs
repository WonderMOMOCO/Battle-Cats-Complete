use eframe::egui;
use crate::core::utils::DragGuard;

mod changelog;

pub fn show(ctx: &egui::Context, drag_guard: &mut DragGuard) {
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
            ui.label(egui::RichText::new("All-In-One Battle Cats Toolkit").size(16.0));
        });
    });

    egui::Area::new("version_area".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .order(egui::Order::Background) 
        .show(ctx, |ui| {
            ui.style_mut().text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            );

            let current_version = env!("CARGO_PKG_VERSION");
            let tag = format!("v{}", current_version);
            let release_url = format!("https://github.com/WonderMOMOCO/Battle-Cats-Complete/releases/tag/{}", tag);

            ui.horizontal(|ui| {
                ui.hyperlink_to(&tag, release_url);
                ui.label("|");

                changelog::link(ui, ctx);
            });
        });

    egui::Area::new("social_links_area".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0]) 
        .order(egui::Order::Background)
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

    changelog::window(ctx, drag_guard);
}