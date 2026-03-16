use std::path::{Path, PathBuf};
use std::fs;
use std::thread;
use std::sync::{Arc, mpsc::{self, Receiver}};
use rayon::prelude::*;
use crate::features::cat::data::unitid::CatRaw;
use crate::features::cat::data::unitbuy::{self, UnitBuyRow};
use crate::features::cat::data::unitlevel::{self, CatLevelCurve};
use crate::features::cat::data::skillacquisition::{self, TalentRaw}; 
use crate::features::cat::data::unitevolve; 
use crate::features::cat::data::unitexplanation; 
use crate::global::utils; 
use crate::features::cat::paths;
use crate::features::settings::logic::state::ScannerConfig;
use crate::global::formats::maanim::Animation;

#[derive(Clone, Debug)]
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
}

impl CatEntry {
    pub fn id_str(&self, form_index: usize) -> String {
        format!("{:03}-{}", self.id, form_index + 1)
    }

    pub fn display_name(&self, form_index: usize) -> String {
        let raw_name = self.names.get(form_index).cloned().unwrap_or_default();
        if raw_name.is_empty() {
            self.id_str(form_index)
        } else {
            raw_name
        }
    }
    
    pub fn base_id_str(&self) -> String {
        format!("{:03}", self.id)
    }
}

pub fn start_scan(config: ScannerConfig) -> Receiver<CatEntry> {
    let (cat_sender, cat_receiver) = mpsc::channel();

    thread::spawn(move || {
        let cats_directory = Path::new(paths::DIR_CATS);
        let priority = &config.language_priority;
        
        // Note: For a truly perfect architecture, these loaders should also be updated 
        // to use resolver::get internally. For now, we pass the priority down
        let level_curves_arc = Arc::new(unitlevel::load_level_curves(cats_directory, priority));
        let unit_buy_map_arc = Arc::new(unitbuy::load_unitbuy(cats_directory, priority));
        let talent_map_arc = Arc::new(skillacquisition::load(cats_directory, priority));
        let evolve_text_map_arc = Arc::new(unitevolve::load(cats_directory, priority));
        
        let folder_entries: Vec<PathBuf> = match fs::read_dir(cats_directory) {
            Ok(read_dir_iter) => read_dir_iter
                .filter_map(|entry_result| entry_result.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect(),
            Err(_) => Vec::new(),
        };

        folder_entries.par_iter().for_each(|folder_path| {
            let sender_clone = cat_sender.clone();
            let curves_clone = Arc::clone(&level_curves_arc);
            let unit_buys_clone = Arc::clone(&unit_buy_map_arc);
            let talents_clone = Arc::clone(&talent_map_arc);
            let evolve_text_clone = Arc::clone(&evolve_text_map_arc);
            
            if let Some(cat_entry) = process_cat_entry(
                folder_path, 
                &curves_clone, 
                &unit_buys_clone, 
                &talents_clone, 
                &evolve_text_clone, 
                &config
            ) {
                let _ = sender_clone.send(cat_entry);
            }
        });
    });
    cat_receiver
}

pub fn process_cat_entry(
    original_folder_path: &Path, 
    level_curves: &[CatLevelCurve], 
    unit_buys: &std::collections::HashMap<u32, UnitBuyRow>,
    talents_map: &std::collections::HashMap<u16, TalentRaw>, 
    evolve_text_map: &std::collections::HashMap<u32, [Vec<String>; 4]>, 
    config: &ScannerConfig
) -> Option<CatEntry> {
    let folder_stem = original_folder_path.file_name()?.to_str()?;
    let cat_id = folder_stem.parse::<u32>().ok()?;
    let ub_row = unit_buys.get(&cat_id)?;

    let is_egg_unit = ub_row.egg_id_normal != -1;
    let is_summon = ub_row.level_cap_standard == 1 && ub_row.level_cap_plus == 0 && ub_row.purchase_cost == 0;

    if !config.show_invalid && !is_egg_unit && is_summon { return None; }

    let cats_root_dir = Path::new(paths::DIR_CATS);
    let egg_ids = (ub_row.egg_id_normal, ub_row.egg_id_evolved);
    let priority = &config.language_priority;

    // Check form existence based on physical folders
    let mut forms_existence = [false; 4];
    for i in 0..4 {
        let folder = paths::folder(cats_root_dir, cat_id, i, egg_ids);
        forms_existence[i] = folder.exists();
    }

    // Resolve Main List Image
    let mut final_image_path_opt = None;
    for form_idx in (0..=config.preferred_form).rev() {
        if form_idx >= 4 || !forms_existence[form_idx] { continue; }
        let dir = paths::folder(cats_root_dir, cat_id, form_idx, egg_ids);
        let form_char = match form_idx { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
        let filename = format!("udi{:03}_{}.png", cat_id, form_char);
        
        if let Some(found) = crate::global::resolver::get(&dir, &filename, priority).into_iter().next() {
            final_image_path_opt = Some(found);
            break; 
        }
    }

    if !config.show_invalid && final_image_path_opt.is_none() {
        return None;
    }
    
    // Resolve Deploy Icons
    let mut deploy_icon_paths: [Option<PathBuf>; 4] = Default::default();
    for form_idx in 0..4 {
        if !forms_existence[form_idx] { continue; }
        let dir = paths::folder(cats_root_dir, cat_id, form_idx, egg_ids);
        let form_char = match form_idx { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
        let filename = format!("uni{:03}_{}00.png", cat_id, form_char);
        
        deploy_icon_paths[form_idx] = crate::global::resolver::get(&dir, &filename, priority).into_iter().next();
    }

    // Resolve Attack Animations
    let mut attack_anim_frames = [0; 4];
    for i in 0..4 {
        if !forms_existence[i] { continue; }
        let p = paths::maanim(cats_root_dir, cat_id, i, egg_ids, 2);
        let parent = p.parent().unwrap();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap();

        if let Some(resolved) = crate::global::resolver::get(parent, name, priority).into_iter().next() {
            // Use lossy reading even for anim files to be safe
            if let Ok(bytes) = fs::read(&resolved) {
                let content = String::from_utf8_lossy(&bytes);
                let duration = Animation::scan_duration(&content);
                attack_anim_frames[i] = if duration > 0 { duration + 1 } else { 0 };
            }
        }
    }
    
    // Resolve Stats
    let mut cat_stats = vec![None; 4];
    let stats_path = paths::stats(cats_root_dir, cat_id);
    let stats_parent = stats_path.parent().unwrap();
    let stats_name = stats_path.file_name().unwrap().to_str().unwrap();

    if let Some(resolved) = crate::global::resolver::get(stats_parent, stats_name, priority).into_iter().next() {
        if let Ok(bytes) = fs::read(resolved) {
            let file_content = String::from_utf8_lossy(&bytes);
            let delimiter = utils::detect_csv_separator(&file_content);
            for (line_index, csv_line) in file_content.lines().enumerate().take(4) {
                cat_stats[line_index] = CatRaw::from_csv_line(csv_line, delimiter);
            }
        }
    }

    // Resolve Names & Descriptions
    let mut cat_names = vec![String::new(); 4];
    let mut cat_descriptions = vec![Vec::new(); 4];
    
    let lang_directory = paths::lang(cats_root_dir, cat_id);
    let base_filename = format!("Unit_Explanation{}.csv", cat_id + 1);
    
    let mut search_dirs = vec![original_folder_path.to_path_buf()];
    if lang_directory.exists() { search_dirs.insert(0, lang_directory); }

    for dir in search_dirs {
        let resolved_paths = crate::global::resolver::get(&dir, &base_filename, priority);
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
    })
}