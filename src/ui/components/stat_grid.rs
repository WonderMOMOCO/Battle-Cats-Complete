use eframe::egui;

pub fn grid_cell_custom<F>(
    ui: &mut egui::Ui, 
    is_header: bool, 
    tooltip_renderer: Option<Box<dyn Fn(&mut egui::Ui)>>, 
    add_contents: F
) where F: FnOnce(&mut egui::Ui) {
    let bg = if is_header { egui::Color32::from_gray(20) } else { egui::Color32::from_gray(60) };
    
    let response = egui::Frame::none().fill(bg).rounding(4.0).inner_margin(1.5).show(ui, |ui| {
        ui.set_min_width(60.0);
        ui.vertical_centered(|ui| add_contents(ui));
    }).response;

    if let Some(renderer) = tooltip_renderer {
        response.on_hover_ui(|ui| {
            renderer(ui);
        });
    }
}

pub fn grid_cell(ui: &mut egui::Ui, text: &str, is_header: bool) {
    let text_clone = text.to_string();
    grid_cell_custom(ui, is_header, 
        Some(Box::new(move |ui| { ui.label(&text_clone); })), 
        |ui| {
            let rt = if is_header { egui::RichText::new(text).strong() } else { egui::RichText::new(text) };
            ui.label(rt);
        }
    );
}

pub fn render_frames(ui: &mut egui::Ui, frames: i32, max_width: f32) {
    let seconds = frames as f32 / 30.0;
    let body_font = ui.style().text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or(egui::FontId::proportional(14.0));
    
    let mut job = egui::text::LayoutJob::default();
    job.append(&format!("{:.2}s", seconds), 0.0, egui::TextFormat {
        font_id: body_font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    });
    job.append(&format!(" {}f", frames), 0.0, egui::TextFormat {
        font_id: egui::FontId::proportional(body_font.size * 0.65), 
        color: egui::Color32::from_gray(200),
        valign: egui::Align::Center, 
        ..Default::default()
    });

    let galley = ui.fonts(|f| f.layout_job(job.clone()));
    
    if galley.rect.width() > max_width {
        ui.label(format!("{:.2}s", seconds));
    } else {
        ui.label(job);
    }
}