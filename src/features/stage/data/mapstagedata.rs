use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::global::resolver;
use crate::global::utils::detect_csv_separator;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MapStageEntry {
    pub energy: u32,
    pub xp: u32,
    pub treasure_type: i32,
    pub drops: Vec<(u32, u32, u32)>,  // chance, id, amount
    pub scores: Vec<(u32, u32, u32)>, // score, id, amount
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> Vec<MapStageEntry> {
    let paths = resolver::get(dir, filename, priority);
    let target = paths.first();
    let Some(path) = target else { return Vec::new(); };
    
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
            energy: parts.get(0).unwrap_or(&"0").parse().unwrap_or(0),
            xp: parts.get(1).unwrap_or(&"0").parse().unwrap_or(0),
            ..Default::default()
        };

        let is_time = parts.len() > 15 && parts[8..15].iter().all(|&x| x == "-2");

        if is_time {
            parse_scores(&mut entry, &parts);
        } else {
            parse_treasures(&mut entry, &parts);
        }

        entries.push(entry);
    }
    entries
}

fn parse_scores(entry: &mut MapStageEntry, parts: &[&str]) {
    let score_block_len = (parts.len() - 17) / 3;
    for i in 0..score_block_len {
        let score = parts.get(16 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let id = parts.get(17 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let amt = parts.get(18 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        entry.scores.push((score, id, amt));
    }
}

fn parse_treasures(entry: &mut MapStageEntry, parts: &[&str]) {
    if parts.len() < 8 { return; }
    
    entry.treasure_type = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);
    let drop_len = (parts.len() - 7) / 3;

    for i in 0..drop_len {
        let chance = parts.get(5 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let id = parts.get(6 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        let amt = parts.get(7 + i * 3).and_then(|s| s.parse().ok()).unwrap_or(0);
        entry.drops.push((chance, id, amt));
    }
}