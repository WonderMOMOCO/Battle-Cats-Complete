use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use crate::features::settings::logic::state::ScannerConfig;
use crate::features::stage::{paths, data};
use crate::features::stage::registry::{StageRegistry, Map, Stage};

pub struct ScanContext<'a> {
    pub lang_priority: &'a [String],
    pub map_names: HashMap<u32, String>,
    pub map_options: HashMap<u32, data::map_option::MapOption>,
    pub stage_options: HashMap<u32, Vec<data::stage_option::StageOption>>,
    pub charagroups: HashMap<u32, data::charagroup::CharaGroup>,
    pub drop_items: HashMap<u32, data::dropitem::DropItem>,
    pub score_bonuses: HashMap<u32, data::scorebonusmap::ScoreBonus>,
    pub special_rules: HashMap<u32, data::specialrulesmap::SpecialRule>,
    pub ex_options: HashMap<u32, u32>,
    pub difficulties: HashMap<u32, Vec<u16>>, 
}

pub fn start_scan(config: &ScannerConfig) -> Receiver<StageRegistry> {
    let (tx_channel, rx_channel) = mpsc::channel();
    let lang_priority_clone = config.language_priority.clone();

    thread::spawn(move || {
        let registry = scan_all(&lang_priority_clone);
        let _ = tx_channel.send(registry);
    });

    rx_channel
}

fn scan_all(lang_priority: &[String]) -> StageRegistry {
    let mut registry = StageRegistry::default();
    let root_path = Path::new(paths::DIR_STAGES);
    
    let ctx = ScanContext {
        lang_priority,
        map_names: data::map_name::load(&root_path.join("Map_Name"), "Map_Name.csv", lang_priority),
        map_options: data::map_option::load(root_path, "Map_option.csv", lang_priority),
        stage_options: data::stage_option::load(root_path, "Stage_option.csv", lang_priority),
        charagroups: data::charagroup::load(root_path, "Charagroup.csv", lang_priority),
        drop_items: data::dropitem::load(root_path, "DropItem.csv", lang_priority),
        score_bonuses: data::scorebonusmap::load(&root_path.join("R"), "ScoreBonusMap.json", lang_priority),
        special_rules: data::specialrulesmap::load(&root_path.join("SR"), "SpecialRulesMap.json", lang_priority),
        ex_options: data::ex_option::load(root_path, "EX_option.csv", lang_priority),
        difficulties: data::difficulty_level::load(root_path, "difficulty_level.tsv", lang_priority),
    };

    let Ok(categories_dir) = fs::read_dir(root_path) else { 
        return registry; 
    };

    for category_entry in categories_dir.flatten() {
        let cat_path = category_entry.path();
        let cat_name = cat_path.file_name().unwrap_or_default().to_string_lossy();
        
        let is_ignored_dir = matches!(
            cat_name.as_ref(), 
            "backgrounds" | "castles" | "fixedlineup" | "MapStageLimitMessage" | 
            "Map_Name" | "Map_option.csv" | "MapConditions.json" | "Stage_option.csv" |
            "DropItem.csv" | "Charagroup.csv" | "EX_option.csv" | "R" | "SR" | "V" | "L" | "G" | "EX" | "difficulty_level.tsv"
        );
        
        if is_ignored_dir || !cat_path.is_dir() {
            continue;
        }

        scan_category(&mut registry, &cat_path, &ctx); 
    }

    registry
}

fn scan_category(registry: &mut StageRegistry, cat_path: &Path, ctx: &ScanContext) {
    let cat_prefix = cat_path.file_name().unwrap_or_default().to_string_lossy().to_string();
    let cat_display_name = data::map_name::get_category_name(&cat_prefix);

    let mut stage_names = data::stagename::load(cat_path, &format!("StageName_{}.csv", cat_prefix), ctx.lang_priority);
    if stage_names.is_empty() {
        stage_names = data::stagename::load(cat_path, &format!("StageName_R{}.csv", cat_prefix), ctx.lang_priority);
    }

    let Ok(maps_dir) = fs::read_dir(cat_path) else { 
        return; 
    };
    
    for map_entry in maps_dir.flatten() {
        let map_path = map_entry.path();
        if !map_path.is_dir() { 
            continue; 
        }

        let map_folder_name = map_path.file_name().unwrap_or_default().to_string_lossy();
        let Ok(map_id) = map_folder_name.parse::<u32>() else { 
            continue; 
        };

        let global_map_id = data::map_name::get_global_map_id(&cat_prefix, map_id);
        
        let map_display_name = global_map_id
            .and_then(|id| ctx.map_names.get(&id))
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:03}", map_id));

        load_map(
            registry, 
            &cat_prefix, 
            map_id, 
            &map_path, 
            &map_display_name, 
            &cat_display_name, 
            &stage_names, 
            ctx, 
            global_map_id
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn load_map(
    registry: &mut StageRegistry, 
    cat_prefix: &str, 
    map_id: u32, 
    map_path: &Path, 
    map_display_name: &str, 
    cat_display_name: &str, 
    stage_names: &HashMap<u32, Vec<String>>, 
    ctx: &ScanContext, 
    global_map_id: Option<u32>
) {
    let global_id_val = global_map_id.unwrap_or(0);
    let map_opt = ctx.map_options.get(&global_id_val).cloned().unwrap_or_default();

    let mut map_struct = Map {
        id: format!("{}_{}", cat_prefix, map_id),
        name: map_display_name.to_string(),
        category: cat_prefix.to_string(),
        category_name: cat_display_name.to_string(),
        map_id,
        stages: Vec::new(),
        max_crowns: map_opt.max_crowns,
        crown_2_mag: map_opt.crown_2_mag,
        crown_3_mag: map_opt.crown_3_mag,
        crown_4_mag: map_opt.crown_4_mag,
        reset_type: map_opt.reset_type,
        max_clears: map_opt.max_clears,
        cooldown_minutes: map_opt.cooldown_minutes,
        hidden_upon_clear: map_opt.hidden_upon_clear,
        ex_invasion: ctx.ex_options.get(&global_id_val).cloned(),
        score_bonuses: ctx.score_bonuses.get(&global_id_val).cloned(),
        special_rules: ctx.special_rules.get(&global_id_val).cloned(),
        drop_items: ctx.drop_items.get(&global_id_val).cloned(),
    };

    let stage_opts = ctx.stage_options.get(&global_id_val).cloned().unwrap_or_default();

    let mut stage_data_entries = Vec::new();
    if let Ok(files_dir) = fs::read_dir(map_path) {
        for file_entry in files_dir.flatten() {
            let filename = file_entry.file_name().to_string_lossy().to_string();
            let is_valid_stage_data = filename.starts_with("MapStageData") && filename.ends_with(".csv");
            
            if !is_valid_stage_data {
                continue;
            }
            
            stage_data_entries = data::mapstagedata::load(map_path, &filename, ctx.lang_priority);
            
            if !stage_data_entries.is_empty() { 
                break; 
            }
        }
    }
    
    if stage_data_entries.is_empty() {
        stage_data_entries = data::mapstagedata::load(map_path, "stage.csv", ctx.lang_priority);
    }

    let Ok(stages_dir) = fs::read_dir(map_path) else { 
        return; 
    };
    
    for stage_entry in stages_dir.flatten() {
        let stage_path = stage_entry.path();
        if !stage_path.is_dir() { 
            continue; 
        }

        let stage_folder = stage_path.file_name().unwrap_or_default().to_string_lossy();
        let Ok(stage_id) = stage_folder.parse::<u32>() else { 
            continue; 
        };

        let mut stage_raw = None;
        if let Ok(files_dir) = fs::read_dir(&stage_path) {
            for file_entry in files_dir.flatten() {
                let filename = file_entry.file_name().to_string_lossy().to_string();
                
                if !filename.ends_with(".csv") {
                    continue;
                }
                
                stage_raw = data::stage::load(&stage_path, &filename, ctx.lang_priority);
                
                if stage_raw.is_some() { 
                    break; 
                }
            }
        }

        let Some(raw_layout) = stage_raw else { 
            continue; 
        };

        let stage_display_name = stage_names.get(&map_id)
            .and_then(|names_list| names_list.get(stage_id as usize))
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:02}", stage_id));

        let current_stage_opt = stage_opts.iter()
            .find(|opt| opt.target_stage == -1 || opt.target_stage == stage_id as i32)
            .cloned()
            .unwrap_or_default();

        let stage_diff = ctx.difficulties.get(&global_id_val).and_then(|diff_list| diff_list.get(stage_id as usize)).copied().unwrap_or(0);
        let current_charagroup = ctx.charagroups.get(&current_stage_opt.charagroup_id).cloned();

        let stage_key = format!("{}_{}_{}", cat_prefix, map_id, stage_id);
        let mut stage_struct = Stage {
            id: stage_key.clone(),
            name: stage_display_name,
            category: cat_prefix.to_string(),
            category_name: cat_display_name.to_string(),
            map_id,
            stage_id,
            base_id: raw_layout.base_id,
            anim_base_id: raw_layout.anim_base_id,
            width: raw_layout.width,
            base_hp: raw_layout.base_hp,
            min_spawn: raw_layout.min_spawn,
            background_id: raw_layout.background_id,
            max_enemies: raw_layout.max_enemies,
            is_no_continues: raw_layout.is_no_continues,
            is_base_indestructible: raw_layout.is_base_indestructible,
            enemies: raw_layout.enemies,
            difficulty: stage_diff,
            max_crowns: map_opt.max_crowns,
            target_crowns: current_stage_opt.target_crowns,
            rarity_mask: current_stage_opt.rarity_mask,
            deploy_limit: current_stage_opt.deploy_limit,
            allowed_rows: current_stage_opt.allowed_rows,
            min_cost: current_stage_opt.min_cost,
            max_cost: current_stage_opt.max_cost,
            charagroup: current_charagroup,
            ..Default::default()
        };

        if let Some(entry) = stage_data_entries.get(stage_id as usize) {
            stage_struct.energy = entry.energy;
            stage_struct.xp = entry.xp;
            stage_struct.init_track = entry.init_track;
            stage_struct.bgm_change_percent = entry.bgm_change_percent;
            stage_struct.boss_track = entry.boss_track;
            stage_struct.rewards = entry.rewards.clone();
        }

        registry.stages.insert(stage_key.clone(), stage_struct);
        map_struct.stages.push(stage_key);
    }

    if !map_struct.stages.is_empty() {
        map_struct.stages.sort();
        registry.maps.insert(map_struct.id.clone(), map_struct);
    }
}