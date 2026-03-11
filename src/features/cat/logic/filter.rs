use std::collections::{HashSet, HashMap};
use crate::features::cat::registry::CAT_ABILITY_REGISTRY;
use crate::features::cat::logic::stats::CatRaw;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::data::skillacquisition::TalentGroupRaw;
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

pub fn get_adv_attributes(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "Metal Killer" => Some(&["Hitpoints (%)"]),
        "Wave Attack" => Some(&["Chance", "Level"]),
        "Mini-Wave" => Some(&["Chance", "Level"]),
        "Surge Attack" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Mini-Surge" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Explosion" => Some(&["Chance", "Min-Range", "Max-Range"]),
        "Savage Blow" => Some(&["Chance", "Boost (%)"]),
        "Critical Hit" => Some(&["Chance"]),
        "Strengthen" => Some(&["Hitpoints (%)", "Boost (%)"]),
        "Survive" => Some(&["Chance"]),
        "Barrier Breaker" => Some(&["Chance"]),
        "Shield Piercer" => Some(&["Chance"]),
        "Dodge" => Some(&["Chance", "Duration (f)"]),
        "Weaken" => Some(&["Chance", "Reduced-To", "Duration (f)"]),
        "Freeze" => Some(&["Chance", "Duration (f)"]),
        "Slow" => Some(&["Chance", "Duration (f)"]),
        "Knockback" => Some(&["Chance"]),
        "Curse" => Some(&["Chance", "Duration (f)"]),
        "Warp" => Some(&["Chance", "Duration (f)", "Min-Distance", "Max-Distance"]),
        _ => None,
    }
}

pub fn get_icon_name(icon_id: usize) -> String {
    CAT_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map(|d| d.name).unwrap_or("Unknown").to_string()
}

pub fn get_ability_value(s: &CatRaw, ability_name: &str, attr: &str) -> i32 {
    match (ability_name, attr) {
        ("Metal Killer", "Hitpoints (%)") => s.metal_killer_percent,
        ("Wave Attack", "Chance") => s.wave_chance,
        ("Wave Attack", "Level") => s.wave_level,
        ("Mini-Wave", "Chance") => s.wave_chance, 
        ("Mini-Wave", "Level") => s.wave_level,
        ("Surge Attack", "Chance") => s.surge_chance,
        ("Surge Attack", "Level") => s.surge_level,
        ("Surge Attack", "Min-Range") => s.surge_spawn_anchor,
        ("Surge Attack", "Max-Range") => s.surge_spawn_anchor + s.surge_spawn_span,
        ("Mini-Surge", "Chance") => s.surge_chance, 
        ("Mini-Surge", "Level") => s.surge_level,
        ("Mini-Surge", "Min-Range") => s.surge_spawn_anchor,
        ("Mini-Surge", "Max-Range") => s.surge_spawn_anchor + s.surge_spawn_span,
        ("Explosion", "Chance") => s.explosion_chance,
        ("Explosion", "Min-Range") => s.explosion_spawn_anchor,
        ("Explosion", "Max-Range") => s.explosion_spawn_anchor + s.explosion_spawn_span,
        ("Savage Blow", "Chance") => s.savage_blow_chance,
        ("Savage Blow", "Boost (%)") => s.savage_blow_boost,
        ("Critical Hit", "Chance") => s.critical_chance,
        ("Strengthen", "Hitpoints (%)") => s.strengthen_threshold,
        ("Strengthen", "Boost (%)") => s.strengthen_boost,
        ("Survive", "Chance") => s.survive,
        ("Barrier Breaker", "Chance") => s.barrier_breaker_chance,
        ("Shield Piercer", "Chance") => s.shield_pierce_chance,
        ("Dodge", "Chance") => s.dodge_chance,
        ("Dodge", "Duration (f)") => s.dodge_duration,
        ("Weaken", "Chance") => s.weaken_chance,
        ("Weaken", "Reduced-To") => s.weaken_to,
        ("Weaken", "Duration (f)") => s.weaken_duration,
        ("Freeze", "Chance") => s.freeze_chance,
        ("Freeze", "Duration (f)") => s.freeze_duration,
        ("Slow", "Chance") => s.slow_chance,
        ("Slow", "Duration (f)") => s.slow_duration,
        ("Knockback", "Chance") => s.knockback_chance,
        ("Curse", "Chance") => s.curse_chance,
        ("Curse", "Duration (f)") => s.curse_duration,
        ("Warp", "Chance") => s.warp_chance,
        ("Warp", "Duration (f)") => s.warp_duration,
        ("Warp", "Min-Distance") => s.warp_distance_minimum,
        ("Warp", "Max-Distance") => s.warp_distance_maximum,
        _ => 0,
    }
}

pub fn get_talent_modifier(g: &TalentGroupRaw, attr: &str) -> i32 {
    match attr {
        "Chance" => g.max_1 as i32,
        "Duration (f)" => if g.max_2 > 0 { g.max_2 as i32 } else { g.max_1 as i32 },
        "Level" => g.max_2 as i32,
        "Hitpoints (%)" => g.max_1 as i32,
        "Boost (%)" => g.max_2 as i32,
        "Reduced-To" => g.max_2 as i32,
        "Min-Distance" | "Min-Range" => g.max_3 as i32,
        "Max-Distance" | "Max-Range" => g.max_4 as i32,
        _ => 0,
    }
}

pub fn has_trait_or_ability(s: &CatRaw, icon_id: usize) -> bool {
    CAT_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map_or(false, |def| (def.getter)(s) > 0)
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

            let (stats_min, stats_max) = if form_idx >= 2 && cat.talent_data.is_some() {
                let t_data = cat.talent_data.as_ref().unwrap();
                let mut min_levels = HashMap::new();
                let mut max_levels = HashMap::new();

                for (idx, g) in t_data.groups.iter().enumerate() {
                    let is_ultra = g.limit == 1;
                    let mode = if is_ultra { filter.ultra_talent_mode } else { filter.talent_mode };
                    
                    if mode == TalentFilterMode::Only {
                        min_levels.insert(idx as u8, g.max_level);
                        max_levels.insert(idx as u8, g.max_level);
                    } else if mode == TalentFilterMode::Consider {
                        max_levels.insert(idx as u8, g.max_level);
                    }
                }
                
                let base_leveled = crate::features::cat::logic::stats::apply_level(stats, cat.curve.as_ref(), filter_level);
                let s_min = apply_talent_stats(&base_leveled, t_data, &min_levels);
                let s_max = apply_talent_stats(&base_leveled, t_data, &max_levels);
                (s_min, s_max)
            } else {
                let base_leveled = crate::features::cat::logic::stats::apply_level(stats, cat.curve.as_ref(), filter_level);
                (base_leveled.clone(), base_leveled)
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

                    let name = get_icon_name(icon_id);
                    let has_inherent = has_trait_or_ability(stats, icon_id);
                    
                    let mut normal_talents = Vec::new();
                    let mut ultra_talents = Vec::new();

                    if form_idx >= 2 {
                        if let Some(t_data) = cat.talent_data.as_ref() {
                            for g in &t_data.groups {
                                let matches_icon = CAT_ABILITY_REGISTRY.iter()
                                    .any(|d| d.icon_id == icon_id && (g.ability_id == d.talent_id || g.name_id as u8 == d.talent_id));

                                if matches_icon {
                                    if g.limit == 1 {
                                        ultra_talents.push(g);
                                    } else {
                                        normal_talents.push(g);
                                    }
                                }
                            }
                        }
                    }

                    let has_normal = !normal_talents.is_empty();
                    let has_ultra = !ultra_talents.is_empty();

                    let valid_inherent = filter.talent_mode != TalentFilterMode::Only && filter.ultra_talent_mode != TalentFilterMode::Only && has_inherent;
                    let valid_normal = filter.talent_mode != TalentFilterMode::Ignore && has_normal;
                    let valid_ultra = filter.ultra_talent_mode != TalentFilterMode::Ignore && has_ultra;

                    let mut icon_passed = false;

                    if valid_inherent || valid_normal || valid_ultra {
                        if let Some(adv_map) = filter.adv_ranges.get(&icon_id) {
                            
                            let mut test_builds = Vec::new();
                            if valid_inherent { test_builds.push(0); } 
                            if valid_normal { test_builds.push(1); }   
                            if valid_ultra { test_builds.push(2); }    

                            let mut any_build_passed = false;

                            for build in test_builds {
                                let mut build_passed_all_attrs = true;
                                
                                for (attr, range) in adv_map {
                                    let mut val = if has_inherent { get_ability_value(stats, &name, attr) } else { 0 };
                                    
                                    if build >= 1 {
                                        for g in &normal_talents { val += get_talent_modifier(g, attr); }
                                    }
                                    if build >= 2 {
                                        for g in &ultra_talents { val += get_talent_modifier(g, attr); }
                                    }

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
                if failed_conditions == 0 {
                    return true;
                }
            } 
            else {
                if passed_conditions > 0 {
                    return true;
                }
            }
        }
    }
    
    false
}