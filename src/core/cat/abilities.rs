use crate::core::files::img015;
use crate::core::settings::Settings;
use super::stats::{self, CatRaw};
use eframe::egui;

pub struct AbilityItem {
    pub icon_id: usize,
    pub text: String,
    pub custom_tex: Option<egui::TextureId>,
}

pub fn collect_ability_data(
    cat_stats: &CatRaw,
    current_level: i32,
    level_curve: Option<&stats::CatLevelCurve>,
    multihit_texture: &Option<egui::TextureHandle>,
    settings: &Settings, 
    is_conjure_unit: bool
) -> (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>) {
    
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    let frames_to_seconds = |frames: i32| format!("{:.2}s^{}f", frames as f32 / 30.0, frames);

    let push_ability = |target_list: &mut Vec<AbilityItem>, condition: bool, icon_id: usize, description: String| {
        if condition {
            target_list.push(AbilityItem { icon_id, text: description, custom_tex: None });
        }
    };

    let target_label = if is_conjure_unit { "Enemies" } else { "Target Traits" };

    // Row 1 Abilities
    push_ability(&mut group_headline_1, cat_stats.attack_only > 0, img015::ICON_ATTACK_ONLY, format!("Only damages {}", target_label));
    push_ability(&mut group_headline_1, cat_stats.strong_against > 0, img015::ICON_STRONG_AGAINST, format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target_label));
    push_ability(&mut group_headline_1, cat_stats.massive_damage > 0, img015::ICON_MASSIVE_DAMAGE, format!("Deals 3×~4× Damage to {}", target_label));
    push_ability(&mut group_headline_1, cat_stats.insane_damage > 0, img015::ICON_INSANE_DAMAGE, format!("Deals 5×~6× Damage to {}", target_label));
    push_ability(&mut group_headline_1, cat_stats.resist > 0, img015::ICON_RESIST, format!("Takes 1/4×~1/5× Damage from {}", target_label));
    push_ability(&mut group_headline_1, cat_stats.insanely_tough > 0, img015::ICON_INSANELY_TOUGH, format!("Takes 1/6×~1/7× Damage from {}", target_label));

    // Row 2 Abilities
    push_ability(&mut group_headline_2, cat_stats.metal > 0, img015::ICON_METAL, "Damage taken is reduced to 1 for Non-Critical attacks".into());
    push_ability(&mut group_headline_2, cat_stats.base_destroyer > 0, img015::ICON_BASE_DESTROYER, "Deals 4× Damage to the Enemy Base".into());
    push_ability(&mut group_headline_2, cat_stats.double_bounty > 0, img015::ICON_DOUBLE_BOUNTY, "Receives 2× Cash from Enemies".into());
    push_ability(&mut group_headline_2, cat_stats.zombie_killer > 0, img015::ICON_ZOMBIE_KILLER, "Prevents Zombies from reviving".into());
    push_ability(&mut group_headline_2, cat_stats.soulstrike > 0, img015::ICON_SOULSTRIKE, "Will attack Zombie corpses".into());
    push_ability(&mut group_headline_2, cat_stats.wave_block > 0, img015::ICON_WAVE_BLOCK, "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into());
    push_ability(&mut group_headline_2, cat_stats.counter_surge > 0, img015::ICON_COUNTER_SURGE, "When hit with a Surge Attack, create a surge of equal level and range".into());
    push_ability(&mut group_headline_2, cat_stats.colossus_slayer > 0, img015::ICON_COLOSSUS_SLAYER, "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into());
    
    if cat_stats.behemoth_slayer > 0 {
        let mut slayer_text = "Deals 2.5× Damage to and takes 0.6× Damage from Behemoth Enemies".to_string();
        if cat_stats.behemoth_dodge_chance > 0 {
            slayer_text.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", cat_stats.behemoth_dodge_chance, frames_to_seconds(cat_stats.behemoth_dodge_duration)));
        }
        push_ability(&mut group_headline_2, true, img015::ICON_BEHEMOTH_SLAYER, slayer_text);
    }

    push_ability(&mut group_headline_2, cat_stats.sage_slayer > 0, img015::ICON_SAGE_SLAYER, "Deals 1.2× Damage to and takes 0.5× Damage from Sage Enemies".into());
    push_ability(&mut group_headline_2, cat_stats.eva_killer > 0, img015::ICON_EVA_KILLER, "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into());
    push_ability(&mut group_headline_2, cat_stats.witch_killer > 0, img015::ICON_WITCH_KILLER, "Deals 5× Damage to and takes 0.1× Damage from Witches".into());

    // Multihit
    let effective_multihit_texture = if settings.game_language == "--" {
        None
    } else {
        multihit_texture.as_ref().map(|t| t.id())
    };

    if cat_stats.attack_2 > 0 {
        let damage_hit_1 = level_curve.map_or(cat_stats.attack_1, |c| c.calculate_stat(cat_stats.attack_1, current_level));
        let damage_hit_2 = level_curve.map_or(cat_stats.attack_2, |c| c.calculate_stat(cat_stats.attack_2, current_level));
        let damage_hit_3 = level_curve.map_or(cat_stats.attack_3, |c| c.calculate_stat(cat_stats.attack_3, current_level));
        
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
        push_ability(&mut group_body_1, is_omni_strike, img015::ICON_OMNI_STRIKE, range_description.clone());
        push_ability(&mut group_body_1, !is_omni_strike && has_long_distance, img015::ICON_LONG_DISTANCE, range_description);
    }

    if !is_conjure_unit {
        push_ability(&mut group_body_1, cat_stats.conjure_unit_id > 0, img015::ICON_CONJURE, "Conjures a Spirit to the battlefield when tapped\nThis Cat may only be deployed one at a time".into());
    }

    // Effects with % chance
    let wave_type = if cat_stats.mini_wave_flag > 0 { "Mini-Wave" } else { "Wave" };
    let wave_icon = if cat_stats.mini_wave_flag > 0 { img015::ICON_MINI_WAVE } else { img015::ICON_WAVE };
    let wave_range = 332.5 + ((cat_stats.wave_level - 1) as f32 * 200.0);
    push_ability(&mut group_body_1, cat_stats.wave_chance > 0, wave_icon, format!("{}% Chance to create a Level {} {} reaching {} Range", cat_stats.wave_chance, cat_stats.wave_level, wave_type, wave_range));

    let surge_type = if cat_stats.mini_surge_flag > 0 { "Mini-Surge" } else { "Surge" };
    let surge_icon = if cat_stats.mini_surge_flag > 0 { img015::ICON_MINI_SURGE } else { img015::ICON_SURGE };
    let surge_start = cat_stats.surge_spawn_anchor;
    let surge_end = cat_stats.surge_spawn_anchor + cat_stats.surge_spawn_span;
    let (surge_min, surge_max) = if surge_start < surge_end { (surge_start, surge_end) } else { (surge_end, surge_start) };
    let surge_position_text = if surge_min == surge_max { format!("at {}", surge_min) } else { format!("between {}~{}", surge_min, surge_max) };
    push_ability(&mut group_body_1, cat_stats.surge_chance > 0, surge_icon, format!("{}% Chance to create a Level {} {} {} Range", cat_stats.surge_chance, cat_stats.surge_level, surge_type, surge_position_text));

    let explosion_start = cat_stats.explosion_spawn_anchor;
    let explosion_end = cat_stats.explosion_spawn_anchor + cat_stats.explosion_spawn_span;
    let (exp_min, exp_max) = if explosion_start < explosion_end { (explosion_start, explosion_end) } else { (explosion_end, explosion_start) };
    let explosion_position_text = if exp_min == exp_max { format!("at {}", exp_min) } else { format!("between {}~{}", exp_min, exp_max) };
    push_ability(&mut group_body_1, cat_stats.explosion_chance > 0, img015::ICON_EXPLOSION, format!("{}% Chance to create an Explosion {} Range", cat_stats.explosion_chance, explosion_position_text));

    let savage_multiplier = (cat_stats.savage_blow_boost as f32 + 100.0) / 100.0;
    push_ability(&mut group_body_1, cat_stats.savage_blow_chance > 0, img015::ICON_SAVAGE_BLOW, format!("{}% Chance to perform a Savage Blow dealing {}× Damage", cat_stats.savage_blow_chance, savage_multiplier));

    push_ability(&mut group_body_1, cat_stats.critical_chance > 0, img015::ICON_CRITICAL_HIT, format!("{}% Chance to perform a Critical Hit dealing 2× Damage while bypassing Metal resistance", cat_stats.critical_chance));
    push_ability(&mut group_body_1, cat_stats.strengthen_threshold > 0, img015::ICON_STRENGTHEN, format!("At {}% HP, Damage dealt increases by +{}%", cat_stats.strengthen_threshold, cat_stats.strengthen_boost));
    push_ability(&mut group_body_1, cat_stats.survive > 0, img015::ICON_SURVIVE, format!("{}% Chance to Survive a lethal strike", cat_stats.survive));
    push_ability(&mut group_body_1, cat_stats.barrier_breaker_chance > 0, img015::ICON_BARRIER_BREAKER, format!("{}% Chance to break enemy Barriers", cat_stats.barrier_breaker_chance));
    push_ability(&mut group_body_1, cat_stats.shield_pierce_chance > 0, img015::ICON_SHIELD_PIERCER, format!("{}% Chance to pierce enemy Shields", cat_stats.shield_pierce_chance));
    push_ability(&mut group_body_1, cat_stats.metal_killer_percent > 0, img015::ICON_METAL_KILLER, format!("Deals {}% of a Metal Enemies current HP upon hit", cat_stats.metal_killer_percent));

    if !is_conjure_unit { 
        push_ability(&mut group_body_2, cat_stats.dodge_chance > 0, img015::ICON_DODGE, format!("{}% Chance to Dodge {} for {}", cat_stats.dodge_chance, target_label, frames_to_seconds(cat_stats.dodge_duration))); 
    }

    // Crowd Control
    push_ability(&mut group_body_2, cat_stats.weaken_chance > 0, img015::ICON_WEAKEN, format!("{}% Chance to weaken {} to {}% Attack Power for {}", cat_stats.weaken_chance, target_label, cat_stats.weaken_to, frames_to_seconds(cat_stats.weaken_duration)));
    push_ability(&mut group_body_2, cat_stats.freeze_chance > 0, img015::ICON_FREEZE, format!("{}% Chance to Freeze {} for {}", cat_stats.freeze_chance, target_label, frames_to_seconds(cat_stats.freeze_duration)));
    push_ability(&mut group_body_2, cat_stats.slow_chance > 0, img015::ICON_SLOW, format!("{}% Chance to Slow {} for {}", cat_stats.slow_chance, target_label, frames_to_seconds(cat_stats.slow_duration)));
    push_ability(&mut group_body_2, cat_stats.knockback_chance > 0, img015::ICON_KNOCKBACK, format!("{}% Chance to Knockback {}", cat_stats.knockback_chance, target_label));
    push_ability(&mut group_body_2, cat_stats.curse_chance > 0, img015::ICON_CURSE, format!("{}% Chance to Curse {} for {}", cat_stats.curse_chance, target_label, frames_to_seconds(cat_stats.curse_duration)));
    push_ability(&mut group_body_2, cat_stats.warp_chance > 0, img015::ICON_WARP, format!("{}% Chance to Warp {} for {} range {}~{}", cat_stats.warp_chance, target_label, frames_to_seconds(cat_stats.warp_duration), cat_stats.warp_distance_minimum, cat_stats.warp_distance_maximum));

    // Immunities
    let immunities = [
        (cat_stats.wave_immune > 0, img015::ICON_IMMUNE_WAVE, "Immune to Wave Attacks"),
        (cat_stats.surge_immune > 0, img015::ICON_IMMUNE_SURGE, "Immune to Surge Attacks"),
        (cat_stats.explosion_immune > 0, img015::ICON_IMMUNE_EXPLOSION, "Immune to Explosions"),
        (cat_stats.weaken_immune > 0, img015::ICON_IMMUNE_WEAKEN, "Immune to Weaken"),
        (cat_stats.freeze_immune > 0, img015::ICON_IMMUNE_FREEZE, "Immune to Freeze"),
        (cat_stats.slow_immune > 0, img015::ICON_IMMUNE_SLOW, "Immune to Slow"),
        (cat_stats.knockback_immune > 0, img015::ICON_IMMUNE_KNOCKBACK, "Immune to Knockback"),
        (cat_stats.curse_immune > 0, img015::ICON_IMMUNE_CURSE, "Immune to Curse"),
        (cat_stats.toxic_immune > 0, img015::ICON_IMMUNE_TOXIC, "Immune to Toxic"),
        (cat_stats.warp_immune > 0, img015::ICON_IMMUNE_WARP, "Immune to Warp"),
    ];
    for (has_immunity, icon, text_content) in immunities {
        push_ability(&mut group_footer, has_immunity, icon, text_content.into());
    }

    (group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}