use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StageRaw {
    pub width: u32,
    pub base_hp: u32,
    pub background_id: u32,
    pub max_enemies: u32,
    pub is_no_continues: bool,
    pub enemies: Vec<EnemyLine>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EnemyLine {
    pub id: u32,
    pub amount: u32,
    pub start_frame: u32,
    pub respawn_min: u32,
    pub respawn_max: u32,
    pub base_hp_perc: u32,
    pub boss_type: u32,
    pub magnification: u32,
    pub atk_magnification: u32,
    pub kill_count: u32,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> Option<StageRaw> {
    let paths = resolver::get(dir, filename, priority);
    let path = paths.first()?;
    let content = fs::read_to_string(path).ok()?;
    
    Some(parse(&content))
}

fn parse(content: &str) -> StageRaw {
    let sep = detect_csv_separator(content);
    let mut lines = content.lines()
        .map(|l| l.split("//").next().unwrap_or("").trim())
        .filter(|l| !l.is_empty());

    let mut stage = StageRaw::default();
    let first_line = lines.next().unwrap_or("");
    let parts: Vec<&str> = first_line.split(sep).collect();

    // Determine if header exists (Legacy EoC stages skip headers)
    let has_header = parts.len() <= 7 || parts.get(6).map_or(true, |s| s.is_empty());

    let config_line = if !has_header {
        first_line
    } else {
        stage.is_no_continues = parts.get(1) == Some(&"1");
        lines.next().unwrap_or("")
    };

    let c_parts: Vec<&str> = config_line.split(sep).collect();
    stage.width = c_parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.base_hp = c_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.background_id = c_parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.max_enemies = c_parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);

    for line in lines {
        let e_parts: Vec<&str> = line.split(sep).collect();
        let num = e_parts.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        if num == 0 { break; }

        let mag = e_parts.get(9).and_then(|s| if *s == "." { None } else { s.parse().ok() }).unwrap_or(100);
        
        stage.enemies.push(EnemyLine {
            id: if num >= 2 { num - 2 } else { 0 },
            amount: e_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
            start_frame: e_parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) * 2,
            respawn_min: e_parts.get(3).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) * 2,
            respawn_max: e_parts.get(4).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) * 2,
            base_hp_perc: e_parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            boss_type: e_parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0),
            magnification: mag,
            atk_magnification: e_parts.get(11).and_then(|s| s.parse().ok()).unwrap_or(mag),
            kill_count: e_parts.get(13).and_then(|s| s.parse().ok()).unwrap_or(0),
        });
    }

    stage
}