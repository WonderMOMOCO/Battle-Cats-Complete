use crate::core::files::img015;
use crate::core::settings::Settings;
use super::stats::{self, CatRaw};
use eframe::egui;
use crate::core::files::skillacquisition::TalentRaw;
use std::collections::HashMap;

pub struct AbilityItem {
    pub icon_id: usize,
    pub text: String,
    pub custom_tex: Option<egui::TextureId>,
    pub border_id: Option<usize>,
}

pub fn collect_ability_data(
    cat_stats: &CatRaw,
    current_level: i32,
    level_curve: Option<&stats::CatLevelCurve>,
    multihit_texture: &Option<egui::TextureHandle>,
    kamikaze_texture: &Option<egui::TextureHandle>,
    boss_wave_texture: &Option<egui::TextureHandle>,
    settings: &Settings, 
    is_conjure_unit: bool,
    talent_data: Option<&TalentRaw>,
    talent_levels: Option<&HashMap<u8, u8>>
) -> (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>) {
    
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    let frames_to_seconds = |frames: i32| format!("{:.2}s^{}f", frames as f32 / 30.0, frames);

    // Helper to determine border based on talent level
    let get_talent_border = |ability_id: u8| -> Option<usize> {
        if ability_id == 0 { return None; }
        
        if let (Some(data), Some(levels)) = (talent_data, talent_levels) {
            // Inner function to check a specific ID
            let check_id = |target_id: u8| -> Option<usize> {
                if let Some((idx, group)) = data.groups.iter().enumerate().find(|(_, g)| g.ability_id == target_id) {
                    let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
                    if lv > 0 {
                        // Handle max_level 0 as 1 for state talents
                        let effective_max = if group.max_level == 0 { 1 } else { group.max_level };
                        return Some(if lv >= effective_max { img015::BORDER_GOLD } else { img015::BORDER_RED });
                    }
                }
                None
            };

            if let Some(border) = check_id(ability_id) {
                return Some(border);
            }

            // Wave Immune can be 23 or 48
            if ability_id == 23 {
                if let Some(border) = check_id(48) { return Some(border); }
            }
        }
        None
    };

    let push_ability = |target_list: &mut Vec<AbilityItem>, condition: bool, icon_id: usize, description: String, abil_id: u8| {
        if condition {
            let border_id = get_talent_border(abil_id);
            target_list.push(AbilityItem { icon_id, text: description, custom_tex: None, border_id });
        }
    };

    let push_custom = |target_list: &mut Vec<AbilityItem>, texture: &Option<egui::TextureHandle>, text: String| {
        if let Some(tex) = texture {
            target_list.push(AbilityItem { icon_id: 0, text, custom_tex: Some(tex.id()), border_id: None });
        }
    };

    let target_label = if is_conjure_unit { "Enemies" } else { "Target Traits" };

    // Row 1 Abilities
    push_ability(&mut group_headline_1, cat_stats.attack_only > 0, img015::ICON_ATTACK_ONLY, format!("Only damages {}", target_label), 4);
    push_ability(&mut group_headline_1, cat_stats.strong_against > 0, img015::ICON_STRONG_AGAINST, format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target_label), 5);
    push_ability(&mut group_headline_1, cat_stats.massive_damage > 0, img015::ICON_MASSIVE_DAMAGE, format!("Deals 3×~4× Damage to {}", target_label), 7);
    push_ability(&mut group_headline_1, cat_stats.insane_damage > 0, img015::ICON_INSANE_DAMAGE, format!("Deals 5×~6× Damage to {}", target_label), 7);
    push_ability(&mut group_headline_1, cat_stats.resist > 0, img015::ICON_RESIST, format!("Takes 1/4×~1/5× Damage from {}", target_label), 6);
    push_ability(&mut group_headline_1, cat_stats.insanely_tough > 0, img015::ICON_INSANELY_TOUGH, format!("Takes 1/6×~1/7× Damage from {}", target_label), 6);

    // Row 2 Abilities
    push_ability(&mut group_headline_2, cat_stats.metal > 0, img015::ICON_METAL, "Damage taken is reduced to 1 for Non-Critical attacks".into(), 0);
    push_ability(&mut group_headline_2, cat_stats.base_destroyer > 0, img015::ICON_BASE_DESTROYER, "Deals 4× Damage to the Enemy Base".into(), 12);
    push_ability(&mut group_headline_2, cat_stats.double_bounty > 0, img015::ICON_DOUBLE_BOUNTY, "Receives 2× Cash from Enemies".into(), 16);
    push_ability(&mut group_headline_2, cat_stats.zombie_killer > 0, img015::ICON_ZOMBIE_KILLER, "Prevents Zombies from reviving".into(), 14);
    push_ability(&mut group_headline_2, cat_stats.soulstrike > 0, img015::ICON_SOULSTRIKE, "Will attack Zombie corpses".into(), 59);
    push_ability(&mut group_headline_2, cat_stats.wave_block > 0, img015::ICON_WAVE_BLOCK, "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(), 0);
    push_ability(&mut group_headline_2, cat_stats.counter_surge > 0, img015::ICON_COUNTER_SURGE, "When hit with a Surge Attack, create a surge of equal level and range".into(), 0);
    push_ability(&mut group_headline_2, cat_stats.colossus_slayer > 0, img015::ICON_COLOSSUS_SLAYER, "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into(), 63);
    
    if cat_stats.behemoth_slayer > 0 {
        let mut slayer_text = "Deals 2.5× Damage to and takes 0.6× Damage from Behemoth Enemies".to_string();
        if cat_stats.behemoth_dodge_chance > 0 {
            slayer_text.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", cat_stats.behemoth_dodge_chance, frames_to_seconds(cat_stats.behemoth_dodge_duration)));
        }
        push_ability(&mut group_headline_2, true, img015::ICON_BEHEMOTH_SLAYER, slayer_text, 64);
    }

    push_ability(&mut group_headline_2, cat_stats.sage_slayer > 0, img015::ICON_SAGE_SLAYER, "Deals 1.2× Damage to and takes 0.5× Damage from Sage Enemies".into(), 66);
    push_ability(&mut group_headline_2, cat_stats.eva_killer > 0, img015::ICON_EVA_KILLER, "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into(), 0);
    push_ability(&mut group_headline_2, cat_stats.witch_killer > 0, img015::ICON_WITCH_KILLER, "Deals 5× Damage to and takes 0.1× Damage from Witches".into(), 0);

    if !is_conjure_unit && cat_stats.kamikaze > 0 {
        push_custom(&mut group_headline_2, kamikaze_texture, "Unit disappears after a single attack".to_string());
    }

    // Multihit
    let effective_multihit_texture = if settings.game_language == "--" {
        None
    } else {
        multihit_texture.as_ref().map(|t| t.id())
    };

    if cat_stats.attack_2 > 0 {
        let damage_hit_1 = level_curve.map_or(cat_stats.attack_1, |curve| curve.calculate_stat(cat_stats.attack_1, current_level));
        let damage_hit_2 = level_curve.map_or(cat_stats.attack_2, |curve| curve.calculate_stat(cat_stats.attack_2, current_level));
        let damage_hit_3 = level_curve.map_or(cat_stats.attack_3, |curve| curve.calculate_stat(cat_stats.attack_3, current_level));
        
        let ability_flag_1 = if cat_stats.attack_1_abilities > 0 { "True" } else { "False" };
        let ability_flag_2 = if cat_stats.attack_2_abilities > 0 { "True" } else { "False" };
        let ability_flag_3 = if cat_stats.attack_3 > 0 { if cat_stats.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
        
        let damage_string = if cat_stats.attack_3 > 0 { 
            format!("{} / {} / {}", damage_hit_1, damage_hit_2, damage_hit_3) 
        } else { 
            format!("{} / {}", damage_hit_1, damage_hit_2) 
        };
        
        let multihit_description = format!("Damage split {}\nAbility split {} / {}{}", damage_string, ability_flag_1, ability_flag_2, ability_flag_3);
        
        group_body_1.push(AbilityItem { 
            icon_id: img015::ICON_MULTIHIT,
            text: multihit_description, 
            custom_tex: effective_multihit_texture,
            border_id: None
        });
    }

    // Range
    let enemy_base_range = {
        let start_range = cat_stats.long_distance_1_anchor;
        let end_range = cat_stats.long_distance_1_anchor + cat_stats.long_distance_1_span;
        let (min_reach, max_reach) = if start_range < end_range { (start_range, end_range) } else { (end_range, start_range) };
        if min_reach > 0 { min_reach } else { max_reach }
    };

    let mut is_omni_strike = false;
    let mut has_long_distance = false;
    let mut range_strings = Vec::new();
    let range_checks = [
        (cat_stats.long_distance_1_anchor, cat_stats.long_distance_1_span),
        (cat_stats.long_distance_2_anchor, if cat_stats.long_distance_2_flag == 1 { cat_stats.long_distance_2_span } else { 0 }),
        (cat_stats.long_distance_3_anchor, if cat_stats.long_distance_3_flag == 1 { cat_stats.long_distance_3_span } else { 0 }),
    ];
    
    for (anchor, span) in range_checks {
        if span != 0 {
            let start = anchor;
            let end = anchor + span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            if min <= 0 { is_omni_strike = true; } else { has_long_distance = true; }
            range_strings.push(format!("{}~{}", min, max));
        }
    }

    if !range_strings.is_empty() {
        let range_description = format!(
            "Damage dealt between ranges {}\nStands at {} Range relative to Enemy Base", 
            range_strings.join(" / "), 
            enemy_base_range
        );
        push_ability(&mut group_body_1, is_omni_strike, img015::ICON_OMNI_STRIKE, range_description.clone(), 0);
        push_ability(&mut group_body_1, !is_omni_strike && has_long_distance, img015::ICON_LONG_DISTANCE, range_description, 0);
    }

    if !is_conjure_unit {
        push_ability(&mut group_body_1, cat_stats.conjure_unit_id > 0, img015::ICON_CONJURE, "Conjures a Spirit to the battlefield when tapped\nThis Cat may only be deployed one at a time".into(), 0);
    }

    // Effects
    let wave_type = if cat_stats.mini_wave_flag > 0 { "Mini-Wave" } else { "Wave" };
    let wave_icon = if cat_stats.mini_wave_flag > 0 { img015::ICON_MINI_WAVE } else { img015::ICON_WAVE };
    let wave_range = 332.5 + ((cat_stats.wave_level - 1) as f32 * 200.0);
    let wave_id = if cat_stats.mini_wave_flag > 0 { 62 } else { 17 };
    push_ability(&mut group_body_1, cat_stats.wave_chance > 0, wave_icon, format!("{}% Chance to create a Level {} {} reaching {} Range", cat_stats.wave_chance, cat_stats.wave_level, wave_type, wave_range), wave_id);

    let surge_type = if cat_stats.mini_surge_flag > 0 { "Mini-Surge" } else { "Surge" };
    let surge_icon = if cat_stats.mini_surge_flag > 0 { img015::ICON_MINI_SURGE } else { img015::ICON_SURGE };
    let surge_start = cat_stats.surge_spawn_anchor;
    let surge_end = cat_stats.surge_spawn_anchor + cat_stats.surge_spawn_span;
    let (surge_min, surge_max) = if surge_start < surge_end { (surge_start, surge_end) } else { (surge_end, surge_start) };
    let surge_position_text = if surge_min == surge_max { format!("at {}", surge_min) } else { format!("between {}~{}", surge_min, surge_max) };
    let surge_id = if cat_stats.mini_surge_flag > 0 { 65 } else { 56 };
    push_ability(&mut group_body_1, cat_stats.surge_chance > 0, surge_icon, format!("{}% Chance to create a Level {} {} {} Range", cat_stats.surge_chance, cat_stats.surge_level, surge_type, surge_position_text), surge_id);

    let explosion_start = cat_stats.explosion_spawn_anchor;
    let explosion_end = cat_stats.explosion_spawn_anchor + cat_stats.explosion_spawn_span;
    let (exp_min, exp_max) = if explosion_start < explosion_end { (explosion_start, explosion_end) } else { (explosion_end, explosion_start) };
    let explosion_position_text = if exp_min == exp_max { format!("at {}", exp_min) } else { format!("between {}~{}", exp_min, exp_max) };
    push_ability(&mut group_body_1, cat_stats.explosion_chance > 0, img015::ICON_EXPLOSION, format!("{}% Chance to create an Explosion {} Range", cat_stats.explosion_chance, explosion_position_text), 67);

    let savage_multiplier = (cat_stats.savage_blow_boost as f32 + 100.0) / 100.0;
    push_ability(&mut group_body_1, cat_stats.savage_blow_chance > 0, img015::ICON_SAVAGE_BLOW, format!("{}% Chance to perform a Savage Blow dealing {}× Damage", cat_stats.savage_blow_chance, savage_multiplier), 50);

    push_ability(&mut group_body_1, cat_stats.critical_chance > 0, img015::ICON_CRITICAL_HIT, format!("{}% Chance to perform a Critical Hit dealing 2× Damage while bypassing Metal resistance", cat_stats.critical_chance), 13);
    push_ability(&mut group_body_1, cat_stats.strengthen_threshold > 0, img015::ICON_STRENGTHEN, format!("At {}% HP, Damage dealt increases by +{}%", cat_stats.strengthen_threshold, cat_stats.strengthen_boost), 10);
    push_ability(&mut group_body_1, cat_stats.survive > 0, img015::ICON_SURVIVE, format!("{}% Chance to Survive a lethal strike", cat_stats.survive), 11);
    push_ability(&mut group_body_1, cat_stats.barrier_breaker_chance > 0, img015::ICON_BARRIER_BREAKER, format!("{}% Chance to break enemy Barriers", cat_stats.barrier_breaker_chance), 15);
    push_ability(&mut group_body_1, cat_stats.shield_pierce_chance > 0, img015::ICON_SHIELD_PIERCER, format!("{}% Chance to pierce enemy Shields", cat_stats.shield_pierce_chance), 58);
    push_ability(&mut group_body_1, cat_stats.metal_killer_percent > 0, img015::ICON_METAL_KILLER, format!("Deals {}% of a Metal Enemies current HP upon hit", cat_stats.metal_killer_percent), 0);

    if !is_conjure_unit { 
        push_ability(&mut group_body_2, cat_stats.dodge_chance > 0, img015::ICON_DODGE, format!("{}% Chance to Dodge {} for {}", cat_stats.dodge_chance, target_label, frames_to_seconds(cat_stats.dodge_duration)), 51); 
    }

    // CC
    push_ability(&mut group_body_2, cat_stats.weaken_chance > 0, img015::ICON_WEAKEN, format!("{}% Chance to weaken {} to {}% Attack Power for {}", cat_stats.weaken_chance, target_label, cat_stats.weaken_to, frames_to_seconds(cat_stats.weaken_duration)), 1);
    push_ability(&mut group_body_2, cat_stats.freeze_chance > 0, img015::ICON_FREEZE, format!("{}% Chance to Freeze {} for {}", cat_stats.freeze_chance, target_label, frames_to_seconds(cat_stats.freeze_duration)), 2);
    push_ability(&mut group_body_2, cat_stats.slow_chance > 0, img015::ICON_SLOW, format!("{}% Chance to Slow {} for {}", cat_stats.slow_chance, target_label, frames_to_seconds(cat_stats.slow_duration)), 3);
    push_ability(&mut group_body_2, cat_stats.knockback_chance > 0, img015::ICON_KNOCKBACK, format!("{}% Chance to Knockback {}", cat_stats.knockback_chance, target_label), 8);
    push_ability(&mut group_body_2, cat_stats.curse_chance > 0, img015::ICON_CURSE, format!("{}% Chance to Curse {} for {}", cat_stats.curse_chance, target_label, frames_to_seconds(cat_stats.curse_duration)), 60);
    push_ability(&mut group_body_2, cat_stats.warp_chance > 0, img015::ICON_WARP, format!("{}% Chance to Warp {} {}~{} Range for {}", cat_stats.warp_chance, target_label, cat_stats.warp_distance_minimum, cat_stats.warp_distance_maximum, frames_to_seconds(cat_stats.warp_duration)), 9);

    // Immunities
    let immunities = [
        (cat_stats.wave_immune > 0, img015::ICON_IMMUNE_WAVE, "Immune to Wave Attacks", 23),
        (cat_stats.surge_immune > 0, img015::ICON_IMMUNE_SURGE, "Immune to Surge Attacks", 55),
        (cat_stats.explosion_immune > 0, img015::ICON_IMMUNE_EXPLOSION, "Immune to Explosions", 0),
        (cat_stats.weaken_immune > 0, img015::ICON_IMMUNE_WEAKEN, "Immune to Weaken", 44),
        (cat_stats.freeze_immune > 0, img015::ICON_IMMUNE_FREEZE, "Immune to Freeze", 45),
        (cat_stats.slow_immune > 0, img015::ICON_IMMUNE_SLOW, "Immune to Slow", 46),
        (cat_stats.knockback_immune > 0, img015::ICON_IMMUNE_KNOCKBACK, "Immune to Knockback", 47),
        (cat_stats.curse_immune > 0, img015::ICON_IMMUNE_CURSE, "Immune to Curse", 29),
        (cat_stats.toxic_immune > 0, img015::ICON_IMMUNE_TOXIC, "Immune to Toxic", 53),
        (cat_stats.warp_immune > 0, img015::ICON_IMMUNE_WARP, "Immune to Warp", 49),
    ];
    for (has_immunity, icon, text_content, id) in immunities {
        push_ability(&mut group_footer, has_immunity, icon, text_content.into(), id);
    }

    if !is_conjure_unit && cat_stats.boss_wave_immune > 0 {
        let effective_tex = boss_wave_texture.as_ref().map(|t| t.id());
        group_footer.push(AbilityItem {
            icon_id: img015::ICON_IMMUNE_BOSS_WAVE,
            text: "Immune to Boss Shockwaves".into(),
            custom_tex: effective_tex,
            border_id: None,
        });
    }

    if let (Some(t_data), Some(levels)) = (talent_data, talent_levels) {
        let mut talent_headline = Vec::new();

        for (idx, group) in t_data.groups.iter().enumerate() {
            let lv = *levels.get(&(idx as u8)).unwrap_or(&0);
            if lv == 0 { continue; }

            let val = crate::core::cat::talents::calculate_talent_value(group.min_1, group.max_1, lv, group.max_level);
            let aid = group.ability_id;

            match group.ability_id {
                // Stat Modifiers
                25 => { // Cost Down
                    let effective = (val as f32 * 1.5).round() as i32;
                    push_ability(&mut talent_headline, true, img015::ICON_COST_DOWN, format!("Deploy Cost Down (-{}¢)", effective), aid);
                },
                26 => push_ability(&mut talent_headline, true, img015::ICON_RECOVER_SPEED_UP, format!("Recover Speed Up (-{}f)", val), aid),
                27 => push_ability(&mut talent_headline, true, img015::ICON_MOVE_SPEED, format!("Move Speed Up (+{})", val), aid),
                31 => push_ability(&mut talent_headline, true, img015::ICON_ATTACK_BUFF, format!("Attack Power Up (+{}%)", val), aid),
                32 => push_ability(&mut talent_headline, true, img015::ICON_HEALTH_BUFF, format!("Health Up (+{}%)", val), aid),
                61 | 82 => { // Attack Freq Up
                    let reduction = (cat_stats.time_before_attack_1 as f32 * val as f32 / 100.0).round() as i32;
                    push_ability(
                        &mut talent_headline, 
                        true, 
                        img015::ICON_TBA_DOWN, 
                        format!("TBA Down (-{}% | -{}f)", val, reduction), 
                        aid
                    );
                }
                
                // Resistances
                18 => push_ability(&mut group_footer, true, img015::ICON_RESIST_WEAKEN, format!("Resist Weaken ({}%)", val), aid),
                19 => push_ability(&mut group_footer, true, img015::ICON_RESIST_FREEZE, format!("Resist Freeze ({}%)", val), aid),
                20 => push_ability(&mut group_footer, true, img015::ICON_RESIST_SLOW, format!("Resist Slow ({}%)", val), aid),
                21 => push_ability(&mut group_footer, true, img015::ICON_RESIST_KNOCKBACK, format!("Resist Knockback ({}%)", val), aid),
                22 => push_ability(&mut group_footer, true, img015::ICON_RESIST_WAVE, format!("Resist Wave ({}%)", val), aid),
                24 => push_ability(&mut group_footer, true, img015::ICON_RESIST_WARP, format!("Resist Warp ({}%)", val), aid),
                30 => push_ability(&mut group_footer, true, img015::ICON_RESIST_CURSE, format!("Resist Curse ({}%)", val), aid),
                52 => push_ability(&mut group_footer, true, img015::ICON_RESIST_TOXIC, format!("Resist Toxic ({}%)", val), aid),
                54 => push_ability(&mut group_footer, true, img015::ICON_SURGE_RESIST, format!("Resist Surge ({}%)", val), aid),
                _ => {}
            }
        }

        let mut new_h2 = talent_headline;
        new_h2.append(&mut group_headline_2);
        group_headline_2 = new_h2;
    }

    (group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}