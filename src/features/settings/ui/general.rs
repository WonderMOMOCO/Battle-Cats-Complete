use eframe::egui;
use crate::features::settings::logic::state::{GeneralSettings, RuntimeState};
use crate::features::settings::logic::{lang, upd::UpdateMode};

pub fn show(ui: &mut egui::Ui, settings: &mut GeneralSettings, runtime: &mut RuntimeState) -> bool {
    let mut refresh_needed = false;
    let ctx = ui.ctx().clone();
    
    // Ensure the list is populated with all codes + the "--" separator
    lang::ensure_complete_list(&mut settings.language_priority);

    egui::ScrollArea::vertical()
        .id_salt("general_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
    
            ui.heading("Updates");
            ui.add_space(5.0);

            let updater_status = ctx.data(|d| d.get_temp::<&'static str>(egui::Id::new("updater_status")).unwrap_or("Idle"));

            match updater_status {
                "Checking" => {
                    let btn = egui::Button::new("Checking for Updates...").fill(egui::Color32::from_rgb(200, 180, 50)); 
                    ui.add_sized([180.0, 30.0], btn);
                },
                "UpToDate" => {
                    let btn = egui::Button::new("Up to Date!").fill(egui::Color32::from_rgb(40, 160, 40)); 
                    ui.add_sized([180.0, 30.0], btn);
                },
                "UpdateFound" => {
                    let btn = egui::Button::new("Update Found!").fill(egui::Color32::from_rgb(40, 160, 40)); 
                    ui.add_sized([180.0, 30.0], btn);
                },
                "CheckFailed" => {
                    let btn = egui::Button::new("Failed to Check!").fill(egui::Color32::from_rgb(180, 50, 50)); 
                    ui.add_sized([180.0, 30.0], btn);
                },
                "Downloading" => {
                    let btn = egui::Button::new("Downloading Update...").fill(egui::Color32::from_rgb(40, 90, 160)); 
                    ui.add_sized([180.0, 30.0], btn);
                },
                "RestartPending" => {
                    let btn = egui::Button::new("Restart Pending!").fill(egui::Color32::from_rgb(200, 180, 50)); 
                    ui.add_sized([180.0, 30.0], btn);
                },
                _ => {
                    if ui.add_sized([180.0, 30.0], egui::Button::new("Check for Update Now")).clicked() {
                        runtime.manual_check_requested = true;
                    }
                }
            }

            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Update Handling:");
                
                egui::ComboBox::from_id_salt("update_mode_selector")
                    .selected_text(settings.update_mode.label())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.update_mode, UpdateMode::AutoReset, "Auto-Reset")
                            .on_hover_text("Automatically downloads updates and restarts the app on startup");
                        ui.selectable_value(&mut settings.update_mode, UpdateMode::AutoLoad, "Auto-Load")
                            .on_hover_text("Automatically downloads updates but waits until the next run to apply them");
                        ui.selectable_value(&mut settings.update_mode, UpdateMode::Prompt, "Prompt")
                            .on_hover_text("Ask permission before downloading updates or restarting");
                        ui.selectable_value(&mut settings.update_mode, UpdateMode::Ignore, "Ignore")
                            .on_hover_text("Never check for updates on startup");
                    });
            });

            ui.add_space(20.0);
            ui.heading("Language Priority");
            ui.add_space(5.0);
            
            ui.label("Drag to reorder. The app prioritizes assets from the top down.");
            ui.small("Languages below 'None' will never be loaded.");
            ui.add_space(5.0);
            
            // If the drag-and-drop function returns true (mouse released), trigger the reload
            if render_drag_list(ui, &mut settings.language_priority) {
                refresh_needed = true;
            }
            
            ui.add_space(10.0);
            if ui.button("Restore Defaults").clicked() {
                settings.language_priority = lang::default_priority();
                refresh_needed = true;
            }
    });

    refresh_needed
}

/// Standalone Drag-and-Drop component for egui
fn render_drag_list(ui: &mut egui::Ui, priority: &mut Vec<String>) -> bool {
    let id_source = egui::Id::new("language_priority_drag_list");
    
    let was_dragging = ui.ctx().data(|d| d.get_temp::<bool>(id_source)).unwrap_or(false);
    let is_dragging = ui.ctx().dragged_id().is_some();
    ui.ctx().data_mut(|d| d.insert_temp(id_source, is_dragging));
    
    let just_dropped = was_dragging && !is_dragging;

    let mut source_idx = None;
    let mut target_idx = None;
    let mut is_disabled_section = false;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 2.0); 

        for (i, code) in priority.clone().iter().enumerate() {
            let is_none = *code == "--";
            if is_none { is_disabled_section = true; }

            let item_id = id_source.with(code);
            let is_dragged = ui.ctx().is_being_dragged(item_id); 

            let mut frame = egui::Frame::none().inner_margin(egui::vec2(5.0, 2.0));
            if is_dragged {
                frame.fill = ui.visuals().widgets.active.bg_fill;
                frame.rounding = ui.visuals().widgets.active.rounding;
            }

            let row_response = ui.scope(|ui| {
                if is_disabled_section && !is_none {
                    ui.visuals_mut().override_text_color = Some(egui::Color32::from_gray(100));
                }

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let label_response = ui.label("☰");
                        let handle = ui.interact(label_response.rect.expand(2.0), item_id, egui::Sense::drag());

                        if handle.hovered() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
                        if handle.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            source_idx = Some(i);
                        }

                        ui.add_space(5.0);
                        
                        if is_none {
                            ui.strong(lang::get_label_for_code(code));
                        } else {
                            ui.label(lang::get_label_for_code(code));
                        }
                    });
                }).response
            }).response;

            let drop_rect = row_response.rect.expand2(egui::vec2(100.0, 0.0));
            if let Some(pos) = ui.ctx().pointer_interact_pos() {
                if drop_rect.contains(pos) { target_idx = Some(i); }
            }
        }
    });

    if let (Some(source), Some(target)) = (source_idx, target_idx) {
        if source != target {
            let item = priority.remove(source);
            priority.insert(target, item);
        }
    }

    just_dropped
}