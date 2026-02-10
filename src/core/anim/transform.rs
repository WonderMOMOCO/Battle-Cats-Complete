use eframe::egui;
use crate::data::global::mamodel::{Model, ModelPart};

#[derive(Clone, Copy, Debug)]
pub struct WorldTransform {
    pub matrix: [f32; 9], 
    pub opacity: f32,
    pub z_order: i32,
    pub sprite_index: usize,
    pub pivot: egui::Vec2,
    pub hidden: bool,
    pub glow: i32,
    pub part_index: usize, 
}

// Represents the normalized local state of a part
#[derive(Clone, Copy, Debug)]
struct LocalState {
    x: f64,
    y: f64,
    scale_x: f64,
    scale_y: f64,
    angle: f64,   
    opacity: f64,
    flip_x: f64,
    flip_y: f64,
}

// Represents the accumulated global state needed for hierarchy calculations
#[derive(Clone, Copy, Debug)]
struct GlobalState {
    // Note: Position is calculated separately via vectors, so not stored here
    scale_x: f64,
    scale_y: f64,
    angle: f64,
    flip_x: f64,
    flip_y: f64,
    opacity: f64,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 0.0,
            flip_x: 1.0,
            flip_y: 1.0,
            opacity: 1.0,
        }
    }
}

pub fn solve_hierarchy(parts: &[ModelPart], model: &Model) -> Vec<WorldTransform> {
    let mut results = Vec::with_capacity(parts.len());

    for (part_idx, _) in parts.iter().enumerate() {
        results.push(solve_single_part(part_idx, parts, model));
    }
    
    // Stable sort by Z-order then ID
    results.sort_by(|a, b| {
        a.z_order.cmp(&b.z_order)
            .then(a.part_index.cmp(&b.part_index))
    });

    results
}

fn solve_single_part(target_index: usize, parts: &[ModelPart], model: &Model) -> WorldTransform {
    let target_part = &parts[target_index];
    
    // Build Parent Chain
    let mut chain = Vec::new();
    let mut curr = target_index;
    let mut safety = 0;
    
    loop {
        chain.push(curr);
        let next_parent = parts[curr].parent_id;
        
        // Root check
        if next_parent == -1 { break; }
        // Cycle check
        if next_parent as usize == curr { break; } 
        
        curr = next_parent as usize;
        // Bounds check
        if curr >= parts.len() { break; }
        
        safety += 1;
        if safety > 100 { break; } // Prevent infinite loops
    }
    
    // Accumulate Global States
    let mut global_states = Vec::with_capacity(chain.len());
    let mut current_global = GlobalState::default();

    // Iterate backwards through chain
    for &part_idx in chain.iter().rev() {
        let local = get_local_state(&parts[part_idx], model);
        let new_flip_x = local.flip_x * current_global.flip_x;
        let new_flip_y = local.flip_y * current_global.flip_y;
        let new_scale_x = local.scale_x * current_global.scale_x;
        let new_scale_y = local.scale_y * current_global.scale_y;
        let new_angle = local.angle * new_flip_x * new_flip_y + current_global.angle;
        let new_opacity = local.opacity * current_global.opacity;

        current_global = GlobalState {
            scale_x: new_scale_x,
            scale_y: new_scale_y,
            angle: new_angle,
            flip_x: new_flip_x,
            flip_y: new_flip_y,
            opacity: new_opacity,
        };
        
        global_states.push(current_global);
    }
    struct VectorStep {
        pos: [f64; 2],      // The local position of the node
        matrix_scale: [f64; 2], // The scale/flip matrix params from the PARENT
        matrix_rot: [f64; 4],   // The rotation matrix params from the PARENT
    }

    let mut vector_steps = Vec::with_capacity(chain.len());
    if chain.len() > 1 {
        
        for idx in 0..chain.len() - 1 {
            let child_idx = chain[chain.len() - 1 - (idx + 1)]; // i+1 from Root
            let parent_idx = chain[chain.len() - 1 - idx];     // i from Root
            
            let child_local = get_local_state(&parts[child_idx], model);
            let parent_local = get_local_state(&parts[parent_idx], model);
            let parent_global_flip_x = global_states[idx].flip_x;
            let parent_global_flip_y = global_states[idx].flip_y;
            let pos = [child_local.x, -child_local.y];
            let scale_x_comp = parent_local.scale_x * parent_local.flip_x;
            let scale_y_comp = parent_local.scale_y * parent_local.flip_y;
            let angle_rad = parent_local.angle.to_radians() * parent_global_flip_x * parent_global_flip_y;
            let cos_val = angle_rad.cos();
            let sin_val = angle_rad.sin();
            let rot_matrix = [cos_val, sin_val, -sin_val, cos_val];

            vector_steps.push(VectorStep {
                pos,
                matrix_scale: [scale_x_comp, scale_y_comp],
                matrix_rot: rot_matrix,
            });
        }
    }

    // Scale Application
    let step_count = vector_steps.len();
    for apply_idx in 0..step_count {
        let scale = vector_steps[apply_idx].matrix_scale;
        for target_idx in apply_idx..step_count {
            vector_steps[target_idx].pos[0] *= scale[0];
            vector_steps[target_idx].pos[1] *= scale[1];
        }
    }

    // Rotation Application and Summation
    let mut final_pos = [0.0, 0.0];
    for apply_idx in 0..step_count {
        let rot_matrix = vector_steps[apply_idx].matrix_rot;
        for target_idx in apply_idx..step_count {
            let x = vector_steps[target_idx].pos[0];
            let y = vector_steps[target_idx].pos[1];
            
            let new_x = x * rot_matrix[0] + y * rot_matrix[1];
            let new_y = x * rot_matrix[2] + y * rot_matrix[3];
            
            vector_steps[target_idx].pos = [new_x, new_y];
        }
        
        final_pos[0] += vector_steps[apply_idx].pos[0];
        final_pos[1] += vector_steps[apply_idx].pos[1];
    }

    // Construct Final Matrix
    let target_global = if !global_states.is_empty() {
        global_states.last().unwrap()
    } else {
        &current_global
    };

    let final_scale_x = target_global.scale_x * target_global.flip_x;
    let final_scale_y = target_global.scale_y * target_global.flip_y;
    
    let angle_rad = target_global.angle.to_radians();
    let cos_final = angle_rad.cos();
    let sin_final = angle_rad.sin();
    
    let matrix = [
        (final_scale_x * cos_final) as f32,     (final_scale_x * sin_final) as f32,          0.0,
        (-final_scale_y * sin_final) as f32,    (final_scale_y * cos_final) as f32,          0.0,
        final_pos[0] as f32, -final_pos[1] as f32,     1.0 
    ];

    WorldTransform {
        matrix,
        opacity: target_global.opacity as f32,
        z_order: target_part.drawing_layer,
        sprite_index: target_part.sprite_index as usize,
        pivot: egui::vec2(target_part.pivot_x as f32, target_part.pivot_y as f32),
        hidden: target_part.unit_id == -1 || target_part.sprite_index == -1 || target_global.opacity < 0.001,
        glow: target_part.glow_mode,
        part_index: target_index,
    }
}

fn get_local_state(part: &ModelPart, model: &Model) -> LocalState {
    let scale_unit = if model.scale_unit == 0.0 { 1000.0 } else { model.scale_unit as f64 };
    let angle_unit = if model.angle_unit == 0.0 { 1000.0 } else { model.angle_unit as f64 };
    let alpha_unit = if model.alpha_unit == 0.0 { 1000.0 } else { model.alpha_unit as f64 };

    LocalState {
        x: part.position_x as f64, // Raw pixels
        y: part.position_y as f64, // Raw pixels
        scale_x: part.scale_x as f64 / scale_unit,
        scale_y: part.scale_y as f64 / scale_unit,
        angle: (part.rotation as f64) * 360.0 / angle_unit,
        opacity: part.alpha as f64 / alpha_unit,
        flip_x: if part.flip_x { -1.0 } else { 1.0 },
        flip_y: if part.flip_y { -1.0 } else { 1.0 },
    }
}