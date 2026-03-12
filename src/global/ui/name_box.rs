use eframe::egui;

pub const NAME_BOX_WIDTH: f32 = 150.0;
pub const NAME_BOX_HEIGHT: f32 = 15.0;

pub fn render(user_interface: &mut egui::Ui, name_text: &str) {
    let text_color = user_interface.visuals().text_color();
    let (allocation_rect, _) = user_interface.allocate_exact_size(
        egui::vec2(NAME_BOX_WIDTH, NAME_BOX_HEIGHT), 
        egui::Sense::hover()
    );

    // Iteratively shrink font until name fits
    let mut current_font_size = 22.0; 
    while current_font_size > 8.0 { 
        let font_identifier = egui::FontId::proportional(current_font_size);
        let layout_job = egui::text::LayoutJob::simple(
            name_text.to_owned(),
            font_identifier,
            text_color,
            NAME_BOX_WIDTH
        );
        
        let text_galley = user_interface.fonts(|font_manager| font_manager.layout_job(layout_job));
        
        if text_galley.rows.len() <= 2 {
            let vertical_offset = (NAME_BOX_HEIGHT - text_galley.rect.height()) / 2.0;
            let draw_position = allocation_rect.min + egui::vec2(0.0, vertical_offset);
            user_interface.painter().galley(draw_position, text_galley, text_color);
            return;
        }
        current_font_size -= 1.0;
    }
    
    // Fallback if name still doesn't fit
    let fallback_font_identifier = egui::FontId::proportional(8.0);
    let fallback_layout_job = egui::text::LayoutJob::simple(
        name_text.to_owned(), 
        fallback_font_identifier, 
        text_color, 
        NAME_BOX_WIDTH
    );
    let fallback_text_galley = user_interface.fonts(|font_manager| font_manager.layout_job(fallback_layout_job));
    let fallback_vertical_offset = (NAME_BOX_HEIGHT - fallback_text_galley.rect.height()) / 2.0;
    
    user_interface.painter().galley(
        allocation_rect.min + egui::vec2(0.0, fallback_vertical_offset), 
        fallback_text_galley, 
        text_color
    );
}