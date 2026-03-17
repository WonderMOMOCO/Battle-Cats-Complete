use crate::global::game::img015;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::global::game::abilities::CustomIcon;

#[derive(PartialEq, Clone, Copy)]
pub enum DisplayGroup {
    Type,      
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

pub struct EnemyAbilityDef {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon_id: usize,
    pub group: DisplayGroup,
    pub custom_icon: CustomIcon,
    pub schema: &'static [(&'static str, AttrUnit)],
    pub get_attributes: fn(&EnemyRaw) -> Vec<(&'static str, i32, AttrUnit)>,
    pub formatter: fn(val: i32, stats: &EnemyRaw, duration_frames: i32, magnification: i32) -> String,
    pub minus_one_is_inf: bool,
}

// --- FORMATTERS ---
fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_range(min: i32, max: i32) -> String {
    if min == max { format!("at {}", min) } else { format!("between {}~{}", min, max) }
}

fn fmt_count(val: i32) -> String {
    match val {
        -1 => "infinitely".to_string(),
        1 => "1 time".to_string(),
        _ => format!("{} times", val),
    }
}

fn fmt_effective_range(e: &EnemyRaw) -> String {
    let enemy_base_range = {
        let start_range = e.long_distance_anchor_1;
        let end_range = e.long_distance_anchor_1 + e.long_distance_span_1;
        let (min_reach, max_reach) = if start_range < end_range { (start_range, end_range) } else { (end_range, start_range) };
        if min_reach > 0 { min_reach } else { max_reach }
    };

    let mut range_strings = Vec::new();
    let range_checks = [
        (e.long_distance_anchor_1, e.long_distance_span_1, 1),
        (e.long_distance_2_anchor, e.long_distance_2_span, e.long_distance_2_flag),
        (e.long_distance_3_anchor, e.long_distance_3_span, e.long_distance_3_flag),
    ];
    
    for (anchor, span, flag) in range_checks {
        if flag > 0 && span != 0 {
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
    format!("{} {}\nStands at {} Range relative to Cat Base", label_prefix, range_strings.join(" / "), enemy_base_range)
}

fn fmt_multihit(e: &EnemyRaw) -> String {
    let ability_flag_1 = if e.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if e.attack_2_abilities > 0 { "True" } else { "False" };
    let ability_flag_3 = if e.attack_3 > 0 { if e.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
    let damage_string = if e.attack_3 > 0 { 
        format!("{} / {} / {}", e.attack_1, e.attack_2, e.attack_3) 
    } else { 
        format!("{} / {}", e.attack_1, e.attack_2) 
    };
    format!("Damage split {}\nAbility split {} / {}{}", damage_string, ability_flag_1, ability_flag_2, ability_flag_3)
}

pub const ENEMY_ABILITY_REGISTRY: &[EnemyAbilityDef] = &[
    // --- SPECIAL HIDDEN ---
    EnemyAbilityDef {
        name: "Single Attack",
        fallback: "Sngl",
        icon_id: img015::ICON_SINGLE_ATTACK,
        group: DisplayGroup::Hidden,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.area_attack == 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Area Attack",
        fallback: "Area",
        icon_id: img015::ICON_AREA_ATTACK,
        group: DisplayGroup::Hidden,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.area_attack == 1 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "".into(),
        minus_one_is_inf: false,
    },

    // --- TYPES ---
    EnemyAbilityDef {
        name: "Red",
        fallback: "Red",
        icon_id: img015::ICON_TRAIT_RED,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_red > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Red".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Floating",
        fallback: "Float",
        icon_id: img015::ICON_TRAIT_FLOATING,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_floating > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Floating".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Black",
        fallback: "Black",
        icon_id: img015::ICON_TRAIT_BLACK,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_black > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Black".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Metal",
        fallback: "Metal",
        icon_id: img015::ICON_TRAIT_METAL,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_metal > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Metal".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Angel",
        fallback: "Angel",
        icon_id: img015::ICON_TRAIT_ANGEL,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_angel > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Angel".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Alien",
        fallback: "Alien",
        icon_id: img015::ICON_TRAIT_ALIEN,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_alien > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Alien".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Zombie",
        fallback: "Zomb",
        icon_id: img015::ICON_TRAIT_ZOMBIE,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_zombie > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Zombie".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Relic",
        fallback: "Relic",
        icon_id: img015::ICON_TRAIT_RELIC,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_relic > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Relic".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Aku",
        fallback: "Aku",
        icon_id: img015::ICON_TRAIT_AKU,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_aku > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Aku".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Traitless",
        fallback: "White",
        icon_id: img015::ICON_TRAIT_TRAITLESS,
        group: DisplayGroup::Type,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_traitless > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Traitless".into(),
        minus_one_is_inf: false,
    },

    // --- HEADLINE 1 ---
    EnemyAbilityDef {
        name: "Dojo",
        fallback: "Dojo",
        icon_id: img015::ICON_DOJO,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::Dojo,
        schema: &[],
        get_attributes: |e| if e.type_dojo > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Dojo".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Starred Alien",
        fallback: "Star",
        icon_id: img015::ICON_STARRED_ALIEN, 
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::StarredAlien,
        schema: &[],
        get_attributes: |e| if e.type_starred_alien > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Starred Alien".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Colossus",
        fallback: "Colos",
        icon_id: img015::ICON_COLOSSUS,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_colossus > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Colossus Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Behemoth",
        fallback: "Behem",
        icon_id: img015::ICON_BEHEMOTH,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_behemoth > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Behemoth Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Sage",
        fallback: "Sage",
        icon_id: img015::ICON_SAGE,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_sage > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Sage Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Supervillain",
        fallback: "Villn",
        icon_id: img015::ICON_SUPERVILLIAN,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_supervillain > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Supervillain Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Witch",
        fallback: "Witch",
        icon_id: img015::ICON_WITCH,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_witch > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Witch Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "EVA Angel",
        fallback: "EVA",
        icon_id: img015::ICON_EVA,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.type_eva > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "EVA Angel".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Kamikaze", 
        fallback: "Kamik", 
        icon_id: img015::ICON_KAMIKAZE,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::Kamikaze, 
        schema: &[
            ("Attacks", AttrUnit::None)
        ],
        get_attributes: |e| {
            if e.attack_count_total > -1 && e.attack_count_state == 2 { 
                vec![("Attacks", e.attack_count_total, AttrUnit::None)] 
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
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Stop", 
        fallback: "Stop", 
        icon_id: img015::ICON_STOP,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::Stop, 
        schema: &[
            ("Attacks", AttrUnit::None)
        ],
        get_attributes: |e| {
            if e.attack_count_total > -1 && e.attack_count_state == 0 { 
                vec![("Attacks", e.attack_count_total, AttrUnit::None)] 
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
        minus_one_is_inf: false,
    },

    // --- HEADLINE 2 ---
    EnemyAbilityDef {
        name: "Base Destroyer",
        fallback: "BaseD",
        icon_id: img015::ICON_BASE_DESTROYER,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.base_destroyer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Deals 4× Damage to the Cat Base".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Wave Block",
        fallback: "W-Blk",
        icon_id: img015::ICON_WAVE_BLOCK,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| {
            if e.wave_blocker > 0 {
                vec![("Active", 1, AttrUnit::None)]
            } else {
                vec![]
            }
        },
        formatter: |_, _, _, _| {
            "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into()
        },
        minus_one_is_inf: false,
    },

    // --- BODY 1 ---
    EnemyAbilityDef {
        name: "Multi-Hit",
        fallback: "Multi",
        icon_id: img015::ICON_MULTIHIT,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::Multihit,
        schema: &[],
        get_attributes: |e| if e.attack_2 > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, e, _, _| fmt_multihit(e),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Long Distance",
        fallback: "LD",
        icon_id: img015::ICON_LONG_DISTANCE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| {
            let ranges = [
                (e.long_distance_anchor_1, e.long_distance_span_1, 1),
                (e.long_distance_2_anchor, e.long_distance_2_span, e.long_distance_2_flag),
                (e.long_distance_3_anchor, e.long_distance_3_span, e.long_distance_3_flag),
            ];
            let mut has_range = false;
            let mut is_omni = false;
            for (anchor, span, flag) in ranges {
                if flag > 0 && span != 0 {
                    has_range = true;
                    let min = anchor.min(anchor + span);
                    if min <= 0 { is_omni = true; }
                }
            }
            if has_range && !is_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_, e, _, _| fmt_effective_range(e),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Omni Strike",
        fallback: "Omni",
        icon_id: img015::ICON_OMNI_STRIKE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| {
            let ranges = [
                (e.long_distance_anchor_1, e.long_distance_span_1, 1),
                (e.long_distance_2_anchor, e.long_distance_2_span, e.long_distance_2_flag),
                (e.long_distance_3_anchor, e.long_distance_3_span, e.long_distance_3_flag),
            ];
            let mut is_omni = false;
            for (anchor, span, flag) in ranges {
                if flag > 0 && span != 0 {
                    let min = anchor.min(anchor + span);
                    if min <= 0 { is_omni = true; }
                }
            }
            if is_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_, e, _, _| fmt_effective_range(e),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Wave Attack",
        fallback: "Wave",
        icon_id: img015::ICON_WAVE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
        ],
        get_attributes: |e| {
            if e.mini_wave == 0 && e.wave_chance > 0 { 
                let reach = (467.5 + ((e.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", e.wave_chance, AttrUnit::Percent), 
                    ("Level", e.wave_level, AttrUnit::None),
                    ("Max Reach", reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let reach = 467.5 + ((e.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", val, e.wave_level, reach)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Mini-Wave",
        fallback: "MiniW",
        icon_id: img015::ICON_MINI_WAVE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
        ],
        get_attributes: |e| {
            if e.mini_wave > 0 && e.wave_chance > 0 { 
                let reach = (467.5 + ((e.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", e.wave_chance, AttrUnit::Percent), 
                    ("Level", e.wave_level, AttrUnit::None),
                    ("Max Reach", reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let reach = 467.5 + ((e.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", val, e.wave_level, reach)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Surge Attack",
        fallback: "Surge",
        icon_id: img015::ICON_SURGE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |e| {
            if e.mini_surge == 0 && e.surge_chance > 0 { 
                vec![
                    ("Chance", e.surge_chance, AttrUnit::Percent), 
                    ("Level", e.surge_level, AttrUnit::None), 
                    ("Min Range", e.surge_spawn_min, AttrUnit::Range), 
                    ("Max Range", e.surge_spawn_min + e.surge_spawn_max, AttrUnit::Range),
                    ("Width", e.surge_spawn_max, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let start = e.surge_spawn_min;
            let end = e.surge_spawn_min + e.surge_spawn_max;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Surge\n{} Range", val, e.surge_level, fmt_range(min, max))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Mini-Surge",
        fallback: "MiniS",
        icon_id: img015::ICON_MINI_SURGE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |e| {
            if e.mini_surge > 0 && e.surge_chance > 0 { 
                vec![
                    ("Chance", e.surge_chance, AttrUnit::Percent), 
                    ("Level", e.surge_level, AttrUnit::None), 
                    ("Min Range", e.surge_spawn_min, AttrUnit::Range), 
                    ("Max Range", e.surge_spawn_min + e.surge_spawn_max, AttrUnit::Range),
                    ("Width", e.surge_spawn_max, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let start = e.surge_spawn_min;
            let end = e.surge_spawn_min + e.surge_spawn_max;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Mini-Surge\n{} Range", val, e.surge_level, fmt_range(min, max))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Death Surge",
        fallback: "DSurg",
        icon_id: img015::ICON_DEATH_SURGE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |e| {
            if e.death_surge_chance > 0 {
                vec![
                    ("Chance", e.death_surge_chance, AttrUnit::Percent), 
                    ("Level", e.death_surge_level, AttrUnit::None), 
                    ("Min Range", e.death_surge_spawn_min, AttrUnit::Range), 
                    ("Max Range", e.death_surge_spawn_min + e.death_surge_spawn_max, AttrUnit::Range),
                    ("Width", e.death_surge_spawn_max, AttrUnit::Range),
                ]
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let start = e.death_surge_spawn_min;
            let end = e.death_surge_spawn_min + e.death_surge_spawn_max;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create a Level {} Surge\n{} Range upon death", val, e.death_surge_level, fmt_range(min, max))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Explosion",
        fallback: "Expl",
        icon_id: img015::ICON_EXPLOSION,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |e| {
            if e.explosion_chance > 0 {
                vec![
                    ("Chance", e.explosion_chance, AttrUnit::Percent), 
                    ("Min Range", e.explosion_anchor, AttrUnit::Range), 
                    ("Max Range", e.explosion_anchor + e.explosion_span, AttrUnit::Range),
                    ("Width", e.explosion_span, AttrUnit::Range),
                ]
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let start = e.explosion_anchor;
            let end = e.explosion_anchor + e.explosion_span;
            let (min, max) = if start < end { (start, end) } else { (end, start) };
            format!("{}% Chance to create an Explosion {} Range", val, fmt_range(min, max))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Critical Hit",
        fallback: "Crit",
        icon_id: img015::ICON_CRITICAL_HIT,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.critical_chance > 0 { 
                vec![
                    ("Chance", e.critical_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to perform a Critical Hit dealing 2× Damage\nCritcal Hits bypass Metal resistance", val),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Savage Blow",
        fallback: "Savge",
        icon_id: img015::ICON_SAVAGE_BLOW,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Boost", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.savage_blow_chance > 0 { 
                vec![
                    ("Chance", e.savage_blow_chance, AttrUnit::Percent), 
                    ("Boost", e.savage_blow_boost, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| {
            let mult = (e.savage_blow_boost as f32 + 100.0) / 100.0;
            format!("{}% Chance to perform a Savage Blow dealing {}× Damage", val, mult)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Strengthen",
        fallback: "Str+",
        icon_id: img015::ICON_STRENGTHEN,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("HP", AttrUnit::Percent), 
            ("Boost", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.strengthen_threshold > 0 { 
                vec![
                    ("HP", e.strengthen_threshold, AttrUnit::Percent), 
                    ("Boost", e.strengthen_boost, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |_, e, _, _| format!("Damage dealt increases by +{}% when reduced to {}% HP", e.strengthen_boost, e.strengthen_threshold),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Survive",
        fallback: "Surv",
        icon_id: img015::ICON_SURVIVE,
        group: DisplayGroup::Body1,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.survive_chance > 0 { 
                vec![
                    ("Chance", e.survive_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to Survive a lethal strike", val),
        minus_one_is_inf: false,
    },

    // --- BODY 2 ---
    EnemyAbilityDef {
        name: "Barrier",
        fallback: "Barri",
        icon_id: img015::ICON_BARRIER,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Hitpoints", AttrUnit::None)
        ],
        get_attributes: |e| {
            if e.barrier_hitpoints > 0 { 
                vec![
                    ("Hitpoints", e.barrier_hitpoints, AttrUnit::None),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("Has a Barrier with {} HP", val),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Aku Shield",
        fallback: "Shiel",
        icon_id: img015::ICON_SHIELD,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Hitpoints", AttrUnit::None), 
            ("Regen", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.shield_hitpoints > 0 { 
                vec![
                    ("Hitpoints", e.shield_hitpoints, AttrUnit::None), 
                    ("Regen", e.shield_regen, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, mag| {
            let scaled_hp = (val as f32 * (mag as f32 / 100.0)).round() as i32;
            if e.shield_regen > 0 {
                format!("Has a Shield with {} HP\nShield regenerates {}% HP when knocked back", scaled_hp, e.shield_regen)
            } else {
                format!("Has a Shield with {} HP", scaled_hp)
            }
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Burrow",
        fallback: "Burro",
        icon_id: img015::ICON_BURROW, 
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::Burrow,
        schema: &[
            ("Count", AttrUnit::None), 
            ("Distance", AttrUnit::Range)
        ],
        get_attributes: |e| {
            if e.burrow_amount != 0 { 
                vec![
                    ("Count", e.burrow_amount, AttrUnit::None), 
                    ("Distance", e.burrow_distance, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| format!("Burrows {} Range {}", e.burrow_distance, fmt_count(val)),
        minus_one_is_inf: true,
    },
    EnemyAbilityDef {
        name: "Revive",
        fallback: "Reviv",
        icon_id: img015::ICON_REVIVE, 
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::Revive,
        schema: &[
            ("Count", AttrUnit::None), 
            ("Duration", AttrUnit::Frames), 
            ("Hitpoints", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.revive_count != 0 { 
                vec![
                    ("Count", e.revive_count, AttrUnit::None), 
                    ("Duration", e.revive_time, AttrUnit::Frames), 
                    ("Hitpoints", e.revive_hp, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| format!("Revives {} with {}% HP after {} \nDoesn't revive if Z-Killed", fmt_count(val), e.revive_hp, fmt_time(e.revive_time)),
        minus_one_is_inf: true,
    },
    EnemyAbilityDef {
        name: "Toxic",
        fallback: "Toxic",
        icon_id: img015::ICON_TOXIC,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Damage", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.toxic_chance > 0 { 
                vec![
                    ("Chance", e.toxic_chance, AttrUnit::Percent), 
                    ("Damage", e.toxic_damage, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, _, _| format!("{}% Chance to deal {}% of a\nCat's Max HP in additional damage", val, e.toxic_damage),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Dodge",
        fallback: "Dodge",
        icon_id: img015::ICON_DODGE,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |e| {
            if e.dodge_chance > 0 { 
                vec![
                    ("Chance", e.dodge_chance, AttrUnit::Percent), 
                    ("Duration", e.dodge_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, dur, _| format!("{}% Chance to Dodge attacks for {}", val, fmt_time(dur)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Weaken",
        fallback: "Weak",
        icon_id: img015::ICON_WEAKEN,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Reduced To", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |e| {
            if e.weaken_chance > 0 { 
                vec![
                    ("Chance", e.weaken_chance, AttrUnit::Percent), 
                    ("Reduced To", e.weaken_percent, AttrUnit::Percent), 
                    ("Duration", e.weaken_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, dur, _| format!("{}% Chance to weaken Cats\nto {}% Attack Power for {}", val, e.weaken_percent, fmt_time(dur)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Freeze",
        fallback: "Freez",
        icon_id: img015::ICON_FREEZE,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |e| {
            if e.freeze_chance > 0 { 
                vec![
                    ("Chance", e.freeze_chance, AttrUnit::Percent), 
                    ("Duration", e.freeze_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, dur, _| format!("{}% Chance to Freeze Cats for {}", val, fmt_time(dur)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Slow",
        fallback: "Slow",
        icon_id: img015::ICON_SLOW,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |e| {
            if e.slow_chance > 0 { 
                vec![
                    ("Chance", e.slow_chance, AttrUnit::Percent), 
                    ("Duration", e.slow_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, dur, _| format!("{}% Chance to Slow Cats for {}", val, fmt_time(dur)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Knockback",
        fallback: "KB",
        icon_id: img015::ICON_KNOCKBACK,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |e| {
            if e.knockback_chance > 0 { 
                vec![
                    ("Chance", e.knockback_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, _, _| format!("{}% Chance to Knockback Cats", val),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Curse",
        fallback: "Curse",
        icon_id: img015::ICON_CURSE,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |e| {
            if e.curse_chance > 0 { 
                vec![
                    ("Chance", e.curse_chance, AttrUnit::Percent), 
                    ("Duration", e.curse_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, _, dur, _| format!("{}% Chance to Curse Cats for {}", val, fmt_time(dur)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Warp",
        fallback: "Warp",
        icon_id: img015::ICON_WARP,
        group: DisplayGroup::Body2,
        custom_icon: CustomIcon::None,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames), 
            ("Min Distance", AttrUnit::Range), 
            ("Max Distance", AttrUnit::Range)
        ],
        get_attributes: |e| {
            if e.warp_chance > 0 { 
                vec![
                    ("Chance", e.warp_chance, AttrUnit::Percent), 
                    ("Duration", e.warp_duration, AttrUnit::Frames), 
                    ("Min Distance", e.warp_distance_min, AttrUnit::Range), 
                    ("Max Distance", e.warp_distance_max, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |val, e, dur, _| format!("{}% Chance to Warp Cats {}~{} Range for {}", val, e.warp_distance_min, e.warp_distance_max, fmt_time(dur)),
        minus_one_is_inf: false,
    },
    
    // --- FOOTER ---
    EnemyAbilityDef { 
        name: "Immune Wave", 
        fallback: "NoWav", 
        icon_id: img015::ICON_IMMUNE_WAVE, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.wave_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Wave Attacks".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Surge", 
        fallback: "NoSrg", 
        icon_id: img015::ICON_IMMUNE_SURGE, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.surge_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Surge Attacks".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Explosion", 
        fallback: "NoExp", 
        icon_id: img015::ICON_IMMUNE_EXPLOSION, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.explosion_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Explosions".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Weaken", 
        fallback: "NoWk", 
        icon_id: img015::ICON_IMMUNE_WEAKEN, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.weaken_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Weaken".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Freeze", 
        fallback: "NoFrz", 
        icon_id: img015::ICON_IMMUNE_FREEZE, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.freeze_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Freeze".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Slow", 
        fallback: "NoSlw", 
        icon_id: img015::ICON_IMMUNE_SLOW, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.slow_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Slow".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Knockback", 
        fallback: "NoKB", 
        icon_id: img015::ICON_IMMUNE_KNOCKBACK, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.knockback_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Knockback".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Curse", 
        fallback: "NoCur", 
        icon_id: img015::ICON_IMMUNE_CURSE, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.curse_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Curse".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Warp", 
        fallback: "NoWrp", 
        icon_id: img015::ICON_IMMUNE_WARP, 
        group: DisplayGroup::Footer, 
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.warp_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Immune to Warp".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Counter Surge",
        fallback: "C-Srg",
        icon_id: img015::ICON_COUNTER_SURGE,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |e| if e.counter_surge > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_, _| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        minus_one_is_inf: false,
    },
];

// --- STATS REGISTRY ---
pub struct EnemyStatsDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub get_value: fn(&EnemyRaw, i32, i32) -> i32, 
    pub formatter: fn(i32) -> String,       
}

pub const ENEMY_STATS_REGISTRY: &[EnemyStatsDef] = &[
    EnemyStatsDef {
        name: "Hitpoints",
        display_name: "Hitpoints",
        get_value: |e, _, mag| (e.hitpoints as f32 * (mag as f32 / 100.0)).round() as i32,
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Knockbacks",
        display_name: "Knockback",
        get_value: |e, _, _| e.knockbacks,
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Speed",
        display_name: "Speed",
        get_value: |e, _, _| e.speed,
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Range",
        display_name: "Range",
        get_value: |e, _, _| e.standing_range,
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Attack",
        display_name: "Attack",
        get_value: |e, _, mag| {
            let mag_f = mag as f32 / 100.0;
            let a1 = (e.attack_1 as f32 * mag_f).round() as i32;
            let a2 = (e.attack_2 as f32 * mag_f).round() as i32;
            let a3 = (e.attack_3 as f32 * mag_f).round() as i32;
            a1 + a2 + a3
        },
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Dps",
        display_name: "DPS",
        get_value: |e, anim_frames, mag| {
            let mag_f = mag as f32 / 100.0;
            let a1 = (e.attack_1 as f32 * mag_f).round() as i32;
            let a2 = (e.attack_2 as f32 * mag_f).round() as i32;
            let a3 = (e.attack_3 as f32 * mag_f).round() as i32;
            let total_atk = a1 + a2 + a3;
            
            let mut effective_foreswing = e.pre_attack_animation;
            if e.attack_3 > 0 && e.time_before_attack_3 > 0 {
                effective_foreswing = e.time_before_attack_3;
            } else if e.attack_2 > 0 && e.time_before_attack_2 > 0 {
                effective_foreswing = e.time_before_attack_2;
            }
            let cooldown_frames = e.time_before_attack_1.saturating_sub(1);
            let cycle = (effective_foreswing + cooldown_frames).max(anim_frames);

            if cycle > 0 { ((total_atk as f32 * 30.0) / cycle as f32).round() as i32 } else { 0 }
        },
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Atk Cycle",
        display_name: "Atk Cycle",
        get_value: |e, anim_frames, _| {
            let mut effective_foreswing = e.pre_attack_animation;
            if e.attack_3 > 0 && e.time_before_attack_3 > 0 {
                effective_foreswing = e.time_before_attack_3;
            } else if e.attack_2 > 0 && e.time_before_attack_2 > 0 {
                effective_foreswing = e.time_before_attack_2;
            }
            let cooldown_frames = e.time_before_attack_1.saturating_sub(1);
            (effective_foreswing + cooldown_frames).max(anim_frames)
        },
        formatter: |val| format!("{}f", val), 
    },
    EnemyStatsDef {
        name: "Atk Type",
        display_name: "Atk Type",
        get_value: |e, _, _| e.area_attack,
        formatter: |val| if val == 0 { "Single".to_string() } else { "Area".to_string() },
    },
    EnemyStatsDef {
        name: "Endure",
        display_name: "Endure",
        get_value: |e, _, mag| {
            let hp = (e.hitpoints as f32 * (mag as f32 / 100.0)).round() as i32;
            if e.knockbacks > 0 { (hp as f32 / e.knockbacks as f32).round() as i32 } else { hp }
        },
        formatter: |val| format!("{}", val),
    },
    EnemyStatsDef {
        name: "Cash Drop",
        display_name: "Cash Drop",
        get_value: |e, _, _| (e.cash_drop as f32 * 3.95).floor() as i32,
        formatter: |val| format!("{}¢", val),
    },
];

// --- REGISTRY HELPER FUNCTIONS ---
pub fn get_enemy_stat(name: &str) -> &'static EnemyStatsDef {
    ENEMY_STATS_REGISTRY.iter().find(|s| s.name == name).expect("Stat not found in registry")
}

pub fn format_enemy_stat(name: &str, stats: &EnemyRaw, anim_frames: i32, mag: i32) -> String {
    let def = get_enemy_stat(name);
    (def.formatter)((def.get_value)(stats, anim_frames, mag))
}

pub fn get_fallback_by_icon(icon_id: usize) -> &'static str {
    ENEMY_ABILITY_REGISTRY.iter().find(|def| def.icon_id == icon_id).map(|def| def.fallback).unwrap_or("???")
}