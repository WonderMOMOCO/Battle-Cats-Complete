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
    fn from(val: u32) -> Self {
        match val {
            0 => Self::None,
            1 => Self::Boss,
            2 => Self::ScreenShake,
            _ => Self::Unknown(val),
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

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> Option<StageRaw> {
    let paths = resolver::get(dir, &[filename], priority);
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

    let has_header = parts.len() <= 7 || parts.get(6).map_or(true, |s| s.is_empty());

    let config_line = if !has_header {
        first_line
    } else {
        stage.base_id = parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
        stage.is_no_continues = parts.get(1) == Some(&"1");
        lines.next().unwrap_or("")
    };

    let c_parts: Vec<&str> = config_line.split(sep).collect();
    stage.width = c_parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.base_hp = c_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.background_id = c_parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.max_enemies = c_parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.anim_base_id = c_parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
    stage.is_base_indestructible = c_parts.get(8).and_then(|s| s.parse::<u8>().ok()).unwrap_or(0) == 1;

    for line in lines {
        let e_parts: Vec<&str> = line.split(sep).collect();
        let num = e_parts.get(0).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        if num == 0 { break; }

        let raw_amount = e_parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let mut amount = if raw_amount == 0 { EnemyAmount::Infinite } else { EnemyAmount::Limit(raw_amount) };
        
        let respawn_min = e_parts.get(3).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) * 2;
        let respawn_max = e_parts.get(4).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) * 2;

        if respawn_min == 0 {
            amount = EnemyAmount::Infinite;
        }

        let boss_val = e_parts.get(8).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
        let mag = e_parts.get(9).and_then(|s| if *s == "." { None } else { s.parse().ok() }).unwrap_or(100);
        
        let id = if num >= 2 { num - 2 } else { 0 };
        let start_frame = e_parts.get(2).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0) * 2;

        if id == 21 && start_frame == 27000 {
            continue;
        }
        
        stage.enemies.push(EnemyLine {
            id,
            amount,
            start_frame,
            respawn_min,
            respawn_max,
            base_hp_perc: e_parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
            boss_type: BossType::from(boss_val),
            magnification: mag,
            atk_magnification: e_parts.get(11).and_then(|s| s.parse().ok()).unwrap_or(mag),
            kill_count: e_parts.get(13).and_then(|s| s.parse().ok()).unwrap_or(0),
            is_base: num != 0 && num == stage.anim_base_id, 
        });
    }

    stage
}