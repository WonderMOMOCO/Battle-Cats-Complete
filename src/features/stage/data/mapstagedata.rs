use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DropReward {
    pub chance: u32,
    pub id: u32,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimedScore {
    pub score: u32,
    pub id: u32,
    pub amount: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RewardStructure {
    None,
    Treasure {
        drop_rule: i32,
        drops: Vec<DropReward>,
    },
    Timed(Vec<TimedScore>),
}

impl Default for RewardStructure {
    fn default() -> Self { Self::None }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MapStageEntry {
    pub energy: u32,
    pub xp: u32,
    pub init_track: u32,
    pub bgm_change_percent: u32,
    pub boss_track: u32,
    pub rewards: RewardStructure,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> Vec<MapStageEntry> {
    let paths = resolver::get(dir, &[filename], priority);
    let Some(path) = paths.first() else { return Vec::new(); };
    let Ok(content) = fs::read_to_string(path) else { return Vec::new(); };
    
    parse(&content)
}

fn parse(content: &str) -> Vec<MapStageEntry> {
    let sep = detect_csv_separator(content);
    let lines = content.lines()
        .map(|l| l.split("//").next().unwrap_or("").trim())
        .filter(|l| !l.is_empty())
        .skip(2); 

    let mut entries = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split(sep).collect();
        if parts.len() < 2 { continue; }

        let mut entry = MapStageEntry {
            energy: parts.get(0).and_then(|s| s.parse().ok()).unwrap_or(0),
            xp: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
            init_track: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
            bgm_change_percent: parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0),
            boss_track: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
            rewards: RewardStructure::None,
        };

        let is_time = parts.len() > 15 && parts[8..15].iter().all(|&x| x == "-2");

        if is_time {
            entry.rewards = parse_scores(&parts);
        } else {
            entry.rewards = parse_treasures(&parts);
        }

        entries.push(entry);
    }
    
    entries
}

fn parse_scores(parts: &[&str]) -> RewardStructure {
    let mut scores = Vec::new();
    let score_block_len = (parts.len().saturating_sub(17)) / 3;
    
    for i in 0..score_block_len {
        let score = parts.get(16 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let id = parts.get(17 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let amount = parts.get(18 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        scores.push(TimedScore { score, id, amount });
    }
    
    RewardStructure::Timed(scores)
}

fn parse_treasures(parts: &[&str]) -> RewardStructure {
    if parts.len() < 8 { return RewardStructure::None; }
    
    let mut drops = Vec::new();
    
    let chance = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
    let id = parts.get(6).and_then(|s| s.parse().ok()).unwrap_or(0);
    let amount = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
    drops.push(DropReward { chance, id, amount });

    let is_multi = parts.len() > 9;
    let drop_rule = if is_multi { parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0) } else { 0 };

    if is_multi {
        let drop_len = (parts.len().saturating_sub(7)) / 3;
        for i in 1..drop_len {
            let c = parts.get(6 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let i_id = parts.get(7 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
            let amt = parts.get(8 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
            drops.push(DropReward { chance: c, id: i_id, amount: amt });
        }
    }
    
    RewardStructure::Treasure { drop_rule, drops }
}