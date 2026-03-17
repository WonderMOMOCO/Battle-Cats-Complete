use crate::global::game::img015;
use crate::features::cat::data::unitid::CatRaw;
use crate::features::cat::data::skillacquisition::TalentGroupRaw;
use crate::global::game::abilities::CustomIcon;

#[derive(PartialEq, Clone, Copy)]
pub enum DisplayGroup {
    Trait,     
    Headline1, 
    Headline2, 
    Body1,     
    Body2,     
    Footer,
    Hidden, 
}

#[derive(PartialEq, Clone, Copy)]
pub enum AttrUnit {
    None,       // For Counts, Levels, raw hitpoints
    Percent,    // For Chances, Boosts, Reductions
    Frames,     // For Time and Durations
    Range,      // For Distances
}

pub struct CatAbilityDef {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon_id: usize,
    pub talent_id: u8, 
    pub group: DisplayGroup,
    pub custom_icon: CustomIcon,
    pub schema: &'static [(&'static str, AttrUnit)],
    pub get_attributes: fn(&CatRaw) -> Vec<(&'static str, i32, AttrUnit)>,
    pub formatter: fn(val: i32, stats: &CatRaw, target: &str, duration_frames: i32) -> String,
    pub apply_func: Option<fn(&mut CatRaw, val1: i32, val2: i32, group: &TalentGroupRaw)>,
}

// --- FORMATTERS ---

fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_range(min: i32, max: i32) -> String {
    if min == max { format!("at {}", min) } else { format!("between {}~{}", min, max) }
}

fn get_dur_val(v1: i32, v2: i32) -> i32 {
    if v1 != 0 { v1 } else { v2 }
}

fn fmt_effective_range(c: &CatRaw) -> String {
    let enemy_base_range = {
        let start_range = c.long_distance_1_anchor;
        let end_range = c.long_distance_1_anchor + c.long_distance_1_span;
        let (min_reach, max_reach) = if start_range < end_range { (start_range, end_range) } else { (end_range, start_range) };
        if min_reach > 0 { min_reach } else { max_reach }
    };
    let mut range_strings = Vec::new();
    let range_checks = [
        (c.long_distance_1_anchor, c.long_distance_1_span),
        (c.long_distance_2_anchor, if c.long_distance_2_flag == 1 { c.long_distance_2_span } else { 0 }),
        (c.long_distance_3_anchor, if c.long_distance_3_flag == 1 { c.long_distance_3_span } else { 0 }),
    ];
    for (anchor, span) in range_checks {
        if span != 0 {
            let start = anchor;
            let end = anchor + span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            range_strings.push(format!("{}~{}", min, max));
        }
    }
    if range_strings.len() > 1 {
        let first = range_strings[0].clone();
        if range_strings.iter().all(|s| s == &first) {
            range_strings.truncate(1);
        }
    }
    let label_prefix = if range_strings.len() > 1 { "Range split" } else { "Effective Range" };
    format!("{} {}\nStands at {} Range relative to Enemy Base", label_prefix, range_strings.join(" / "), enemy_base_range)
}

fn fmt_multihit(c: &CatRaw) -> String {
    let damage_hit_1 = c.attack_1;
    let damage_hit_2 = c.attack_2;
    let damage_hit_3 = c.attack_3;
    let ability_flag_1 = if c.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if c.attack_2_abilities > 0 { "True" } else { "False" };
    let ability_flag_3 = if c.attack_3 > 0 { if c.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
    let damage_string = if c.attack_3 > 0 { 
        format!("{} / {} / {}", damage_hit_1, damage_hit_2, damage_hit_3) 
    } else { 
        format!("{} / {}", damage_hit_1, damage_hit_2) 
    };
    format!("Damage split {}\nAbility split {} / {}{}", damage_string, ability_flag_1, ability_flag_2, ability_flag_3)
}

// --- ABILITY REGISTRY ---

pub const CAT_ABILITY_REGISTRY: &[CatAbilityDef] = &[
    // --- SPECIAL HIDDEN ---
    CatAbilityDef {
        name: "Single Attack",
        fallback: "Sngl",
        icon_id: img015::ICON_SINGLE_ATTACK,
        talent_id: 0,
        group: DisplayGroup::Hidden,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.area_attack == 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "".into(),
        apply_func: None,
    },
    CatAbilityDef {
        name: "Area Attack",
        fallback: "Area",
        icon_id: img015::ICON_AREA_ATTACK,
        talent_id: 0,
        group: DisplayGroup::Hidden,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.area_attack == 1 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "".into(),
        apply_func: None,
    },

    // --- TRAITS ---
    CatAbilityDef {
        name: "Target Red",
        fallback: "Red",
        icon_id: img015::ICON_TRAIT_RED,
        talent_id: 33,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_red > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Red Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_red = 1),
    },
    CatAbilityDef {
        name: "Target Float",
        fallback: "Float",
        icon_id: img015::ICON_TRAIT_FLOATING,
        talent_id: 34,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_floating > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Floating Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_floating = 1),
    },
    CatAbilityDef {
        name: "Target Black",
        fallback: "Black",
        icon_id: img015::ICON_TRAIT_BLACK,
        talent_id: 35,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_black > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Black Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_black = 1),
    },
    CatAbilityDef {
        name: "Target Metal",
        fallback: "Metal",
        icon_id: img015::ICON_TRAIT_METAL,
        talent_id: 36,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_metal > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Metal Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_metal = 1),
    },
    CatAbilityDef {
        name: "Target Angel",
        fallback: "Angel",
        icon_id: img015::ICON_TRAIT_ANGEL,
        talent_id: 37,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_angel > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Angel Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_angel = 1),
    },
    CatAbilityDef {
        name: "Target Alien",
        fallback: "Alien",
        icon_id: img015::ICON_TRAIT_ALIEN,
        talent_id: 38,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_alien > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Alien Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_alien = 1),
    },
    CatAbilityDef {
        name: "Target Zombie",
        fallback: "Zomb",
        icon_id: img015::ICON_TRAIT_ZOMBIE,
        talent_id: 39,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_zombie > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Zombie Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_zombie = 1),
    },
    CatAbilityDef {
        name: "Target Relic",
        fallback: "Relic",
        icon_id: img015::ICON_TRAIT_RELIC,
        talent_id: 40,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_relic > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Relic Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_relic = 1),
    },
    CatAbilityDef {
        name: "Target Aku",
        fallback: "Aku",
        icon_id: img015::ICON_TRAIT_AKU,
        talent_id: 57,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_aku > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Aku Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_aku = 1),
    },
    CatAbilityDef {
        name: "Target White",
        fallback: "White",
        icon_id: img015::ICON_TRAIT_TRAITLESS,
        talent_id: 41,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.target_traitless > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Traitless Enemies".into(),
        apply_func: Some(|c,_,_,_| c.target_traitless = 1),
    },

    // --- HEADLINE 1 ---
    CatAbilityDef {
        name: "Attack Only",
        fallback: "AtkOnly",
        icon_id: img015::ICON_ATTACK_ONLY,
        talent_id: 4,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.attack_only > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Only damages {}", target),
        apply_func: Some(|c, _, _, _| c.attack_only = 1),
    },
    CatAbilityDef {
        name: "Strong Against",
        fallback: "Strng",
        icon_id: img015::ICON_STRONG_AGAINST,
        talent_id: 5,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.strong_against > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target),
        apply_func: Some(|c, _, _, _| c.strong_against = 1),
    },
    CatAbilityDef {
        name: "Massive Damage",
        fallback: "Massv",
        icon_id: img015::ICON_MASSIVE_DAMAGE,
        talent_id: 7,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.massive_damage > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Deals 3×~4× Damage to {}", target),
        apply_func: Some(|c, _, _, _| c.massive_damage = 1),
    },
    CatAbilityDef {
        name: "Insane Damage",
        fallback: "InsDmg",
        icon_id: img015::ICON_INSANE_DAMAGE,
        talent_id: 7,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.insane_damage > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Deals 5×~6× Damage to {}", target),
        apply_func: None,
    },
    CatAbilityDef {
        name: "Resist",
        fallback: "Resist",
        icon_id: img015::ICON_RESIST,
        talent_id: 6,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.resist > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Takes 1/4×~1/5× Damage from {}", target),
        apply_func: Some(|c, _, _, _| c.resist = 1),
    },
    CatAbilityDef {
        name: "Insanely Tough",
        fallback: "InsRes",
        icon_id: img015::ICON_INSANELY_TOUGH,
        talent_id: 6,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.insanely_tough > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Takes 1/6×~1/7× Damage from {}", target),
        apply_func: None,
    },

    // --- HEADLINE 2 ---
    CatAbilityDef {
        name: "Metal",
        fallback: "Metal",
        icon_id: img015::ICON_METAL,
        talent_id: 43,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.metal > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Damage taken is reduced to 1 for Non-Critical attacks".into(),
        apply_func: Some(|c,_,_,_| c.metal = 1),
    },
    CatAbilityDef {
        name: "Base Destroyer",
        fallback: "Base",
        icon_id: img015::ICON_BASE_DESTROYER,
        talent_id: 12,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.base_destroyer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Deals 4× Damage to the Enemy Base".into(),
        apply_func: Some(|c, _, _, _| c.base_destroyer = 1),
    },
    
    CatAbilityDef {
        name: "Double Bounty",
        fallback: "2×$",
        icon_id: img015::ICON_DOUBLE_BOUNTY,
        talent_id: 16,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.double_bounty > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Receives 2× Cash from Enemies".into(),
        apply_func: Some(|c, _, _, _| c.double_bounty = 1),
    },
    CatAbilityDef {
        name: "Zombie Killer",
        fallback: "Zkill",
        icon_id: img015::ICON_ZOMBIE_KILLER,
        talent_id: 14,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.zombie_killer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Prevents Zombies from reviving".into(),
        apply_func: Some(|c, _, _, _| c.zombie_killer = 1),
    },
    CatAbilityDef {
        name: "Soulstrike",
        fallback: "SolStk",
        icon_id: img015::ICON_SOULSTRIKE,
        talent_id: 59,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.soulstrike == 2 || (c.soulstrike > 0 && c.zombie_killer > 0) { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Will attack Zombie corpses".into(),
        apply_func: Some(|c, _, _, _| c.soulstrike = 2),
    },
    CatAbilityDef {
        name: "Colossus Slayer",
        fallback: "Colos",
        icon_id: img015::ICON_COLOSSUS_SLAYER,
        talent_id: 63,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.colossus_slayer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into(),
        apply_func: Some(|c, _, _, _| c.colossus_slayer = 1),
    },
    CatAbilityDef {
        name: "Sage Slayer",
        fallback: "Sage",
        icon_id: img015::ICON_SAGE_SLAYER,
        talent_id: 66,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.sage_slayer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Deals 1.2× Damage to and takes 0.5× Damage from Sage Enemies\nCrowd Control effects originating from Sage Enemies reduced by 70%".into(),
        apply_func: Some(|c, _, _, _| c.sage_slayer = 1),
    },
    CatAbilityDef {
        name: "Behemoth Slayer",
        fallback: "Behem",
        icon_id: img015::ICON_BEHEMOTH_SLAYER,
        talent_id: 64,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Dodge Chance", AttrUnit::Percent), 
            ("Dodge Duration", AttrUnit::Frames)
        ],
        get_attributes: |c| {
            if c.behemoth_slayer > 0 {
                if c.behemoth_dodge_chance > 0 {
                    vec![
                        ("Active", 1, AttrUnit::None), 
                        ("Dodge Chance", c.behemoth_dodge_chance, AttrUnit::Percent), 
                        ("Dodge Duration", c.behemoth_dodge_duration, AttrUnit::Frames),
                    ]
                } else {
                    vec![("Active", 1, AttrUnit::None)]
                }
            } else {
                vec![]
            }
        },
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
    },
    CatAbilityDef {
        name: "Eva Killer",
        fallback: "Eva",
        icon_id: img015::ICON_EVA_KILLER,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.eva_killer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into(),
        apply_func: Some(|c,_,_,_| c.eva_killer = 1),
    },
    CatAbilityDef {
        name: "Witch Killer",
        fallback: "Witch",
        icon_id: img015::ICON_WITCH_KILLER,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.witch_killer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Deals 5× Damage to and takes 0.1× Damage from Witches".into(),
        apply_func: Some(|c,_,_,_| c.witch_killer = 1),
    },
    CatAbilityDef {
        name: "Wave Block",
        fallback: "W-Blk",
        icon_id: img015::ICON_WAVE_BLOCK,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.wave_block > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(),
        apply_func: Some(|c, _, _, _| c.wave_block = 1),
    },
    CatAbilityDef {
        name: "Counter Surge",
        fallback: "C-Srg",
        icon_id: img015::ICON_COUNTER_SURGE,
        talent_id: 68,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.counter_surge > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        apply_func: Some(|c,_,_,_| c.counter_surge = 1),
    },
    CatAbilityDef {
        name: "Kamikaze", 
        fallback: "Kamik", 
        icon_id: img015::ICON_KAMIKAZE,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::Kamikaze, 
        schema: &[
            ("Attacks", AttrUnit::None)
        ],
        get_attributes: |c| {
            if c.attack_count_total > -1 && c.attack_count_state == 2 { 
                vec![("Attacks", c.attack_count_total, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| {
            let suffix = match val {
                0 => "immediately".to_string(),
                1 => "after 1 attack".to_string(),
                n => format!("after {} attacks", n),
            };
            format!("Unit disappears {}", suffix)
        },
        apply_func: None,
    },
    CatAbilityDef {
        name: "Stop", 
        fallback: "Stop", 
        icon_id: img015::ICON_STOP,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::Stop, 
        schema: &[
            ("Attacks", AttrUnit::None)
        ],
        get_attributes: |c| {
            if c.attack_count_total > -1 && c.attack_count_state == 0 { 
                vec![("Attacks", c.attack_count_total, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| {
            let suffix = match val {
                0 => "immediately".to_string(),
                1 => "after 1 attack".to_string(),
                n => format!("after {} attacks", n),
            };
            format!("Unit stops moving {}", suffix)
        },
        apply_func: None,
    },

    // --- BODY 1 ---
    CatAbilityDef {
        name: "Multi-Hit",
        fallback: "Multi",
        icon_id: img015::ICON_MULTIHIT,
        talent_id: 0,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::Multihit,
        schema: &[],
        get_attributes: |c| if c.attack_2 > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, c, _, _| fmt_multihit(c),
        apply_func: None,
    },
    CatAbilityDef {
        name: "Long Distance",
        fallback: "LD",
        icon_id: img015::ICON_LONG_DISTANCE,
        talent_id: 0,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| {
            let ranges = [
                (c.long_distance_1_anchor, c.long_distance_1_span),
                (c.long_distance_2_anchor, if c.long_distance_2_flag == 1 { c.long_distance_2_span } else { 0 }),
                (c.long_distance_3_anchor, if c.long_distance_3_flag == 1 { c.long_distance_3_span } else { 0 }),
            ];
            let mut has_range = false;
            let mut is_omni = false;
            for (anchor, span) in ranges {
                if span != 0 {
                    has_range = true;
                    let start = anchor;
                    let end = anchor + span;
                    let min = if start < end { start } else { end };
                    if min <= 0 { is_omni = true; }
                }
            }
            if has_range && !is_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_, c, _, _| fmt_effective_range(c),
        apply_func: None,
    },
    CatAbilityDef {
        name: "Omni Strike",
        fallback: "Omni",
        icon_id: img015::ICON_OMNI_STRIKE,
        talent_id: 0,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| {
            let ranges = [
                (c.long_distance_1_anchor, c.long_distance_1_span),
                (c.long_distance_2_anchor, if c.long_distance_2_flag == 1 { c.long_distance_2_span } else { 0 }),
                (c.long_distance_3_anchor, if c.long_distance_3_flag == 1 { c.long_distance_3_span } else { 0 }),
            ];
            let mut is_omni = false;
            for (anchor, span) in ranges {
                if span != 0 {
                    let start = anchor;
                    let end = anchor + span;
                    let min = if start < end { start } else { end };
                    if min <= 0 { is_omni = true; }
                }
            }
            if is_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_, c, _, _| fmt_effective_range(c),
        apply_func: None,
    },
    CatAbilityDef {
        name: "Conjure / Spirit",
        fallback: "Spirit",
        icon_id: img015::ICON_CONJURE,
        talent_id: 0,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.conjure_unit_id > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Conjures a Spirit to the battlefield when tapped\nThis Cat may only be deployed one at a time".into(),
        apply_func: None,
    },
    CatAbilityDef {
        name: "Metal Killer",
        fallback: "MetKil",
        icon_id: img015::ICON_METAL_KILLER,
        talent_id: 0,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Damage", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.metal_killer_percent > 0 { 
                vec![
                    ("Damage", c.metal_killer_percent, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val,_,_,_| format!("Reduces Metal enemies current HP by {}% upon hit", val),
        apply_func: Some(|c,v1,_,_| c.metal_killer_percent = v1),
    },
    CatAbilityDef {
        name: "Wave Attack",
        fallback: "Wave",
        icon_id: img015::ICON_WAVE,
        talent_id: 17,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
        ],
        get_attributes: |c| {
            if c.mini_wave_flag == 0 && c.wave_chance > 0 { 
                let reach = (332.5 + ((c.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", c.wave_chance, AttrUnit::Percent), 
                    ("Level", c.wave_level, AttrUnit::None),
                    ("Max Reach", reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, _, _| {
            let reach = 332.5 + ((c.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", val, c.wave_level, reach)
        },
        apply_func: Some(|c, v1, v2, _| { c.wave_chance += v1; c.wave_level = v2; }),
    },
    CatAbilityDef {
        name: "Mini-Wave",
        fallback: "MiniW",
        icon_id: img015::ICON_MINI_WAVE,
        talent_id: 62,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
        ],
        get_attributes: |c| {
            if c.mini_wave_flag > 0 && c.wave_chance > 0 { 
                let reach = (332.5 + ((c.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", c.wave_chance, AttrUnit::Percent), 
                    ("Level", c.wave_level, AttrUnit::None),
                    ("Max Reach", reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, _, _| {
             let reach = 332.5 + ((c.wave_level - 1) as f32 * 200.0);
             format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", val, c.wave_level, reach)
        },
        apply_func: Some(|c, v1, v2, _| { c.mini_wave_flag = 1; c.wave_chance += v1; c.wave_level = v2; }),
    },
    CatAbilityDef {
        name: "Surge Attack",
        fallback: "Surge",
        icon_id: img015::ICON_SURGE,
        talent_id: 56,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |c| {
            if c.mini_surge_flag == 0 && c.surge_chance > 0 { 
                vec![
                    ("Chance", c.surge_chance, AttrUnit::Percent), 
                    ("Level", c.surge_level, AttrUnit::None), 
                    ("Min Range", c.surge_spawn_anchor, AttrUnit::Range), 
                    ("Max Range", c.surge_spawn_anchor + c.surge_spawn_span, AttrUnit::Range),
                    ("Width", c.surge_spawn_span, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, _, _| {
            let start = c.surge_spawn_anchor;
            let end = c.surge_spawn_anchor + c.surge_spawn_span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Surge\n{} Range", val, c.surge_level, fmt_range(min, max))
        },
        apply_func: Some(|c, v1, v2, g| { 
            c.surge_chance += v1; c.surge_level = v2; 
            c.surge_spawn_anchor = g.min_3 as i32 / 4;
            c.surge_spawn_span = g.min_4 as i32 / 4;
        }),
    },
    CatAbilityDef {
        name: "Mini-Surge",
        fallback: "MiniS",
        icon_id: img015::ICON_MINI_SURGE,
        talent_id: 65,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |c| {
            if c.mini_surge_flag > 0 && c.surge_chance > 0 { 
                vec![
                    ("Chance", c.surge_chance, AttrUnit::Percent), 
                    ("Level", c.surge_level, AttrUnit::None), 
                    ("Min Range", c.surge_spawn_anchor, AttrUnit::Range), 
                    ("Max Range", c.surge_spawn_anchor + c.surge_spawn_span, AttrUnit::Range),
                    ("Width", c.surge_spawn_span, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, _, _| {
            let start = c.surge_spawn_anchor;
            let end = c.surge_spawn_anchor + c.surge_spawn_span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Mini-Surge\n{} Range", val, c.surge_level, fmt_range(min, max))
        },
        apply_func: Some(|c, v1, v2, g| { 
            c.mini_surge_flag = 1; c.surge_chance += v1; c.surge_level = v2; 
            c.surge_spawn_anchor = g.min_3 as i32 / 4;
            c.surge_spawn_span = g.min_4 as i32 / 4;
        }),
    },
    CatAbilityDef {
        name: "Explosion",
        fallback: "Expl",
        icon_id: img015::ICON_EXPLOSION,
        talent_id: 67,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |c| {
            if c.explosion_chance > 0 { 
                vec![
                    ("Chance", c.explosion_chance, AttrUnit::Percent), 
                    ("Min Range", c.explosion_spawn_anchor, AttrUnit::Range), 
                    ("Max Range", c.explosion_spawn_anchor + c.explosion_spawn_span, AttrUnit::Range),
                    ("Width", c.explosion_spawn_span, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
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
    },
    CatAbilityDef {
        name: "Savage Blow",
        fallback: "Savge",
        icon_id: img015::ICON_SAVAGE_BLOW,
        talent_id: 50,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Boost", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.savage_blow_chance > 0 { 
                vec![
                    ("Chance", c.savage_blow_chance, AttrUnit::Percent), 
                    ("Boost", c.savage_blow_boost, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, _, _| {
            let mult = (c.savage_blow_boost as f32 + 100.0) / 100.0;
            format!("{}% Chance to perform a Savage Blow dealing {}× Damage", val, mult)
        },
        apply_func: Some(|c, v1, v2, _| { c.savage_blow_chance += v1; if v2 > 0 { c.savage_blow_boost = v2; } }),
    },
    CatAbilityDef {
        name: "Critical Hit",
        fallback: "Crit",
        icon_id: img015::ICON_CRITICAL_HIT,
        talent_id: 13,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.critical_chance > 0 { 
                vec![
                    ("Chance", c.critical_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to perform a Critical Hit dealing 2× Damage\nCritcal Hits bypass Metal resistance", val),
        apply_func: Some(|c, v1, _, _| c.critical_chance += v1),
    },
    CatAbilityDef {
        name: "Strengthen",
        fallback: "Str+",
        icon_id: img015::ICON_STRENGTHEN,
        talent_id: 10,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("HP", AttrUnit::Percent), 
            ("Boost", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.strengthen_threshold > 0 { 
                vec![
                    ("HP", c.strengthen_threshold, AttrUnit::Percent), 
                    ("Boost", c.strengthen_boost, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |_, c, _, _| format!("Damage dealt increases by +{}% when reduced to {}% HP", c.strengthen_boost, c.strengthen_threshold),
        apply_func: Some(|c, v1, v2, _| {
             if c.strengthen_boost == 0 {
                 c.strengthen_threshold = 100 - v1; 
                 c.strengthen_boost = v2;
             } else {
                 c.strengthen_boost += if v1 != 0 { v1 } else { v2 };
             }
        }),
    },
    CatAbilityDef {
        name: "Survive",
        fallback: "Surv",
        icon_id: img015::ICON_SURVIVE,
        talent_id: 11,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.survive > 0 { 
                vec![
                    ("Chance", c.survive, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to Survive a lethal strike", val),
        apply_func: Some(|c, v1, _, _| c.survive += v1),
    },
    CatAbilityDef {
        name: "Barrier Breaker",
        fallback: "Brkr",
        icon_id: img015::ICON_BARRIER_BREAKER,
        talent_id: 15,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.barrier_breaker_chance > 0 { 
                vec![
                    ("Chance", c.barrier_breaker_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to break enemy Barriers", val),
        apply_func: Some(|c, v1, _, _| c.barrier_breaker_chance += v1),
    },
    CatAbilityDef {
        name: "Shield Piercer",
        fallback: "Spierc",
        icon_id: img015::ICON_SHIELD_PIERCER,
        talent_id: 58,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.shield_pierce_chance > 0 { 
                vec![
                    ("Chance", c.shield_pierce_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to pierce enemy Shields", val),
        apply_func: Some(|c, v1, _, _| c.shield_pierce_chance += v1),
    },
    
    // --- BODY 2 ---
    CatAbilityDef {
        name: "Dodge",
        fallback: "Dodge",
        icon_id: img015::ICON_DODGE,
        talent_id: 51,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |c| {
            if c.dodge_chance > 0 { 
                vec![
                    ("Chance", c.dodge_chance, AttrUnit::Percent), 
                    ("Duration", c.dodge_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, target, dur| format!("{}% Chance to Dodge {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, _| { c.dodge_chance += v1; c.dodge_duration += v2; }),
    },
    CatAbilityDef {
        name: "Weaken",
        fallback: "Weak",
        icon_id: img015::ICON_WEAKEN,
        talent_id: 1,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Reduced To", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |c| {
            if c.weaken_chance > 0 { 
                vec![
                    ("Chance", c.weaken_chance, AttrUnit::Percent), 
                    ("Reduced To", c.weaken_to, AttrUnit::Percent), 
                    ("Duration", c.weaken_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, target, dur| format!("{}% Chance to weaken {}\nto {}% Attack Power for {}", val, target, c.weaken_to, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, group| {
            if c.weaken_chance == 0 {
                 c.weaken_chance = v1; c.weaken_duration = v2; c.weaken_to = (100 - group.min_3) as i32; 
            } else if group.text_id == 42 { c.weaken_duration += get_dur_val(v1, v2); } 
            else { c.weaken_chance += v1; c.weaken_duration += v2; }
        }),
    },
    CatAbilityDef {
        name: "Freeze",
        fallback: "Freez",
        icon_id: img015::ICON_FREEZE,
        talent_id: 2,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |c| {
            if c.freeze_chance > 0 { 
                vec![
                    ("Chance", c.freeze_chance, AttrUnit::Percent), 
                    ("Duration", c.freeze_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, target, dur| format!("{}% Chance to Freeze {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, g| {
            if c.freeze_chance == 0 { c.freeze_chance = v1; c.freeze_duration = v2; } 
            else if g.text_id == 74 { c.freeze_chance += v1; } 
            else { c.freeze_duration += get_dur_val(v1, v2); }
        }),
    },
    CatAbilityDef {
        name: "Slow",
        fallback: "Slow",
        icon_id: img015::ICON_SLOW,
        talent_id: 3,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |c| {
            if c.slow_chance > 0 { 
                vec![
                    ("Chance", c.slow_chance, AttrUnit::Percent), 
                    ("Duration", c.slow_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, target, dur| format!("{}% Chance to Slow {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, g| {
            if c.slow_chance == 0 { c.slow_chance = v1; c.slow_duration = v2; } 
            else if g.text_id == 63 { c.slow_chance += v1; } 
            else { c.slow_duration += get_dur_val(v1, v2); }
        }),
    },
    CatAbilityDef {
        name: "Knockback",
        fallback: "KB",
        icon_id: img015::ICON_KNOCKBACK,
        talent_id: 8,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |c| {
            if c.knockback_chance > 0 { 
                vec![
                    ("Chance", c.knockback_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, target, _| format!("{}% Chance to Knockback {}", val, target),
        apply_func: Some(|c, v1, _, _| c.knockback_chance += v1),
    },
    CatAbilityDef {
        name: "Curse",
        fallback: "Curse",
        icon_id: img015::ICON_CURSE,
        talent_id: 60,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |c| {
            if c.curse_chance > 0 { 
                vec![
                    ("Chance", c.curse_chance, AttrUnit::Percent), 
                    ("Duration", c.curse_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, target, dur| format!("{}% Chance to Curse {} for {}", val, target, fmt_time(dur)),
        apply_func: Some(|c, v1, v2, g| {
             if c.curse_chance == 0 { c.curse_chance = v1; c.curse_duration = v2; } 
             else if g.text_id == 93 { c.curse_duration += get_dur_val(v1, v2); } 
             else { c.curse_chance += v1; }
        }),
    },
    CatAbilityDef {
        name: "Warp",
        fallback: "Warp",
        icon_id: img015::ICON_WARP,
        talent_id: 9,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames), 
            ("Min Distance", AttrUnit::Range), 
            ("Max Distance", AttrUnit::Range)
        ],
        get_attributes: |c| {
            if c.warp_chance > 0 { 
                vec![
                    ("Chance", c.warp_chance, AttrUnit::Percent), 
                    ("Duration", c.warp_duration, AttrUnit::Frames), 
                    ("Min Distance", c.warp_distance_minimum, AttrUnit::Range), 
                    ("Max Distance", c.warp_distance_maximum, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, c, target, dur| format!("{}% Chance to Warp {} {}~{} Range for {}", val, target, c.warp_distance_minimum, c.warp_distance_maximum, fmt_time(dur)),
        apply_func: None,
    },
    
    // --- FOOTER ---
    CatAbilityDef {
        name: "Immune Wave",
        fallback: "NoWav",
        icon_id: img015::ICON_IMMUNE_WAVE,
        talent_id: 48,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.wave_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Wave Attacks".into(),
        apply_func: Some(|c,_,_,_| c.wave_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Surge",
        fallback: "NoSrg",
        icon_id: img015::ICON_IMMUNE_SURGE,
        talent_id: 55,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.surge_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Surge Attacks".into(),
        apply_func: Some(|c,_,_,_| c.surge_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Explosion",
        fallback: "NoExp",
        icon_id: img015::ICON_IMMUNE_EXPLOSION,
        talent_id: 116,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.explosion_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Explosions".into(),
        apply_func: Some(|c,_,_,_| c.explosion_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Weaken",
        fallback: "NoWk",
        icon_id: img015::ICON_IMMUNE_WEAKEN,
        talent_id: 44,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.weaken_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Weaken".into(),
        apply_func: Some(|c,_,_,_| c.weaken_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Freeze",
        fallback: "NoFrz",
        icon_id: img015::ICON_IMMUNE_FREEZE,
        talent_id: 45,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.freeze_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Freeze".into(),
        apply_func: Some(|c,_,_,_| c.freeze_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Slow",
        fallback: "NoSlw",
        icon_id: img015::ICON_IMMUNE_SLOW,
        talent_id: 46,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.slow_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Slow".into(),
        apply_func: Some(|c,_,_,_| c.slow_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Knockback",
        fallback: "NoKB",
        icon_id: img015::ICON_IMMUNE_KNOCKBACK,
        talent_id: 47,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.knockback_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Knockback".into(),
        apply_func: Some(|c,_,_,_| c.knockback_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Curse",
        fallback: "NoCur",
        icon_id: img015::ICON_IMMUNE_CURSE,
        talent_id: 29,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.curse_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Curse".into(),
        apply_func: Some(|c,_,_,_| c.curse_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Toxic",
        fallback: "NoTox",
        icon_id: img015::ICON_IMMUNE_TOXIC,
        talent_id: 53,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.toxic_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Toxic".into(),
        apply_func: Some(|c,_,_,_| c.toxic_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Warp",
        fallback: "NoWrp",
        icon_id: img015::ICON_IMMUNE_WARP,
        talent_id: 49,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |c| if c.warp_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Warp".into(),
        apply_func: Some(|c,_,_,_| c.warp_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Boss Wave",
        fallback: "NoBos",
        icon_id: img015::ICON_IMMUNE_BOSS_WAVE,
        talent_id: 0,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::BossWave,
        schema: &[],
        get_attributes: |c| if c.boss_wave_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Boss Shockwaves".into(),
        apply_func: Some(|c,_,_,_| c.boss_wave_immune = 1),
    },

    // --- RESISTANCES ---
    CatAbilityDef {
        name: "Resist Weaken",
        fallback: "ReWkn",
        icon_id: img015::ICON_RESIST_WEAKEN,
        talent_id: 18,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Weaken ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Freeze",
        fallback: "ReFrz",
        icon_id: img015::ICON_RESIST_FREEZE,
        talent_id: 19,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Freeze ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Slow",
        fallback: "ReSlw",
        icon_id: img015::ICON_RESIST_SLOW,
        talent_id: 20,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Slow ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Knockback",
        fallback: "ReKB",
        icon_id: img015::ICON_RESIST_KNOCKBACK,
        talent_id: 21,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Knockback ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Wave",
        fallback: "ReWav",
        icon_id: img015::ICON_RESIST_WAVE,
        talent_id: 22,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Wave ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Warp",
        fallback: "ReWrp",
        icon_id: img015::ICON_RESIST_WARP,
        talent_id: 24,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Warp ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Curse",
        fallback: "ReCur",
        icon_id: img015::ICON_RESIST_CURSE,
        talent_id: 30,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Curse ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Toxic",
        fallback: "ReTox",
        icon_id: img015::ICON_RESIST_TOXIC,
        talent_id: 52,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Toxic ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },
    CatAbilityDef {
        name: "Resist Surge",
        fallback: "ReSrg",
        icon_id: img015::ICON_SURGE_RESIST,
        talent_id: 54,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |val,_,_,_| format!("Resist Surge ({}%)", val),
        apply_func: Some(|_,_,_,_| {}),
    },

    // --- STAT TALENTS ---
    CatAbilityDef {
        name: "Cost Down",
        fallback: "Cost-",
        icon_id: img015::ICON_COST_DOWN,
        talent_id: 25,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.eoc1_cost = c.eoc1_cost.saturating_sub(v1)),
    },
    CatAbilityDef {
        name: "Recover Speed Up",
        fallback: "Rec+",
        icon_id: img015::ICON_RECOVER_SPEED_UP,
        talent_id: 26,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.cooldown = c.cooldown.saturating_sub(v1)),
    },
    CatAbilityDef {
        name: "Move Speed Up",
        fallback: "Spd",
        icon_id: img015::ICON_MOVE_SPEED,
        talent_id: 27,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.speed += v1),
    },
    CatAbilityDef {
        name: "Attack Buff",
        fallback: "Atk+",
        icon_id: img015::ICON_ATTACK_BUFF,
        talent_id: 31,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| {
            let factor = (100 + v1) as f32 / 100.0;
            c.attack_1 = (c.attack_1 as f32 * factor).round() as i32;
            c.attack_2 = (c.attack_2 as f32 * factor).round() as i32;
            c.attack_3 = (c.attack_3 as f32 * factor).round() as i32;
        }),
    },
    CatAbilityDef {
        name: "Health Buff",
        fallback: "HP+",
        icon_id: img015::ICON_HEALTH_BUFF,
        talent_id: 32,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| {
            let factor = (100 + v1) as f32 / 100.0;
            c.hitpoints = (c.hitpoints as f32 * factor).round() as i32;
        }),
    },
    CatAbilityDef {
        name: "TBA Down",
        fallback: "TBA-",
        icon_id: img015::ICON_TBA_DOWN,
        talent_id: 61,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| {
             let reduction = (c.time_before_attack_1 as f32 * v1 as f32 / 100.0).round() as i32;
             c.time_before_attack_1 = c.time_before_attack_1.saturating_sub(reduction);
        }),
    },
    CatAbilityDef {
        name: "Improve Knockbacks",
        fallback: "KB+",
        icon_id: img015::ICON_IMPROVE_KNOCKBACK_COUNT,
        talent_id: 28,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_c| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|c, v1, _, _| c.knockbacks += v1),
    },
];

// --- STATS REGISTRY ---

pub struct CatStatsDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub get_value: fn(&CatRaw, i32) -> i32, 
    pub formatter: fn(i32) -> String,
    pub linked_talent_id: Option<u8>,
    pub talent_modifier_fmt: Option<fn(i32, i32) -> String>,
}

pub const CAT_STATS_REGISTRY: &[CatStatsDef] = &[
    CatStatsDef {
        name: "Hitpoints",
        display_name: "Hitpoints",
        get_value: |c, _| c.hitpoints,
        formatter: |val| format!("{}", val),
        linked_talent_id: Some(32),
        talent_modifier_fmt: Some(|v1, _| format!("(+{}%)", v1)),
    },
    CatStatsDef {
        name: "Knockbacks",
        display_name: "Knockback",
        get_value: |c, _| c.knockbacks,
        formatter: |val| format!("{}", val),
        linked_talent_id: Some(28),
        talent_modifier_fmt: Some(|v1, _| format!("(+{})", v1)),
    },
    CatStatsDef {
        name: "Speed",
        display_name: "Speed",
        get_value: |c, _| c.speed,
        formatter: |val| format!("{}", val),
        linked_talent_id: Some(27),
        talent_modifier_fmt: Some(|v1, _| format!("(+{})", v1)),
    },
    CatStatsDef {
        name: "Range",
        display_name: "Range",
        get_value: |c, _| c.standing_range,
        formatter: |val| format!("{}", val),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Attack",
        display_name: "Attack",
        get_value: |c, _| c.attack_1 + c.attack_2 + c.attack_3,
        formatter: |val| format!("{}", val),
        linked_talent_id: Some(31),
        talent_modifier_fmt: Some(|v1, _| format!("(+{}%)", v1)),
    },
    CatStatsDef {
        name: "Dps",
        display_name: "DPS",
        get_value: |c, anim_frames| {
            let total_atk = c.attack_1 + c.attack_2 + c.attack_3;
            let mut effective_foreswing = c.pre_attack_animation;
            if c.attack_3 > 0 && c.time_before_attack_3 > 0 { effective_foreswing = c.time_before_attack_3; } 
            else if c.attack_2 > 0 && c.time_before_attack_2 > 0 { effective_foreswing = c.time_before_attack_2; }
            let cooldown_frames = c.time_before_attack_1.saturating_sub(1);
            let cycle = (effective_foreswing + cooldown_frames).max(anim_frames);
            if cycle > 0 { ((total_atk as f32 * 30.0) / cycle as f32).round() as i32 } else { 0 }
        },
        formatter: |val| format!("{}", val),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Atk Cycle",
        display_name: "Atk Cycle",
        get_value: |c, anim_frames| {
            let mut effective_foreswing = c.pre_attack_animation;
            if c.attack_3 > 0 && c.time_before_attack_3 > 0 { effective_foreswing = c.time_before_attack_3; } 
            else if c.attack_2 > 0 && c.time_before_attack_2 > 0 { effective_foreswing = c.time_before_attack_2; }
            let cooldown_frames = c.time_before_attack_1.saturating_sub(1);
            (effective_foreswing + cooldown_frames).max(anim_frames)
        },
        formatter: |val| format!("{}f", val), 
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Atk Type",
        display_name: "Atk Type",
        get_value: |c, _| c.area_attack,
        formatter: |val| if val == 0 { "Single".to_string() } else { "Area".to_string() },
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Cost",
        display_name: "Cost",
        get_value: |c, _| (c.eoc1_cost as f32 * 1.5).round() as i32,
        formatter: |val| format!("{}¢", val),
        linked_talent_id: Some(25),
        talent_modifier_fmt: Some(|v1, _| format!("(-{}¢)", (v1 as f32 * 1.5).round() as i32)),
    },
    CatStatsDef {
        name: "Cooldown",
        display_name: "Cooldown",
        get_value: |c, _| (c.cooldown - 264).max(60),
        formatter: |val| format!("{:.2}s^{}f", val as f32 / 30.0, val),
        linked_talent_id: Some(26),
        talent_modifier_fmt: Some(|v1, _| format!("(-{}f)", v1)),
    },
    CatStatsDef {
        name: "TBA",
        display_name: "TBA",
        get_value: |c, _| c.time_before_attack_1,
        formatter: |val| format!("{}f", val),
        linked_talent_id: Some(61),
        talent_modifier_fmt: Some(|v1, _| format!("(-{}%)", v1)),
    },
];

// --- REGISTRY HELPER FUNCTIONS ---

pub fn get_cat_stat(name: &str) -> &'static CatStatsDef {
    CAT_STATS_REGISTRY.iter().find(|s| s.name == name).expect("Stat not found in registry")
}

pub fn format_cat_stat(name: &str, stats: &CatRaw, anim_frames: i32) -> String {
    let def = get_cat_stat(name);
    (def.formatter)((def.get_value)(stats, anim_frames))
}

pub fn get_by_talent_id(id: u8) -> Option<&'static CatAbilityDef> {
    CAT_ABILITY_REGISTRY.iter().find(|def| def.talent_id == id)
}

pub fn get_fallback_by_icon(icon_id: usize) -> &'static str {
    CAT_ABILITY_REGISTRY.iter().find(|def| def.icon_id == icon_id).map(|def| def.fallback).unwrap_or("???")
}