use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BossType {
    None,
    Boss,
    ScreenShake,
    Unknown(u32),
}

impl From<u32> for BossType {
    fn from(boss_val: u32) -> Self {
        match boss_val {
            0 => Self::None,
            1 => Self::Boss,
            2 => Self::ScreenShake,
            _ => Self::Unknown(boss_val),
        }
    }
}

impl Default for BossType {
    fn default() -> Self { Self::None }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnemyAmount {
    Infinite,
    Limit(u32),
}

impl Default for EnemyAmount {
    fn default() -> Self { Self::Infinite }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct StageRaw {
    pub base_id: i32,
    pub width: u32,
    pub base_hp: u32,
    pub min_spawn: u32,
    pub background_id: u32,
    pub max_enemies: u32,
    pub anim_base_id: u32,
    pub is_no_continues: bool,
    pub is_base_indestructible: bool, 
    pub enemies: Vec<EnemyLine>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct EnemyLine {
    pub id: u32,
    pub amount: EnemyAmount,
    pub start_frame: u32,
    pub respawn_min: u32,
    pub respawn_max: u32,
    pub base_hp_perc: u32,
    pub boss_type: BossType,
    pub magnification: u32,
    pub atk_magnification: u32,
    pub kill_count: u32,
    pub is_base: bool, 
}

pub fn load(dir_path: &Path, filename: &str, lang_priority: &[String]) -> Option<StageRaw> {
    let file_paths = resolver::get(dir_path, &[filename], lang_priority);
    let first_path = file_paths.first()?;
    let file_content = fs::read_to_string(first_path).ok()?;
    
    Some(parse(&file_content))
}

fn parse(file_content: &str) -> StageRaw {
    let csv_separator = detect_csv_separator(file_content);
    let mut clean_lines = file_content.lines()
        .map(|line| line.split("//").next().unwrap_or("").trim())
        .filter(|line| !line.is_empty());

    let mut stage_raw = StageRaw::default();
    let first_line = clean_lines.next().unwrap_or("");
    let first_line_parts: Vec<&str> = first_line.split(csv_separator).collect();

    let has_header = first_line_parts.len() <= 7 || first_line_parts.get(6).map_or(true, |part| part.is_empty());

    let config_line = if !has_header {
        first_line
    } else {
        stage_raw.base_id = first_line_parts.get(0).and_then(|part| part.parse().ok()).unwrap_or(0);
        stage_raw.is_no_continues = first_line_parts.get(1) == Some(&"1");
        clean_lines.next().unwrap_or("")
    };

    let config_parts: Vec<&str> = config_line.split(csv_separator).collect();
    stage_raw.width = config_parts.get(0).and_then(|part| part.parse().ok()).unwrap_or(0);
    stage_raw.base_hp = config_parts.get(1).and_then(|part| part.parse().ok()).unwrap_or(0);
    stage_raw.min_spawn = config_parts.get(2).and_then(|part| part.parse().ok()).unwrap_or(0);
    stage_raw.background_id = config_parts.get(4).and_then(|part| part.parse().ok()).unwrap_or(0);
    stage_raw.max_enemies = config_parts.get(5).and_then(|part| part.parse().ok()).unwrap_or(0);
    stage_raw.anim_base_id = config_parts.get(6).and_then(|part| part.parse().ok()).unwrap_or(0);
    stage_raw.is_base_indestructible = config_parts.get(8).and_then(|part| part.parse::<u8>().ok()).unwrap_or(0) == 1;

    for enemy_line in clean_lines {
        let enemy_parts: Vec<&str> = enemy_line.split(csv_separator).collect();
        let enemy_id = enemy_parts.get(0).and_then(|part| part.parse::<u32>().ok()).unwrap_or(0);
        
        if enemy_id == 0 { 
            break; 
        }

        let raw_amount = enemy_parts.get(1).and_then(|part| part.parse::<u32>().ok()).unwrap_or(0);
        let mut spawn_amount = if raw_amount == 0 { EnemyAmount::Infinite } else { EnemyAmount::Limit(raw_amount) };
        
        let respawn_min = enemy_parts.get(3).and_then(|part| part.parse::<u32>().ok()).unwrap_or(0) * 2;
        let respawn_max = enemy_parts.get(4).and_then(|part| part.parse::<u32>().ok()).unwrap_or(0) * 2;

        if respawn_min == 0 {
            spawn_amount = EnemyAmount::Infinite;
        }

        let boss_type_val = enemy_parts.get(8).and_then(|part| part.parse::<u32>().ok()).unwrap_or(0);
        let mag_percent = enemy_parts.get(9).and_then(|part| if *part == "." { None } else { part.parse().ok() }).unwrap_or(100);
        
        let actual_enemy_id = if enemy_id >= 2 { enemy_id - 2 } else { 0 };
        let start_frame = enemy_parts.get(2).and_then(|part| part.parse::<u32>().ok()).unwrap_or(0) * 2;

        let is_ms_sign_default = actual_enemy_id == 21 && start_frame == 27000;
        if is_ms_sign_default {
            continue;
        }
        
        stage_raw.enemies.push(EnemyLine {
            id: actual_enemy_id,
            amount: spawn_amount,
            start_frame,
            respawn_min,
            respawn_max,
            base_hp_perc: enemy_parts.get(5).and_then(|part| part.parse().ok()).unwrap_or(0),
            boss_type: BossType::from(boss_type_val),
            magnification: mag_percent,
            atk_magnification: enemy_parts.get(11).and_then(|part| part.parse().ok()).unwrap_or(mag_percent),
            kill_count: enemy_parts.get(13).and_then(|part| part.parse().ok()).unwrap_or(0),
            is_base: enemy_id != 0 && enemy_id == stage_raw.anim_base_id, 
        });
    }

    stage_raw
}