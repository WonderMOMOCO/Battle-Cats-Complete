// 'animate.rs' but modified for better
// sub-frame interpolation support
use crate::global::formats::mamodel::{Model, ModelPart};
use crate::global::formats::maanim::{Animation, AnimModification};

pub fn animate(model: &Model, animation: &Animation, global_frame: f32) -> Vec<ModelPart> {
    let mut parts = model.parts.clone();

    // Pre-Calculate Parent Switch Frames
    let mut parent_switches: Vec<Vec<i32>> = vec![Vec::new(); parts.len()];
    
    for curve in &animation.curves {
        if curve.modification_type != 0 { continue; }
        if curve.part_id >= parent_switches.len() { continue; }
        
        for keyframe in &curve.keyframes {
            parent_switches[curve.part_id].push(keyframe.frame);
        }
    }

    // Process Curves
    for curve in &animation.curves {
        if curve.part_id >= parts.len() { continue; }
        
        let keyframe_min = curve.keyframes.first().map(|keyframe| keyframe.frame as f32).unwrap_or(0.0);
        let keyframe_max = curve.keyframes.last().map(|keyframe| keyframe.frame as f32).unwrap_or(0.0);

        let duration = (keyframe_max - keyframe_min).max(1.0);
        let mut local_frame = global_frame;
        let animation_loop_count = curve.loop_count; 

        if animation_loop_count != 1 {
            local_frame = (global_frame - keyframe_min).rem_euclid(duration) + keyframe_min;
        } 

        // Discrete types are always snapped
        let is_discrete = matches!(curve.modification_type, 0 | 1 | 2 | 3 | 13 | 14);
        
        // Pass the parent_switches map to the interpolator
        let Some(interpolated_value) = interpolate_curve(curve, local_frame, is_discrete, &parent_switches) else {
            continue;
        };
        
        let part = &mut parts[curve.part_id];
        
        match curve.modification_type {
            0 => {
                let parent_index = interpolated_value as i32;
                if parent_index != curve.part_id as i32 {
                    part.parent_id = parent_index;
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
                let scale_factor = interpolated_value / model.scale_unit;
                part.scale_x = model.parts[curve.part_id].scale_x * scale_factor;
                part.scale_y = model.parts[curve.part_id].scale_y * scale_factor;
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
    
    parts
}

fn interpolate_curve(
    curve: &AnimModification, 
    frame: f32, 
    is_discrete: bool, 
    parent_switches: &[Vec<i32>]
) -> Option<f32> {
    
    if curve.keyframes.is_empty() { return None; }

    let first_keyframe = &curve.keyframes[0];
    if frame < first_keyframe.frame as f32 {
        return None; 
    }

    let mut start_index = 0;
    let mut end_index = 0;
    let mut is_found = false;

    // Search Keyframes
    for (index, keyframe) in curve.keyframes.iter().enumerate() {
        if (keyframe.frame as f32) > frame {
            end_index = index;
            start_index = if index > 0 { index - 1 } else { 0 };
            is_found = true;
            break;
        }
    }
    
    // Hold last value if past
    if !is_found {
        let Some(last_keyframe) = curve.keyframes.last() else { return None; };
        return Some(last_keyframe.value as f32);
    }
    
    // Before first keyframe
    if end_index == 0 {
         return Some(curve.keyframes[0].value as f32);
    }

    let start_keyframe = &curve.keyframes[start_index];
    let end_keyframe = &curve.keyframes[end_index];

    // Snap for discrete
    if is_discrete { return Some(start_keyframe.value as f32); }
    
    // Identical Frame Handling
    if start_keyframe.frame == end_keyframe.frame { return Some(start_keyframe.value as f32); }

    // Parent Sync Snap
    if curve.part_id < parent_switches.len() && parent_switches[curve.part_id].contains(&end_keyframe.frame) {
        return Some(start_keyframe.value as f32);
    }

    // Velocity Heuristic
    let value_delta = (end_keyframe.value - start_keyframe.value).abs() as f32;
    let frame_delta = (end_keyframe.frame - start_keyframe.frame).abs() as f32;

    if frame_delta <= 2.1 {
        let should_snap = match curve.modification_type {
            // Position/Pivot
            4..=7 => (value_delta / frame_delta) > 20.0, 
            
            // Rotation
            11 => (value_delta / frame_delta) > 15.0,    
            
            // Scale
            8..=10 => (value_delta / frame_delta) > 0.2,
            
            // Alpha
            12 => (value_delta / frame_delta) > 0.2,
            
            _ => false,
        };

        if should_snap {
            return Some(start_keyframe.value as f32);
        }
    }

    // Interpolation Math
    if start_keyframe.ease_mode == 3 {
        let mut points = Vec::new();
        let mut backward_index = start_index as isize;
        
        while backward_index >= 0 {
            let current_keyframe = &curve.keyframes[backward_index as usize];
            if (backward_index as usize) != start_index && current_keyframe.ease_mode != 3 { break; }
            points.push((current_keyframe.frame as f32, current_keyframe.value as f32));
            backward_index -= 1;
        }
        
        points.reverse(); 
        
        let mut forward_index = end_index;
        while forward_index < curve.keyframes.len() {
            let current_keyframe = &curve.keyframes[forward_index];
            points.push((current_keyframe.frame as f32, current_keyframe.value as f32));
            if current_keyframe.ease_mode != 3 { break; }
            forward_index += 1;
        }

        let mut final_result = 0.0;
        let total_points = points.len();
        
        for outer_index in 0..total_points {
            let (xj, yj) = points[outer_index];
            let mut polynomial_product = yj;
            
            for inner_index in 0..total_points {
                if outer_index == inner_index { continue; }
                let (xm, _) = points[inner_index];
                if (xj - xm).abs() > 0.0001 {
                    polynomial_product *= (frame - xm) / (xj - xm);
                }
            }
            final_result += polynomial_product;
        }
        return Some(final_result);
    }

    let time_duration = frame_delta; 
    let time_current = frame - (start_keyframe.frame as f32);
    let x = time_current / time_duration;

    let start_value = start_keyframe.value as f32;
    let value_change = (end_keyframe.value - start_keyframe.value) as f32;

    match start_keyframe.ease_mode {
        0 => Some(start_value + (value_change * x)), 
        1 => Some(if x >= 1.0 { end_keyframe.value as f32 } else { start_value }), 
        2 => { 
            let ease_power = if start_keyframe.ease_power != 0 { start_keyframe.ease_power as f32 } else { 1.0 };
            let x_clamped = x.clamp(0.0, 1.0);
            let ease_factor = if ease_power >= 0.0 {
                1.0 - (1.0 - x_clamped.powf(ease_power)).sqrt()
            } else {
                (1.0 - (1.0 - x_clamped).powf(-ease_power)).sqrt()
            };
            Some(if ease_factor.is_nan() { start_value + (value_change * x) } else { start_value + (value_change * ease_factor) })
        },
        _ => Some(start_value + (value_change * x)) 
    }
}