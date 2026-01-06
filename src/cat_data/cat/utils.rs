use eframe::egui;
use image::imageops;

pub fn paint_fallback_at(ui: &mut egui::Ui, rect: egui::Rect, text: &str, border_color: egui::Color32) {
    let rounding = 5.5;
    
    if ui.is_rect_visible(rect) {
        ui.painter().rect_stroke(
            rect,
            rounding,
            egui::Stroke::new(1.5, border_color),
        );

        // Standardizes font size
        let font_id = egui::FontId::proportional(10.0); 
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            font_id,
            ui.visuals().text_color(),
        );
    }
}

pub fn render_fallback_icon(ui: &mut egui::Ui, text: &str, border_color: egui::Color32) -> egui::Response {
    let size = egui::vec2(40.0, 40.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    paint_fallback_at(ui, rect, text, border_color);
    response
}

pub fn text_with_superscript(ui: &mut egui::Ui, text: &str) {
    if text.contains('^') {
        let parts: Vec<&str> = text.split('^').collect();
        if parts.len() >= 2 {
            let body_font = ui.style().text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or(egui::FontId::proportional(14.0));
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
            return;
        }
    }
    ui.label(text);
}

pub fn autocrop(img: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 { 
            min_x = min_x.min(x); min_y = min_y.min(y);
            max_x = max_x.max(x); max_y = max_y.max(y);
            found = true;
        }
    }
    if !found { return img; }
    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}