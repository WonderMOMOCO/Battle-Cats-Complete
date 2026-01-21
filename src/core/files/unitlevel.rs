use std::fs;
use std::path::Path;
use crate::core::utils;

#[derive(Clone, Debug, Default)]
pub struct CatLevelCurve {
    pub increments: Vec<u16>, 
}

impl CatLevelCurve {
    pub fn from_csv_line(csv_line: &str, delimiter: char) -> Self {
        let line_parts: Vec<&str> = csv_line.split(delimiter).collect();
        let mut increment_values = Vec::new();
        for part in line_parts {
            if let Ok(value) = part.trim().parse::<u16>() {
                increment_values.push(value);
            }
        }
        Self { increments: increment_values }
    }

    pub fn calculate_stat(&self, base_value: i32, target_level: i32) -> i32 {
        let base_float = base_value as f64;
        let mut current_stat = base_float;

        let max_scaled_level = (self.increments.len() * 10) as i32;
        let level_limit = std::cmp::min(target_level, max_scaled_level);

        for level_step in 2..=level_limit {
            let curve_index = ((level_step as f64 / 10.0).ceil() as usize).saturating_sub(1);
            if let Some(&scaling_factor) = self.increments.get(curve_index) {
                current_stat += base_float * (scaling_factor as f64) / 100.0;
            }
        }

        if target_level > max_scaled_level {
            let levels_above_limit = target_level - max_scaled_level;
            if let Some(&last_scaling_factor) = self.increments.last() {
                current_stat += base_float * (last_scaling_factor as f64) * (levels_above_limit as f64) / 100.0;
            }
        }

        let rounded_stat = current_stat.round();
        let final_stat = (rounded_stat * 2.5).floor();
        final_stat as i32
    }
}

pub fn load_level_curves(cats_directory: &Path) -> Vec<CatLevelCurve> {
    let mut curves_list = Vec::new();
    let level_file_path = cats_directory.join("unitlevel.csv");
    if let Ok(file_content) = fs::read_to_string(&level_file_path) {
        let delimiter = utils::detect_csv_separator(&file_content);

        for csv_line in file_content.lines() {
            curves_list.push(CatLevelCurve::from_csv_line(csv_line, delimiter));
        }
    }
    curves_list
}