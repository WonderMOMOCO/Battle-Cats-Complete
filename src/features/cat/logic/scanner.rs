use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::{Arc, mpsc::{self, Receiver}};
use std::sync::Mutex; 
use rayon::prelude::*;
use std::io::Read;
use serde::{Serialize, Deserialize};

use crate::features::cat::data::unitid::CatRaw;
use crate::features::cat::data::unitbuy::{self, UnitBuyRow};
use crate::features::cat::data::unitlevel::{self, CatLevelCurve};
use crate::features::cat::data::skillacquisition::{self, TalentRaw}; 
use crate::features::cat::data::unitevolve; 
use crate::features::cat::data::unitexplanation; 
use crate::features::cat::data::skilllevel;
use crate::features::cat::data::skilldescriptions;
use crate::global::utils; 
use crate::features::cat::paths;
use crate::features::settings::logic::state::ScannerConfig;
use crate::global::formats::maanim::Animation;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CatEntry {
    pub id: u32,
    pub image_path: Option<PathBuf>,          
    pub deploy_icon_paths: [Option<PathBuf>; 4],
    pub names: Vec<String>,
    pub description: Vec<Vec<String>>,
    pub forms: [bool; 4],
    pub stats: Vec<Option<CatRaw>>,
    pub curve: Option<CatLevelCurve>,
    pub atk_anim_frames: [i32; 4], 
    pub egg_ids: (i32, i32),
    pub talent_data: Option<TalentRaw>,
    pub unit_buy: UnitBuyRow,
    pub evolve_text: [Vec<String>; 4], 
    #[serde(skip)] pub talent_costs: Arc<std::collections::HashMap<u8, skilllevel::TalentCost>>,
    #[serde(skip)] pub skill_descriptions: Arc<Vec<String>>,
}

impl CatEntry {
    pub fn id_str(&self, form_index: usize) -> String { format!("{:03}-{}", self.id, form_index + 1) }
    pub fn display_name(&self, form_index: usize) -> String {
        let raw_name = self.names.get(form_index).cloned().unwrap_or_default();
        if raw_name.is_empty() { self.id_str(form_index) } else { raw_name }
    }
    pub fn base_id_str(&self) -> String { format!("{:03}", self.id) }
}

fn is_valid_png(path: &Path) -> bool {
    let mut file = match fs::File::open(path) { Ok(f) => f, Err(_) => return false, };
    let mut buffer = [0u8; 25];
    if file.read_exact(&mut buffer).is_err() { return false; }
    const PNG_SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    if buffer[0..8] != PNG_SIG { return false; }
    buffer[24] >= 8
}

pub fn start_scan(config: ScannerConfig) -> Receiver<CatEntry> {
    let (cat_sender, cat_receiver) = mpsc::channel();

    thread::spawn(move || {
        let cats_directory = Path::new(paths::DIR_CATS);
        let priority = &config.language_priority;

        let unitbuy_resolved = crate::global::resolver::get(cats_directory, &[paths::UNIT_BUY], priority).into_iter().next();
        let unitlevel_resolved = crate::global::resolver::get(cats_directory, &[paths::UNIT_LEVEL], priority).into_iter().next();
        
        if unitbuy_resolved.is_none() || unitlevel_resolved.is_none() {
            return;
        }
        
        let level_curves_arc = Arc::new(unitlevel::load_level_curves(cats_directory, priority));
        let unit_buy_map_arc = Arc::new(unitbuy::load_unitbuy(cats_directory, priority));
        let talent_map_arc = Arc::new(skillacquisition::load(cats_directory, priority));
        let evolve_text_map_arc = Arc::new(unitevolve::load(cats_directory, priority));
        let talent_costs_arc = Arc::new(skilllevel::load(cats_directory, priority));
        let skill_descriptions_arc = Arc::new(skilldescriptions::load(cats_directory, priority));
        
        let folder_entries: Vec<PathBuf> = match fs::read_dir(cats_directory) {
            Ok(read_dir_iter) => read_dir_iter
                .filter_map(|entry_result| entry_result.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect(),
            Err(_) => Vec::new(),
        };

        let stream_sender = Arc::new(Mutex::new(cat_sender));

        let mut parsed_cats: Vec<CatEntry> = folder_entries.par_iter().filter_map(|folder_path| {
            let cat = process_cat_entry(
                folder_path, 
                &level_curves_arc, 
                &unit_buy_map_arc, 
                &talent_map_arc, 
                &evolve_text_map_arc,
                &talent_costs_arc,
                &skill_descriptions_arc,
                &config
            );
            
            if let Some(c) = &cat {
                if let Ok(sender) = stream_sender.lock() {
                    let _ = sender.send(c.clone());
                }
            }
            
            cat
        }).collect();

        parsed_cats.sort_by_key(|cat| cat.id);

        if !crate::global::resolver::is_mod_active() {
            let current_hash = crate::global::io::cache::get_game_hash(None);
            crate::global::io::cache::save("cats_cache.bin", current_hash, &parsed_cats);
        }
    });
    
    cat_receiver
}

pub fn scan_single(id: u32, config: &ScannerConfig) -> Option<CatEntry> {
    let cats_directory = Path::new(paths::DIR_CATS);
    let priority = &config.language_priority;

    let unitbuy_resolved = crate::global::resolver::get(cats_directory, &[paths::UNIT_BUY], priority).into_iter().next();
    let unitlevel_resolved = crate::global::resolver::get(cats_directory, &[paths::UNIT_LEVEL], priority).into_iter().next();
    if unitbuy_resolved.is_none() || unitlevel_resolved.is_none() { return None; }

    let curves = unitlevel::load_level_curves(cats_directory, priority);
    let buys = unitbuy::load_unitbuy(cats_directory, priority);
    let talents = skillacquisition::load(cats_directory, priority);
    let evolve = unitevolve::load(cats_directory, priority);
    let costs = Arc::new(skilllevel::load(cats_directory, priority));
    let descs = Arc::new(skilldescriptions::load(cats_directory, priority));

    let folder_path = cats_directory.join(format!("{:03}", id));
    
    if !folder_path.exists() { return None; }

    process_cat_entry(&folder_path, &curves, &buys, &talents, &evolve, &costs, &descs, config)
}

pub fn process_cat_entry(
    original_folder_path: &Path, 
    level_curves: &[CatLevelCurve], 
    unit_buys: &std::collections::HashMap<u32, UnitBuyRow>,
    talents_map: &std::collections::HashMap<u16, TalentRaw>, 
    evolve_text_map: &std::collections::HashMap<u32, [Vec<String>; 4]>, 
    talent_costs: &Arc<std::collections::HashMap<u8, skilllevel::TalentCost>>,
    skill_descriptions: &Arc<Vec<String>>,
    config: &ScannerConfig
) -> Option<CatEntry> {
    let folder_stem = original_folder_path.file_name()?.to_str()?;
    let cat_id = folder_stem.parse::<u32>().ok()?;
    let cats_root_dir = Path::new(paths::DIR_CATS);
    let priority = &config.language_priority;

    let stats_path = paths::stats(cats_root_dir, cat_id);
    
    let Some(stats_parent) = stats_path.parent() else { return None; };
    let Some(stats_name) = stats_path.file_name().and_then(|n| n.to_str()) else { return None; };
    
    let resolved_stats = crate::global::resolver::get(stats_parent, &[stats_name], priority).into_iter().next();

    if !config.show_invalid_cats && resolved_stats.is_none() {
        return None;
    }

    let ub_row = unit_buys.get(&cat_id)?;
    let egg_ids = (ub_row.egg_id_normal, ub_row.egg_id_evolved);

    let mut forms_existence = [false; 4];
    let mut deploy_icon_paths: [Option<PathBuf>; 4] = Default::default();
    let mut final_image_path_opt = None;

    for form_idx in 0..4 {
        let dir = paths::folder(cats_root_dir, cat_id, form_idx, egg_ids);
        
        let banner_stem = paths::image_stem(paths::AssetType::Banner, cat_id, form_idx, egg_ids);
        let banner_name = format!("{}.png", banner_stem);
        let mut resolved_banner = crate::global::resolver::get(&dir, &[banner_name.as_str()], priority).into_iter().next();
        
        if resolved_banner.is_none() && form_idx == 1 && egg_ids.1 != -1 {
            let fallback_stem = format!("udi{:03}_m00", egg_ids.1);
            let fallback_name = format!("{}.png", fallback_stem);
            resolved_banner = crate::global::resolver::get(&dir, &[fallback_name.as_str()], priority).into_iter().next();
        }

        let icon_stem = paths::image_stem(paths::AssetType::Icon, cat_id, form_idx, egg_ids);
        let icon_name = format!("{}.png", icon_stem);
        let mut resolved_icon = crate::global::resolver::get(&dir, &[icon_name.as_str()], priority).into_iter().next();

        if resolved_icon.is_none() && form_idx == 1 && egg_ids.1 != -1 {
            let fallback_stem = format!("uni{:03}_m00", egg_ids.1);
            let fallback_name = format!("{}.png", fallback_stem);
            resolved_icon = crate::global::resolver::get(&dir, &[fallback_name.as_str()], priority).into_iter().next();
        }

        let mut form_valid = false;
        
        if let Some(b_path) = &resolved_banner {
            if config.show_invalid_cats || is_valid_png(b_path) {
                form_valid = true;
            }
        } else if config.show_invalid_cats {
            form_valid = dir.exists();
        }

        forms_existence[form_idx] = form_valid;
        
        if form_valid {
            deploy_icon_paths[form_idx] = resolved_icon;
        }
    }

    if !config.show_invalid_cats && forms_existence.iter().all(|&e| !e) {
        return None; 
    }

    for form_idx in (0..=config.preferred_form).rev() {
        if forms_existence[form_idx] {
            let dir = paths::folder(cats_root_dir, cat_id, form_idx, egg_ids);
            let banner_stem = paths::image_stem(paths::AssetType::Banner, cat_id, form_idx, egg_ids);
            let banner_name = format!("{}.png", banner_stem);
            let mut b = crate::global::resolver::get(&dir, &[banner_name.as_str()], priority).into_iter().next();
            
            if b.is_none() && form_idx == 1 && egg_ids.1 != -1 {
                let fallback_stem = format!("udi{:03}_m00", egg_ids.1);
                let fallback_name = format!("{}.png", fallback_stem);
                b = crate::global::resolver::get(&dir, &[fallback_name.as_str()], priority).into_iter().next();
            }
            final_image_path_opt = b;
            break;
        }
    }

    let mut attack_anim_frames = [0; 4];
    for i in 0..4 {
        if !forms_existence[i] { continue; }
        let p = paths::maanim(cats_root_dir, cat_id, i, egg_ids, 2);
        
        if let (Some(parent), Some(name)) = (p.parent(), p.file_name().and_then(|n| n.to_str())) {
            if let Some(resolved) = crate::global::resolver::get(parent, &[name], priority).into_iter().next() {
                if let Ok(bytes) = fs::read(&resolved) {
                    let content = String::from_utf8_lossy(&bytes);
                    let duration = Animation::scan_duration(&content);
                    attack_anim_frames[i] = if duration > 0 { duration + 1 } else { 0 };
                }
            }
        }
    }
    
    let mut cat_stats = vec![None; 4];
    if let Some(resolved) = resolved_stats {
        if let Ok(bytes) = fs::read(resolved) {
            let file_content = String::from_utf8_lossy(&bytes);
            let delimiter = utils::detect_csv_separator(&file_content);
            for (line_index, csv_line) in file_content.lines().enumerate().take(4) {
                cat_stats[line_index] = CatRaw::from_csv_line(csv_line, delimiter);
            }
        }
    }

    let mut cat_names = vec![String::new(); 4];
    let mut cat_descriptions = vec![Vec::new(); 4];
    
    let lang_directory = paths::lang(cats_root_dir, cat_id);
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);
    
    let mut search_dirs = vec![original_folder_path.to_path_buf()];
    if lang_directory.exists() { search_dirs.insert(0, lang_directory); }

    for dir in search_dirs {
        let resolved_paths = crate::global::resolver::get(&dir, &[base_filename.as_str()], priority);
        for name_file_path in resolved_paths {
            if let Some(explanation) = unitexplanation::UnitExplanation::load(&name_file_path) {
                for i in 0..4 {
                    if !forms_existence[i] || !cat_names[i].is_empty() { continue; }
                    let name = explanation.names.get(i).cloned().unwrap_or_default();
                    if name.is_empty() { continue; }
                    cat_names[i] = name;
                    cat_descriptions[i] = explanation.descriptions.get(i).cloned().unwrap_or_default();
                }
            }
        }
        if (0..4).any(|i| forms_existence[i] && !cat_names[i].is_empty()) { break; }
    }
    
    Some(CatEntry { 
        id: cat_id, 
        image_path: final_image_path_opt, 
        deploy_icon_paths,
        names: cat_names,
        description: cat_descriptions, 
        forms: forms_existence, 
        stats: cat_stats, 
        curve: level_curves.get(cat_id as usize).cloned(), 
        atk_anim_frames: attack_anim_frames,
        egg_ids, 
        talent_data: talents_map.get(&(cat_id as u16)).cloned(), 
        unit_buy: ub_row.clone(), 
        evolve_text: evolve_text_map.get(&cat_id).cloned().unwrap_or_default(),
        talent_costs: Arc::clone(talent_costs),
        skill_descriptions: Arc::clone(skill_descriptions),
    })
}