use std::fs;
use std::path::Path;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::global::resolver;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RuleType {
    TrustFund(Vec<u32>),
    CooldownEquality(Vec<u32>),
    RarityLimit(Vec<u32>),
    CheapLabor(Vec<u32>),
    RestrictPrice(Vec<u32>),
    RestrictCd(Vec<u32>),
    DeployLimit(Vec<u32>),
    AwesomeCatSpawn(Vec<u32>),
    AwesomeCatCannon(Vec<u32>),
    AwesomeUnitSpeed(Vec<u32>),
    Unknown(u8, Vec<u32>),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct SpecialRule {
    pub contents_type: u8,
    pub rules: Vec<RuleType>,
    pub name_label: String,
}

#[derive(Deserialize)]
struct RawRuleType {
    #[serde(rename = "Parameters")]
    parameters: Vec<u32>,
}

#[derive(Deserialize)]
struct RawRuleData {
    #[serde(rename = "ContentsType")]
    contents_type: u8,
    #[serde(rename = "RuleType", default)]
    rule_type: HashMap<String, RawRuleType>,
    #[serde(rename = "RuleNameLabel")]
    rule_name_label: Option<String>,
}

#[derive(Deserialize)]
struct RulesMap {
    #[serde(rename = "MapID", default)]
    map_id: HashMap<String, RawRuleData>,
}

pub fn load(dir: &Path, filename: &str, priority: &[String]) -> HashMap<u32, SpecialRule> {
    let mut map = HashMap::new();
    let paths = resolver::get(dir, &[filename], priority);

    let Some(path) = paths.first() else { return map; };
    let Ok(content) = fs::read_to_string(path) else { return map; };
    let Ok(json_data) = serde_json::from_str::<RulesMap>(&content) else { return map; };

    for (map_id_str, raw_data) in json_data.map_id {
        let Ok(map_id) = map_id_str.parse::<u32>() else { continue; };

        let mut rules = Vec::new();
        for (key_str, raw_type) in raw_data.rule_type {
            let Ok(key) = key_str.parse::<u8>() else { continue; };
            let params = raw_type.parameters;
            
            let rule_enum = match key {
                0 => RuleType::TrustFund(params),
                1 => RuleType::CooldownEquality(params),
                3 => RuleType::RarityLimit(params),
                4 => RuleType::CheapLabor(params),
                5 => RuleType::RestrictPrice(params),
                6 => RuleType::RestrictCd(params),
                7 => RuleType::DeployLimit(params),
                8 => RuleType::AwesomeCatSpawn(params),
                9 => RuleType::AwesomeCatCannon(params),
                10 => RuleType::AwesomeUnitSpeed(params),
                _ => RuleType::Unknown(key, params),
            };
            
            rules.push(rule_enum);
        }

        map.insert(map_id, SpecialRule {
            contents_type: raw_data.contents_type,
            rules,
            name_label: raw_data.rule_name_label.unwrap_or_default(),
        });
    }

    map
}