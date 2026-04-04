use std::collections::{HashSet, HashMap};
use crate::features::enemy::registry::{ENEMY_ABILITY_REGISTRY, ENEMY_STATS_REGISTRY, AbilityIcon, Magnification};
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::global::game::abilities::CustomIcon;
use crate::global::game::img015;

pub const ATTACK_TYPE_ICONS: &[AbilityIcon] = &[
    AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
    AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
    AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
    AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
    AbilityIcon::Custom(CustomIcon::Multihit),
];

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
pub struct EnemyFilterState {
    pub is_open: bool,
    pub active_icons: HashSet<AbilityIcon>,
    pub match_mode: MatchMode,
    pub adv_ranges: HashMap<AbilityIcon, HashMap<&'static str, RangeInput>>,
    pub mag_input: String,
    pub stat_ranges: HashMap<&'static str, RangeInput>,
}

impl Default for EnemyFilterState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_icons: HashSet::new(),
            match_mode: MatchMode::And,
            adv_ranges: HashMap::new(),
            mag_input: String::new(),
            stat_ranges: HashMap::new(),
        }
    }
}

impl EnemyFilterState {
    pub fn is_active(&self) -> bool {
        !self.active_icons.is_empty()
            || self.stat_ranges.values().any(|r| !r.min.is_empty() || !r.max.is_empty())
    }
}

pub fn get_stat_value(s: &EnemyRaw, stat: &str, anim_frames: i32, mag: i32) -> i32 {
    let reg_name = match stat {
        "Atk Cycle (f)" => "Atk Cycle",
        _ => stat,
    };
    
    if let Some(def) = ENEMY_STATS_REGISTRY.iter().find(|d| d.name == reg_name) {
        let magnification = Magnification { hitpoints: mag, attack: mag };
        return (def.get_value)(s, anim_frames, magnification);
    }
    0 
}

pub fn get_icon_name(icon: AbilityIcon) -> String {
    ENEMY_ABILITY_REGISTRY.iter().find(|d| d.icon == icon).map(|d| d.name).unwrap_or("Unknown").to_string()
}

pub fn has_trait_or_ability(s: &EnemyRaw, icon: AbilityIcon) -> bool {
    ENEMY_ABILITY_REGISTRY.iter().find(|d| d.icon == icon).map_or(false, |def| {
        !(def.get_attributes)(s).is_empty()
    })
}

pub fn entity_passes_filter(enemy: &EnemyEntry, filter: &EnemyFilterState) -> bool {
    let mag = filter.mag_input.parse::<i32>().unwrap_or(100);
    let has_stat_filters = filter.stat_ranges.values().any(|r| !r.min.is_empty() || !r.max.is_empty());
    let has_icon_filters = !filter.active_icons.is_empty();

    if !has_stat_filters && !has_icon_filters {
        return true;
    }

    let stats = &enemy.stats;
    let mut active_conditions = 0;
    let mut passed_conditions = 0;
    let mut failed_conditions = 0;

    if has_stat_filters {
        for (stat_name, range) in &filter.stat_ranges {
            if range.min.is_empty() && range.max.is_empty() { continue; }
            active_conditions += 1;
            
            let val = get_stat_value(stats, stat_name, enemy.atk_anim_frames, mag);

            let r_min = range.min.parse::<i32>().unwrap_or(i32::MIN);
            let r_max = range.max.parse::<i32>().unwrap_or(i32::MAX);

            if val <= r_max && val >= r_min {
                passed_conditions += 1;
            } else {
                failed_conditions += 1;
            }
        }
    }

    if has_icon_filters {
        for &ability_icon in &filter.active_icons {
            active_conditions += 1;

            let has_inherent = has_trait_or_ability(stats, ability_icon);
            let mut icon_passed = false;

            let ability_def = ENEMY_ABILITY_REGISTRY.iter().find(|d| d.icon == ability_icon);

            if has_inherent {
                if let Some(adv_map) = filter.adv_ranges.get(&ability_icon) {
                    let mut build_passed_all_attrs = true;
                    
                    let attrs = ability_def.map(|def| (def.get_attributes)(stats)).unwrap_or_default();
                    
                    for (attr, range) in adv_map {
                        let mut val = attrs.iter()
                            .find(|(k, _, _)| k == attr)
                            .map(|(_, v, _)| *v)
                            .unwrap_or(0);
                        
                        if let Some(def) = ability_def {
                            if def.minus_one_is_inf && val == -1 {
                                val = i32::MAX;
                            }
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
        failed_conditions == 0
    } else {
        passed_conditions > 0
    }
}