use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BonusType {
    Weaken(Vec<u32>),
    Freeze(Vec<u32>),
    Slow(Vec<u32>),
    Knockback(Vec<u32>),
    Strong(Vec<u32>),
    MassiveDamage(Vec<u32>),
    Unknown(u8, Vec<u32>),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ScoreBonus {
    pub bonuses: Vec<BonusType>,
    pub name_label: String,
}

#[derive(Deserialize)]
struct RawBonusType {
    #[serde(rename = "Parameters")]
    parameters: Vec<u32>,
}

#[derive(Deserialize)]
struct RawBonusData {
    #[serde(rename = "BonusType", default)]
    bonus_type: HashMap<String, RawBonusType>,
    #[serde(rename = "BonusNameLabel")]
    bonus_name_label: Option<String>,
}

#[derive(Deserialize)]
struct BonusesMap {
    #[serde(rename = "MapID", default)]
    map_id: HashMap<String, RawBonusData>,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, ScoreBonus> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);

    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let Ok(json_data) = serde_json::from_str::<BonusesMap>(&content) else { return map; };

    for (map_id_str, raw_data) in json_data.map_id {
        let Ok(map_id) = map_id_str.parse::<u32>() else { continue; };

        let mut bonuses = Vec::new();
        for (key_str, raw_type) in raw_data.bonus_type {
            let Ok(key) = key_str.parse::<u8>() else { continue; };
            let params = raw_type.parameters;
            
            let bonus_enum = match key {
                0 => BonusType::Weaken(params),
                1 => BonusType::Freeze(params),
                2 => BonusType::Slow(params),
                3 => BonusType::Knockback(params),
                13 => BonusType::Strong(params),
                14 => BonusType::MassiveDamage(params),
                _ => BonusType::Unknown(key, params),
            };
            
            bonuses.push(bonus_enum);
        }

        map.insert(map_id, ScoreBonus {
            bonuses,
            name_label: raw_data.bonus_name_label.unwrap_or_default(),
        });
    }

    map
}