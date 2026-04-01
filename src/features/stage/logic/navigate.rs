use crate::features::stage::registry::{StageRegistry, Map, Stage};

pub fn get_categories(registry: &StageRegistry) -> Vec<(String, String)> {
    let mut categories: Vec<(String, String)> = registry.maps.values()
        .map(|m| (m.category.clone(), m.category_name.clone()))
        .collect();
    
    categories.sort_by(|a, b| a.0.cmp(&b.0));
    categories.dedup();
    categories
}

pub fn get_maps(registry: &StageRegistry, category: &str) -> Vec<Map> {
    let mut maps: Vec<Map> = registry.maps.values()
        .filter(|m| m.category == category)
        .cloned()
        .collect();
    
    maps.sort_by_key(|m| m.map_id);
    maps
}

pub fn get_stages(registry: &StageRegistry, map_id: &str) -> Vec<Stage> {
    let Some(map) = registry.maps.get(map_id) else { return Vec::new(); };
    
    let mut stages: Vec<Stage> = map.stages.iter()
        .filter_map(|s_key| registry.stages.get(s_key))
        .cloned()
        .collect();
    
    stages.sort_by_key(|s| s.stage_id);
    stages
}