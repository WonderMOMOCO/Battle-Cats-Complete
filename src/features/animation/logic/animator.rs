use crate::global::formats::mamodel::{Model, ModelPart};
use crate::global::formats::maanim::{Animation, AnimModification};

pub fn animate(model: &Model, animation: &Animation, global_frame: f32) -> Vec<ModelPart> {
    let mut parts = model.parts.clone();

    for curve in &animation.curves {
        if curve.part_id >= parts.len() { continue; }
        if curve.keyframes.is_empty() { continue; }
        
        let keyframe_min = curve.keyframes.first().map(|keyframe| keyframe.frame as f32).unwrap_or(0.0);
        let keyframe_max = curve.keyframes.last().map(|keyframe| keyframe.frame as f32).unwrap_or(0.0);

        let duration = (keyframe_max - keyframe_min).max(1.0);
        let mut local_frame = global_frame;

        if curve.loop_count != 1 {
            local_frame = (global_frame - keyframe_min).rem_euclid(duration) + keyframe_min;
        }

        let is_discrete = matches!(curve.modification_type, 0 | 1 | 3 | 13 | 14);
        
        let Some(interpolated_value) = interpolate_curve(curve, local_frame, is_discrete) else {
            continue;
        };
        
        let part = &mut parts[curve.part_id];
        let base_part = &model.parts[curve.part_id];
        
        match curve.modification_type {
            0 => part.parent_id = interpolated_value as i32,
            1 => part.unit_id = interpolated_value as i32,
            3 => part.drawing_layer = interpolated_value as i32, 
            
            2 => { part.sprite_index = interpolated_value as i32; },

            4 => part.position_x = base_part.position_x + interpolated_value, 
            5 => part.position_y = base_part.position_y + interpolated_value,
            6 => part.pivot_x = base_part.pivot_x + interpolated_value,
            7 => part.pivot_y = base_part.pivot_y + interpolated_value,
            8 => { 
                let scale_factor = interpolated_value / model.scale_unit;
                part.scale_x = base_part.scale_x * scale_factor;
                part.scale_y = base_part.scale_y * scale_factor;
            },
            9 => {
                let scale_factor = interpolated_value / model.scale_unit;
                part.scale_x = base_part.scale_x * scale_factor;
            },
            10 => {
                let scale_factor = interpolated_value / model.scale_unit;
                part.scale_y = base_part.scale_y * scale_factor;
            },
            11 => part.rotation = base_part.rotation + interpolated_value,
            12 => part.alpha = base_part.alpha * (interpolated_value / model.alpha_unit),
            
            13 => {
                part.flip_x = interpolated_value != 0.0;
            },
            14 => {
                part.flip_y = interpolated_value != 0.0;
            },
            _ => {}
        }
    }
    
    parts
}

fn interpolate_curve(curve: &AnimModification, frame: f32, is_discrete: bool) -> Option<f32> {
    if curve.keyframes.is_empty() { return None; }

    let first_keyframe = &curve.keyframes[0];
    if frame < first_keyframe.frame as f32 {
        return None; 
    }

    let mut start_index = 0;
    let mut end_index = 0;
    let mut is_found = false;

    for (index, keyframe) in curve.keyframes.iter().enumerate() {
        if (keyframe.frame as f32) > frame {
            end_index = index;
            start_index = if index > 0 { index - 1 } else { 0 };
            is_found = true;
            break;
        }
    }
    
    if !is_found {
        let Some(last_keyframe) = curve.keyframes.last() else { return None; };
        return Some((last_keyframe.value as f32).trunc());
    }
    
    if end_index == 0 {
         return Some((curve.keyframes[0].value as f32).trunc());
    }

    let start_keyframe = &curve.keyframes[start_index];
    let end_keyframe = &curve.keyframes[end_index];

    if is_discrete { return Some((start_keyframe.value as f32).trunc()); }
    if start_keyframe.frame == end_keyframe.frame { return Some((start_keyframe.value as f32).trunc()); }

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
        return Some(final_result.trunc());
    }

    let time_duration = (end_keyframe.frame - start_keyframe.frame) as f32;
    let time_current = frame - (start_keyframe.frame as f32);
    let x = time_current / time_duration;

    let start_value = start_keyframe.value as f32;
    let value_change = (end_keyframe.value - start_keyframe.value) as f32;

    let interpolated_value = match start_keyframe.ease_mode {
        0 => start_value + (value_change * x), 
        1 => if x >= 1.0 { end_keyframe.value as f32 } else { start_value }, 
        2 => { 
            let ease_power = if start_keyframe.ease_power != 0 { start_keyframe.ease_power as f32 } else { 1.0 };
            let x_clamped = x.clamp(0.0, 1.0);
            let ease_factor = if ease_power >= 0.0 {
                1.0 - (1.0 - x_clamped.powf(ease_power)).sqrt()
            } else {
                (1.0 - (1.0 - x_clamped).powf(-ease_power)).sqrt()
            };
            
            if ease_factor.is_nan() { 
                start_value + (value_change * x) 
            } else { 
                start_value + (value_change * ease_factor) 
            }
        },
        _ => start_value + (value_change * x) 
    };

    if curve.modification_type == 2 {
        if value_change < 0.0 {
            return Some(interpolated_value.ceil());
        } else {
            return Some(interpolated_value.floor());
        }
    }

    Some(interpolated_value.trunc())
}