use std::collections::HashMap;
use std::path::Path;
use std::fs;
use crate::features::stage::{paths, data};

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Stage {
    pub id: String,
    pub name: String,
    pub category: String,
    pub category_name: String,
    pub map_id: u32,
    pub stage_id: u32,
    pub width: u32,
    pub base_hp: u32,
    pub background_id: u32,
    pub energy: u32,
    pub xp: u32,
    pub is_no_continues: bool,
    pub enemies: Vec<data::stage::EnemyLine>,
    pub rewards: data::mapstagedata::MapStageEntry,
}

#[derive(Default, Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Map {
    pub id: String,
    pub name: String,
    pub category: String,
    pub category_name: String,
    pub map_id: u32,
    pub stages: Vec<String>,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct StageRegistry {
    pub maps: HashMap<String, Map>,
    pub stages: HashMap<String, Stage>,
}

impl StageRegistry {
    pub fn clear_cache(&mut self) {
        self.maps.clear();
        self.stages.clear();
    }

    pub fn load_all(&mut self, priority: &[String]) {
        let root = Path::new(paths::DIR_STAGES);
        let map_name_dir = root.join("Map_Name");
        let global_map_names = data::map_name::load(&map_name_dir, "Map_Name.csv", priority);

        let Ok(categories) = fs::read_dir(root) else { return; };

        for cat_entry in categories.flatten() {
            let path = cat_entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            
            if matches!(
                name.as_ref(), 
                "backgrounds" | "castles" | "fixedlineup" | "MapStageLimitMessage" | 
                "Map_Name" | "Map_option.csv" | "MapConditions.json" | "Stage_option.csv"
            ) {
                continue;
            }
            
            if path.is_dir() { 
                self.scan_category(&path, &global_map_names, priority); 
            }
        }
    }

    fn scan_category(&mut self, cat_path: &Path, global_map_names: &HashMap<u32, String>, priority: &[String]) {
        let prefix = cat_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let category_name = data::map_name::get_category_name(&prefix);

        let mut stage_names = data::stagename::load(cat_path, &format!("StageName_{}.csv", prefix), priority);
        if stage_names.is_empty() {
            stage_names = data::stagename::load(cat_path, &format!("StageName_R{}.csv", prefix), priority);
        }

        let Ok(maps) = fs::read_dir(cat_path) else { return; };
        for map_entry in maps.flatten() {
            let map_path = map_entry.path();
            if !map_path.is_dir() { continue; }

            let folder_name = map_path.file_name().unwrap_or_default().to_string_lossy();
            let Ok(map_id) = folder_name.parse::<u32>() else { continue; };

            let global_id = data::map_name::get_global_map_id(&prefix, map_id);
            
            let name = global_id
                .and_then(|id| global_map_names.get(&id))
                .filter(|n| !n.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{:03}", map_id));

            self.load_map(&prefix, map_id, &map_path, &name, &category_name, &stage_names, priority);
        }
    }

    fn load_map(&mut self, prefix: &str, map_id: u32, map_path: &Path, map_name: &str, category_name: &str, stage_names: &HashMap<u32, Vec<String>>, priority: &[String]) {
        let mut map_struct = Map {
            id: format!("{}_{}", prefix, map_id),
            name: map_name.to_string(),
            category: prefix.to_string(),
            category_name: category_name.to_string(),
            map_id,
            stages: Vec::new(),
        };

        let mut map_entries = Vec::new();
        if let Ok(files) = fs::read_dir(map_path) {
            for f in files.flatten() {
                let fname = f.file_name().to_string_lossy().to_string();
                if fname.starts_with("MapStageData") && fname.ends_with(".csv") {
                    map_entries = data::mapstagedata::load(map_path, &fname, priority);
                    if !map_entries.is_empty() { break; }
                }
            }
        }
        
        if map_entries.is_empty() {
            map_entries = data::mapstagedata::load(map_path, "stage.csv", priority);
        }

        let Ok(stages) = fs::read_dir(map_path) else { return; };
        for entry in stages.flatten() {
            let stage_path = entry.path();
            if !stage_path.is_dir() { continue; }

            let folder_name = stage_path.file_name().unwrap_or_default().to_string_lossy();
            let Ok(sid) = folder_name.parse::<u32>() else { continue; };

            let mut raw_layout = None;
            if let Ok(files) = fs::read_dir(&stage_path) {
                for f in files.flatten() {
                    let fname = f.file_name().to_string_lossy().to_string();
                    if fname.ends_with(".csv") {
                        raw_layout = data::stage::load(&stage_path, &fname, priority);
                        if raw_layout.is_some() { break; }
                    }
                }
            }

            let Some(raw) = raw_layout else { continue; };

            let s_name = stage_names.get(&map_id)
                .and_then(|names| names.get(sid as usize))
                .filter(|s| !s.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{:02}", sid));

            let key = format!("{}_{}_{}", prefix, map_id, sid);
            let mut stage = Stage {
                id: key.clone(),
                name: s_name,
                category: prefix.to_string(),
                category_name: category_name.to_string(),
                map_id,
                stage_id: sid,
                width: raw.width,
                base_hp: raw.base_hp,
                background_id: raw.background_id,
                is_no_continues: raw.is_no_continues,
                enemies: raw.enemies,
                ..Default::default()
            };

            if let Some(m) = map_entries.get(sid as usize) {
                stage.energy = m.energy;
                stage.xp = m.xp;
                stage.rewards = m.clone();
            }

            self.stages.insert(key.clone(), stage);
            map_struct.stages.push(key);
        }

        if !map_struct.stages.is_empty() {
            map_struct.stages.sort();
            self.maps.insert(map_struct.id.clone(), map_struct);
        }
    }
}