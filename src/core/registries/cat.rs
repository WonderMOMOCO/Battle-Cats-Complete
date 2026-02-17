use crate::data::global::img015;
use crate::data::cat::unitid::CatRaw;
use crate::data::cat::unitlevel::CatLevelCurve;
use crate::data::cat::skillacquisition::TalentGroupRaw;

#[derive(PartialEq, Clone, Copy)]
pub enum DisplayGroup {
    Trait,     
    Headline1, 
    Headline2, 
    Body1,     
    Body2,     
    Footer,    
}

pub struct AbilityDef {
    pub name: &'static str,
    pub icon_id: usize,
    pub talent_id: u8, 
    pub group: DisplayGroup,
    pub getter: fn(&CatRaw) -> i32,
    pub duration_getter: Option<fn(&CatRaw) -> i32>,
    pub formatter: fn(val: i32, stats: &CatRaw, target: &str, duration_frames: i32) -> String,
    pub apply_func: Option<fn(&mut CatRaw, val1: i32, val2: i32, group: &TalentGroupRaw)>,
    pub talent_desc_func: Option<fn(val1: i32, val2: i32, stats: &CatRaw, curve: Option<&CatLevelCurve>, unit_level: i32, group: &TalentGroupRaw, talent_level: u8) -> String>,
}

// Formatters
fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_f(frames: i32) -> String {
    format!("{}f", frames)
}

fn fmt_range(min: i32, max: i32) -> String {
    if min == max { format!("at {}", min) } else { format!("between {}~{}", min, max) }
}

fn fmt_additive(base: i32, bonus: i32, unit: &str) -> String {
    format!("{}{unit} (+{bonus}{unit}) -> {}{unit}", base, base + bonus)
}

fn fmt_additive_f(base: i32, bonus: i32) -> String {
    format!("{} (+{}) -> {}", fmt_f(base), fmt_f(bonus), fmt_f(base + bonus))
}

fn fmt_multi_stat(base: i32, pct: i32) -> String {
    let new_total = (base as f32 * (1.0 + pct as f32 / 100.0)).round() as i32;
    format!("{} (+{}%) -> {}", base, pct, new_total)
}

fn fmt_state(level: u8) -> String {
    if level > 0 { "Inactive -> Active".into() } else { "Inactive -> Inactive".into() }
}

fn resolve_stat(val: i32, min_raw: u16, max_raw: u16) -> i32 {
    if val == 0 && min_raw == max_raw { min_raw as i32 } else { val }
}

fn get_dur_val(v1: i32, v2: i32) -> i32 {
    if v1 != 0 { v1 } else { v2 }
}

pub const ABILITY_REGISTRY: &[AbilityDef] = &[
    // --- TRAITS ---
    AbilityDef {
        name: "Target Red",
        icon_id: img015::ICON_TRAIT_RED,
        talent_id: 33,
        group: DisplayGroup::Trait,
        getter: |c| c.target_red,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Red Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_red = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Float",
        icon_id: img015::ICON_TRAIT_FLOATING,
        talent_id: 34,
        group: DisplayGroup::Trait,
        getter: |c| c.target_floating,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Floating Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_floating = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Black",
        icon_id: img015::ICON_TRAIT_BLACK,
        talent_id: 35,
        group: DisplayGroup::Trait,
        getter: |c| c.target_black,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Black Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_black = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Metal",
        icon_id: img015::ICON_TRAIT_METAL,
        talent_id: 36,
        group: DisplayGroup::Trait,
        getter: |c| c.target_metal,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Metal Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_metal = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Angel",
        icon_id: img015::ICON_TRAIT_ANGEL,
        talent_id: 37,
        group: DisplayGroup::Trait,
        getter: |c| c.target_angel,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Angel Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_angel = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Alien",
        icon_id: img015::ICON_TRAIT_ALIEN,
        talent_id: 38,
        group: DisplayGroup::Trait,
        getter: |c| c.target_alien,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Alien Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_alien = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Zombie",
        icon_id: img015::ICON_TRAIT_ZOMBIE,
        talent_id: 39,
        group: DisplayGroup::Trait,
        getter: |c| c.target_zombie,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Zombie Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_zombie = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Relic",
        icon_id: img015::ICON_TRAIT_RELIC,
        talent_id: 40,
        group: DisplayGroup::Trait,
        getter: |c| c.target_relic,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Relic Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_relic = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target Aku",
        icon_id: img015::ICON_TRAIT_AKU,
        talent_id: 57,
        group: DisplayGroup::Trait,
        getter: |c| c.target_aku,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Aku Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_aku = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Target White",
        icon_id: img015::ICON_TRAIT_TRAITLESS,
        talent_id: 41,
        group: DisplayGroup::Trait,
        getter: |c| c.target_traitless,
        duration_getter: None,
        formatter: |_,_,_,_| "Targets Traitless Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_traitless = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },

    // --- HEADLINE 1 ---
    AbilityDef {
        name: "Attack Only",
        icon_id: img015::ICON_ATTACK_ONLY,
        talent_id: 4,
        group: DisplayGroup::Headline1,
        getter: |c| c.attack_only,
        duration_getter: None,
        formatter: |_, _, target, _| format!("Only damages {}", target),
        apply_func: Some(|c, _, _, _| c.attack_only = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Strong Against",
        icon_id: img015::ICON_STRONG_AGAINST,
        talent_id: 5,
        group: DisplayGroup::Headline1,
        getter: |c| c.strong_against,
        duration_getter: None,
        formatter: |_, _, target, _| format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target),
        apply_func: Some(|c, _, _, _| c.strong_against = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Massive Damage",
        icon_id: img015::ICON_MASSIVE_DAMAGE,
        talent_id: 7,
        group: DisplayGroup::Headline1,
        getter: |c| c.massive_damage,
        duration_getter: None,
        formatter: |_, _, target, _| format!("Deals 3×~4× Damage to {}", target),
        apply_func: Some(|c, _, _, _| c.massive_damage = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Insane Damage",
        icon_id: img015::ICON_INSANE_DAMAGE,
        talent_id: 7,
        group: DisplayGroup::Headline1,
        getter: |c| c.insane_damage,
        duration_getter: None,
        formatter: |_, _, target, _| format!("Deals 5×~6× Damage to {}", target),
        apply_func: None,
        talent_desc_func: None
    },
    AbilityDef {
        name: "Resist",
        icon_id: img015::ICON_RESIST,
        talent_id: 6,
        group: DisplayGroup::Headline1,
        getter: |c| c.resist,
        duration_getter: None,
        formatter: |_, _, target, _| format!("Takes 1/4×~1/5× Damage from {}", target),
        apply_func: Some(|c, _, _, _| c.resist = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Insanely Tough",
        icon_id: img015::ICON_INSANELY_TOUGH,
        talent_id: 6,
        group: DisplayGroup::Headline1,
        getter: |c| c.insanely_tough,
        duration_getter: None,
        formatter: |_, _, target, _| format!("Takes 1/6×~1/7× Damage from {}", target),
        apply_func: None,
        talent_desc_func: None
    },

    // --- HEADLINE 2 ---
    AbilityDef {
        name: "Metal",
        icon_id: img015::ICON_METAL,
        talent_id: 43,
        group: DisplayGroup::Headline2,
        getter: |c| c.metal,
        duration_getter: None,
        formatter: |_, _, _, _| "Damage taken is reduced to 1 for Non-Critical attacks".into(),
        apply_func: Some(|c,_,_,_| c.metal = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Base Destroyer",
        icon_id: img015::ICON_BASE_DESTROYER,
        talent_id: 12,
        group: DisplayGroup::Headline2,
        getter: |c| c.base_destroyer,
        duration_getter: None,
        formatter: |_,_,_,_| "Deals 4× Damage to the Enemy Base".into(),
        apply_func: Some(|c, _, _, _| c.base_destroyer = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Double Bounty",
        icon_id: img015::ICON_DOUBLE_BOUNTY,
        talent_id: 16,
        group: DisplayGroup::Headline2,
        getter: |c| c.double_bounty,
        duration_getter: None,
        formatter: |_,_,_,_| "Receives 2× Cash from Enemies".into(),
        apply_func: Some(|c, _, _, _| c.double_bounty = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Zombie Killer",
        icon_id: img015::ICON_ZOMBIE_KILLER,
        talent_id: 14,
        group: DisplayGroup::Headline2,
        getter: |c| c.zombie_killer,
        duration_getter: None,
        formatter: |_, _, _, _| "Prevents Zombies from reviving".into(),
        apply_func: Some(|c, _, _, _| c.zombie_killer = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Soulstrike",
        icon_id: img015::ICON_SOULSTRIKE,
        talent_id: 59,
        group: DisplayGroup::Headline2,
        getter: |c| {
            if c.soulstrike == 2 {
                // If its a talent, bypass z-kill
                1
            } else if c.soulstrike > 0 && c.zombie_killer > 0 {
                // if its native, check for z-kill
                1
            } else {
                // if its native with no z-kill, hide
                0
            }
        },
        duration_getter: None,
        formatter: |_, _, _, _| "Will attack Zombie corpses".into(),
        apply_func: Some(|c, _, _, _| c.soulstrike = 2),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Colossus Slayer",
        icon_id: img015::ICON_COLOSSUS_SLAYER,
        talent_id: 63,
        group: DisplayGroup::Headline2,
        getter: |c| c.colossus_slayer,
        duration_getter: None,
        formatter: |_, _, _, _| "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into(),
        apply_func: Some(|c, _, _, _| c.colossus_slayer = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Sage Slayer",
        icon_id: img015::ICON_SAGE_SLAYER,
        talent_id: 66,
        group: DisplayGroup::Headline2,
        getter: |c| c.sage_slayer,
        duration_getter: None,
        formatter: |_, _, _, _| "Deals 1.2× Damage to and takes 0.5× Damage from Sage Enemies".into(),
        apply_func: Some(|c, _, _, _| c.sage_slayer = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Behemoth Slayer",
        icon_id: img015::ICON_BEHEMOTH_SLAYER,
        talent_id: 64,
        group: DisplayGroup::Headline2,
        getter: |c| c.behemoth_slayer,
        duration_getter: None,
        formatter: |_, c, _, _| {
            let mut txt = "Deals 2.5× Damage to and takes 0.6× Damage from Behemoth Enemies".to_string();
            if c.behemoth_dodge_chance > 0 {
                txt.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", c.behemoth_dodge_chance, fmt_time(c.behemoth_dodge_duration)));
            }
            txt
        },
        apply_func: Some(|c, v1, v2, _| {
            c.behemoth_slayer = 1;
            c.behemoth_dodge_chance = if v1 > 0 { v1 } else { 5 };
            c.behemoth_dodge_duration = if v2 > 0 { v2 } else { 30 };
        }),
        talent_desc_func: Some(|v1, v2, _, _, _, _, l| {
             let chance = if v1 > 0 { v1 } else { 5 };
             let duration = if v2 > 0 { v2 } else { 30 };
             format!("{}\nDodge Chance: {}%\nDodge Duration: {}f", fmt_state(l), chance, duration)
        }),
    },
    AbilityDef {
        name: "Eva Killer",
        icon_id: img015::ICON_EVA_KILLER,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        getter: |c| c.eva_killer,
        duration_getter: None,
        formatter: |_,_,_,_| "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into(),
        apply_func: Some(|c,_,_,_| c.eva_killer = 1),
        talent_desc_func: None
    },
    AbilityDef {
        name: "Witch Killer",
        icon_id: img015::ICON_WITCH_KILLER,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        getter: |c| c.witch_killer,
        duration_getter: None,
        formatter: |_,_,_,_| "Deals 5× Damage to and takes 0.1× Damage from Witches".into(),
        apply_func: Some(|c,_,_,_| c.witch_killer = 1),
        talent_desc_func: None
    },
    AbilityDef {
        name: "Wave Block",
        icon_id: img015::ICON_WAVE_BLOCK,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        getter: |c| c.wave_block,
        duration_getter: None,
        formatter: |_, _, _, _| "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(),
        apply_func: Some(|c, _, _, _| c.wave_block = 1),
        talent_desc_func: Some(|_, _, _, _, _, _, l| fmt_state(l))
    },
    AbilityDef {
        name: "Counter Surge",
        icon_id: img015::ICON_COUNTER_SURGE,
        talent_id: 68,
        group: DisplayGroup::Headline2,
        getter: |c| c.counter_surge,
        duration_getter: None,
        formatter: |_,_,_,_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        apply_func: Some(|c,_,_,_| c.counter_surge = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },

    // --- BODY 1 ---
    AbilityDef {
        name: "Metal Killer",
        icon_id: img015::ICON_METAL_KILLER,
        talent_id: 0,
        group: DisplayGroup::Body1,
        getter: |c| c.metal_killer_percent,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Reduces Metal enemies current HP by {}% upon hit", val),
        apply_func: Some(|c,v1,_,_| c.metal_killer_percent = v1),
        talent_desc_func: None
    },
    AbilityDef {
        name: "Wave Attack",
        icon_id: img015::ICON_WAVE,
        talent_id: 17,
        group: DisplayGroup::Body1,
        getter: |c| if c.mini_wave_flag == 0 { c.wave_chance } else { 0 },
        duration_getter: None,
        formatter: |val, c, _, _| {
            let range = 332.5 + ((c.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Wave reaching {} Range", val, c.wave_level, range)
        },
        apply_func: Some(|c, v1, v2, _| { c.wave_chance += v1; c.wave_level = v2; }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _| {
            let level = resolve_stat(v2, g.min_2, g.max_2);
            let range = 332.5 + ((level - 1) as f32 * 200.0);
            let range_str = format!("{}", range);
            if c.wave_chance == 0 {
                format!("Chance: {}\nLevel: {}\nRange: {}", fmt_additive(0, v1, "%"), level, range_str)
            } else {
                format!("Chance: {}\nLevel: {}\nRange: {}", fmt_additive(c.wave_chance, v1, "%"), level, range_str)
            }
        }),
    },
    AbilityDef {
        name: "Mini-Wave",
        icon_id: img015::ICON_MINI_WAVE,
        talent_id: 62,
        group: DisplayGroup::Body1,
        getter: |c| if c.mini_wave_flag > 0 { c.wave_chance } else { 0 },
        duration_getter: None,
        formatter: |val, c, _, _| {
             let range = 332.5 + ((c.wave_level - 1) as f32 * 200.0);
             format!("{}% Chance to create a Level {} Mini-Wave reaching {} Range", val, c.wave_level, range)
        },
        apply_func: Some(|c, v1, v2, _| { c.mini_wave_flag = 1; c.wave_chance += v1; c.wave_level = v2; }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _| {
            let level = resolve_stat(v2, g.min_2, g.max_2);
            let range = 332.5 + ((level - 1) as f32 * 200.0);
            let range_str = format!("{}", range);
            if c.wave_chance == 0 {
                format!("Chance: {}\nLevel: {} (Mini)\nRange: {}", fmt_additive(0, v1, "%"), level, range_str)
            } else {
                format!("Chance: {}\nLevel: {} (Mini)\nRange: {}", fmt_additive(c.wave_chance, v1, "%"), level, range_str)
            }
        }),
    },
    AbilityDef {
        name: "Surge Attack",
        icon_id: img015::ICON_SURGE,
        talent_id: 56,
        group: DisplayGroup::Body1,
        getter: |c| if c.mini_surge_flag == 0 { c.surge_chance } else { 0 },
        duration_getter: None,
        formatter: |val, c, _, _| {
            let start = c.surge_spawn_anchor;
            let end = c.surge_spawn_anchor + c.surge_spawn_span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Surge {} Range", val, c.surge_level, fmt_range(min, max))
        },
        apply_func: Some(|c, v1, v2, g| { 
            c.surge_chance += v1; c.surge_level = v2; 
            c.surge_spawn_anchor = g.min_3 as i32 / 4;
            c.surge_spawn_span = g.min_4 as i32 / 4;
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _| {
             let level = resolve_stat(v2, g.min_2, g.max_2);
             let start = g.min_3 as i32 / 4;
             let end = start + (g.min_4 as i32 / 4);
             let (min, max) = if start < end { (start, end) } else { (end, start) };
             let range_str = if min == max { format!("{}", min) } else { format!("{}~{}", min, max) };
             if c.surge_chance == 0 {
                 format!("Chance: {}\nLevel: {}\nRange: {}", fmt_additive(0, v1, "%"), level, range_str)
             } else {
                 format!("Chance: {}\nLevel: {}\nRange: {}", fmt_additive(c.surge_chance, v1, "%"), level, range_str)
             }
        }),
    },
    AbilityDef {
        name: "Mini-Surge",
        icon_id: img015::ICON_MINI_SURGE,
        talent_id: 65,
        group: DisplayGroup::Body1,
        getter: |c| if c.mini_surge_flag > 0 { c.surge_chance } else { 0 },
        duration_getter: None,
        formatter: |val, c, _, _| {
            let start = c.surge_spawn_anchor;
            let end = c.surge_spawn_anchor + c.surge_spawn_span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Mini-Surge {} Range", val, c.surge_level, fmt_range(min, max))
        },
        apply_func: Some(|c, v1, v2, g| { 
            c.mini_surge_flag = 1; c.surge_chance += v1; c.surge_level = v2; 
            c.surge_spawn_anchor = g.min_3 as i32 / 4;
            c.surge_spawn_span = g.min_4 as i32 / 4;
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _| {
             let level = resolve_stat(v2, g.min_2, g.max_2);
             let start = g.min_3 as i32 / 4;
             let end = start + (g.min_4 as i32 / 4);
             let (min, max) = if start < end { (start, end) } else { (end, start) };
             let range_str = if min == max { format!("{}", min) } else { format!("{}~{}", min, max) };
             if c.surge_chance == 0 {
                 format!("Chance: {}\nLevel: {} (Mini)\nRange: {}", fmt_additive(0, v1, "%"), level, range_str)
             } else {
                 format!("Chance: {}\nLevel: {} (Mini)\nRange: {}", fmt_additive(c.surge_chance, v1, "%"), level, range_str)
             }
        }),
    },
    AbilityDef {
        name: "Explosion",
        icon_id: img015::ICON_EXPLOSION,
        talent_id: 67,
        group: DisplayGroup::Body1,
        getter: |c| c.explosion_chance,
        duration_getter: None,
        formatter: |val, c, _, _| {
             let start = c.explosion_spawn_anchor;
             let end = c.explosion_spawn_anchor + c.explosion_spawn_span;
             let (min, max) = if start < end { (start, end) } else { (end, start) };
             format!("{}% Chance to create an Explosion {} Range", val, fmt_range(min, max))
        },
        apply_func: Some(|c, v1, _, g| {
            c.explosion_chance += v1;
            c.explosion_spawn_anchor = g.min_2 as i32 / 4;
            c.explosion_spawn_span = g.min_3 as i32 / 4;
        }),
        talent_desc_func: Some(|v1, _, c, _, _, g, _| {
             let start = g.min_2 as i32 / 4;
             let end = start + (g.min_3 as i32 / 4);
             let (min, max) = if start < end { (start, end) } else { (end, start) };
             let range_str = if min == max { format!("{}", min) } else { format!("{}~{}", min, max) };
             if c.explosion_chance == 0 {
                 format!("Chance: {}\nRange: {}", fmt_additive(0, v1, "%"), range_str)
             } else {
                 format!("Chance: {}\nRange: {}", fmt_additive(c.explosion_chance, v1, "%"), range_str)
             }
        }),
    },
    AbilityDef {
        name: "Savage Blow",
        icon_id: img015::ICON_SAVAGE_BLOW,
        talent_id: 50,
        group: DisplayGroup::Body1,
        getter: |c| c.savage_blow_chance,
        duration_getter: None,
        formatter: |val, c, _, _| {
            let mult = (c.savage_blow_boost as f32 + 100.0) / 100.0;
            format!("{}% Chance to perform a Savage Blow dealing {}× Damage", val, mult)
        },
        apply_func: Some(|c, v1, v2, _| { c.savage_blow_chance += v1; if v2 > 0 { c.savage_blow_boost = v2; } }),
        talent_desc_func: Some(|v1, v2, c, _, _, _, _| {
             if c.savage_blow_chance == 0 {
                 let mut s = format!("Chance: {}", fmt_additive(0, v1, "%"));
                 if v2 > 0 { s.push_str(&format!("\nDamage Boost: +{}%", v2)); }
                 s
             } else {
                 let mut s = format!("Chance: {}", fmt_additive(c.savage_blow_chance, v1, "%"));
                 if v2 > 0 { s.push_str(&format!("\nDamage Boost: +{}%", v2)); }
                 s
             }
        }),
    },
    AbilityDef {
        name: "Critical Hit",
        icon_id: img015::ICON_CRITICAL_HIT,
        talent_id: 13,
        group: DisplayGroup::Body1,
        getter: |c| c.critical_chance,
        duration_getter: None,
        formatter: |val, _, _, _| format!("{}% Chance to perform a Critical Hit dealing 2× Damage\nCritcal Hits bypass Metal resistance", val),
        apply_func: Some(|c, v1, _, _| c.critical_chance += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            if c.critical_chance == 0 {
                format!("Chance: {}", fmt_additive(0, v1, "%"))
            } else {
                format!("Chance: {}", fmt_additive(c.critical_chance, v1, "%"))
            }
        }),
    },
    AbilityDef {
        name: "Strengthen",
        icon_id: img015::ICON_STRENGTHEN,
        talent_id: 10,
        group: DisplayGroup::Body1,
        getter: |c| c.strengthen_threshold,
        duration_getter: None,
        formatter: |_, c, _, _| format!("Damage dealt increases by +{}% when reduced to {}% HP", c.strengthen_boost, c.strengthen_threshold),
        apply_func: Some(|c, v1, v2, _| {
             if c.strengthen_boost == 0 {
                 c.strengthen_threshold = 100 - v1; 
                 c.strengthen_boost = v2;
             } else {
                 c.strengthen_boost += if v1 != 0 { v1 } else { v2 };
             }
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, _, _| {
             if c.strengthen_boost == 0 {
                 format!("+{}% Atk\nTrigger at: {}% HP", v2, 100 - v1)
             } else {
                 fmt_additive(c.strengthen_boost, if v1 != 0 { v1 } else { v2 }, "%")
             }
        }),
    },
    AbilityDef {
        name: "Survive",
        icon_id: img015::ICON_SURVIVE,
        talent_id: 11,
        group: DisplayGroup::Body1,
        getter: |c| c.survive,
        duration_getter: None,
        formatter: |val, _, _, _| format!("{}% Chance to Survive a lethal strike", val),
        apply_func: Some(|c, v1, _, _| c.survive += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            if c.survive == 0 {
                format!("Chance: {}", fmt_additive(0, v1, "%"))
            } else {
                format!("Chance: {}", fmt_additive(c.survive, v1, "%"))
            }
        }),
    },
    AbilityDef {
        name: "Barrier Breaker",
        icon_id: img015::ICON_BARRIER_BREAKER,
        talent_id: 15,
        group: DisplayGroup::Body1,
        getter: |c| c.barrier_breaker_chance,
        duration_getter: None,
        formatter: |val, _, _, _| format!("{}% Chance to break enemy Barriers", val),
        apply_func: Some(|c, v1, _, _| c.barrier_breaker_chance += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            if c.barrier_breaker_chance == 0 {
                format!("Chance: {}", fmt_additive(0, v1, "%"))
            } else {
                format!("Chance: {}", fmt_additive(c.barrier_breaker_chance, v1, "%"))
            }
        }),
    },
    AbilityDef {
        name: "Shield Piercer",
        icon_id: img015::ICON_SHIELD_PIERCER,
        talent_id: 58,
        group: DisplayGroup::Body1,
        getter: |c| c.shield_pierce_chance,
        duration_getter: None,
        formatter: |val, _, _, _| format!("{}% Chance to pierce enemy Shields", val),
        apply_func: Some(|c, v1, _, _| c.shield_pierce_chance += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            if c.shield_pierce_chance == 0 {
                format!("Chance: {}", fmt_additive(0, v1, "%"))
            } else {
                format!("Chance: {}", fmt_additive(c.shield_pierce_chance, v1, "%"))
            }
        }),
    },
    
    // --- BODY 2 ---
    AbilityDef {
        name: "Dodge",
        icon_id: img015::ICON_DODGE,
        talent_id: 51,
        group: DisplayGroup::Body2,
        getter: |c| c.dodge_chance,
        duration_getter: Some(|c| c.dodge_duration),
        formatter: |val, _, target, dur| format!("{}% Chance to Dodge {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, _| { 
            c.dodge_chance += v1; 
            c.dodge_duration += v2; 
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _l| {
             let dur = resolve_stat(v2, g.min_2, g.max_2);
             let chance = resolve_stat(v1, g.min_1, g.max_1);
             
             let chance_changing = g.min_1 != g.max_1;
             let dur_changing = g.min_2 != g.max_2;

             if c.dodge_chance == 0 {
                 if chance_changing {
                     format!("Chance: {}\nDuration: {}", fmt_additive(0, chance, "%"), fmt_f(dur))
                 } else {
                     format!("Duration: {}\nChance: {}%", fmt_additive_f(0, dur), chance)
                 }
             } else {
                 if chance_changing && dur_changing {
                     format!("Chance: {}\nDuration: {}", fmt_additive(c.dodge_chance, v1, "%"), fmt_additive_f(c.dodge_duration, v2))
                 } else if chance_changing {
                     format!("Chance: {}", fmt_additive(c.dodge_chance, v1, "%"))
                 } else {
                     format!("Duration: {}", fmt_additive_f(c.dodge_duration, v2))
                 }
             }
        }),
    },
    AbilityDef {
        name: "Weaken",
        icon_id: img015::ICON_WEAKEN,
        talent_id: 1,
        group: DisplayGroup::Body2,
        getter: |c| c.weaken_chance,
        duration_getter: Some(|c| c.weaken_duration),
        formatter: |val, c, target, dur| format!("{}% Chance to weaken {} to {}% Attack Power for {}", val, target, c.weaken_to, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, group| {
            if c.weaken_chance == 0 {
                 c.weaken_chance = v1; 
                 c.weaken_duration = v2; 
                 c.weaken_to = (100 - group.min_3) as i32; 
            } else if group.text_id == 42 { c.weaken_duration += get_dur_val(v1, v2); } 
            else { c.weaken_chance += v1; c.weaken_duration += v2; }
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _l| {
             let dur = resolve_stat(v2, g.min_2, g.max_2);
             let chance = resolve_stat(v1, g.min_1, g.max_1);
             let weaken_to = 100 - g.min_3; 

             let chance_changing = g.min_1 != g.max_1;
             let dur_changing = g.min_2 != g.max_2;

             if c.weaken_chance == 0 {
                 if g.text_id == 42 || (!chance_changing && dur_changing) {
                     format!("Duration: {}\nChance: {}%\nWeaken to: {}%", fmt_additive_f(0, dur), chance, weaken_to)
                 } else {
                     format!("Chance: {}\nDuration: {}\nWeaken to: {}%", fmt_additive(0, chance, "%"), fmt_f(dur), weaken_to)
                 }
             } else {
                 if g.text_id == 42 || (!chance_changing && dur_changing) {
                     format!("Duration: {}", fmt_additive_f(c.weaken_duration, get_dur_val(v1, v2)))
                 } else {
                     format!("Chance: {}", fmt_additive(c.weaken_chance, v1, "%"))
                 }
             }
        }),
    },
    AbilityDef {
        name: "Freeze",
        icon_id: img015::ICON_FREEZE,
        talent_id: 2,
        group: DisplayGroup::Body2,
        getter: |c| c.freeze_chance,
        duration_getter: Some(|c| c.freeze_duration),
        formatter: |val, _, target, dur| format!("{}% Chance to Freeze {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, g| {
            if c.freeze_chance == 0 { 
                c.freeze_chance = v1; 
                c.freeze_duration = v2; 
            } else if g.text_id == 74 { 
                c.freeze_chance += v1; 
            } else { 
                c.freeze_duration += get_dur_val(v1, v2); 
            }
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _l| {
             let dur = resolve_stat(v2, g.min_2, g.max_2);
             let chance = resolve_stat(v1, g.min_1, g.max_1);
             
             let chance_changing = g.min_1 != g.max_1;
             let dur_changing = g.min_2 != g.max_2;

             if c.freeze_chance == 0 {
                 if g.text_id == 74 || (chance_changing && !dur_changing) {
                     format!("Chance: {}\nDuration: {}", fmt_additive(0, chance, "%"), fmt_f(dur))
                 } else {
                     format!("Duration: {}\nChance: {}%", fmt_additive_f(0, dur), chance)
                 }
             } else {
                 if g.text_id == 74 || (chance_changing && !dur_changing) {
                     format!("Chance: {}", fmt_additive(c.freeze_chance, v1, "%"))
                 } else {
                     format!("Duration: {}", fmt_additive_f(c.freeze_duration, get_dur_val(v1, v2)))
                 }
             }
        }),
    },
    AbilityDef {
        name: "Slow",
        icon_id: img015::ICON_SLOW,
        talent_id: 3,
        group: DisplayGroup::Body2,
        getter: |c| c.slow_chance,
        duration_getter: Some(|c| c.slow_duration),
        formatter: |val, _, target, dur| format!("{}% Chance to Slow {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, g| {
            if c.slow_chance == 0 { 
                c.slow_chance = v1; 
                c.slow_duration = v2; 
            } else if g.text_id == 63 { 
                c.slow_chance += v1; 
            } else { 
                c.slow_duration += get_dur_val(v1, v2); 
            }
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _l| {
             let dur = resolve_stat(v2, g.min_2, g.max_2);
             let chance = resolve_stat(v1, g.min_1, g.max_1);
             
             let chance_changing = g.min_1 != g.max_1;
             let dur_changing = g.min_2 != g.max_2;

             if c.slow_chance == 0 {
                 if g.text_id == 63 || (chance_changing && !dur_changing) {
                     format!("Chance: {}\nDuration: {}", fmt_additive(0, chance, "%"), fmt_f(dur))
                 } else {
                     format!("Duration: {}\nChance: {}%", fmt_additive_f(0, dur), chance)
                 }
             } else {
                 if g.text_id == 63 || (chance_changing && !dur_changing) {
                     format!("Chance: {}", fmt_additive(c.slow_chance, v1, "%"))
                 } else {
                     format!("Duration: {}", fmt_additive_f(c.slow_duration, get_dur_val(v1, v2)))
                 }
             }
        }),
    },
    AbilityDef {
        name: "Knockback",
        icon_id: img015::ICON_KNOCKBACK,
        talent_id: 8,
        group: DisplayGroup::Body2,
        getter: |c| c.knockback_chance,
        duration_getter: None,
        formatter: |val, _, target, _| format!("{}% Chance to Knockback {}", val, target),
        apply_func: Some(|c, v1, _, _| c.knockback_chance += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            if c.knockback_chance == 0 {
                format!("Chance: {}", fmt_additive(0, v1, "%"))
            } else {
                format!("Chance: {}", fmt_additive(c.knockback_chance, v1, "%"))
            }
        }),
    },
    AbilityDef {
        name: "Curse",
        icon_id: img015::ICON_CURSE,
        talent_id: 60,
        group: DisplayGroup::Body2,
        getter: |c| c.curse_chance,
        duration_getter: Some(|c| c.curse_duration),
        formatter: |val, _, target, dur| format!("{}% Chance to Curse {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, g| {
             if c.curse_chance == 0 { c.curse_chance = v1; c.curse_duration = v2; } 
             else if g.text_id == 93 { c.curse_duration += get_dur_val(v1, v2); } 
             else { c.curse_chance += v1; }
        }),
        talent_desc_func: Some(|v1, v2, c, _, _, g, _l| {
            if c.curse_chance == 0 { 
                 let dur = resolve_stat(v2, g.min_2, g.max_2);
                 let chance = resolve_stat(v1, g.min_1, g.max_1);
                 if g.text_id == 93 {
                     format!("Duration: {}\nChance: {}%", fmt_additive_f(0, dur), chance) 
                 } else {
                     format!("Chance: {}\nDuration: {}", fmt_additive(0, chance, "%"), fmt_f(dur)) 
                 }
            } 
            else if g.text_id == 93 { format!("Duration: {}", fmt_additive_f(c.curse_duration, get_dur_val(v1, v2))) } 
            else { format!("Chance: {}", fmt_additive(c.curse_chance, v1, "%")) }
        }),
    },
    AbilityDef {
        name: "Warp",
        icon_id: img015::ICON_WARP,
        talent_id: 9,
        group: DisplayGroup::Body2,
        getter: |c| c.warp_chance,
        duration_getter: Some(|c| c.warp_duration),
        formatter: |val, c, target, dur| format!("{}% Chance to Warp {} {}~{} Range for {}", val, target, c.warp_distance_minimum, c.warp_distance_maximum, fmt_time(dur)),
        apply_func: None,
        talent_desc_func: None
    },
    
    // --- FOOTER ---
    AbilityDef {
        name: "Immune Wave",
        icon_id: img015::ICON_IMMUNE_WAVE,
        talent_id: 23,
        group: DisplayGroup::Footer,
        getter: |c| c.wave_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Wave Attacks".into(),
        apply_func: Some(|c,_,_,_| c.wave_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Surge",
        icon_id: img015::ICON_IMMUNE_SURGE,
        talent_id: 55,
        group: DisplayGroup::Footer,
        getter: |c| c.surge_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Surge Attacks".into(),
        apply_func: Some(|c,_,_,_| c.surge_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Explosion",
        icon_id: img015::ICON_IMMUNE_EXPLOSION,
        talent_id: 116,
        group: DisplayGroup::Footer,
        getter: |c| c.explosion_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Explosions".into(),
        apply_func: Some(|c,_,_,_| c.explosion_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Weaken",
        icon_id: img015::ICON_IMMUNE_WEAKEN,
        talent_id: 44,
        group: DisplayGroup::Footer,
        getter: |c| c.weaken_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Weaken".into(),
        apply_func: Some(|c,_,_,_| c.weaken_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Freeze",
        icon_id: img015::ICON_IMMUNE_FREEZE,
        talent_id: 45,
        group: DisplayGroup::Footer,
        getter: |c| c.freeze_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Freeze".into(),
        apply_func: Some(|c,_,_,_| c.freeze_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Slow",
        icon_id: img015::ICON_IMMUNE_SLOW,
        talent_id: 46,
        group: DisplayGroup::Footer,
        getter: |c| c.slow_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Slow".into(),
        apply_func: Some(|c,_,_,_| c.slow_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Knockback",
        icon_id: img015::ICON_IMMUNE_KNOCKBACK,
        talent_id: 47,
        group: DisplayGroup::Footer,
        getter: |c| c.knockback_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Knockback".into(),
        apply_func: Some(|c,_,_,_| c.knockback_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Curse",
        icon_id: img015::ICON_IMMUNE_CURSE,
        talent_id: 29,
        group: DisplayGroup::Footer,
        getter: |c| c.curse_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Curse".into(),
        apply_func: Some(|c,_,_,_| c.curse_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Toxic",
        icon_id: img015::ICON_IMMUNE_TOXIC,
        talent_id: 53,
        group: DisplayGroup::Footer,
        getter: |c| c.toxic_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Toxic".into(),
        apply_func: Some(|c,_,_,_| c.toxic_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Warp",
        icon_id: img015::ICON_IMMUNE_WARP,
        talent_id: 49,
        group: DisplayGroup::Footer,
        getter: |c| c.warp_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Warp".into(),
        apply_func: Some(|c,_,_,_| c.warp_immune = 1),
        talent_desc_func: Some(|_,_,_,_,_,_,l| fmt_state(l))
    },
    AbilityDef {
        name: "Immune Boss Wave",
        icon_id: img015::ICON_IMMUNE_BOSS_WAVE,
        talent_id: 0,
        group: DisplayGroup::Footer,
        getter: |c| c.boss_wave_immune,
        duration_getter: None,
        formatter: |_,_,_,_| "Immune to Boss Shockwaves".into(),
        apply_func: Some(|c,_,_,_| c.boss_wave_immune = 1),
        talent_desc_func: None
    },

    // RESISTANCES
    AbilityDef {
        name: "Resist Weaken",
        icon_id: img015::ICON_RESIST_WEAKEN,
        talent_id: 18,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Weaken ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Freeze",
        icon_id: img015::ICON_RESIST_FREEZE,
        talent_id: 19,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Freeze ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Slow",
        icon_id: img015::ICON_RESIST_SLOW,
        talent_id: 20,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Slow ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Knockback",
        icon_id: img015::ICON_RESIST_KNOCKBACK,
        talent_id: 21,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Knockback ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Wave",
        icon_id: img015::ICON_RESIST_WAVE,
        talent_id: 22,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Wave ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Warp",
        icon_id: img015::ICON_RESIST_WARP,
        talent_id: 24,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Warp ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Curse",
        icon_id: img015::ICON_RESIST_CURSE,
        talent_id: 30,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Curse ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Toxic",
        icon_id: img015::ICON_RESIST_TOXIC,
        talent_id: 52,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Toxic ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },
    AbilityDef {
        name: "Resist Surge",
        icon_id: img015::ICON_SURGE_RESIST,
        talent_id: 54,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |val,_,_,_| format!("Resist Surge ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
        talent_desc_func: Some(|v1,_,_,_,_,_,_| fmt_additive(0, v1, "%"))
    },

    // STATS
    AbilityDef {
        name: "Cost Down",
        icon_id: img015::ICON_COST_DOWN,
        talent_id: 25,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.eoc1_cost = c.eoc1_cost.saturating_sub(v1)),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            let reduction = (v1 as f32 * 1.5).round() as i32;
            let base_disp = (c.eoc1_cost as f32 * 1.5).round() as i32;
            let new_disp = base_disp - reduction;
            format!("{}¢ (-{}¢) -> {}¢", base_disp, reduction, new_disp)
        }),
    },
    AbilityDef {
        name: "Recover Speed Up",
        icon_id: img015::ICON_RECOVER_SPEED_UP,
        talent_id: 26,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.cooldown = c.cooldown.saturating_sub(v1)),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            let base_eff = c.effective_cooldown();
            let new_eff = (c.cooldown - v1 - 264).max(60);
            let diff = base_eff - new_eff;
            format!("{}f (-{}f) -> {}f", base_eff, diff, new_eff)
        }),
    },
    AbilityDef {
        name: "Move Speed Up",
        icon_id: img015::ICON_MOVE_SPEED,
        talent_id: 27,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.speed += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            fmt_additive(c.speed, v1, "")
        }),
    },
    AbilityDef {
        name: "Attack Buff",
        icon_id: img015::ICON_ATTACK_BUFF,
        talent_id: 31,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| {
            let factor = (100 + v1) as f32 / 100.0;
            c.attack_1 = (c.attack_1 as f32 * factor) as i32;
            c.attack_2 = (c.attack_2 as f32 * factor) as i32;
            c.attack_3 = (c.attack_3 as f32 * factor) as i32;
        }),
        talent_desc_func: Some(|v1, _, c, curve, unit_level, _, _| {
            let total_base = c.attack_1 + c.attack_2 + c.attack_3;
            let leveled_base = curve.map_or(total_base, |cv| cv.calculate_stat(total_base, unit_level));
            fmt_multi_stat(leveled_base, v1)
        }),
    },
    AbilityDef {
        name: "Health Buff",
        icon_id: img015::ICON_HEALTH_BUFF,
        talent_id: 32,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| {
            let factor = (100 + v1) as f32 / 100.0;
            c.hitpoints = (c.hitpoints as f32 * factor) as i32;
        }),
        talent_desc_func: Some(|v1, _, c, curve, unit_level, _, _| {
            let leveled_base = curve.map_or(c.hitpoints, |cv| cv.calculate_stat(c.hitpoints, unit_level));
            fmt_multi_stat(leveled_base, v1)
        }),
    },
    AbilityDef {
        name: "TBA Down",
        icon_id: img015::ICON_TBA_DOWN,
        talent_id: 61,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| {
             let reduction = (c.time_before_attack_1 as f32 * v1 as f32 / 100.0).round() as i32;
             c.time_before_attack_1 = c.time_before_attack_1.saturating_sub(reduction);
        }),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
             let reduction = (c.time_before_attack_1 as f32 * v1 as f32 / 100.0).round() as i32;
             let new_tba = c.time_before_attack_1 - reduction;
             format!("{}f (-{}%) -> {}f", c.time_before_attack_1, v1, new_tba)
        }),
    },
    AbilityDef {
        name: "Improve Knockbacks",
        icon_id: img015::ICON_IMPROVE_KNOCKBACK_COUNT,
        talent_id: 28,
        group: DisplayGroup::Footer,
        getter: |_c| 0,
        duration_getter: None,
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.knockbacks += v1),
        talent_desc_func: Some(|v1, _, c, _, _, _, _| {
            format!("{} (+{}) -> {}", c.knockbacks, v1, c.knockbacks + v1)
        }),
    },
];

pub fn get_by_talent_id(id: u8) -> Option<&'static AbilityDef> {
    ABILITY_REGISTRY.iter().find(|def| def.talent_id == id)
}