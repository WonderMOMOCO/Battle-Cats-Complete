use eframe::egui;
use crate::global::formats::mamodel::{Model, ModelPart};

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

    for (part_index, _) in parts.iter().enumerate() {
        results.push(solve_single_part(part_index, parts, model));
    }
    
    // Stable sort by Z-order then ID
    results.sort_by(|part_a, part_b| {
        part_a.z_order.cmp(&part_b.z_order)
            .then(part_a.part_index.cmp(&part_b.part_index))
    });

    results
}

fn solve_single_part(target_index: usize, parts: &[ModelPart], model: &Model) -> WorldTransform {
    let target_part = &parts[target_index];
    
    // Build Parent Chain
    let mut chain = Vec::new();
    let mut current_index = target_index;
    let mut safety_counter = 0;
    
    loop {
        chain.push(current_index);
        let next_parent = parts[current_index].parent_id;
        
        // Root check
        if next_parent == -1 { break; }
        // Cycle check
        if next_parent as usize == current_index { break; } 
        
        current_index = next_parent as usize;
        // Bounds check
        if current_index >= parts.len() { break; }
        
        safety_counter += 1;
        if safety_counter > 100 { break; } // Prevent infinite loops
    }
    
    // Accumulate Global States
    let mut global_states = Vec::with_capacity(chain.len());
    let mut current_global = GlobalState::default();

    // Iterate backwards through chain
    for &part_index in chain.iter().rev() {
        let local_state = get_local_state(&parts[part_index], model);
        let new_flip_x = local_state.flip_x * current_global.flip_x;
        let new_flip_y = local_state.flip_y * current_global.flip_y;
        let new_scale_x = local_state.scale_x * current_global.scale_x;
        let new_scale_y = local_state.scale_y * current_global.scale_y;
        let new_angle = local_state.angle * new_flip_x * new_flip_y + current_global.angle;
        let new_opacity = local_state.opacity * current_global.opacity;

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
        position: [f64; 2],        // The local position of the node
        matrix_scale: [f64; 2],    // The scale/flip matrix params from the PARENT
        matrix_rotation: [f64; 4], // The rotation matrix params from the PARENT
    }

    let mut vector_steps = Vec::with_capacity(chain.len());
    if chain.len() > 1 {
        for step_index in 0..chain.len() - 1 {
            let child_index = chain[chain.len() - 1 - (step_index + 1)]; // i+1 from Root
            let parent_index = chain[chain.len() - 1 - step_index];      // i from Root
            
            let child_local_state = get_local_state(&parts[child_index], model);
            let parent_local_state = get_local_state(&parts[parent_index], model);
            let parent_global_flip_x = global_states[step_index].flip_x;
            let parent_global_flip_y = global_states[step_index].flip_y;
            
            let position = [child_local_state.x, -child_local_state.y];
            let scale_x_computed = parent_local_state.scale_x * parent_local_state.flip_x;
            let scale_y_computed = parent_local_state.scale_y * parent_local_state.flip_y;
            
            let angle_radians = parent_local_state.angle.to_radians() * parent_global_flip_x * parent_global_flip_y;
            let cosine_value = angle_radians.cos();
            let sine_value = angle_radians.sin();
            let rotation_matrix = [cosine_value, sine_value, -sine_value, cosine_value];

            vector_steps.push(VectorStep {
                position,
                matrix_scale: [scale_x_computed, scale_y_computed],
                matrix_rotation: rotation_matrix,
            });
        }
    }

    // Scale Application
    let step_count = vector_steps.len();
    for apply_index in 0..step_count {
        let current_scale = vector_steps[apply_index].matrix_scale;
        for modify_index in apply_index..step_count {
            vector_steps[modify_index].position[0] *= current_scale[0];
            vector_steps[modify_index].position[1] *= current_scale[1];
        }
    }

    let root_index = chain.last().copied().unwrap_or(target_index);
    let root_state = get_local_state(&parts[root_index], model);
    let root_part = &parts[root_index];
    
    // Inject the root pivot into the starting coordinates
    let mut final_position = [
        (root_state.x + root_part.pivot_x as f64) * root_state.scale_x * root_state.flip_x, 
        -(root_state.y + root_part.pivot_y as f64) * root_state.scale_y * root_state.flip_y
    ];

    for apply_index in 0..step_count {
        let current_rotation_matrix = vector_steps[apply_index].matrix_rotation;
        for modify_index in apply_index..step_count {
            let x = vector_steps[modify_index].position[0];
            let y = vector_steps[modify_index].position[1];
            
            let new_x = x * current_rotation_matrix[0] + y * current_rotation_matrix[1];
            let new_y = x * current_rotation_matrix[2] + y * current_rotation_matrix[3];
            
            vector_steps[modify_index].position = [new_x, new_y];
        }
        
        final_position[0] += vector_steps[apply_index].position[0];
        final_position[1] += vector_steps[apply_index].position[1];
    }

    // Construct Final Matrix
    let target_global = global_states.last().unwrap_or(&current_global);

    let final_scale_x = target_global.scale_x * target_global.flip_x;
    let final_scale_y = target_global.scale_y * target_global.flip_y;
    
    let angle_radians = target_global.angle.to_radians();
    let cosine_final = angle_radians.cos();
    let sine_final = angle_radians.sin();
    
    let matrix = [
        (final_scale_x * cosine_final) as f32,     (final_scale_x * sine_final) as f32,          0.0,
        (-final_scale_y * sine_final) as f32,      (final_scale_y * cosine_final) as f32,          0.0,
        final_position[0] as f32, -final_position[1] as f32,     1.0 
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