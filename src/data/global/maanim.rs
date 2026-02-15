use std::fs;
use std::path::Path;
use crate::core::utils;

// Math Helpers
fn gcd(number1: i32, number2: i32) -> i32 {
    if number2 == 0 { number1 } else { gcd(number2, number1 % number2) }
}

fn lcm(number1: i32, number2: i32) -> i64 {
    if number1 == 0 || number2 == 0 { 
        0 
    } else { 
        (number1 as i64 * number2 as i64).abs() / gcd(number1, number2) as i64 
    }
}

#[derive(Clone, Debug)]
pub struct Keyframe {
    pub frame: i32,
    pub value: i32,
    pub ease_mode: i32,
    pub ease_power: i32,
}

#[derive(Clone, Debug)]
pub struct AnimModification {
    pub part_id: usize,
    pub modification_type: i32,
    pub loop_count: i32,
    pub keyframes: Vec<Keyframe>,
    #[allow(dead_code)] pub min_frame: i32,
    #[allow(dead_code)] pub max_frame: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Animation {
    pub curves: Vec<AnimModification>,
    pub max_frame: i32,
}

impl Animation {
    pub fn load(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        let delimiter = utils::detect_csv_separator(&content);
        let lines: Vec<&str> = content.lines().filter(|line| !line.trim().is_empty()).collect();

        if lines.is_empty() { return None; }

        // Helper to replace repetitive parsing logic
        fn parse_num<T: std::str::FromStr + Default>(input_string: &str) -> T {
            input_string.trim().parse().unwrap_or_default()
        }

        let mut curves = Vec::new();
        let mut current_line_idx = 0;

        // Skip standard headers
        if current_line_idx < lines.len() && lines[current_line_idx].starts_with("[") { 
            current_line_idx += 1; 
        }
        if current_line_idx < lines.len() { current_line_idx += 1; } 
        if current_line_idx < lines.len() { current_line_idx += 1; } 

        while current_line_idx < lines.len() {
            let current_line = lines[current_line_idx];
            let parts: Vec<&str> = current_line.split(delimiter).collect();
            current_line_idx += 1;

            if parts.len() < 5 { continue; }

            // Using the helper function
            let part_id: usize = parse_num(parts[0]);
            let mod_type: i32 = parse_num(parts[1]);
            let loop_behavior: i32 = parse_num(parts[2]);
            let min_frame: i32 = parse_num(parts[3]);
            let max_frame: i32 = parse_num(parts[4]);
            
            if current_line_idx >= lines.len() { break; }
            let count_line = lines[current_line_idx];
            current_line_idx += 1;
            
            let keyframe_count: usize = parse_num(count_line);

            let mut keyframes = Vec::new();

            for _ in 0..keyframe_count {
                if current_line_idx >= lines.len() { break; }
                let keyframe_line = lines[current_line_idx];
                current_line_idx += 1;
                let keyframe_parts: Vec<&str> = keyframe_line.split(delimiter).collect();
                
                if keyframe_parts.len() >= 2 {
                    let frame: i32 = parse_num(keyframe_parts[0]);
                    let value: i32 = parse_num(keyframe_parts[1]);
                    
                    // Explicit variable names for closures
                    let ease_mode = keyframe_parts.get(2)
                        .map_or(0, |text_part| parse_num(text_part));
                        
                    let ease_power = keyframe_parts.get(3)
                        .map_or(0, |text_part| parse_num(text_part));
                    
                    keyframes.push(Keyframe { frame, value, ease_mode, ease_power });
                }
            }

            if !keyframes.is_empty() {
                curves.push(AnimModification {
                    part_id,
                    modification_type: mod_type,
                    loop_count: loop_behavior,
                    keyframes,
                    min_frame,
                    max_frame,
                });
            }
        }

        let mut max_len = 0;
        for curve in &curves {
            if let Some(last_keyframe) = curve.keyframes.last() {
                if last_keyframe.frame > max_len { max_len = last_keyframe.frame; }
            }
        }

        Some(Self { curves, max_frame: max_len })
    }

    pub fn calculate_true_loop(&self) -> Option<i32> {
        let mut overall_lcm: i64 = 1;
        let mut found_looping_part = false;
        
        for curve in &self.curves {
            if curve.loop_count == 1 {
                return None;
            }

            if curve.loop_count != 1 {
                if let (Some(first_keyframe), Some(last_keyframe)) = (curve.keyframes.first(), curve.keyframes.last()) {
                    let duration = (last_keyframe.frame - first_keyframe.frame) as i32; 
                    if duration > 0 {
                        overall_lcm = lcm(overall_lcm as i32, duration);
                        found_looping_part = true;
                    }
                }
            }
        }
        
        if !found_looping_part {
            return Some(self.max_frame);
        }
        
        if overall_lcm > 999_999 {
            return None; 
        }

        Some(std::cmp::max(overall_lcm as i32, self.max_frame))
    }

    pub fn scan_duration(file_content: &str) -> i32 {
        let mut max_frame_count = 0;
        let delimiter = utils::detect_csv_separator(file_content);
        
        let maanim_lines: Vec<Vec<i32>> = file_content.lines().map(|line| {
            line.split(delimiter)
                .filter_map(|text_part| text_part.trim().parse::<i32>().ok())
                .collect()
        }).collect();

        for (line_index, line_values) in maanim_lines.iter().enumerate() {
            if line_values.len() < 5 { continue; }
            
            let following_lines_count = maanim_lines
                .get(line_index + 1)
                .and_then(|line| line.get(0))
                .cloned()
                .unwrap_or(0) as usize;
                
            if following_lines_count == 0 { continue; }
            
            let first_frame = maanim_lines
                .get(line_index + 2)
                .and_then(|line| line.get(0))
                .cloned()
                .unwrap_or(0);
                
            let last_frame = maanim_lines
                .get(line_index + following_lines_count + 1)
                .and_then(|line| line.get(0))
                .cloned()
                .unwrap_or(0);
            
            let duration = last_frame - first_frame;
            let repeats = std::cmp::max(line_values[2], 1);
            
            max_frame_count = std::cmp::max((duration * repeats) + first_frame, max_frame_count);
        }
        max_frame_count
    }
}