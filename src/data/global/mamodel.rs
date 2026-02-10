use std::fs;
use std::path::Path;
use crate::core::utils;

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
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.is_empty() { return None; }

        let mut part_count = 0;
        let mut data_start_index = 0;

        for (i, line) in lines.iter().take(5).enumerate() {
            if !line.contains(',') {
                if let Ok(val) = line.trim().parse::<usize>() {
                    if val > 0 && val < 1000 {
                        part_count = val;
                        data_start_index = i + 1;
                    }
                }
            } else { break; }
        }

        if part_count == 0 { return None; }

        let unit_line_index = data_start_index + part_count;
        
        // Initialize with defaults (Standard BC values)
        let mut scale_unit = 1000.0;
        let mut angle_unit = 3600.0; 
        let mut alpha_unit = 1000.0;

        // Try to read custom units if the line exists
        if lines.len() > unit_line_index {
            for i in unit_line_index..lines.len() {
                let p: Vec<&str> = lines[i].split(delimiter).collect();
                // FIX: Changed '==' to '>=' to handle files with extra parameters (like Unit 564)
                if p.len() >= 3 {
                     if let (Ok(s), Ok(a), Ok(o)) = (
                        p[0].trim().parse::<f32>(), 
                        p[1].trim().parse::<f32>(), 
                        p[2].trim().parse::<f32>()
                    ) {
                        if s != 0.0 { scale_unit = s; }
                        if a != 0.0 { angle_unit = a; }
                        if o != 0.0 { alpha_unit = o; }
                        break;
                    }
                }
            }
        }

        let mut parts = Vec::new();

        for i in 0..part_count {
            let line_idx = data_start_index + i;
            if line_idx >= lines.len() { break; }
            let line = lines[line_idx];
            let p: Vec<&str> = line.split(delimiter).collect();
            if p.len() < 13 { continue; } 

            let is_root = parts.is_empty();
            let raw_name = if p.len() > 13 { p[13].trim().to_string() } else { String::new() };

            let part = ModelPart {
                parent_id:     p[0].trim().parse().unwrap_or(-1),
                unit_id:       p[1].trim().parse().unwrap_or(0),
                sprite_index:  p[2].trim().parse().unwrap_or(0),
                drawing_layer: p[3].trim().parse().unwrap_or(0),
                position_x:    if is_root { 0.0 } else { p[4].trim().parse().unwrap_or(0.0) },
                position_y:    if is_root { 0.0 } else { p[5].trim().parse().unwrap_or(0.0) },
                pivot_x:       if is_root { 0.0 } else { p[6].trim().parse().unwrap_or(0.0) },
                pivot_y:       if is_root { 0.0 } else { p[7].trim().parse().unwrap_or(0.0) },
                scale_x:       p[8].trim().parse().unwrap_or(scale_unit), 
                scale_y:       p[9].trim().parse().unwrap_or(scale_unit),
                rotation:      p[10].trim().parse().unwrap_or(0.0),
                alpha:         p[11].trim().parse().unwrap_or(alpha_unit),
                glow_mode:     p[12].trim().parse().unwrap_or(0),
                flip_x:        false,
                flip_y:        false,
                name:          raw_name,
            };
            parts.push(part);
        }

        Some(Model { parts, version: 1, scale_unit, angle_unit, alpha_unit })
    }
}