#![allow(dead_code)]
use std::fs;
use std::path::Path;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TalentCost {
    pub costs: Vec<u16>,
}

pub fn load(cats_directory: &Path) -> HashMap<u8, TalentCost> {
    let mut map = HashMap::new();
    let file_path = cats_directory.join("SkillLevel.csv");
    
    if let Ok(content) = fs::read_to_string(&file_path) {
        for line in content.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.is_empty() { continue; }
            
            if let Ok(id) = parts[0].trim().parse::<u8>() {
                let costs: Vec<u16> = parts.iter()
                    .skip(1)
                    .filter_map(|s| s.trim().parse::<u16>().ok())
                    .collect();
                
                map.insert(id, TalentCost { costs });
            }
        }
    } 
    map
}