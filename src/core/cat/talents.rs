use crate::core::files::skillacquisition::{TalentRaw, TalentGroupRaw};
use crate::core::files::unitid::CatRaw;
use crate::core::files::unitlevel::CatLevelCurve;
use std::collections::HashMap;

pub fn calculate_talent_value(min: u16, max: u16, level: u8, max_level: u8) -> i32 {
    if level == 0 { return 0; }
    if max_level <= 1 { return min as i32; }
    if level == 1 { return min as i32; }
    if level == max_level { return max as i32; }

    let min_f = min as f32;
    let max_f = max as f32;
    let lvl_f = level as f32;
    let max_lvl_f = max_level as f32;

    let val = min_f + (max_f - min_f) * (lvl_f - 1.0) / (max_lvl_f - 1.0);
    val.round() as i32
}

fn fmt_additive(base: i32, bonus: i32, unit: &str) -> String {
    format!("{}{} (+{}{}) -> {}{}", base, unit, bonus, unit, base + bonus, unit)
}

fn fmt_multi(base: i32, pct: i32) -> String {
    let bonus_val = (base as f32 * (pct as f32 / 100.0)).round() as i32;
    format!("{} (+{}%) -> {}", base, pct, base + bonus_val)
}

fn fmt_state(talent_level: u8) -> String {
    if talent_level > 0 {
        "Inactive -> Active".to_string()
    } else {
        "Inactive -> Inactive".to_string()
    }
}

fn fmt_range(min: i32, max: i32) -> String {
    if min == max {
        format!("Range: {}", min)
    } else {
        format!("Range: {}~{}", min, max)
    }
}

pub fn calculate_talent_display(
    group: &TalentGroupRaw, 
    stats: &CatRaw, 
    talent_level: u8, 
    curve: Option<&CatLevelCurve>, 
    unit_level: i32
) -> Option<String> {
    
    let get_val = |min, max| calculate_talent_value(min, max, talent_level, group.max_level);
    
    let get_val_fallback = || {
        let v1 = get_val(group.min_1, group.max_1);
        if v1 != 0 { v1 } else { get_val(group.min_2, group.max_2) }
    };

    match group.ability_id {
        // State Talents
        5 | 6 | 7 | 12 | 14 | 16 | 23 | 29 | 33 | 34 | 35 | 36 | 37 | 38 | 39 | 40 | 41 | 
        44 | 45 | 46 | 47 | 48 | 49 | 53 | 55 | 57 | 63 | 66 | 67 | 92 => {
            return Some(fmt_state(talent_level));
        },

        // Behemoth Slayer Default
        64 => {
            let chance_val = get_val(group.min_1, group.max_1);
            let duration_val = get_val(group.min_2, group.max_2);
            let chance = if chance_val > 0 { chance_val } else { 5 };
            let duration = if duration_val > 0 { duration_val } else { 30 };
            return Some(format!("Inactive -> Active\nDodge Chance: {}%\nDodge Duration: {}f", chance, duration));
        },

        // Resistances
        18 | 19 | 20 | 21 | 22 | 24 | 30 | 52 | 54 => {
            let bonus = get_val(group.min_1, group.max_1);
            return Some(fmt_additive(0, bonus, "%"));
        },
        
        _ => {} 
    }

    match group.text_id {
        1 | 70 | 71 => { // Gain Weaken
            let chance = group.min_1; 
            let bonus_duration = get_val(group.min_2, group.max_2);
            let weaken_to = 100 - group.min_3; 
            Some(format!(
                "Duration: {}\nChance: {}%\nReduction: to {}% Atk",
                fmt_additive(stats.weaken_duration, bonus_duration, "f"), 
                chance,
                weaken_to
            ))
        },
        42 => { // Upgrade Weaken Duration
            let bonus = get_val_fallback();
            Some(format!("Duration: {}", fmt_additive(stats.weaken_duration, bonus, "f")))
        },

        2 | 76 => { // Freeze
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("Duration: {}\nChance: {}%", fmt_additive(stats.freeze_duration, bonus, "f"), chance))
        },
        43 => { // Upgrade Freeze
            let bonus = get_val_fallback();
            Some(format!("Duration: {}", fmt_additive(stats.freeze_duration, bonus, "f")))
        },
        74 => { // Upgrade Freeze Chance
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.freeze_chance, bonus, "%")))
        },

        3 | 69 | 72 => { // Slow
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("Duration: {}\nChance: {}%", fmt_additive(stats.slow_duration, bonus, "f"), chance))
        },
        44 => { // Upgrade Slow
            let bonus = get_val_fallback();
            Some(format!("Duration: {}", fmt_additive(stats.slow_duration, bonus, "f")))
        },
        63 => { // Upgrade Slow Chance
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.slow_chance, bonus, "%")))
        },

        8 | 73 | 75 => { // Knockback
            let mut bonus = get_val(group.min_1, group.max_1);
            if bonus == 0 && group.min_1 == 0 {
                bonus = get_val(group.min_2, group.max_2);
            }
            Some(format!("Chance: {}", fmt_additive(stats.knockback_chance, bonus, "%")))
        },
        45 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.knockback_chance, bonus, "%")))
        },

        10 => { // Gain Strengthen
            let hp_limit = 100 - group.min_1; 
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("{}\nTrigger at: {}% HP", fmt_additive(stats.strengthen_boost, bonus, "%"), hp_limit))
        },
        46 => { // Upgrade Strengthen
            let bonus = get_val_fallback();
            Some(fmt_additive(stats.strengthen_boost, bonus, "%"))
        },
        11 => { // Survive
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.survive, bonus, "%")))
        },
        47 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.survive, bonus, "%")))
        },
        13 => { // Critical
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.critical_chance, bonus, "%")))
        },
        48 => { // Critical Upgrade
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.critical_chance, bonus, "%")))
        },
        15 => { // Barrier Breaker
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.barrier_breaker_chance, bonus, "%")))
        },
        49 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.barrier_breaker_chance, bonus, "%")))
        },
        17 => { // Wave
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let range = 332.5 + ((level - 1) as f32 * 200.0);
            Some(format!("Chance: {}\nLevel: {}\nRange: {}", fmt_additive(stats.wave_chance, bonus, "%"), level, range))
        },
        50 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.wave_chance, bonus, "%")))
        },
        31 => { // Cost Down
            let reduction = get_val(group.min_1, group.max_1);
            let effective_reduction = (reduction as f32 * 1.5).round() as i32;
            let base = stats.eoc1_cost * 3 / 2;
            Some(format!("{}¢ (-{}¢) -> {}¢", base, effective_reduction, base.saturating_sub(effective_reduction)))
        },
        32 => { // Recover Speed
            let reduction = get_val(group.min_1, group.max_1);
            let base = stats.effective_cooldown();
            Some(format!("{}f (-{}f) -> {}f", base, reduction, base.saturating_sub(reduction)))
        },
        59 => { // Savage Blow
            let bonus = get_val(group.min_1, group.max_1);
            let dmg_boost = group.min_2;
            Some(format!("Chance: {}\nDamage Boost: +{}%", fmt_additive(stats.savage_blow_chance, bonus, "%"), dmg_boost))
        },
        61 => {
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.savage_blow_chance, bonus, "%")))
        },
        60 | 84 | 87 => { // Dodge
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("Duration: {}\nChance: {}%", fmt_additive(stats.dodge_duration, bonus, "f"), chance))
        },
        62 | 81 => { // Upgrade Dodge
            if group.min_1 != group.max_1 {
                 let bonus = get_val(group.min_1, group.max_1);
                 Some(format!("Chance: {}", fmt_additive(stats.dodge_chance, bonus, "%")))
            } else {
                 let bonus = get_val(group.min_2, group.max_2);
                 Some(format!("Duration: {}", fmt_additive(stats.dodge_duration, bonus, "f")))
            }
        },
        68 => { // Surge
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let min_range = group.min_3 / 4;
            let max_range = min_range + (group.min_4 / 4);
            Some(format!("Chance: {}\nLevel: {}\n{}", fmt_additive(stats.surge_chance, bonus, "%"), level, fmt_range(min_range as i32, max_range as i32)))
        },
        78 => { // Shield Pierce
            let bonus = get_val(group.min_1, group.max_1);
            Some(format!("Chance: {}", fmt_additive(stats.shield_pierce_chance, bonus, "%")))
        },
        80 => { // Curse
            let chance = group.min_1;
            let bonus = get_val(group.min_2, group.max_2);
            Some(format!("Duration: {}\nChance: {}%", fmt_additive(stats.curse_duration, bonus, "f"), chance))
        },
        93 => { // Upgrade Curse
            let bonus = get_val_fallback();
            Some(format!("Duration: {}", fmt_additive(stats.curse_duration, bonus, "f")))
        },
        82 => { // Attack Freq Up
            let percent = get_val(group.min_1, group.max_1);
            let base = stats.time_before_attack_1;
            let reduction = (base as f32 * percent as f32 / 100.0).round() as i32;
            Some(format!("{}f (-{}%) -> {}f", base, percent, base.saturating_sub(reduction)))
        },
        83 => { // Mini-Wave
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let range = 332.5 + ((level - 1) as f32 * 200.0);
            Some(format!("Chance: {}\nLevel: {} (Mini)\nRange: {}", fmt_additive(stats.wave_chance, bonus, "%"), level, range))
        },
        86 => { // Behemoth Slayer
            let chance = group.min_1;
            let duration = group.min_2;
            Some(format!("Inactive -> Active\nDodge Chance: {}%\nDodge Duration: {}f", chance, duration))
        },
        89 => { // Mini-Surge
            let bonus = get_val(group.min_1, group.max_1);
            let level = group.min_2;
            let min_range = group.min_3 / 4;
            let max_range = min_range + (group.min_4 / 4);
            Some(format!("Chance: {}\nLevel: {} (Mini)\n{}", fmt_additive(stats.surge_chance, bonus, "%"), level, fmt_range(min_range as i32, max_range as i32)))
        },
        88 | 90 | 95 => { // Unlock Dodge
            let bonus = get_val(group.min_1, group.max_1);
            let duration = group.min_2;
            Some(format!("Chance: {}\nDuration: {}f", fmt_additive(stats.dodge_chance, bonus, "%"), duration))
        },
        94 => { // Explosion
            let bonus = get_val(group.min_1, group.max_1);
            let min_range = group.min_2 / 4;
            let max_range = min_range + (group.min_3 / 4);
            Some(format!("Chance: {}\n{}", fmt_additive(stats.explosion_chance, bonus, "%"), fmt_range(min_range as i32, max_range as i32)))
        },
        29 => { // Speed
            let bonus = get_val(group.min_1, group.max_1);
            Some(fmt_additive(stats.speed, bonus, ""))
        },
        27 => { // Health Buff
            let pct = get_val(group.min_1, group.max_1);
            let base_hp = curve.map_or(stats.hitpoints, |c| c.calculate_stat(stats.hitpoints, unit_level));
            Some(fmt_multi(base_hp, pct))
        },
        28 => { // Attack Buff
            let pct = get_val(group.min_1, group.max_1);
            let total_base = stats.attack_1 + stats.attack_2 + stats.attack_3;
            let real_atk = curve.map_or(total_base, |c| c.calculate_stat(total_base, unit_level));
            Some(fmt_multi(real_atk, pct))
        },
        
        _ => None,
    }
}

fn apply_target_traits(stats: &mut CatRaw, name_id: i16, type_id: u16) {
    match name_id {
        0 => stats.target_red = 1,
        1 => stats.target_floating = 1,
        2 => stats.target_black = 1,
        3 => stats.target_metal = 1,
        4 => stats.target_angel = 1,
        5 => stats.target_alien = 1,
        6 => stats.target_zombie = 1,
        7 => stats.target_relic = 1,
        8 => stats.target_traitless = 1,
        9 => stats.target_witch = 1,
        10 => stats.target_eva = 1,
        11 => stats.target_aku = 1,
        _ => {}
    }

    if type_id > 0 {
        if (type_id & (1 << 0)) != 0 { stats.target_red = 1; }
        if (type_id & (1 << 1)) != 0 { stats.target_floating = 1; }
        if (type_id & (1 << 2)) != 0 { stats.target_black = 1; }
        if (type_id & (1 << 3)) != 0 { stats.target_metal = 1; }
        if (type_id & (1 << 4)) != 0 { stats.target_angel = 1; }
        if (type_id & (1 << 5)) != 0 { stats.target_alien = 1; }
        if (type_id & (1 << 6)) != 0 { stats.target_zombie = 1; }
        if (type_id & (1 << 7)) != 0 { stats.target_relic = 1; }
        if (type_id & (1 << 8)) != 0 { stats.target_traitless = 1; }
        if (type_id & (1 << 9)) != 0 { stats.target_witch = 1; }
        if (type_id & (1 << 10)) != 0 { stats.target_eva = 1; }
        if (type_id & (1 << 11)) != 0 { stats.target_aku = 1; }
    }
}

pub fn apply_talent_stats(base_stats: &CatRaw, talent_data: &TalentRaw, levels: &HashMap<u8, u8>) -> CatRaw {
    let mut stats = base_stats.clone();
    
    for (index, group) in talent_data.groups.iter().enumerate() {
        let current_level = *levels.get(&(index as u8)).unwrap_or(&0);
        
        if current_level > 0 && group.name_id != -1 {
            apply_target_traits(&mut stats, group.name_id, talent_data.type_id);
        }

        if current_level == 0 { continue; }
        
        let value_1 = calculate_talent_value(group.min_1, group.max_1, current_level, group.max_level);
        let value_2 = calculate_talent_value(group.min_2, group.max_2, current_level, group.max_level);

        let duration_val = if value_1 != 0 { value_1 } else { value_2 };

        match group.ability_id {
            1 => { // Weaken
                if stats.weaken_chance == 0 {
                    stats.weaken_chance = group.min_1 as i32; 
                    stats.weaken_duration = value_2;
                    stats.weaken_to = (100 - group.min_3) as i32; 
                } else {
                    stats.weaken_duration += duration_val;
                }
            },
            2 => { // Freeze
                if stats.freeze_chance == 0 {
                    stats.freeze_chance = group.min_1 as i32;
                    stats.freeze_duration = value_2;
                } else if group.text_id == 74 {
                    stats.freeze_chance += value_1;
                } else {
                    stats.freeze_duration += duration_val;
                }
            },
            3 => { // Slow
                if stats.slow_chance == 0 {
                    stats.slow_chance = group.min_1 as i32;
                    stats.slow_duration = value_2;
                } else if group.text_id == 63 {
                    stats.slow_chance += value_1;
                } else {
                    stats.slow_duration += duration_val;
                }
            },
            
            // The Sisters
            5 => stats.strong_against = 1, 
            6 => stats.resist = 1,         
            7 => stats.massive_damage = 1, 
            
            8 => stats.knockback_chance += value_1,
            10 => { // Strengthen
                if stats.strengthen_boost == 0 {
                    // Gain Strengthen
                    stats.strengthen_threshold = (100 - group.min_1) as i32;
                    stats.strengthen_boost = value_2;
                } else {
                    stats.strengthen_boost += if value_1 != 0 { value_1 } else { value_2 };
                }
            },
            11 => stats.survive += value_1,
            
            12 => stats.base_destroyer = 1,
            13 => stats.critical_chance += value_1,
            14 => stats.zombie_killer = 1,
            15 => stats.barrier_breaker_chance += value_1,
            16 => stats.double_bounty = 1,
            
            17 => { // Wave
                stats.wave_chance += value_1;
                stats.wave_level = group.min_2 as i32;
            },
            
            25 => stats.eoc1_cost -= value_1, 
            26 => stats.cooldown -= value_1,
            27 => stats.speed += value_1,
            31 => { // Attack Buff
                let factor = (100 + value_1) as f32 / 100.0;
                stats.attack_1 = (stats.attack_1 as f32 * factor) as i32;
                stats.attack_2 = (stats.attack_2 as f32 * factor) as i32;
                stats.attack_3 = (stats.attack_3 as f32 * factor) as i32;
            },
            32 => { // Health Buff
                let factor = (100 + value_1) as f32 / 100.0;
                stats.hitpoints = (stats.hitpoints as f32 * factor) as i32;
            },
            50 => { // Savage Blow
                stats.savage_blow_chance += value_1;
                stats.savage_blow_boost = group.min_2 as i32;
            },
            51 => { // Dodge
                stats.dodge_chance += value_1;
                stats.dodge_duration += value_2;
            },
            56 => { // Surge
                stats.surge_chance += value_1;
                stats.surge_level = group.min_2 as i32;
                stats.surge_spawn_anchor = group.min_3 as i32 / 4; 
                stats.surge_spawn_span = group.min_4 as i32 / 4;   
            },
            58 => stats.shield_pierce_chance += value_1,
            60 => { // Curse
                if stats.curse_chance == 0 {
                    stats.curse_chance += value_1;
                    stats.curse_duration += value_2;
                } else if group.text_id == 93 {
                    stats.curse_duration += duration_val;
                } else {
                    stats.curse_chance += value_1;
                    if value_2 > 0 { stats.curse_duration += value_2; }
                }
            },
            61 | 82 => { // Attack Freq Up
                let reduction = (stats.time_before_attack_1 as f32 * value_1 as f32 / 100.0).round() as i32;
                stats.time_before_attack_1 = stats.time_before_attack_1.saturating_sub(reduction);
            },
            62 => { // Mini-Wave
                stats.mini_wave_flag = 1;
                stats.wave_chance += value_1;
                stats.wave_level = group.min_2 as i32;
            },
            
            // Slayers
            63 => stats.colossus_slayer = 1,
            64 => { // Behemoth Slayer
                stats.behemoth_slayer = 1;
                let chance = if value_1 > 0 { value_1 } else { 5 };
                let duration = if value_2 > 0 { value_2 } else { 30 };
                stats.behemoth_dodge_chance = chance;
                stats.behemoth_dodge_duration = duration;
            },
            66 => stats.sage_slayer = 1,
            
            65 => { // Mini-Surge
                stats.mini_surge_flag = 1;
                stats.surge_chance += value_1;
                stats.surge_level = group.min_2 as i32;
                stats.surge_spawn_anchor = group.min_3 as i32 / 4; 
                stats.surge_spawn_span = group.min_4 as i32 / 4;
            },
            67 => { // Explosion
                stats.explosion_chance += value_1;
                stats.explosion_spawn_anchor = group.min_2 as i32 / 4; 
                stats.explosion_spawn_span = group.min_3 as i32 / 4;
            },
            
            // Immunities
            23 => stats.wave_immune = 1,
            29 => stats.curse_immune = 1, 
            44 => stats.weaken_immune = 1,
            45 => stats.freeze_immune = 1,
            46 => stats.slow_immune = 1,
            47 => stats.knockback_immune = 1,
            48 => stats.wave_immune = 1,
            49 => stats.warp_immune = 1,
            53 => stats.toxic_immune = 1,
            55 => stats.surge_immune = 1,
            57 => stats.target_aku = 1,
            
            // Target Traits
            33 => stats.target_red = 1,
            34 => stats.target_floating = 1,
            35 => stats.target_black = 1,
            36 => stats.target_metal = 1,
            37 => stats.target_angel = 1,
            38 => stats.target_alien = 1,
            39 => stats.target_zombie = 1,
            40 => stats.target_relic = 1,
            41 => stats.target_traitless = 1,
            _ => {}
        }
    }
    stats
}