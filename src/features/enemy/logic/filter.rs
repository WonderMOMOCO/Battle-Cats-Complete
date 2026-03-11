use std::collections::{HashSet, HashMap};
use crate::features::enemy::registry::ENEMY_ABILITY_REGISTRY;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::logic::scanner::EnemyEntry;
use crate::features::enemy::registry::ENEMY_STATS_REGISTRY;
use crate::global::game::img015;

pub const ATTACK_TYPE_ICONS: &[usize] = &[
    img015::ICON_SINGLE_ATTACK,
    img015::ICON_AREA_ATTACK,
    img015::ICON_OMNI_STRIKE,
    img015::ICON_LONG_DISTANCE,
    img015::ICON_MULTIHIT,
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
    pub active_icons: HashSet<usize>,
    pub match_mode: MatchMode,
    pub adv_ranges: HashMap<usize, HashMap<&'static str, RangeInput>>,
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
        return (def.get_value)(s, anim_frames, mag);
    }

    0 
}

pub fn get_adv_attributes(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "Wave Attack" => Some(&["Chance", "Level"]),
        "Mini-Wave" => Some(&["Chance", "Level"]),
        "Surge Attack" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Mini-Surge" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Death Surge" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Explosion" => Some(&["Chance", "Min-Range", "Max-Range"]),
        "Savage Blow" => Some(&["Chance", "Boost (%)"]),
        "Critical Hit" => Some(&["Chance"]),
        "Strengthen" => Some(&["Hitpoints (%)", "Boost (%)"]),
        "Survive" => Some(&["Chance"]),
        "Barrier" => Some(&["Hitpoints"]),
        "Aku Shield" => Some(&["Hitpoints", "Regen (%)"]),
        "Dodge" => Some(&["Chance", "Duration (f)"]),
        "Weaken" => Some(&["Chance", "Reduced-To", "Duration (f)"]),
        "Freeze" => Some(&["Chance", "Duration (f)"]),
        "Slow" => Some(&["Chance", "Duration (f)"]),
        "Knockback" => Some(&["Chance"]),
        "Curse" => Some(&["Chance", "Duration (f)"]),
        "Warp" => Some(&["Chance", "Duration (f)", "Min-Distance", "Max-Distance"]),
        "Toxic" => Some(&["Chance", "Damage (%)"]),
        "Burrow" => Some(&["Count", "Distance"]),
        "Revive" => Some(&["Count", "Duration (f)", "Hitpoints (%)"]),
        _ => None,
    }
}

pub fn get_icon_name(icon_id: usize) -> String {
    ENEMY_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map(|d| d.name).unwrap_or("Unknown").to_string()
}

pub fn get_ability_value(s: &EnemyRaw, ability_name: &str, attr: &str) -> i32 {
    match (ability_name, attr) {
        ("Wave Attack", "Chance") => s.wave_chance,
        ("Wave Attack", "Level") => s.wave_level,
        ("Mini-Wave", "Chance") => s.wave_chance, 
        ("Mini-Wave", "Level") => s.wave_level,
        ("Surge Attack", "Chance") => s.surge_chance,
        ("Surge Attack", "Level") => s.surge_level,
        ("Surge Attack", "Min-Range") => s.surge_spawn_min,
        ("Surge Attack", "Max-Range") => s.surge_spawn_min + s.surge_spawn_max,
        ("Mini-Surge", "Chance") => s.surge_chance, 
        ("Mini-Surge", "Level") => s.surge_level,
        ("Mini-Surge", "Min-Range") => s.surge_spawn_min,
        ("Mini-Surge", "Max-Range") => s.surge_spawn_min + s.surge_spawn_max,
        ("Death Surge", "Chance") => s.death_surge_chance,
        ("Death Surge", "Level") => s.death_surge_level,
        ("Death Surge", "Min-Range") => s.death_surge_spawn_min,
        ("Death Surge", "Max-Range") => s.death_surge_spawn_min + s.death_surge_spawn_max,
        ("Explosion", "Chance") => s.explosion_chance,
        ("Explosion", "Min-Range") => s.explosion_anchor,
        ("Explosion", "Max-Range") => s.explosion_anchor + s.explosion_span,
        ("Savage Blow", "Chance") => s.savage_blow_chance,
        ("Savage Blow", "Boost (%)") => s.savage_blow_boost,
        ("Critical Hit", "Chance") => s.critical_chance,
        ("Strengthen", "Hitpoints (%)") => s.strengthen_threshold,
        ("Strengthen", "Boost (%)") => s.strengthen_boost,
        ("Survive", "Chance") => s.survive_chance,
        ("Barrier", "Hitpoints") => s.barrier_hitpoints,
        ("Aku Shield", "Hitpoints") => s.shield_hitpoints,
        ("Aku Shield", "Regen (%)") => s.shield_regen,
        ("Dodge", "Chance") => s.dodge_chance,
        ("Dodge", "Duration (f)") => s.dodge_duration,
        ("Weaken", "Chance") => s.weaken_chance,
        ("Weaken", "Reduced-To") => s.weaken_percent,
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
        ("Warp", "Min-Distance") => s.warp_distance_min,
        ("Warp", "Max-Distance") => s.warp_distance_max,
        ("Toxic", "Chance") => s.toxic_chance,
        ("Toxic", "Damage (%)") => s.toxic_damage,
        ("Burrow", "Count") => s.burrow_amount,
        ("Burrow", "Distance") => s.burrow_distance,
        ("Revive", "Count") => s.revive_count,
        ("Revive", "Duration (f)") => s.revive_time,
        ("Revive", "Hitpoints (%)") => s.revive_hp,
        _ => 0,
    }
}

pub fn has_trait_or_ability(s: &EnemyRaw, icon_id: usize) -> bool {
    ENEMY_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map_or(false, |def| {
        let val = (def.getter)(s);
        if def.minus_one_is_inf {
            val != 0 
        } else {
            val > 0 
        }
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
        for &icon_id in &filter.active_icons {
            active_conditions += 1;

            let name = get_icon_name(icon_id);
            let has_inherent = has_trait_or_ability(stats, icon_id);
            let mut icon_passed = false;

            let ability_def = ENEMY_ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id);

            if has_inherent {
                if let Some(adv_map) = filter.adv_ranges.get(&icon_id) {
                    let mut build_passed_all_attrs = true;
                    
                    for (attr, range) in adv_map {
                        let mut val = get_ability_value(stats, &name, attr);
                        
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