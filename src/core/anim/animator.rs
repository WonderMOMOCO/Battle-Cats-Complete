use crate::data::global::mamodel::{Model, ModelPart};
use crate::data::global::maanim::{Animation, AnimModification};

pub fn animate(model: &Model, animation: &Animation, global_frame: f32) -> Vec<ModelPart> {
    let mut parts = model.parts.clone();

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

        let is_discrete = matches!(curve.modification_type, 0 | 1 | 3 | 13 | 14);
        
        if let Some(val) = interpolate_curve(curve, local_frame, is_discrete) {
            
            let part = &mut parts[curve.part_id];
            let base_part = &model.parts[curve.part_id];
            
            match curve.modification_type {
                0 => part.parent_id = val as i32,
                1 => part.unit_id = val as i32,
                3 => part.drawing_layer = val as i32, 
                
                2 => { part.sprite_index = val as i32; },

                4 => part.position_x = base_part.position_x + val, 
                5 => part.position_y = base_part.position_y + val,
                6 => part.pivot_x = base_part.pivot_x + val,
                7 => part.pivot_y = base_part.pivot_y + val,
                 // Change to modifications 8/9/10 are essential
                 // for Kaihime, thanks SweetDonut0
                8 => { 
                    let factor = val / model.scale_unit;
                    part.scale_x = base_part.scale_x * factor;
                    part.scale_y = base_part.scale_y * factor;
                },
                9 => {
                    let factor = val / model.scale_unit;
                    part.scale_x = base_part.scale_x * factor;
                },
                10 => {
                    let factor = val / model.scale_unit;
                    part.scale_y = base_part.scale_y * factor;
                },
                11 => part.rotation = base_part.rotation + val,
                12 => part.alpha = base_part.alpha * (val / model.alpha_unit),
                
                13 => {
                    part.flip_x = val != 0.0;
                },
                14 => {
                    part.flip_y = val != 0.0;
                },
                _ => {}
            }
        }
    }
    
    parts
}

fn interpolate_curve(curve: &AnimModification, frame: f32, is_discrete: bool) -> Option<f32> {
    if curve.keyframes.is_empty() { return None; }

    let first_k = &curve.keyframes[0];
    if frame < first_k.frame as f32 {
        return None; 
    }

    let mut start_idx = 0;
    let mut end_idx = 0;
    let mut found = false;

    for (i, k) in curve.keyframes.iter().enumerate() {
        if (k.frame as f32) > frame {
            end_idx = i;
            start_idx = if i > 0 { i - 1 } else { 0 };
            found = true;
            break;
        }
    }
    
    if !found {
        return Some(curve.keyframes.last().unwrap().value as f32);
    }
    if end_idx == 0 {
         return Some(curve.keyframes[0].value as f32);
    }

    let start_k = &curve.keyframes[start_idx];
    let end_k = &curve.keyframes[end_idx];

    if is_discrete { return Some(start_k.value as f32); }
    if start_k.frame == end_k.frame { return Some(start_k.value as f32); }

    if start_k.ease_mode == 3 {
        let mut points = Vec::new();
        let mut i = start_idx as isize;
        while i >= 0 {
            let k = &curve.keyframes[i as usize];
            if (i as usize) != start_idx && k.ease_mode != 3 { break; }
            points.push((k.frame as f32, k.value as f32));
            i -= 1;
        }
        points.reverse(); 
        let mut i = end_idx;
        while i < curve.keyframes.len() {
            let k = &curve.keyframes[i];
            points.push((k.frame as f32, k.value as f32));
            if k.ease_mode != 3 { break; }
            i += 1;
        }

        let mut result = 0.0;
        let n = points.len();
        for j in 0..n {
            let (xj, yj) = points[j];
            let mut prod = yj;
            for m in 0..n {
                if j == m { continue; }
                let (xm, _) = points[m];
                if (xj - xm).abs() > 0.0001 {
                    prod *= (frame - xm) / (xj - xm);
                }
            }
            result += prod;
        }
        return Some(result);
    }

    let t_duration = (end_k.frame - start_k.frame) as f32;
    let t_current = frame - (start_k.frame as f32);
    let x = t_current / t_duration;

    let start_val = start_k.value as f32;
    let change = (end_k.value - start_k.value) as f32;

    let interpolated_val = match start_k.ease_mode {
        0 => start_val + (change * x), 
        1 => if x >= 1.0 { end_k.value as f32 } else { start_val }, 
        2 => { 
            let p = if start_k.ease_power != 0 { start_k.ease_power as f32 } else { 1.0 };
            let x_clamped = x.clamp(0.0, 1.0);
            let factor = if p >= 0.0 {
                1.0 - (1.0 - x_clamped.powf(p)).sqrt()
            } else {
                (1.0 - (1.0 - x_clamped).powf(-p)).sqrt()
            };
            if factor.is_nan() { start_val + (change * x) } else { start_val + (change * factor) }
        },
        _ => start_val + (change * x) 
    };

    if curve.modification_type == 2 {
        if change < 0.0 {
            return Some(interpolated_val.ceil());
        } else {
            return Some(interpolated_val.floor());
        }
    }

    Some(interpolated_val)
}