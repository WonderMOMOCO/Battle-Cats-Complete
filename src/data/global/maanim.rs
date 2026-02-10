use std::fs;
use std::path::Path;
use crate::core::utils;

// Math Helpers
fn gcd(a: i32, b: i32) -> i32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

// CHANGED: Returns i64 to prevent overflow during calculation
fn lcm(a: i32, b: i32) -> i64 {
    if a == 0 || b == 0 { 0 } else { (a as i64 * b as i64).abs() / gcd(a, b) as i64 }
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
        let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.is_empty() { return None; }

        let mut curves = Vec::new();
        let mut i = 0;

        if i < lines.len() && lines[i].starts_with("[") { i += 1; }
        if i < lines.len() { i += 1; } 
        if i < lines.len() { i += 1; } 

        while i < lines.len() {
            let line = lines[i];
            let parts: Vec<&str> = line.split(delimiter).collect();
            i += 1;

            if parts.len() < 5 { continue; }

            let part_id = parts[0].trim().parse().unwrap_or(0);
            let mod_type = parts[1].trim().parse().unwrap_or(0);
            let loop_behavior = parts[2].trim().parse().unwrap_or(0);
            let min_f = parts[3].trim().parse().unwrap_or(0);
            let max_f = parts[4].trim().parse().unwrap_or(0);
            
            if i >= lines.len() { break; }
            let count_line = lines[i];
            i += 1;
            let keyframe_count = count_line.trim().parse::<usize>().unwrap_or(0);

            let mut keyframes = Vec::new();

            for _ in 0..keyframe_count {
                if i >= lines.len() { break; }
                let k_line = lines[i];
                i += 1;
                let kp: Vec<&str> = k_line.split(delimiter).collect();
                if kp.len() >= 2 {
                    let frame = kp[0].trim().parse().unwrap_or(0);
                    let value = kp[1].trim().parse().unwrap_or(0);
                    let ease = kp.get(2).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    let power = kp.get(3).and_then(|s| s.trim().parse().ok()).unwrap_or(0);
                    keyframes.push(Keyframe { frame, value, ease_mode: ease, ease_power: power });
                }
            }

            if !keyframes.is_empty() {
                curves.push(AnimModification {
                    part_id,
                    modification_type: mod_type,
                    loop_count: loop_behavior,
                    keyframes,
                    min_frame: min_f,
                    max_frame: max_f,
                });
            }
        }

        let mut max_len = 0;
        for c in &curves {
            if let Some(last) = c.keyframes.last() {
                if last.frame > max_len { max_len = last.frame; }
            }
        }

        Some(Self { curves, max_frame: max_len })
    }

    /// Calculates the LCM (Least Common Multiple) of all looping curves.
    /// Returns:
    /// - `Some(frame_count)` if the loop is finite and within the safety limit (999,999).
    /// - `None` if the loop is effectively infinite, too large, or causes overflow.
    pub fn calculate_true_loop(&self) -> Option<i32> {
        let mut overall_lcm: i64 = 1;
        let mut found_looping_part = false;
        
        for curve in &self.curves {
            // Check parts that loop infinitely (or standard loops)
            if curve.loop_count != 1 {
                if let (Some(first), Some(last)) = (curve.keyframes.first(), curve.keyframes.last()) {
                    let duration = (last.frame - first.frame) as i32; 
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
        
        // CHANGED: Fallback Logic
        // 1. If calculation resulted in overflow (very unlikely with i64 but good hygiene)
        // 2. If result exceeds 999,999 (User requested limit)
        if overall_lcm > 999_999 {
            return None; // Treat as "???", Infinite / Continuous
        }

        Some(std::cmp::max(overall_lcm as i32, self.max_frame))
    }

    pub fn scan_duration(file_content: &str) -> i32 {
        let mut max_frame_count = 0;
        let delimiter = utils::detect_csv_separator(file_content);
        let maanim_lines: Vec<Vec<i32>> = file_content.lines().map(|line| {
            line.split(delimiter).filter_map(|c| c.trim().parse::<i32>().ok()).collect()
        }).collect();

        for (i, val) in maanim_lines.iter().enumerate() {
            if val.len() < 5 { continue; }
            let follow = maanim_lines.get(i+1).and_then(|l| l.get(0)).cloned().unwrap_or(0) as usize;
            if follow == 0 { continue; }
            let first = maanim_lines.get(i+2).and_then(|l| l.get(0)).cloned().unwrap_or(0);
            let last = maanim_lines.get(i+follow+1).and_then(|l| l.get(0)).cloned().unwrap_or(0);
            let dur = last - first;
            let reps = std::cmp::max(val[2], 1);
            max_frame_count = std::cmp::max((dur * reps) + first, max_frame_count);
        }
        max_frame_count
    }
}