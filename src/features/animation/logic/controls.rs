use eframe::egui;

// Handles all user input for the animation viewport
pub fn handle_viewport_input(
    ui: &egui::Ui,
    response: &egui::Response,
    pan_offset: &mut egui::Vec2,
    zoom_level: &mut f32,
    target_zoom_level: &mut f32,
    pending_initial_center: &mut bool,
    block_input: bool, 
    is_viewport_dragging: &mut bool, 
) {
    // Determine Drag Validity on Start
    if response.drag_started() {
        if block_input {
            *is_viewport_dragging = false; // Started on controls, ignore
        } else {
            *is_viewport_dragging = true;  // Started on viewport, valid
        }
    }

    // Clear state on release
    if response.drag_stopped() {
        *is_viewport_dragging = false;
    }

    // Pan Logic
    if response.dragged() && *is_viewport_dragging {
        *pan_offset += response.drag_delta() / *zoom_level;
        
        // Cancel any pending auto-center if the user takes control
        *pending_initial_center = false;
    }

    // Mouse Zoom
    if !block_input && response.hovered() {
        ui.input(|i| {
            let scroll = i.raw_scroll_delta.y;
            if scroll != 0.0 {
                let zoom_factor = 1.0 + (scroll * 0.006);
                *target_zoom_level = (*target_zoom_level * zoom_factor).clamp(0.1, 10.0);
            }
        });
    }

    // Pinch Zoom
    if !block_input {
        ui.input(|i| {
            let delta = i.zoom_delta();
            if delta != 1.0 {
                *target_zoom_level *= delta;
                *zoom_level = *target_zoom_level;
            }
        });
    }
}