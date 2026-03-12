use std::collections::{HashSet, HashMap};
use crate::features::cat::registry::CAT_ABILITY_REGISTRY;
use crate::features::cat::logic::stats::CatRaw;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::logic::talents::apply_talent_stats;
use crate::features::cat::registry::CAT_STATS_REGISTRY;
use crate::global::game::img015;

pub const ATTACK_TYPE_ICONS: &[usize] = &[
    img015::ICON_SINGLE_ATTACK,
    img015::ICON_AREA_ATTACK,
    img015::ICON_OMNI_STRIKE,
    img015::ICON_LONG_DISTANCE,
    img015::ICON_MULTIHIT,
];

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TalentFilterMode {
    #[default]
    Ignore,
    Consider,
    Only,
}

impl TalentFilterMode {
    pub fn label(&self) -> &'static str {
        match self {
            TalentFilterMode::Ignore => "Ignore",
            TalentFilterMode::Consider => "Consider",
            TalentFilterMode::Only => "Only",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum MatchMode {
    #[default]
    And,
    Or,
}

#[derive(Clone, PartialEq, Default)]
pub struct RangeInput {
    pub min: String,
    pub max: String,
}

#[derive(Clone, PartialEq)]
pub struct CatFilterState {
    pub is_open: bool,
    pub active_icons: HashSet<usize>,
    pub rarities: [bool; 6], 
    pub forms: [bool; 4],    
    pub match_mode: MatchMode,
    pub talent_mode: TalentFilterMode,
    pub ultra_talent_mode: TalentFilterMode,
    pub adv_ranges: HashMap<usize, HashMap<&'static str, RangeInput>>,
    pub level_input: String,
    pub stat_ranges: HashMap<&'static str, RangeInput>,
}

impl Default for CatFilterState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_icons: HashSet::new(),
            rarities: [false; 6],
            forms: [false; 4],
            match_mode: MatchMode::And,
            talent_mode: TalentFilterMode::Ignore,
            ultra_talent_mode: TalentFilterMode::Ignore,
            adv_ranges: HashMap::new(),
            level_input: String::new(),
            stat_ranges: HashMap::new(),
        }
    }
}

impl CatFilterState {
    pub fn is_active(&self) -> bool {
        !self.active_icons.is_empty()
            || self.rarities.iter().any(|&r| r)
            || self.forms.iter().any(|&f| f)
            || self.talent_mode == TalentFilterMode::Only
            || self.ultra_talent_mode == TalentFilterMode::Only
            || self.stat_ranges.values().any(|r| !r.min.is_empty() || !r.max.is_empty())
    }
}

pub fn get_stat_value(s: &CatRaw, stat: &str, anim_frames: i32) -> i32 {
    let reg_name = match stat {
        "Cooldown (f)" => "Cooldown", 
        "Atk Cycle (f)" => "Atk Cycle",
        _ => stat,
    };
    
    if let Some(def) = CAT_STATS_REGISTRY.iter().find(|d| d.name == reg_name) {
        return (def.get_value)(s, anim_frames);
    }
    0 
}

pub fn get_icon_name(icon_id: usize) -> String {
    CAT_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map(|d| d.name).unwrap_or("Unknown").to_string()
}

pub fn has_trait_or_ability(s: &CatRaw, icon_id: usize) -> bool {
    CAT_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map_or(false, |def| {
        !(def.get_attributes)(s).is_empty()
    })
}

pub fn entity_passes_filter(cat: &CatEntry, filter: &CatFilterState) -> bool {
    let any_rarity_selected = filter.rarities.iter().any(|&r| r);
    if any_rarity_selected {
        let r_idx = cat.unit_buy.rarity as usize;
        if r_idx >= filter.rarities.len() || !filter.rarities[r_idx] {
            return false; 
        }
    }

    let any_form_selected = filter.forms.iter().any(|&f| f);
    let mut forms_to_check = Vec::new();
    
    for i in 0..4 {
        if cat.forms[i] {
            if !any_form_selected || filter.forms[i] {
                forms_to_check.push(i);
            }
        }
    }
    
    if forms_to_check.is_empty() { return false; } 

    let req_normal = filter.talent_mode == TalentFilterMode::Only;
    let req_ultra = filter.ultra_talent_mode == TalentFilterMode::Only;
    let filter_level = filter.level_input.parse::<i32>().unwrap_or(50);
    let has_stat_filters = filter.stat_ranges.values().any(|r| !r.min.is_empty() || !r.max.is_empty());
    let has_icon_filters = !filter.active_icons.is_empty();

    if !has_stat_filters && !has_icon_filters && !req_normal && !req_ultra {
        return true;
    }

    if !has_stat_filters && !has_icon_filters {
        for &form_idx in &forms_to_check {
            let mut has_any_normal = false;
            let mut has_any_ultra = false;

            if form_idx >= 2 {
                if let Some(t_data) = cat.talent_data.as_ref() {
                    for g in &t_data.groups {
                        if g.limit == 1 { has_any_ultra = true; } 
                        else { has_any_normal = true; }
                    }
                }
            }

            let passed = if req_normal && req_ultra {
                has_any_normal || has_any_ultra
            } else if req_normal {
                has_any_normal
            } else if req_ultra {
                has_any_ultra
            } else {
                true
            };

            if passed { return true; }
        }
        return false;
    }

    for &form_idx in &forms_to_check {
        if let Some(Some(stats)) = cat.stats.get(form_idx) {
            
            let mut passes_talent_only = true;
            if req_normal || req_ultra {
                let mut has_any_normal = false;
                let mut has_any_ultra = false;

                if form_idx >= 2 {
                    if let Some(t_data) = cat.talent_data.as_ref() {
                        for g in &t_data.groups {
                            if g.limit == 1 { has_any_ultra = true; } 
                            else { has_any_normal = true; }
                        }
                    }
                }

                passes_talent_only = if req_normal && req_ultra {
                    has_any_normal || has_any_ultra
                } else if req_normal {
                    has_any_normal
                } else {
                    has_any_ultra
                };
            }
            if !passes_talent_only { continue; }

            let mut active_conditions = 0;
            let mut passed_conditions = 0;
            let mut failed_conditions = 0;

            let base_leveled = crate::features::cat::logic::stats::apply_level(stats, cat.curve.as_ref(), filter_level);
            
            let mut state_normal = base_leveled.clone();
            let mut state_ultra = base_leveled.clone();

            let (stats_min, stats_max) = if form_idx >= 2 && cat.talent_data.is_some() {
                let t_data = cat.talent_data.as_ref().unwrap();
                let mut min_levels = HashMap::new();
                let mut max_levels = HashMap::new();
                let mut norm_map = HashMap::new();
                let mut ultra_map = HashMap::new();

                for (idx, g) in t_data.groups.iter().enumerate() {
                    let is_ultra = g.limit == 1;
                    let mode = if is_ultra { filter.ultra_talent_mode } else { filter.talent_mode };
                    
                    if mode == TalentFilterMode::Only {
                        min_levels.insert(idx as u8, g.max_level);
                        max_levels.insert(idx as u8, g.max_level);
                    } else if mode == TalentFilterMode::Consider {
                        max_levels.insert(idx as u8, g.max_level);
                    }

                    if is_ultra {
                        ultra_map.insert(idx as u8, g.max_level);
                    } else {
                        norm_map.insert(idx as u8, g.max_level);
                        ultra_map.insert(idx as u8, g.max_level); // Ultra includes normal
                    }
                }
                
                state_normal = apply_talent_stats(&base_leveled, t_data, &norm_map);
                state_ultra = apply_talent_stats(&base_leveled, t_data, &ultra_map);

                let s_min = apply_talent_stats(&base_leveled, t_data, &min_levels);
                let s_max = apply_talent_stats(&base_leveled, t_data, &max_levels);
                (s_min, s_max)
            } else {
                (base_leveled.clone(), base_leveled.clone())
            };

            if has_stat_filters {
                for (stat_name, range) in &filter.stat_ranges {
                    if range.min.is_empty() && range.max.is_empty() { continue; }
                    active_conditions += 1;
                    
                    let val_a = get_stat_value(&stats_min, stat_name, cat.atk_anim_frames[form_idx]);
                    let val_b = get_stat_value(&stats_max, stat_name, cat.atk_anim_frames[form_idx]);
                    
                    let s_min = val_a.min(val_b);
                    let s_max = val_a.max(val_b);

                    let r_min = range.min.parse::<i32>().unwrap_or(i32::MIN);
                    let r_max = range.max.parse::<i32>().unwrap_or(i32::MAX);

                    if s_min <= r_max && s_max >= r_min {
                        passed_conditions += 1;
                    } else {
                        failed_conditions += 1;
                    }
                }
            }

            if has_icon_filters {
                for &icon_id in &filter.active_icons {
                    active_conditions += 1;

                    let has_inherent = has_trait_or_ability(stats, icon_id);
                    let ability_def = CAT_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id);
                    
                    let mut has_normal = false;
                    let mut has_ultra = false;

                    if form_idx >= 2 {
                        if let Some(t_data) = cat.talent_data.as_ref() {
                            for g in &t_data.groups {
                                let matches_icon = ability_def.map_or(false, |d| g.ability_id == d.talent_id || g.name_id as u8 == d.talent_id);
                                if matches_icon {
                                    if g.limit == 1 { has_ultra = true; } 
                                    else { has_normal = true; }
                                }
                            }
                        }
                    }

                    let valid_inherent = filter.talent_mode != TalentFilterMode::Only && filter.ultra_talent_mode != TalentFilterMode::Only && has_inherent;
                    let valid_normal = filter.talent_mode != TalentFilterMode::Ignore && has_normal;
                    let valid_ultra = filter.ultra_talent_mode != TalentFilterMode::Ignore && has_ultra;

                    let mut icon_passed = false;

                    if valid_inherent || valid_normal || valid_ultra {
                        if let Some(adv_map) = filter.adv_ranges.get(&icon_id) {
                            
                            let mut test_builds = Vec::new();
                            if valid_inherent { test_builds.push(&base_leveled); }
                            if valid_normal { test_builds.push(&state_normal); }
                            if valid_ultra { test_builds.push(&state_ultra); }
                            
                            let mut any_build_passed = false;

                            for build_stats in test_builds {
                                let mut build_passed_all_attrs = true;
                                
                                let attrs = ability_def.map(|def| (def.get_attributes)(build_stats)).unwrap_or_default();
                                
                                for (attr, range) in adv_map {
                                    let val = attrs.iter()
                                        .find(|(k, _, _)| k == attr)
                                        .map(|(_, v, _)| *v)
                                        .unwrap_or(0);
                                        
                                    if let Some(min) = range.min.parse::<i32>().ok() {
                                        if val < min {
                                            build_passed_all_attrs = false;
                                            break;
                                        }
                                    }
                                    
                                    if let Some(max) = range.max.parse::<i32>().ok() {
                                        if val > max {
                                            build_passed_all_attrs = false;
                                            break;
                                        }
                                    }
                                }

                                if build_passed_all_attrs {
                                    any_build_passed = true;
                                    break;
                                }
                            }

                            if any_build_passed {
                                icon_passed = true;
                            }
                        } else {
                            icon_passed = true;
                        }
                    }

                    if icon_passed {
                        passed_conditions += 1;
                    } else {
                        failed_conditions += 1;
                    }
                }
            }

            if active_conditions == 0 {
                return true; 
            }

            if filter.match_mode == MatchMode::And {
                if failed_conditions == 0 { return true; }
            } 
            else {
                if passed_conditions > 0 { return true; }
            }
        }
    }
    
    false
}