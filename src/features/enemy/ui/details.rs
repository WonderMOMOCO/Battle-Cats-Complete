use eframe::egui;

pub fn render(ui: &mut egui::Ui, description: &[String]) {
    ui.add_space(10.0);
    ui.vertical_centered(|ui| {
        ui.heading(egui::RichText::new("Description").size(20.0).strong());
    });
    ui.add_space(8.0);

    ui.vertical_centered(|ui| {
        if description.is_empty() {
            ui.label(egui::RichText::new("No description available").weak().italics());
            
            // Pad the remaining 3 lines to hold the seat
            for _ in 1..4 {
                ui.label(" ");
            }
        } else {
            // Force exactly 4 lines to render for consistent UI height
            for i in 0..4 {
                if let Some(line) = description.get(i) {
                    if line.trim().is_empty() {
                        ui.label(" "); 
                    } else {
                        ui.add(egui::Label::new(egui::RichText::new(line).size(15.0)).wrap());
                    }
                } else {
                    // Seat holder for missing lines
                    ui.label(" "); 
                }
            }
        }
    });

    ui.add_space(15.0);
    ui.separator(); 
    ui.add_space(10.0);
}