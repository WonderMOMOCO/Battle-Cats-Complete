use std::fs;
use std::path::Path;
use crate::global::utils;

#[derive(Clone, Debug)]
pub struct ModelPart {
    pub parent_id: i32,
    pub unit_id: i32,
    pub sprite_index: i32,
    pub drawing_layer: i32,
    pub position_x: f32,
    pub position_y: f32,
    pub pivot_x: f32,
    pub pivot_y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub rotation: f32,
    pub alpha: f32,
    pub glow_mode: i32,
    pub flip_x: bool,
    pub flip_y: bool,
    #[allow(dead_code)] pub name: String,
}

impl Default for ModelPart {
    fn default() -> Self {
        Self {
            parent_id: -1,
            unit_id: 0,
            sprite_index: 0,
            drawing_layer: 0,
            position_x: 0.0,
            position_y: 0.0,
            pivot_x: 0.0,
            pivot_y: 0.0,
            scale_x: 1000.0,
            scale_y: 1000.0,
            rotation: 0.0,
            alpha: 1000.0,
            glow_mode: 0,
            flip_x: false,
            flip_y: false,
            name: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Model {
    pub parts: Vec<ModelPart>,
    #[allow(dead_code)] pub version: u32,
    pub scale_unit: f32, 
    pub angle_unit: f32,
    pub alpha_unit: f32,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            parts: Vec::new(),
            version: 0,
            scale_unit: 1000.0,
            angle_unit: 3600.0,
            alpha_unit: 1000.0,
        }
    }
}

impl Model {
    pub fn load(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        let delimiter = utils::detect_csv_separator(&content);
        let lines: Vec<&str> = content.lines().filter(|line_ref| !line_ref.trim().is_empty()).collect();

        if lines.is_empty() { return None; }

        let mut part_count = 0;
        let mut data_start_index = 0;

        // Get Part Count
        for (index, line) in lines.iter().take(5).enumerate() {
            if line.contains(',') { break; }
            
            let Ok(parsed_count) = line.trim().parse::<usize>() else { continue; };
            if parsed_count == 0 || parsed_count >= 1000 { continue; }
            
            part_count = parsed_count;
            data_start_index = index + 1;
        }

        if part_count == 0 { return None; }

        let unit_line_index = data_start_index + part_count;
        
        let mut scale_unit = 1000.0;
        let mut angle_unit = 3600.0; 
        let mut alpha_unit = 1000.0;
        let mut metadata_start_index = usize::MAX;

        // Read Custom Units
        for index in unit_line_index..lines.len() {
            let columns: Vec<&str> = lines[index].split(delimiter).collect();
            if columns.len() < 3 { continue; }
            
            let Ok(scale_val) = columns[0].trim().parse::<f32>() else { continue; };
            let Ok(angle_val) = columns[1].trim().parse::<f32>() else { continue; };
            let Ok(alpha_val) = columns[2].trim().parse::<f32>() else { continue; };

            if scale_val != 0.0 { scale_unit = scale_val; }
            if angle_val != 0.0 { angle_unit = angle_val; }
            if alpha_val != 0.0 { alpha_unit = alpha_val; }
            
            metadata_start_index = index + 1;
            break;
        }

        let mut parts = Vec::new();

        // Parse Parts
        for index in 0..part_count {
            let target_line_idx = data_start_index + index;
            if target_line_idx >= lines.len() { break; }
            
            let columns: Vec<&str> = lines[target_line_idx].split(delimiter).collect();
            if columns.len() < 13 { continue; } 

            let is_root = parts.is_empty();
            let raw_name = if columns.len() > 13 { columns[13].trim().to_string() } else { String::new() };

            parts.push(ModelPart {
                parent_id:     columns[0].trim().parse().unwrap_or(-1),
                unit_id:       columns[1].trim().parse().unwrap_or(0),
                sprite_index:  columns[2].trim().parse().unwrap_or(0),
                drawing_layer: columns[3].trim().parse().unwrap_or(0),
                position_x:    if is_root { 0.0 } else { columns[4].trim().parse().unwrap_or(0.0) },
                position_y:    if is_root { 0.0 } else { columns[5].trim().parse().unwrap_or(0.0) },
                pivot_x:       columns[6].trim().parse().unwrap_or(0.0),
                pivot_y:       columns[7].trim().parse().unwrap_or(0.0),
                scale_x:       columns[8].trim().parse().unwrap_or(scale_unit), 
                scale_y:       columns[9].trim().parse().unwrap_or(scale_unit),
                rotation:      columns[10].trim().parse().unwrap_or(0.0),
                alpha:         columns[11].trim().parse().unwrap_or(alpha_unit),
                glow_mode:     columns[12].trim().parse().unwrap_or(0),
                flip_x:        false,
                flip_y:        false,
                name:          raw_name,
            });
        }

        // Apply Metadata Global Offset
        let _ = (|| -> Option<()> {
            if parts.is_empty() { return None; }
            
            let metadata_count = lines.get(metadata_start_index)?.trim().parse::<usize>().ok()?;
            if metadata_count == 0 { return None; }
            
            let columns: Vec<&str> = lines.get(metadata_start_index + 1)?.split(delimiter).collect();
            if columns.len() < 4 { return None; }

            parts[0].position_x = -columns[2].trim().parse::<f32>().unwrap_or(0.0);
            parts[0].position_y = -columns[3].trim().parse::<f32>().unwrap_or(0.0);
            
            Some(())
        })();

        Some(Model { parts, version: 1, scale_unit, angle_unit, alpha_unit })
    }
}