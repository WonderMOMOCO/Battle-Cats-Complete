use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use self_update;
use regex::Regex;
use crate::global::ui::shared::DragGuard;

// --- UI TUNING VARIABLES ---
const HEADER_TEXT_SIZE: f32 = 22.0;
const DROPDOWN_ITEM_SIZE: f32 = 15.0;
const HEADER_SPACING_X: f32 = 6.0;
const HEADER_SPACING_Y: f32 = 6.0;
const LABEL_OFFSET_Y: f32 = 7.0; // Nudge the title down manually
// ---------------------------

#[derive(Clone)]
struct ChangelogState {
    is_open: bool,
    is_loading: bool,
    fetched: bool,
    releases: Vec<(String, String)>, 
    selected_version: String,
    error: bool,
    fetch_start: Option<Instant>,
}

impl Default for ChangelogState {
    fn default() -> Self {
        Self {
            is_open: false,
            is_loading: false,
            fetched: false,
            releases: Vec::new(),
            selected_version: String::new(),
            error: false,
            fetch_start: None,
        }
    }
}

pub fn link(ui: &mut egui::Ui, ctx: &egui::Context) {
    if ui.link("Changelogs").clicked() {
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
            let current_version = env!("CARGO_PKG_VERSION");
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
                        let mut formatted_releases = Vec::new();
                        for r in releases {
                            let clean_version = r.version.trim_start_matches('v').to_string();
                            
                            if !clean_version.is_empty() && clean_version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                                let raw_body = r.body.unwrap_or_else(|| "No notes.".to_string());
                                formatted_releases.push((clean_version, strip_markdown(&raw_body)));
                            }
                        }
                        locked_thread.releases = formatted_releases;
                        
                        let clean_current = current_version.trim_start_matches('v');
                        if locked_thread.releases.iter().any(|(v, _)| v == clean_current) {
                            locked_thread.selected_version = clean_current.to_string();
                        } else if let Some(first) = locked_thread.releases.first() {
                            locked_thread.selected_version = first.0.clone();
                        } else {
                            locked_thread.selected_version = "Unknown".to_string();
                            locked_thread.releases.push(("Unknown".to_string(), "No releases found.".to_string()));
                        }
                        
                        locked_thread.error = false;
                    }
                    Err(_) => { locked_thread.error = true; }
                }
                ctx_clone.request_repaint();
            });
        }
        
        ctx.data_mut(|temp_storage| temp_storage.insert_temp(state_id, state));
    }
}

pub fn window(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("changelog_state");
    let state_arc = ctx.data(|temp_storage| temp_storage.get_temp::<Arc<Mutex<ChangelogState>>>(state_id));

    if let Some(state) = state_arc {
        let mut locked = state.lock().unwrap();
        
        if locked.is_open {
            let time_expired = locked.fetch_start.map_or(false, |t| t.elapsed().as_secs_f32() > 3.0);
            let should_show_window = locked.fetched || time_expired;

            if should_show_window {
                let show_error = locked.error || (!locked.fetched && time_expired);
                
                let window_id = egui::Id::new("Changelogs");
                let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

                let mut is_open = true;
                let mut window = egui::Window::new("Changelogs")
                    .id(window_id)
                    .open(&mut is_open)
                    .collapsible(false)
                    .resizable(false) 
                    .constrain(false)
                    .movable(allow_drag)
                    .default_pos(ctx.screen_rect().center() - egui::vec2(300.0, 200.0));
                    
                if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
                    
                window.show(ctx, |ui| {
                        ui.set_max_size([600.0, 400.0].into());

                        if show_error {
                            ui.centered_and_justified(|ui| {
                                ui.heading("Couldn't connect to GitHub");
                            });
                        } else if locked.is_loading {
                            ui.centered_and_justified(|ui| { ui.spinner(); });
                        } else {
                            
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = HEADER_SPACING_X;
                                
                                ui.vertical(|ui| {
                                    ui.add_space(LABEL_OFFSET_Y);
                                    ui.label(egui::RichText::new("Battle Cats Complete").size(HEADER_TEXT_SIZE).strong());
                                });

                                let mut selected = locked.selected_version.clone();
                                let display_text = format!("v{}", selected);

                                egui::ComboBox::from_id_salt("changelog_version_select")
                                    .selected_text(egui::RichText::new(&display_text).size(HEADER_TEXT_SIZE).strong())
                                    .show_ui(ui, |ui| {
                                        for (version, _) in &locked.releases {
                                            let item_text = format!("v{}", version);
                                            ui.selectable_value(
                                                &mut selected, 
                                                version.clone(), 
                                                egui::RichText::new(item_text).size(DROPDOWN_ITEM_SIZE)
                                            );
                                        }
                                    });
                                
                                if selected != locked.selected_version {
                                    locked.selected_version = selected;
                                }
                            });
                            
                            ui.add_space(HEADER_SPACING_Y);

                            let content = locked.releases.iter()
                                .find(|(v, _)| v == &locked.selected_version)
                                .map(|(_, c)| c.as_str())
                                .unwrap_or("No notes available.");

                            egui::ScrollArea::vertical()
                                .auto_shrink([false, true]) 
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 0.0;

                                    for line in content.lines() {
                                        let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();
                                        let trimmed = line.trim();

                                        if trimmed.is_empty() {
                                            ui.add_space(10.0);
                                            continue;
                                        }

                                        ui.horizontal_top(|ui| {
                                            if leading_spaces > 0 {
                                                ui.add_space(leading_spaces as f32 * 6.0); 
                                            }

                                            if trimmed.starts_with("•") {
                                                ui.spacing_mut().item_spacing.x = 3.0;
                                                ui.label("•");    

                                                let text = trimmed.trim_start_matches('•').trim();
                                                ui.add(egui::Label::new(text).wrap());
                                            } else {
                                                ui.spacing_mut().item_spacing.x = 3.0;
                                                ui.add(egui::Label::new(trimmed).wrap());
                                            }
                                        });
                                    }
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
        text = re_list.replace_all(&text, "${1}• ").to_string();
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