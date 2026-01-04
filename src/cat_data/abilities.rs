use crate::definitions;
use super::stats::{self, CatRaw};
use eframe::egui;

pub struct AbilityItem {
    pub icon_id: usize,
    pub text: String,
    pub custom_tex: Option<egui::TextureId>,
}

pub fn collect_ability_data(
    s: &CatRaw,
    level: i32,
    curve: Option<&stats::CatLevelCurve>,
    multihit_tex: &Option<egui::TextureHandle>,
    is_conjure: bool
) -> (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>) {
    
    let mut grp_headline_1 = Vec::new();
    let mut grp_headline_2 = Vec::new();
    let mut grp_body_1 = Vec::new();
    let mut grp_body_2 = Vec::new();
    let mut grp_footer = Vec::new();

    let f_to_s = |f: i32| format!("{:.2}s^{}f", f as f32 / 30.0, f);

    let push_ab = |vec: &mut Vec<AbilityItem>, cond: bool, icon: usize, txt: String| {
        if cond {
            vec.push(AbilityItem {
                icon_id: icon,
                text: txt,
                custom_tex: None,
            });
        }
    };


    let target_s = if is_conjure { "Enemies" } else { "Target Traits" };

    push_ab(&mut grp_headline_1, s.attack_only > 0, definitions::ICON_ATTACK_ONLY, format!("Only damages {}", target_s));
    push_ab(&mut grp_headline_1, s.strong_against > 0, definitions::ICON_STRONG_AGAINST, format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target_s));
    push_ab(&mut grp_headline_1, s.massive_damage > 0, definitions::ICON_MASSIVE_DAMAGE, format!("Deals 3×~4× Damage to {}", target_s));
    push_ab(&mut grp_headline_1, s.insane_damage > 0, definitions::ICON_INSANE_DAMAGE, format!("Deals 5×~6× Damage to {}", target_s));
    push_ab(&mut grp_headline_1, s.resist > 0, definitions::ICON_RESIST, format!("Takes 1/4×~1/5× Damage from {}", target_s));
    push_ab(&mut grp_headline_1, s.insanely_tough > 0, definitions::ICON_INSANELY_TOUGH, format!("Takes 1/6×~1/7× Damage from {}", target_s));

    push_ab(&mut grp_headline_2, s.metal > 0, definitions::ICON_METAL, "Damage taken is reduced to 1 for Non-Critical attacks".into());
    push_ab(&mut grp_headline_2, s.base_destroyer > 0, definitions::ICON_BASE_DESTROYER, "Deals 4× Damage to the Enemy Base".into());
    push_ab(&mut grp_headline_2, s.double_bounty > 0, definitions::ICON_DOUBLE_BOUNTY, "Receives 2× Cash from Enemies".into());
    push_ab(&mut grp_headline_2, s.zombie_killer > 0, definitions::ICON_ZOMBIE_KILLER, "Prevents Zombies from reviving".into());
    push_ab(&mut grp_headline_2, s.soulstrike > 0, definitions::ICON_SOULSTRIKE, "Will attack Zombie corpses".into());
    push_ab(&mut grp_headline_2, s.wave_block > 0, definitions::ICON_WAVE_BLOCK, "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into());
    push_ab(&mut grp_headline_2, s.counter_surge > 0, definitions::ICON_COUNTER_SURGE, "When hit with a Surge Attack, create a surge of equal level and range".into());
    push_ab(&mut grp_headline_2, s.colossus_slayer > 0, definitions::ICON_COLOSSUS_SLAYER, "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into());
    
    if s.behemoth_slayer > 0 {
        let mut text = "Deals 2.5× Damage to and takes 0.6× Damage from Behemoth Enemies".to_string();
        if s.behemoth_dodge_chance > 0 {
            text.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", s.behemoth_dodge_chance, f_to_s(s.behemoth_dodge_duration)));
        }
        push_ab(&mut grp_headline_2, true, definitions::ICON_BEHEMOTH_SLAYER, text);
    }

    push_ab(&mut grp_headline_2, s.sage_slayer > 0, definitions::ICON_SAGE_SLAYER, "Deals 1.2× Damage to and takes 0.5× Damage from Sage Enemies".into());
    push_ab(&mut grp_headline_2, s.eva_killer > 0, definitions::ICON_EVA_KILLER, "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into());
    push_ab(&mut grp_headline_2, s.witch_killer > 0, definitions::ICON_WITCH_KILLER, "Deals 5× Damage to and takes 0.1× Damage from Witches".into());

    if s.attack_2 > 0 {
        let a1 = curve.map_or(s.attack_1, |c| c.calculate_stat(s.attack_1, level));
        let a2 = curve.map_or(s.attack_2, |c| c.calculate_stat(s.attack_2, level));
        let a3 = curve.map_or(s.attack_3, |c| c.calculate_stat(s.attack_3, level));
        
        let ab1 = if s.attack_1_abilities > 0 { "True" } else { "False" };
        let ab2 = if s.attack_2_abilities > 0 { "True" } else { "False" };
        let ab3 = if s.attack_3 > 0 {
             if s.attack_3_abilities > 0 { "/True" } else { "/False" }
        } else { "" };
        let dmg_str = if s.attack_3 > 0 { format!("{}/{}/{}", a1, a2, a3) } else { format!("{}/{}", a1, a2) };

        let mh_desc = format!("Damage split {}\nAbility split {}/{}{}", dmg_str, ab1, ab2, ab3);
        
        if let Some(tex) = multihit_tex {
            grp_body_1.push(AbilityItem { icon_id: 0, text: mh_desc, custom_tex: Some(tex.id()) });
        }
    }

    let mut is_omni = false;
    let mut has_ld = false;
    let check_hits = [
        (s.long_distance_1_anchor, s.long_distance_1_span),
        (s.long_distance_2_anchor, if s.long_distance_2_flag == 1 { s.long_distance_2_span } else { 0 }),
        (s.long_distance_3_anchor, if s.long_distance_3_flag == 1 { s.long_distance_3_span } else { 0 }),
    ];
    let mut range_strs = Vec::new();
    for (anchor, span) in check_hits {
        if span != 0 {
            let start = anchor;
            let end = anchor + span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };

            if min <= 0 { is_omni = true; } else { has_ld = true; }
            range_strs.push(format!("{}~{}", min, max));
        }
    }
    let range_desc = format!("Damage dealt between ranges {}", range_strs.join(" / "));
    push_ab(&mut grp_body_1, is_omni, definitions::ICON_OMNI_STRIKE, range_desc.clone());
    push_ab(&mut grp_body_1, !is_omni && has_ld, definitions::ICON_LONG_DISTANCE, range_desc);

    if !is_conjure {
        push_ab(&mut grp_body_1, s.conjure_unit_id > 0, definitions::ICON_CONJURE, "Conjures a Spirit".into());
    }

    let wave_type = if s.mini_wave_flag > 0 { "Mini-Wave" } else { "Wave" };
    let wave_icon = if s.mini_wave_flag > 0 { definitions::ICON_MINI_WAVE } else { definitions::ICON_WAVE };
    let wave_range = 332.5 + ((s.wave_level - 1) as f32 * 200.0);
    push_ab(&mut grp_body_1, s.wave_chance > 0, wave_icon, format!("{}% Chance to create a Level {} {} reaching {} Range", s.wave_chance, s.wave_level, wave_type, wave_range));

    let surge_type = if s.mini_surge_flag > 0 { "Mini-Surge" } else { "Surge" };
    let surge_icon = if s.mini_surge_flag > 0 { definitions::ICON_MINI_SURGE } else { definitions::ICON_SURGE };
    let s_start = s.surge_spawn_anchor;
    let s_end = s.surge_spawn_anchor + s.surge_spawn_span;
    let (s_min, s_max) = if s_start < s_end { (s_start, s_end) } else { (s_end, s_start) };
    let s_pos = if s_min == s_max { format!("at {}", s_min) } else { format!("between {}~{}", s_min, s_max) };
    push_ab(&mut grp_body_1, s.surge_chance > 0, surge_icon, format!("{}% Chance to create a Level {} {} {} Range", s.surge_chance, s.surge_level, surge_type, s_pos));

    let e_start = s.explosion_spawn_anchor;
    let e_end = s.explosion_spawn_anchor + s.explosion_spawn_span;
    let (e_min, e_max) = if e_start < e_end { (e_start, e_end) } else { (e_end, e_start) };
    let e_pos = if e_min == e_max { format!("at {}", e_min) } else { format!("between {}~{}", e_min, e_max) };
    push_ab(&mut grp_body_1, s.explosion_chance > 0, definitions::ICON_EXPLOSION, format!("{}% Chance to create an Explosion {} Range", s.explosion_chance, e_pos));

    let savage_mult = (s.savage_blow_boost as f32 + 100.0) / 100.0;
    push_ab(&mut grp_body_1, s.savage_blow_chance > 0, definitions::ICON_SAVAGE_BLOW, format!("{}% Chance to perform a Savage Blow dealing {}× Damage", s.savage_blow_chance, savage_mult));

    push_ab(&mut grp_body_1, s.critical_chance > 0, definitions::ICON_CRITICAL_HIT, format!("{}% Chance to perform a Critical Hit dealing 2× Damage while bypassing Metal resistance", s.critical_chance));

    push_ab(&mut grp_body_1, s.strengthen_threshold > 0, definitions::ICON_STRENGTHEN, format!("At {}% HP, Damage dealt increases by +{}%", s.strengthen_threshold, s.strengthen_boost));
    
    push_ab(&mut grp_body_1, s.survive > 0, definitions::ICON_SURVIVE, format!("{}% Chance to Survive a lethal strike", s.survive));

    push_ab(&mut grp_body_1, s.barrier_breaker_chance > 0, definitions::ICON_BARRIER_BREAKER, format!("{}% Chance to break enemy Barriers", s.barrier_breaker_chance));
    push_ab(&mut grp_body_1, s.shield_pierce_chance > 0, definitions::ICON_SHIELD_PEIRCER, format!("{}% Chance to pierce enemy Shields", s.shield_pierce_chance));
    push_ab(&mut grp_body_1, s.metal_killer_percent > 0, definitions::ICON_METAL_KILLER, format!("Deals {}% of a Metal Enemies current HP upon hit", s.metal_killer_percent));

    if !is_conjure {
        push_ab(&mut grp_body_2, s.dodge_chance > 0, definitions::ICON_DODGE, format!("{}% Chance to Dodge {} for {}", s.dodge_chance, target_s, f_to_s(s.dodge_duration)));
    }
    
    push_ab(&mut grp_body_2, s.weaken_chance > 0, definitions::ICON_WEAKEN, format!("{}% Chance to weaken {} to {}% Attack Power for {}", s.weaken_chance, target_s, s.weaken_to, f_to_s(s.weaken_duration)));
    push_ab(&mut grp_body_2, s.freeze_chance > 0, definitions::ICON_FREEZE, format!("{}% Chance to Freeze {} for {}", s.freeze_chance, target_s, f_to_s(s.freeze_duration)));
    push_ab(&mut grp_body_2, s.slow_chance > 0, definitions::ICON_SLOW, format!("{}% Chance to Slow {} for {}", s.slow_chance, target_s, f_to_s(s.slow_duration)));
    push_ab(&mut grp_body_2, s.knockback_chance > 0, definitions::ICON_KNOCKBACK, format!("{}% Chance to Knockback {}", s.knockback_chance, target_s));
    push_ab(&mut grp_body_2, s.curse_chance > 0, definitions::ICON_CURSE, format!("{}% Chance to Curse {} for {}", s.curse_chance, target_s, f_to_s(s.curse_duration)));
    push_ab(&mut grp_body_2, s.warp_chance > 0, definitions::ICON_WARP, format!("{}% Chance to Warp {} for {} range {}~{}", s.warp_chance, target_s, f_to_s(s.warp_duration), s.warp_distance_minimum, s.warp_distance_maximum));

    let immunities = [
        (s.wave_immune > 0, definitions::ICON_IMMUNE_WAVE, "Immune to Wave Attacks"),
        (s.surge_immune > 0, definitions::ICON_IMMUNE_SURGE, "Immune to Surge Attacks"),
        (s.explosion_immune > 0, definitions::ICON_IMMUNE_EXPLOSION, "Immune to Explosions"),
        (s.weaken_immune > 0, definitions::ICON_IMMUNE_WEAKEN, "Immune to Weaken"),
        (s.freeze_immune > 0, definitions::ICON_IMMUNE_FREEZE, "Immune to Freeze"),
        (s.slow_immune > 0, definitions::ICON_IMMUNE_SLOW, "Immune to Slow"),
        (s.knockback_immune > 0, definitions::ICON_IMMUNE_KNOCKBACK, "Immune to Knockback"),
        (s.curse_immune > 0, definitions::ICON_IMMUNE_CURSE, "Immune to Curse"),
        (s.toxic_immune > 0, definitions::ICON_IMMUNE_TOXIC, "Immune to Toxic"),
        (s.warp_immune > 0, definitions::ICON_IMMUNE_WARP, "Immune to Warp"),
    ];
    for (has, icon, txt) in immunities {
        push_ab(&mut grp_footer, has, icon, txt.into());
    }

    (grp_headline_1, grp_headline_2, grp_body_1, grp_body_2, grp_footer)
}