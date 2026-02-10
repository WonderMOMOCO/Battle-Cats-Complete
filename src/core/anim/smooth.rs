// 'animate.rs' but modified for better
// sub-frame interpolation support
use crate::data::global::mamodel::{Model, ModelPart};
use crate::data::global::maanim::{Animation, AnimModification};

pub fn animate(model: &Model, animation: &Animation, global_frame: f32) -> Vec<ModelPart> {
    let mut parts = model.parts.clone();

    // Pre-Calculate Parent Switch Frames
    let mut parent_switches: Vec<Vec<i32>> = vec![Vec::new(); parts.len()];
    
    for curve in &animation.curves {
        if curve.modification_type == 0 {
            if curve.part_id < parent_switches.len() {
                for keyframe in &curve.keyframes {
                    parent_switches[curve.part_id].push(keyframe.frame);
                }
            }
        }
    }

    // Process Curves
    for curve in &animation.curves {
        if curve.part_id >= parts.len() { continue; }
        
        let (k_min, k_max) = if let (Some(first), Some(last)) = (curve.keyframes.first(), curve.keyframes.last()) {
            (first.frame as f32, last.frame as f32)
        } else {
            (0.0, 0.0)
        };

        let duration = (k_max - k_min).max(1.0);
        let mut local_frame = global_frame;
        let loop_count = curve.loop_count; 

        if loop_count != 1 {
            local_frame = (global_frame - k_min).rem_euclid(duration) + k_min;
        } 

        // Discrete types are always snapped
        let is_discrete = matches!(curve.modification_type, 0 | 1 | 2 | 3 | 13 | 14);
        
        // Pass the parent_switches map to the interpolator
        if let Some(interpolated_value) = interpolate_curve(curve, local_frame, is_discrete, &parent_switches) {
            let part = &mut parts[curve.part_id];
            
            match curve.modification_type {
                0 => {
                    let parent_idx = interpolated_value as i32;
                    if parent_idx != curve.part_id as i32 {
                        part.parent_id = parent_idx;
                    }
                },
                1 => part.unit_id = interpolated_value as i32,
                2 => part.sprite_index = interpolated_value as i32,
                3 => part.drawing_layer = interpolated_value as i32, 
                4 => part.position_x = model.parts[curve.part_id].position_x + interpolated_value, 
                5 => part.position_y = model.parts[curve.part_id].position_y + interpolated_value,
                6 => part.pivot_x = model.parts[curve.part_id].pivot_x + interpolated_value,
                7 => part.pivot_y = model.parts[curve.part_id].pivot_y + interpolated_value,

                // Absolute Scaling (Fixes Kaihime Distortion)
                8 => { 
                    let factor = interpolated_value / model.scale_unit;
                    part.scale_x = model.parts[curve.part_id].scale_x * factor;
                    part.scale_y = model.parts[curve.part_id].scale_y * factor;
                },
                9 => part.scale_x = model.parts[curve.part_id].scale_x * (interpolated_value / model.scale_unit),
                10 => part.scale_y = model.parts[curve.part_id].scale_y * (interpolated_value / model.scale_unit),
                
                // Absolute Rotation
                11 => part.rotation = model.parts[curve.part_id].rotation + interpolated_value,
                
                // Absolute Alpha
                12 => part.alpha = model.parts[curve.part_id].alpha * (interpolated_value / model.alpha_unit),
                
                13 => { part.flip_x = interpolated_value != 0.0; },
                14 => { part.flip_y = interpolated_value != 0.0; },
                _ => {}
            }
        }
    }
    
    parts
}

fn interpolate_curve(
    curve: &AnimModification, 
    frame: f32, 
    is_discrete: bool, 
    parent_switches: &[Vec<i32>]
) -> Option<f32> {
    
    if curve.keyframes.is_empty() { return None; }

    let first_k = &curve.keyframes[0];
    if frame < first_k.frame as f32 {
        return None; 
    }

    let mut start_idx = 0;
    let mut end_idx = 0;
    let mut found = false;

    // Search Keyframes
    for (index, keyframe) in curve.keyframes.iter().enumerate() {
        if (keyframe.frame as f32) > frame {
            end_idx = index;
            start_idx = if index > 0 { index - 1 } else { 0 };
            found = true;
            break;
        }
    }
    
    // Hold last value if past
    if !found {
        return Some(curve.keyframes.last().unwrap().value as f32);
    }
    // Before first keyframe
    if end_idx == 0 {
         return Some(curve.keyframes[0].value as f32);
    }

    let start_k = &curve.keyframes[start_idx];
    let end_k = &curve.keyframes[end_idx];

    // Snap for discrete
    if is_discrete { return Some(start_k.value as f32); }
    
    // Identical Frame Handling
    if start_k.frame == end_k.frame { return Some(start_k.value as f32); }

    // Parent Sync Snap
    if curve.part_id < parent_switches.len() {
        if parent_switches[curve.part_id].contains(&end_k.frame) {
             return Some(start_k.value as f32);
        }
    }

    // Velocity Heuristic
    let delta = (end_k.value - start_k.value).abs() as f32;
    let frames = (end_k.frame - start_k.frame).abs() as f32;

    if frames <= 2.1 {
        let should_snap = match curve.modification_type {
            // Position/Pivot
            4..=7 => (delta / frames) > 20.0, 
            
            // Rotation
            11 => (delta / frames) > 15.0,    
            
            // Scale
            8..=10 => (delta / frames) > 0.2,
            
            // Alpha
            12 => (delta / frames) > 0.2,
            
            _ => false,
        };

        if should_snap {
            return Some(start_k.value as f32);
        }
    }

    // Interpoltion Math
    if start_k.ease_mode == 3 {
        let mut points = Vec::new();
        let mut idx_i = start_idx as isize;
        while idx_i >= 0 {
            let keyframe = &curve.keyframes[idx_i as usize];
            if (idx_i as usize) != start_idx && keyframe.ease_mode != 3 { break; }
            points.push((keyframe.frame as f32, keyframe.value as f32));
            idx_i -= 1;
        }
        points.reverse(); 
        let mut idx_i = end_idx;
        while idx_i < curve.keyframes.len() {
            let keyframe = &curve.keyframes[idx_i];
            points.push((keyframe.frame as f32, keyframe.value as f32));
            if keyframe.ease_mode != 3 { break; }
            idx_i += 1;
        }

        let mut result = 0.0;
        let count = points.len();
        for idx_j in 0..count {
            let (frame_j, val_j) = points[idx_j];
            let mut prod = val_j;
            for idx_m in 0..count {
                if idx_j == idx_m { continue; }
                let (frame_m, _) = points[idx_m];
                if (frame_j - frame_m).abs() > 0.0001 {
                    prod *= (frame - frame_m) / (frame_j - frame_m);
                }
            }
            result += prod;
        }
        return Some(result);
    }

    let t_duration = frames; 
    let t_current = frame - (start_k.frame as f32);
    let x = t_current / t_duration;

    let start_val = start_k.value as f32;
    let change = (end_k.value - start_k.value) as f32;

    match start_k.ease_mode {
        0 => Some(start_val + (change * x)), 
        1 => Some(if x >= 1.0 { end_k.value as f32 } else { start_val }), 
        2 => { 
            let power = if start_k.ease_power != 0 { start_k.ease_power as f32 } else { 1.0 };
            let x_clamped = x.clamp(0.0, 1.0);
            let factor = if power >= 0.0 {
                1.0 - (1.0 - x_clamped.powf(power)).sqrt()
            } else {
                (1.0 - (1.0 - x_clamped).powf(-power)).sqrt()
            };
            Some(if factor.is_nan() { start_val + (change * x) } else { start_val + (change * factor) })
        },
        _ => Some(start_val + (change * x)) 
    }
}