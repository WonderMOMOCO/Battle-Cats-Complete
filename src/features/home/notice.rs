use eframe::egui;
use serde::{Deserialize, Serialize};
use crate::global::utils::process_markdown;
use crate::global::ui::shared::DragGuard;

// Note: No notice will appear if NOTICE_CONTENT is empty
pub const NOTICE_TITLE: &str = "NOTICE";
pub const NOTICE_CONTENT: &str = r#"
Data tab's backend logic has been rewritten from scratch to improve (and hopefully perfect) region merging accuracy!
It is recommended you delete your "game/metadata" folder at the least. At most do a full game import from scatch to buid new metadata
"#;

#[derive(Serialize, Deserialize, Default)]
struct AppMeta {
    app_version: String,
}

pub fn check_and_show(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    if NOTICE_CONTENT.trim().is_empty() {
        return;
    }

    let state_id = egui::Id::new("notice_state");
    let mut is_open = ctx.data(|d| d.get_temp::<Option<bool>>(state_id)).flatten();

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    // Check version once per session using the universal JSON loader
    if is_open.is_none() {
        let needs_notice = match crate::global::io::json::load::<AppMeta>("meta.json") {
            Some(meta) => meta.app_version != current_version,
            None => true, // If file doesn't exist, show notice
        };

        is_open = Some(needs_notice);
        ctx.data_mut(|d| d.insert_temp(state_id, Some(needs_notice)));
    }

    let mut show_window = is_open.unwrap_or(false);

    if show_window {
        let window_id = egui::Id::new("NoticeWindow");
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

        let mut window = egui::Window::new(NOTICE_TITLE)
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .movable(allow_drag)
            .default_pos(ctx.screen_rect().center() - egui::vec2(250.0, 150.0));

        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

        window.show(ctx, |ui| {
            ui.set_max_size([500.0, 400.0].into());

            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                
                process_markdown(ui, NOTICE_CONTENT);
            });

            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                if ui.add_sized([120.0, 35.0], egui::Button::new(egui::RichText::new("Acknowledge").size(16.0).strong())).clicked() {
                    show_window = false;
                    ctx.data_mut(|d| d.insert_temp(state_id, Some(false)));
                    
                    // Mark as read by atomically saving version to disk via utility
                    let new_meta = AppMeta { app_version: current_version.clone() };
                    crate::global::io::json::save("meta.json", &new_meta);
                }
            });
        });
    }
}