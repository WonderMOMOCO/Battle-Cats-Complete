use eframe::egui;

pub fn paint_fallback_at(ui: &mut egui::Ui, rect: egui::Rect, text: &str, border_color: egui::Color32) {
    if !ui.is_rect_visible(rect) { return; }

    ui.painter().rect_stroke(
        rect,
        5.5,
        egui::Stroke::new(1.5, border_color),
    );

    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(10.0),
        ui.visuals().text_color(),
    );
}

pub fn render_fallback_icon(ui: &mut egui::Ui, text: &str, border_color: egui::Color32) -> egui::Response {
    let size = egui::vec2(40.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint_fallback_at(ui, rect, text, border_color);
    response
}

pub fn text_with_superscript(ui: &mut egui::Ui, text: &str) {
    if !text.contains('^') {
        ui.label(text);
        return;
    }

    let parts: Vec<&str> = text.split('^').collect();
    if parts.len() < 2 {
        ui.label(text);
        return;
    }

    let body_font = ui.style().text_styles.get(&egui::TextStyle::Body)
        .cloned().unwrap_or(egui::FontId::proportional(14.0));
        
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.spacing().tooltip_width;

    job.append(parts[0], 0.0, egui::TextFormat {
        font_id: body_font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    });

    job.append(parts[1], 0.0, egui::TextFormat {
        font_id: egui::FontId::proportional(body_font.size * 0.70), 
        color: ui.visuals().text_color(),
        valign: egui::Align::Min, 
        ..Default::default()
    });
    
    ui.label(job);
}