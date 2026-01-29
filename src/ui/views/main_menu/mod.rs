use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use self_update; 
use regex::Regex; 
use crate::core::utils::DragGuard;

#[derive(Clone)]
struct ChangelogState {
    is_open: bool,
    is_loading: bool,
    fetched: bool,
    content: String,
    error: bool,
    fetch_start: Option<Instant>,
}

impl Default for ChangelogState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_loading: false,
            fetched: false,
            content: String::new(),
            error: false,
            fetch_start: None,
        }
    }
}

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
            ui.label(egui::RichText::new("User-Handled Battle Cats Database").size(16.0));
        });
    });

    egui::Area::new("version_area".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0]) 
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

                if ui.link("Changelog").clicked() {
                    let state_id = egui::Id::new("changelog_state");
                    let state = ctx.data(|temp_storage| temp_storage.get_temp::<Arc<Mutex<ChangelogState>>>(state_id))
                        .unwrap_or_else(|| Arc::new(Mutex::new(ChangelogState::default())));
                    
                    let should_fetch = {
                        let mut locked = state.lock().unwrap();
                        locked.is_open = true;
                        
                        if !locked.fetched && !locked.is_loading {
                            locked.is_loading = true;
                            locked.fetch_start = Some(Instant::now());
                            true
                        } else {
                            false
                        }
                    };

                    if should_fetch {
                        let state_clone = state.clone();
                        let ctx_clone = ctx.clone();
                        let repo_owner = "WonderMOMOCO";
                        let repo_name = "Battle-Cats-Complete";

                        thread::spawn(move || {
                            let releases_result = self_update::backends::github::ReleaseList::configure()
                                .repo_owner(repo_owner)
                                .repo_name(repo_name)
                                .build()
                                .and_then(|r| r.fetch());

                            let mut locked_thread = state_clone.lock().unwrap();
                            locked_thread.is_loading = false;
                            locked_thread.fetched = true;

                            match releases_result {
                                Ok(releases) => {
                                    let match_found = releases.iter().find(|r| r.version == current_version);
                                    if let Some(release) = match_found {
                                        let raw_body = release.body.clone().unwrap_or_else(|| "No notes.".to_string());
                                        locked_thread.content = strip_markdown(&raw_body);
                                        locked_thread.error = false;
                                    } else {
                                        locked_thread.error = true;
                                        locked_thread.content = "Current version not found in releases.".to_string();
                                    }
                                }
                                Err(_) => { locked_thread.error = true; }
                            }
                            ctx_clone.request_repaint();
                        });
                    }
                    
                    ctx.data_mut(|temp_storage| temp_storage.insert_temp(state_id, state));
                }
            });
        });

    egui::Area::new("social_links_area".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0]) 
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

    let state_id = egui::Id::new("changelog_state");
    let state_arc = ctx.data(|temp_storage| temp_storage.get_temp::<Arc<Mutex<ChangelogState>>>(state_id));

    if let Some(state) = state_arc {
        let mut locked = state.lock().unwrap();
        
        if locked.is_open {
            let time_expired = locked.fetch_start.map_or(false, |t| t.elapsed().as_secs_f32() > 3.0);
            let should_show_window = locked.fetched || time_expired;

            if should_show_window {
                let show_error = locked.error || (!locked.fetched && time_expired);
                let allow_drag = drag_guard.update(ctx);

                let mut is_open = true;
                egui::Window::new("Changelog")
                    .open(&mut is_open)
                    .collapsible(false)
                    .resizable(false) 
                    .movable(allow_drag)
                    .pivot(egui::Align2::CENTER_CENTER)
                    .default_pos(ctx.screen_rect().center())
                    .show(ctx, |ui| {
                        ui.set_max_size([600.0, 400.0].into());

                        if show_error {
                            ui.centered_and_justified(|ui| {
                                ui.heading("Couldn't connect to GitHub");
                            });
                        } else if locked.is_loading {
                            ui.centered_and_justified(|ui| { ui.spinner(); });
                        } else {
                            egui::ScrollArea::vertical()
                                .auto_shrink([true, true]) 
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(&locked.content).wrap());
                                });
                        }
                    });
                
                locked.is_open = is_open;
            } else {
                ctx.request_repaint();
            }
        }
    }
}

fn strip_markdown(text: &str) -> String {
    let mut text = text.to_string();

    if let Ok(re_link) = Regex::new(r"\[([^\]]+)\]\([^\)]+\)") {
        text = re_link.replace_all(&text, "$1").to_string();
    }

    if let Ok(re_list) = Regex::new(r"(?m)^(\s*)[\*\-]\s+") {
        text = re_list.replace_all(&text, "${1}${1}• ").to_string();
    }
    
    if let Ok(re_header) = Regex::new(r"(?m)^#+\s*") {
        text = re_header.replace_all(&text, "").to_string();
    }

    text = text.replace("**", "");
    text = text.replace("__", "");
    text = text.replace("*", ""); 
    text = text.replace("_", "");
    text = text.replace("`", "");

    text
}