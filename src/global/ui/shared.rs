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

    let body_font = ui.style().text_styles.get(&egui::TextStyle::Body)
        .cloned().unwrap_or(egui::FontId::proportional(14.0));
        
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = ui.spacing().tooltip_width;

    let normal_format = egui::TextFormat {
        font_id: body_font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    };

    let super_format = egui::TextFormat {
        font_id: egui::FontId::proportional(body_font.size * 0.70), 
        color: ui.visuals().text_color(),
        valign: egui::Align::Min, 
        ..Default::default()
    };

    let mut parts = text.split('^');

    // First part is always standard text before any ^ character
    if let Some(first) = parts.next() {
        if !first.is_empty() {
            job.append(first, 0.0, normal_format.clone());
        }
    }

    // Subsequent parts start as superscript, and revert to normal at the first space
    for part in parts {
        if let Some(space_idx) = part.find(' ') {
            let super_str = &part[..space_idx];
            let normal_str = &part[space_idx..]; // Includes the space

            if !super_str.is_empty() {
                job.append(super_str, 0.0, super_format.clone()); 
            }
            if !normal_str.is_empty() {
                job.append(normal_str, 0.0, normal_format.clone());
            }
        } else {
            // No space found, the entire remaining text is superscript
            if !part.is_empty() {
                job.append(part, 0.0, super_format.clone());
            }
        }
    }
    
    ui.label(job);
}

#[derive(Default)]
pub struct DragGuard {
    broken: bool,
}

impl DragGuard {
    pub fn update(&mut self, ctx: &egui::Context) -> bool {
        let screen_rect = ctx.screen_rect();
        let (pointer_pos, mouse_down) = ctx.input(|i| {
            (i.pointer.interact_pos(), i.pointer.primary_down())
        });
        let in_window = pointer_pos.map_or(false, |p| screen_rect.contains(p));

        if !mouse_down {
            self.broken = false;
        } else if !in_window {
            self.broken = true;
        }

        in_window && !self.broken
    }

    pub fn assign_bounds(&mut self, ctx: &egui::Context, window_id: egui::Id) -> (bool, Option<egui::Pos2>) {
        (self.update(ctx), clamp_window_to_screen(ctx, window_id))
    }
}

pub fn clamp_window_to_screen(ctx: &egui::Context, window_id: egui::Id) -> Option<egui::Pos2> {
    if let Some(rect) = ctx.memory(|mem| mem.area_rect(window_id)) {
        let screen_rect = ctx.screen_rect();
        let mut new_pos = rect.min;
        let mut changed = false;
        if new_pos.y < screen_rect.top() { new_pos.y = screen_rect.top(); changed = true; }
        if new_pos.y > screen_rect.bottom() - 30.0 { new_pos.y = screen_rect.bottom() - 30.0; changed = true; }
        if new_pos.x + rect.width() - 50.0 < screen_rect.left() { new_pos.x = screen_rect.left() - rect.width() + 50.0; changed = true; }
        if new_pos.x + 50.0 > screen_rect.right() { new_pos.x = screen_rect.right() - 50.0; changed = true; }
        if changed { return Some(new_pos); }
    }
    None
}