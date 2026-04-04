use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use crate::global::utils::process_markdown;
use crate::global::ui::shared::DragGuard;

// Note: No notice will appear if NOTICE_CONTENT is empty
pub const NOTICE_TITLE: &str = "NOTICE";
pub const NOTICE_CONTENT: &str = r#"
# Database Restructure Required

- With this update the Database's fundamental structure has changed, the app will not work out of the box post-update.
- To fix this, go to Game > Data, select "Raw", change the Source to "Folder", and select your "game" folder as the source, running the Job should begin the Database Reorganization and fix the App.
- To see this message again, go to "Changelogs" on the bottom left of the Home page.
"#;

#[derive(Serialize, Deserialize, Default)]
struct AppMeta {
    app_version: String,
}

fn get_meta_path() -> PathBuf {
    let mut path = if cfg!(target_os = "windows") {
        PathBuf::from(std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string()))
    } else {
        PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string())).join(".config")
    };
    path.push("battle_cats_complete");
    path.push("data");
    fs::create_dir_all(&path).ok();
    path.push("meta.json");
    path
}

pub fn check_and_show(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    if NOTICE_CONTENT.trim().is_empty() {
        return;
    }

    let state_id = egui::Id::new("notice_state");
    let mut is_open = ctx.data(|d| d.get_temp::<Option<bool>>(state_id)).flatten();

    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let meta_path = get_meta_path();

    if is_open.is_none() {
        let needs_notice = if meta_path.exists() {
            if let Ok(data) = fs::read_to_string(&meta_path) {
                if let Ok(meta) = serde_json::from_str::<AppMeta>(&data) {
                    meta.app_version != current_version
                } else { true }
            } else { true }
        } else { true };

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
                    
                    let new_meta = AppMeta { app_version: current_version.clone() };
                    if let Ok(json) = serde_json::to_string_pretty(&new_meta) {
                        let _ = fs::write(&meta_path, json);
                    }
                }
            });
        });
    }
}