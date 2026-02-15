use eframe::egui;
use crate::data::global::mamodel::Model;
use crate::data::global::maanim::Animation;
use crate::data::global::imgcut::SpriteSheet;
use crate::core::anim::{animator, transform};

// Calculates a tight bounding box using the exact renderer logic for the ENTIRE animation
// Uses "Smart Filtering" to ignore faint effects while keeping giant solid limbs
pub fn calculate_tight_bounds(
    model: &Model,
    anim: Option<&Animation>,
    sheet: &SpriteSheet
) -> Option<egui::Rect> {
    // PASS 1: Strict Body Scan (Full Range)
    let solid_bounds = scan_bounds(model, anim, sheet, true, None);
    
    if solid_bounds.is_some() {
        return solid_bounds;
    }

    // PASS 2: Fallback (Full Range)
    scan_bounds(model, anim, sheet, false, None)
}

// Calculates the initial Camera Pan and Zoom to fit the unit in the viewport
// Uses only Frame 0 (or static pose) to determine the resting position
pub fn calculate_initial_view(
    model: &Model,
    anim: Option<&Animation>,
    sheet: &SpriteSheet,
    viewport_size: egui::Vec2
) -> Option<(egui::Vec2, f32)> {
    
    // We only scan Frame 0 for the initial centering
    let frame_zero = Some((0, 0));

    // Try Strict Scan first
    let bounds = scan_bounds(model, anim, sheet, true, frame_zero)
        .or_else(|| scan_bounds(model, anim, sheet, false, frame_zero));

    if let Some(b) = bounds {
        // Calculate Pan
        // We invert the center because pan_offset moves the camera, not the object.
        let center = b.center();
        let pan = egui::vec2(-center.x, -center.y);

        // Calculate Zoom Fit
        // Prevent divide by zero or tiny bounds
        let w = b.width().max(1.0);
        let h = b.height().max(1.0);

        let scale_x = viewport_size.x / w;
        let scale_y = viewport_size.y / h;

        // "Breathing Room" Factor
        let breathing_room = 0.45;

        // Clamp to reasonable limits to prevent micro-units from exploding
        let zoom = scale_x.min(scale_y).clamp(0.05, 5.0) * breathing_room;

        return Some((pan, zoom));
    }

    None
}

fn scan_bounds(
    model: &Model,
    anim: Option<&Animation>,
    sheet: &SpriteSheet,
    strict_mode: bool,
    override_range: Option<(i32, i32)>
) -> Option<egui::Rect> {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut found_any = false;

    let (start, end) = if let Some(r) = override_range {
        r
    } else if let Some(a) = anim { 
        (0, a.max_frame) 
    } else { 
        (0, 0) 
    };

    let step = 1; 

    for f in (start..=end).step_by(step) {
        let frame = f as f32;
        let posed_parts = if let Some(a) = anim { animator::animate(model, a, frame) } else { model.parts.clone() };
        let world_parts = transform::solve_hierarchy(&posed_parts, model);

        for part in world_parts {
            
            // STRICT MODE FILTERING
            if strict_mode {
                // Minimum Opacity Floor
                if part.opacity < 0.25 { continue; }

                // Faint Glow Filter
                if part.glow > 0 && part.opacity < 0.75 { continue; }

                // SCALE HEURISTIC
                let scale_x = (part.matrix[0].powi(2) + part.matrix[1].powi(2)).sqrt();
                let scale_y = (part.matrix[3].powi(2) + part.matrix[4].powi(2)).sqrt();
                let max_scale = scale_x.max(scale_y);

                if max_scale > 3.0 {
                    if part.opacity < 0.95 || part.glow > 0 {
                        continue;
                    }
                }
            } else {
                // Just check visibility
                if part.opacity <= 0.01 || part.hidden { continue; }
            }

            if let Some(cut) = sheet.cuts_map.get(&part.sprite_index) {
                let w = cut.original_size.x;
                let h = cut.original_size.y;
                let px = part.pivot.x;
                let py = part.pivot.y;

                // Local corners
                let local_corners = [
                    egui::vec2(-px, -py),
                    egui::vec2(w - px, -py),
                    egui::vec2(w - px, h - py),
                    egui::vec2(-px, h - py),
                ];

                // World Bounds
                let m = part.matrix;
                let mut p_min_x = f32::MAX;
                let mut p_min_y = f32::MAX;
                let mut p_max_x = f32::MIN;
                let mut p_max_y = f32::MIN;

                for p in local_corners {
                    let wx = p.x * m[0] + p.y * m[3] + m[6];
                    let wy = p.x * m[1] + p.y * m[4] + m[7];
                    
                    p_min_x = p_min_x.min(wx);
                    p_max_x = p_max_x.max(wx);
                    p_min_y = p_min_y.min(wy);
                    p_max_y = p_max_y.max(wy);
                }

                if strict_mode {
                    // BEAM FILTER
                    // If visually taller than 1000px AND narrow (H > W*2).
                    let part_h = p_max_y - p_min_y;
                    let part_w = p_max_x - p_min_x;
                    
                    if part_h > 1000.0 && part_h > part_w * 2.0 {
                        continue; 
                    }

                    // SKY FILTER
                    if p_max_y < -1200.0 {
                        continue;
                    }
                }

                // Accumulate
                min_x = min_x.min(p_min_x);
                max_x = max_x.max(p_max_x);
                min_y = min_y.min(p_min_y);
                max_y = max_y.max(p_max_y);
                
                found_any = true;
            }
        }
    }

    if found_any {
        Some(egui::Rect::from_min_max(
            egui::pos2(min_x, min_y),
            egui::pos2(max_x, max_y),
        ))
    } else {
        None
    }
}