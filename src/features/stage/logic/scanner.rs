use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use crate::features::settings::logic::state::ScannerConfig;
use crate::features::stage::{paths, data};
use crate::features::stage::registry::{StageRegistry, Map, Stage};

pub struct ScanContext<'a> {
    pub priority: &'a [String],
    pub global_map_names: HashMap<u32, String>,
    pub map_options: HashMap<u32, data::map_option::MapOption>,
    pub stage_options: HashMap<u32, Vec<data::stage_option::StageOption>>,
    pub charagroups: HashMap<u32, data::charagroup::CharaGroup>,
    pub drop_items: HashMap<u32, data::dropitem::DropItem>,
    pub score_bonuses: HashMap<u32, data::scorebonusmap::ScoreBonus>,
    pub special_rules: HashMap<u32, data::specialrulesmap::SpecialRule>,
    pub ex_options: HashMap<u32, u32>,
}

pub fn start_scan(config: &ScannerConfig) -> Receiver<StageRegistry> {
    let (tx, rx) = mpsc::channel();
    let priority = config.language_priority.clone();

    thread::spawn(move || {
        let registry = scan_all(&priority);
        let _ = tx.send(registry);
    });

    rx
}

fn scan_all(priority: &[String]) -> StageRegistry {
    let mut registry = StageRegistry::default();
    let root = Path::new(paths::DIR_STAGES);
    
    let ctx = ScanContext {
        priority,
        global_map_names: data::map_name::load(&root.join("Map_Name"), "Map_Name.csv", priority),
        map_options: data::map_option::load(root, "Map_option.csv", priority),
        stage_options: data::stage_option::load(root, "Stage_option.csv", priority),
        charagroups: data::charagroup::load(root, "Charagroup.csv", priority),
        drop_items: data::dropitem::load(root, "DropItem.csv", priority),
        score_bonuses: data::scorebonusmap::load(&root.join("R"), "ScoreBonusMap.json", priority),
        special_rules: data::specialrulesmap::load(&root.join("SR"), "SpecialRulesMap.json", priority),
        ex_options: data::ex_option::load(root, "EX_option.csv", priority),
    };

    let Ok(categories) = fs::read_dir(root) else { return registry; };

    for cat_entry in categories.flatten() {
        let path = cat_entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        
        if matches!(
            name.as_ref(), 
            "backgrounds" | "castles" | "fixedlineup" | "MapStageLimitMessage" | 
            "Map_Name" | "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" |
            "DropItem.csv" | "Charagroup.csv" | "EX_option.csv" | "R" | "SR" | "V" | "L" | "G" | "EX"
        ) {
            continue;
        }
        
        if path.is_dir() { 
            scan_category(&mut registry, &path, &ctx); 
        }
    }

    registry
}

fn scan_category(registry: &mut StageRegistry, cat_path: &Path, ctx: &ScanContext) {
    let prefix = cat_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let category_name = data::map_name::get_category_name(&prefix);

    let mut stage_names = data::stagename::load(cat_path, &format!("StageName_{}.csv", prefix), ctx.priority);
    if stage_names.is_empty() {
        stage_names = data::stagename::load(cat_path, &format!("StageName_R{}.csv", prefix), ctx.priority);
    }

    let Ok(maps) = fs::read_dir(cat_path) else { return; };
    for map_entry in maps.flatten() {
        let map_path = map_entry.path();
        if !map_path.is_dir() { continue; }

        let folder_name = map_path.file_name().unwrap_or_default().to_string_lossy();
        let Ok(map_id) = folder_name.parse::<u32>() else { continue; };

        let global_id = data::map_name::get_global_map_id(&prefix, map_id);
        
        let name = global_id
            .and_then(|id| ctx.global_map_names.get(&id))
            .filter(|n| !n.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:03}", map_id));

        load_map(registry, &prefix, map_id, &map_path, &name, &category_name, &stage_names, ctx, global_id);
    }
}

#[allow(clippy::too_many_arguments)]
fn load_map(
    registry: &mut StageRegistry, 
    prefix: &str, 
    map_id: u32, 
    map_path: &Path, 
    map_name: &str, 
    category_name: &str, 
    stage_names: &HashMap<u32, Vec<String>>, 
    ctx: &ScanContext, 
    global_id: Option<u32>
) {
    let g_id = global_id.unwrap_or(0);
    let m_opt = ctx.map_options.get(&g_id).cloned().unwrap_or_default();

    let mut map_struct = Map {
        id: format!("{}_{}", prefix, map_id),
        name: map_name.to_string(),
        category: prefix.to_string(),
        category_name: category_name.to_string(),
        map_id,
        stages: Vec::new(),
        max_crowns: m_opt.max_crowns,
        crown_2_mag: m_opt.crown_2_mag,
        crown_3_mag: m_opt.crown_3_mag,
        crown_4_mag: m_opt.crown_4_mag,
        reset_type: m_opt.reset_type,
        max_clears: m_opt.max_clears,
        cooldown_minutes: m_opt.cooldown_minutes,
        hidden_upon_clear: m_opt.hidden_upon_clear,
        ex_invasion: ctx.ex_options.get(&g_id).cloned(),
        score_bonuses: ctx.score_bonuses.get(&g_id).cloned(),
        special_rules: ctx.special_rules.get(&g_id).cloned(),
        drop_items: ctx.drop_items.get(&g_id).cloned(),
    };

    let stage_options_for_map = ctx.stage_options.get(&g_id).cloned().unwrap_or_default();

    let mut map_entries = Vec::new();
    if let Ok(files) = fs::read_dir(map_path) {
        for f in files.flatten() {
            let fname = f.file_name().to_string_lossy().to_string();
            if fname.starts_with("MapStageData") && fname.ends_with(".csv") {
                map_entries = data::mapstagedata::load(map_path, &fname, ctx.priority);
                if !map_entries.is_empty() { break; }
            }
        }
    }
    
    if map_entries.is_empty() {
        map_entries = data::mapstagedata::load(map_path, "stage.csv", ctx.priority);
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
                    raw_layout = data::stage::load(&stage_path, &fname, ctx.priority);
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

        let s_opt = stage_options_for_map.iter()
            .find(|opt| opt.target_stage == -1 || opt.target_stage == sid as i32)
            .cloned()
            .unwrap_or_default();

        let charagroup = ctx.charagroups.get(&s_opt.charagroup_id).cloned();

        let key = format!("{}_{}_{}", prefix, map_id, sid);
        let mut stage = Stage {
            id: key.clone(),
            name: s_name,
            category: prefix.to_string(),
            category_name: category_name.to_string(),
            map_id,
            stage_id: sid,
            base_id: raw.base_id,
            anim_base_id: raw.anim_base_id,
            width: raw.width,
            base_hp: raw.base_hp,
            background_id: raw.background_id,
            max_enemies: raw.max_enemies,
            is_no_continues: raw.is_no_continues,
            is_base_indestructible: raw.is_base_indestructible,
            enemies: raw.enemies,
            target_crowns: s_opt.target_crowns,
            rarity_mask: s_opt.rarity_mask,
            deploy_limit: s_opt.deploy_limit,
            allowed_rows: s_opt.allowed_rows,
            min_cost: s_opt.min_cost,
            max_cost: s_opt.max_cost,
            charagroup,
            ..Default::default()
        };

        if let Some(m) = map_entries.get(sid as usize) {
            stage.energy = m.energy;
            stage.xp = m.xp;
            stage.init_track = m.init_track;
            stage.bgm_change_percent = m.bgm_change_percent;
            stage.boss_track = m.boss_track;
            stage.rewards = m.rewards.clone();
        }

        registry.stages.insert(key.clone(), stage);
        map_struct.stages.push(key);
    }

    if !map_struct.stages.is_empty() {
        map_struct.stages.sort();
        registry.maps.insert(map_struct.id.clone(), map_struct);
    }
}