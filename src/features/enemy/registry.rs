use crate::global::game::img015;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::global::game::abilities::CustomIcon;
use crate::global::game::param::Param;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Magnification {
    pub hitpoints: i32,
    pub attack: i32,
}

impl Default for Magnification {
    fn default() -> Self {
        Self { hitpoints: 100, attack: 100 }
    }
}

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

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub enum AbilityIcon {
    Standard(usize),
    Custom(CustomIcon),
}

pub struct EnemyAbilityDef {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon: AbilityIcon,
    pub group: DisplayGroup,
    pub schema: &'static [(&'static str, AttrUnit)],
    pub get_attributes: fn(&EnemyRaw) -> Vec<(&'static str, i32, AttrUnit)>,
    pub formatter: fn(primary_value: i32, stats: &EnemyRaw, duration_frames: i32, magnification: Magnification, param: &Param) -> String,
    pub minus_one_is_inf: bool,
}

// --- FORMATTERS ---
fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_range(min_range: i32, max_range: i32) -> String {
    if min_range == max_range { format!("at {}", min_range) } else { format!("between {}~{}", min_range, max_range) }
}

fn fmt_compress(min_val: i32, max_val: i32) -> String {
    if min_val == max_val { format!("{}", min_val) } else { format!("{}~{}", min_val, max_val) }
}

fn fmt_count(count: i32) -> String {
    match count {
        -1 => "infinitely".to_string(),
        1 => "1 time".to_string(),
        _ => format!("{} times", count),
    }
}

fn fmt_effective_range(stats: &EnemyRaw) -> String {
    // Standing distance is ALWAYS dictated by Hit 1
    let primary_anchor = if stats.long_distance_anchor_1 != 0 { 
        stats.long_distance_anchor_1 
    } else { 
        stats.standing_range 
    };

    let mut range_strings = Vec::new();
    
    // Does the unit have LD or Omni on ANY hit?
    let has_ld_or_omni = (stats.long_distance_span_1 != 0 || stats.long_distance_anchor_1 != 0) ||
                         (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span != 0 || stats.long_distance_2_anchor != 0)) ||
                         (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span != 0 || stats.long_distance_3_anchor != 0));

    // ONLY populate the split strings if this is an LD/Omni unit
    if has_ld_or_omni {
        let hit_data = [
            (true, stats.long_distance_anchor_1, stats.long_distance_span_1, 1),
            (stats.attack_2 > 0, stats.long_distance_2_anchor, stats.long_distance_2_span, stats.long_distance_2_flag),
            (stats.attack_3 > 0, stats.long_distance_3_anchor, stats.long_distance_3_span, stats.long_distance_3_flag),
        ];
        
        for (is_active, anchor, span, flag) in hit_data {
            if is_active {
                // If it's an active LD/Omni hit...
                if flag > 0 && (span != 0 || anchor != 0) {
                    let start = anchor;
                    let end = anchor + span;
                    let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                    range_strings.push(format!("{}~{}", min_r, max_r));
                } else {
                    // It's a standard hit! Show its true reach using hitbox_width
                    range_strings.push(format!("{}~{}", -stats.hitbox_width, stats.standing_range));
                }
            }
        }
    }

    // ONLY merge if ALL hits are exactly the same
    if range_strings.len() > 1 {
        let first_string = range_strings[0].clone();
        if range_strings.iter().all(|s| s == &first_string) {
            range_strings.truncate(1);
        }
    }

    let label_prefix = if range_strings.len() > 1 { "Range split" } else { "Effective Range" };
    format!("{} {}\nStands at {} Range relative to Cat Base", label_prefix, range_strings.join(" / "), primary_anchor)
}

fn fmt_multihit(stats: &EnemyRaw) -> String {
    let ability_flag_1 = if stats.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if stats.attack_2_abilities > 0 { "True" } else { "False" };
    let ability_flag_3 = if stats.attack_3 > 0 { if stats.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
    let damage_string = if stats.attack_3 > 0 { 
        format!("{} / {} / {}", stats.attack_1, stats.attack_2, stats.attack_3) 
    } else { 
        format!("{} / {}", stats.attack_1, stats.attack_2) 
    };
    format!("Damage split {}\nAbility split {} / {}{}", damage_string, ability_flag_1, ability_flag_2, ability_flag_3)
}

fn fmt_sage(param: &Param) -> String {
    let mut resistance_groups_by_percentage: HashMap<i32, Vec<&str>> = HashMap::new();

    let to_percentage = |multiplier: f32| (multiplier * 100.0).round() as i32;

    resistance_groups_by_percentage.entry(to_percentage(param.sage_type_resist_weaken)).or_default().push("Weaken");
    resistance_groups_by_percentage.entry(to_percentage(param.sage_type_resist_freeze)).or_default().push("Freeze");
    resistance_groups_by_percentage.entry(to_percentage(param.sage_type_resist_slow)).or_default().push("Slow");
    resistance_groups_by_percentage.entry(to_percentage(param.sage_type_resist_curse)).or_default().push("Curse");
    resistance_groups_by_percentage.entry(to_percentage(param.sage_type_resist_knockback)).or_default().push("Knockback");

    let base_description = "Crowd Control effects inflicted upon Sage Enemies are reduced by";

    if resistance_groups_by_percentage.len() == 1 {
        let (percentage, _) = resistance_groups_by_percentage.into_iter().next().unwrap();
        format!("{} {}%", base_description, percentage)
    } else {
        let mut formatted_resistance_lines = Vec::new();
        let mut sorted_resistance_groups: Vec<_> = resistance_groups_by_percentage.into_iter().collect();
        
        // Sort highest percentage first
        sorted_resistance_groups.sort_by(|group_a, group_b| group_b.0.cmp(&group_a.0)); 

        for (percentage, effect_names) in sorted_resistance_groups {
            let formatted_effect_list = match effect_names.len() {
                1 => effect_names[0].to_string(),
                2 => format!("{} and {}", effect_names[0], effect_names[1]),
                _ => {
                    let all_effects_except_last = effect_names[..effect_names.len() - 1].join(", ");
                    format!("{}, and {}", all_effects_except_last, effect_names.last().unwrap())
                }
            };
            formatted_resistance_lines.push(format!("{}% for {}", percentage, formatted_effect_list));
        }
        format!("{}\n{}", base_description, formatted_resistance_lines.join("\n"))
    }
}

pub const ENEMY_ABILITY_REGISTRY: &[EnemyAbilityDef] = &[
    // --- SPECIAL HIDDEN ---
    EnemyAbilityDef {
        name: "Single Attack",
        fallback: "Sngl",
        icon: AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
        group: DisplayGroup::Hidden,
        schema: &[],
        get_attributes: |stats| if stats.area_attack == 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Area Attack",
        fallback: "Area",
        icon: AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
        group: DisplayGroup::Hidden,
        schema: &[],
        get_attributes: |stats| if stats.area_attack == 1 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "".into(),
        minus_one_is_inf: false,
    },

    // --- TYPES ---
    EnemyAbilityDef {
        name: "Red",
        fallback: "Red",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_RED),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_red > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Red".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Floating",
        fallback: "Float",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_FLOATING),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_floating > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Floating".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Black",
        fallback: "Black",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_BLACK),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_black > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Black".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Metal",
        fallback: "Metal",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_METAL),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_metal > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Metal".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Angel",
        fallback: "Angel",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_ANGEL),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_angel > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Angel".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Alien",
        fallback: "Alien",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_ALIEN),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_alien > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Alien".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Zombie",
        fallback: "Zomb",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_ZOMBIE),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_zombie > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Zombie".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Relic",
        fallback: "Relic",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_RELIC),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_relic > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Relic".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Aku",
        fallback: "Aku",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_AKU),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_aku > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Aku".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Traitless",
        fallback: "White",
        icon: AbilityIcon::Standard(img015::ICON_TRAIT_TRAITLESS),
        group: DisplayGroup::Type,
        schema: &[],
        get_attributes: |stats| if stats.type_traitless > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Traitless".into(),
        minus_one_is_inf: false,
    },

    // --- HEADLINE 1 ---
    EnemyAbilityDef {
        name: "Dojo",
        fallback: "Dojo",
        icon: AbilityIcon::Custom(CustomIcon::Dojo),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_dojo > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Dojo".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Starred Alien",
        fallback: "Star",
        icon: AbilityIcon::Custom(CustomIcon::StarredAlien),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_starred_alien == 1 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Starred Alien".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Cat God",
        fallback: "God",
        icon: AbilityIcon::Custom(CustomIcon::God),
        group: DisplayGroup::Headline1,
        schema: &[
            ("Type", AttrUnit::None)
        ],
        get_attributes: |stats| {
            if stats.type_starred_alien >= 2 && stats.type_starred_alien <= 4 { 
                vec![("Type", stats.type_starred_alien, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |type_val,_,_,_,_| format!("CotC {} Cat God", type_val - 1),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Colossus",
        fallback: "Colos",
        icon: AbilityIcon::Standard(img015::ICON_COLOSSUS),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_colossus > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Colossus Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Behemoth",
        fallback: "Behem",
        icon: AbilityIcon::Standard(img015::ICON_BEHEMOTH),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_behemoth > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Behemoth Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Sage",
        fallback: "Sage",
        icon: AbilityIcon::Standard(img015::ICON_SAGE),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_sage > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,param| fmt_sage(param),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Supervillain",
        fallback: "Villn",
        icon: AbilityIcon::Standard(img015::ICON_SUPERVILLIAN),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_supervillain > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Supervillain Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Witch",
        fallback: "Witch",
        icon: AbilityIcon::Standard(img015::ICON_WITCH),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_witch > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Witch Enemy".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "EVA Angel",
        fallback: "EVA",
        icon: AbilityIcon::Standard(img015::ICON_EVA),
        group: DisplayGroup::Headline1,
        schema: &[],
        get_attributes: |stats| if stats.type_eva > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "EVA Angel".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Kamikaze", 
        fallback: "Kamik", 
        icon: AbilityIcon::Custom(CustomIcon::Kamikaze),
        group: DisplayGroup::Headline2,
        schema: &[
            ("Attacks", AttrUnit::None)
        ],
        get_attributes: |stats| {
            if stats.attack_count_total > -1 && stats.attack_count_state == 2 { 
                vec![("Attacks", stats.attack_count_total, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |attacks,_,_,_,_| {
            let limit_suffix = match attacks {
                0 => "immediately".to_string(),
                1 => "after 1 attack".to_string(),
                n => format!("after {} attacks", n),
            };
            format!("Unit disappears {}", limit_suffix)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Stop", 
        fallback: "Stop", 
        icon: AbilityIcon::Custom(CustomIcon::Stop),
        group: DisplayGroup::Headline2,
        schema: &[
            ("Attacks", AttrUnit::None)
        ],
        get_attributes: |stats| {
            if stats.attack_count_total > -1 && stats.attack_count_state == 0 { 
                vec![("Attacks", stats.attack_count_total, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |attacks,_,_,_,_| {
            let limit_suffix = match attacks {
                0 => "immediately".to_string(),
                1 => "after 1 attack".to_string(),
                n => format!("after {} attacks", n),
            };
            format!("Unit stops moving {}", limit_suffix)
        },
        minus_one_is_inf: false,
    },

    // --- HEADLINE 2 ---
    EnemyAbilityDef {
        name: "Base Destroyer",
        fallback: "BaseD",
        icon: AbilityIcon::Standard(img015::ICON_BASE_DESTROYER),
        group: DisplayGroup::Headline2,
        schema: &[],
        get_attributes: |stats| if stats.base_destroyer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Deals 4× Damage to the Cat Base".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Wave Block",
        fallback: "W-Blk",
        icon: AbilityIcon::Standard(img015::ICON_WAVE_BLOCK),
        group: DisplayGroup::Headline2,
        schema: &[],
        get_attributes: |stats| {
            if stats.wave_blocker > 0 {
                vec![("Active", 1, AttrUnit::None)]
            } else {
                vec![]
            }
        },
        formatter: |_,_,_,_,_| {
            "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into()
        },
        minus_one_is_inf: false,
    },

    // --- BODY 1 ---
    EnemyAbilityDef {
        name: "Multi-Hit",
        fallback: "Multi",
        icon: AbilityIcon::Custom(CustomIcon::Multihit),
        group: DisplayGroup::Body1,
        schema: &[],
        get_attributes: |stats| if stats.attack_2 > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,stats,_,_,_| fmt_multihit(stats),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Long Distance",
        fallback: "LD",
        icon: AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
        group: DisplayGroup::Body1,
        schema: &[],
        get_attributes: |stats| {
            let has_omni = (stats.long_distance_span_1 < 0 || (stats.long_distance_span_1 == 0 && stats.long_distance_anchor_1 != 0)) ||
                           (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span < 0 || (stats.long_distance_2_span == 0 && stats.long_distance_2_anchor != 0))) ||
                           (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span < 0 || (stats.long_distance_3_span == 0 && stats.long_distance_3_anchor != 0)));

            let has_ld = (stats.long_distance_span_1 > 0) || 
                         (stats.long_distance_2_flag > 0 && stats.long_distance_2_span > 0) || 
                         (stats.long_distance_3_flag > 0 && stats.long_distance_3_span > 0);

            if has_ld && !has_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_,stats,_,_,_| fmt_effective_range(stats),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Omni Strike",
        fallback: "Omni",
        icon: AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
        group: DisplayGroup::Body1,
        schema: &[],
        get_attributes: |stats| {
            let has_omni = (stats.long_distance_span_1 < 0 || (stats.long_distance_span_1 == 0 && stats.long_distance_anchor_1 != 0)) ||
                           (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span < 0 || (stats.long_distance_2_span == 0 && stats.long_distance_2_anchor != 0))) ||
                           (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span < 0 || (stats.long_distance_3_span == 0 && stats.long_distance_3_anchor != 0)));

            if has_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_,stats,_,_,_| fmt_effective_range(stats),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Wave Attack",
        fallback: "Wave",
        icon: AbilityIcon::Standard(img015::ICON_WAVE),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
        ],
        get_attributes: |stats| {
            if stats.mini_wave == 0 && stats.wave_chance > 0 { 
                let maximum_reach = (467.5 + ((stats.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", stats.wave_chance, AttrUnit::Percent), 
                    ("Level", stats.wave_level, AttrUnit::None),
                    ("Max Reach", maximum_reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            let maximum_reach = 467.5 + ((stats.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", chance, stats.wave_level, maximum_reach)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Mini-Wave",
        fallback: "MiniW",
        icon: AbilityIcon::Standard(img015::ICON_MINI_WAVE),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
        ],
        get_attributes: |stats| {
            if stats.mini_wave > 0 && stats.wave_chance > 0 { 
                let maximum_reach = (467.5 + ((stats.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", stats.wave_chance, AttrUnit::Percent), 
                    ("Level", stats.wave_level, AttrUnit::None),
                    ("Max Reach", maximum_reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            let maximum_reach = 467.5 + ((stats.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", chance, stats.wave_level, maximum_reach)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Surge Attack",
        fallback: "Surge",
        icon: AbilityIcon::Standard(img015::ICON_SURGE),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |stats| {
            if stats.mini_surge == 0 && stats.surge_chance > 0 { 
                vec![
                    ("Chance", stats.surge_chance, AttrUnit::Percent), 
                    ("Level", stats.surge_level, AttrUnit::None), 
                    ("Min Range", stats.surge_spawn_min, AttrUnit::Range), 
                    ("Max Range", stats.surge_spawn_min + stats.surge_spawn_max, AttrUnit::Range),
                    ("Width", stats.surge_spawn_max, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            let start_bound = stats.surge_spawn_min;
            let end_bound = stats.surge_spawn_min + stats.surge_spawn_max;
            let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
            format!("{}% Chance to create a Level {} Surge\n{} Range", chance, stats.surge_level, fmt_range(minimum_range, maximum_range))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Mini-Surge",
        fallback: "MiniS",
        icon: AbilityIcon::Standard(img015::ICON_MINI_SURGE),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |stats| {
            if stats.mini_surge > 0 && stats.surge_chance > 0 { 
                vec![
                    ("Chance", stats.surge_chance, AttrUnit::Percent), 
                    ("Level", stats.surge_level, AttrUnit::None), 
                    ("Min Range", stats.surge_spawn_min, AttrUnit::Range), 
                    ("Max Range", stats.surge_spawn_min + stats.surge_spawn_max, AttrUnit::Range),
                    ("Width", stats.surge_spawn_max, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            let start_bound = stats.surge_spawn_min;
            let end_bound = stats.surge_spawn_min + stats.surge_spawn_max;
            let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
            format!("{}% Chance to create a Level {} Mini-Surge\n{} Range", chance, stats.surge_level, fmt_range(minimum_range, maximum_range))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Death Surge",
        fallback: "DSurg",
        icon: AbilityIcon::Standard(img015::ICON_DEATH_SURGE),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Level", AttrUnit::None), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |stats| {
            if stats.death_surge_chance > 0 {
                vec![
                    ("Chance", stats.death_surge_chance, AttrUnit::Percent), 
                    ("Level", stats.death_surge_level, AttrUnit::None), 
                    ("Min Range", stats.death_surge_spawn_min, AttrUnit::Range), 
                    ("Max Range", stats.death_surge_spawn_min + stats.death_surge_spawn_max, AttrUnit::Range),
                    ("Width", stats.death_surge_spawn_max, AttrUnit::Range),
                ]
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            let start_bound = stats.death_surge_spawn_min;
            let end_bound = stats.death_surge_spawn_min + stats.death_surge_spawn_max;
            let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
            format!("{}% Chance to create a Level {} Surge\n{} Range upon death", chance, stats.death_surge_level, fmt_range(minimum_range, maximum_range))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Explosion",
        fallback: "Expl",
        icon: AbilityIcon::Standard(img015::ICON_EXPLOSION),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Min Range", AttrUnit::Range), 
            ("Max Range", AttrUnit::Range), 
        ],
        get_attributes: |stats| {
            if stats.explosion_chance > 0 {
                vec![
                    ("Chance", stats.explosion_chance, AttrUnit::Percent), 
                    ("Min Range", stats.explosion_anchor, AttrUnit::Range), 
                    ("Max Range", stats.explosion_anchor + stats.explosion_span, AttrUnit::Range),
                    ("Width", stats.explosion_span, AttrUnit::Range),
                ]
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            let start_bound = stats.explosion_anchor;
            let end_bound = stats.explosion_anchor + stats.explosion_span;
            let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
            format!("{}% Chance to create an Explosion {} Range", chance, fmt_range(minimum_range, maximum_range))
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Critical Hit",
        fallback: "Crit",
        icon: AbilityIcon::Standard(img015::ICON_CRITICAL_HIT),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.critical_chance > 0 { 
                vec![
                    ("Chance", stats.critical_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,_,_,_| format!("{}% Chance to Critical Hit dealing +100% Damage\nCritcal Hits bypass Metal resistance", chance),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Savage Blow",
        fallback: "Savge",
        icon: AbilityIcon::Standard(img015::ICON_SAVAGE_BLOW),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Boost", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.savage_blow_chance > 0 { 
                vec![
                    ("Chance", stats.savage_blow_chance, AttrUnit::Percent), 
                    ("Boost", stats.savage_blow_boost, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            format!("{}% Chance to Savage Blow\ndealing +{}% Damage", chance, stats.savage_blow_boost)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Strengthen",
        fallback: "Str+",
        icon: AbilityIcon::Standard(img015::ICON_STRENGTHEN),
        group: DisplayGroup::Body1,
        schema: &[
            ("HP", AttrUnit::Percent), 
            ("Boost", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.strengthen_threshold > 0 { 
                vec![
                    ("HP", stats.strengthen_threshold, AttrUnit::Percent), 
                    ("Boost", stats.strengthen_boost, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |_,stats,_,_,_| format!("When reduced to or below {}% HP\nDamage dealt increases by +{}%", stats.strengthen_threshold, stats.strengthen_boost),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Survive",
        fallback: "Surv",
        icon: AbilityIcon::Standard(img015::ICON_SURVIVE),
        group: DisplayGroup::Body1,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.survive_chance > 0 { 
                vec![
                    ("Chance", stats.survive_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,_,_,_| format!("{}% Chance to Survive a lethal strike", chance),
        minus_one_is_inf: false,
    },

    // --- BODY 2 ---
    EnemyAbilityDef {
        name: "Barrier",
        fallback: "Barri",
        icon: AbilityIcon::Standard(img015::ICON_BARRIER),
        group: DisplayGroup::Body2,
        schema: &[
            ("Hitpoints", AttrUnit::None)
        ],
        get_attributes: |stats| {
            if stats.barrier_hitpoints > 0 { 
                vec![
                    ("Hitpoints", stats.barrier_hitpoints, AttrUnit::None),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |hp,_,_,_,_| format!("Has a Barrier with {} HP", hp),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Aku Shield",
        fallback: "Shiel",
        icon: AbilityIcon::Standard(img015::ICON_SHIELD),
        group: DisplayGroup::Body2,
        schema: &[
            ("Hitpoints", AttrUnit::None), 
            ("Regen", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.shield_hitpoints > 0 { 
                vec![
                    ("Hitpoints", stats.shield_hitpoints, AttrUnit::None), 
                    ("Regen", stats.shield_regen, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |hp,stats,_,magnification,_| {
            let scaled_hp = (hp as f32 * (magnification.hitpoints as f32 / 100.0)).round() as i32;
            if stats.shield_regen > 0 {
                format!("Has a Shield with {} HP\nShield regenerates {}% HP when knocked back", scaled_hp, stats.shield_regen)
            } else {
                format!("Has a Shield with {} HP", scaled_hp)
            }
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Burrow",
        fallback: "Burro",
        icon: AbilityIcon::Custom(CustomIcon::Burrow), 
        group: DisplayGroup::Body2,
        schema: &[
            ("Count", AttrUnit::None), 
            ("Distance", AttrUnit::Range)
        ],
        get_attributes: |stats| {
            if stats.burrow_amount != 0 { 
                vec![
                    ("Count", stats.burrow_amount, AttrUnit::None), 
                    ("Distance", stats.burrow_distance, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |count,stats,_,_,_| format!("Burrows {} Range {}", stats.burrow_distance, fmt_count(count)),
        minus_one_is_inf: true,
    },
    EnemyAbilityDef {
        name: "Revive",
        fallback: "Reviv",
        icon: AbilityIcon::Custom(CustomIcon::Revive), 
        group: DisplayGroup::Body2,
        schema: &[
            ("Count", AttrUnit::None), 
            ("Duration", AttrUnit::Frames), 
            ("Hitpoints", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.revive_count != 0 { 
                vec![
                    ("Count", stats.revive_count, AttrUnit::None), 
                    ("Duration", stats.revive_time, AttrUnit::Frames), 
                    ("Hitpoints", stats.revive_hp, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |count,stats,_,_,_| format!("Revives {} with {}% HP after {} \nDoesn't revive if Z-Killed", fmt_count(count), stats.revive_hp, fmt_time(stats.revive_time)),
        minus_one_is_inf: true,
    },
    EnemyAbilityDef {
        name: "Toxic",
        fallback: "Toxic",
        icon: AbilityIcon::Standard(img015::ICON_TOXIC),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Damage", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.toxic_chance > 0 { 
                vec![
                    ("Chance", stats.toxic_chance, AttrUnit::Percent), 
                    ("Damage", stats.toxic_damage, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| format!("{}% Chance to deal {}% of a\nCat's Max HP in additional damage", chance, stats.toxic_damage),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Cut Cooldown",
        fallback: "CDown",
        icon: AbilityIcon::Standard(img015::ICON_CUT_COOLDOWN),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Amount", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.cut_cooldown_chance > 0 { 
                vec![
                    ("Chance", stats.cut_cooldown_chance, AttrUnit::Percent), 
                    ("Amount", stats.cut_cooldown_percent, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,_,_,_| {
            format!("{}% Chance to cut\nongoing Cat cooldown by {}%", chance, stats.cut_cooldown_percent)
        },
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Dodge",
        fallback: "Dodge",
        icon: AbilityIcon::Standard(img015::ICON_DODGE),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |stats| {
            if stats.dodge_chance > 0 { 
                vec![
                    ("Chance", stats.dodge_chance, AttrUnit::Percent), 
                    ("Duration", stats.dodge_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Dodge attacks for {}", chance, fmt_time(duration_frames)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Weaken",
        fallback: "Weak",
        icon: AbilityIcon::Standard(img015::ICON_WEAKEN),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Reduced To", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |stats| {
            if stats.weaken_chance > 0 { 
                vec![
                    ("Chance", stats.weaken_chance, AttrUnit::Percent), 
                    ("Reduced To", stats.weaken_percent, AttrUnit::Percent), 
                    ("Duration", stats.weaken_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,duration_frames,_,_| format!("{}% Chance to weaken Cats\nto {}% Attack Power for {}", chance, stats.weaken_percent, fmt_time(duration_frames)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Freeze",
        fallback: "Freez",
        icon: AbilityIcon::Standard(img015::ICON_FREEZE),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |stats| {
            if stats.freeze_chance > 0 { 
                vec![
                    ("Chance", stats.freeze_chance, AttrUnit::Percent), 
                    ("Duration", stats.freeze_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Freeze Cats for {}", chance, fmt_time(duration_frames)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Slow",
        fallback: "Slow",
        icon: AbilityIcon::Standard(img015::ICON_SLOW),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |stats| {
            if stats.slow_chance > 0 { 
                vec![
                    ("Chance", stats.slow_chance, AttrUnit::Percent), 
                    ("Duration", stats.slow_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Slow Cats for {}", chance, fmt_time(duration_frames)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Knockback",
        fallback: "KB",
        icon: AbilityIcon::Standard(img015::ICON_KNOCKBACK),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent)
        ],
        get_attributes: |stats| {
            if stats.knockback_chance > 0 { 
                vec![
                    ("Chance", stats.knockback_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,_,_,_| format!("{}% Chance to Knockback Cats", chance),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Curse",
        fallback: "Curse",
        icon: AbilityIcon::Standard(img015::ICON_CURSE),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames)
        ],
        get_attributes: |stats| {
            if stats.curse_chance > 0 { 
                vec![
                    ("Chance", stats.curse_chance, AttrUnit::Percent), 
                    ("Duration", stats.curse_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,_,duration_frames,_,_| format!("{}% Chance to Curse Cats for {}", chance, fmt_time(duration_frames)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Warp",
        fallback: "Warp",
        icon: AbilityIcon::Standard(img015::ICON_WARP),
        group: DisplayGroup::Body2,
        schema: &[
            ("Chance", AttrUnit::Percent), 
            ("Duration", AttrUnit::Frames), 
            ("Min Distance", AttrUnit::Range), 
            ("Max Distance", AttrUnit::Range)
        ],
        get_attributes: |stats| {
            if stats.warp_chance > 0 { 
                vec![
                    ("Chance", stats.warp_chance, AttrUnit::Percent), 
                    ("Duration", stats.warp_duration, AttrUnit::Frames), 
                    ("Min Distance", stats.warp_distance_minimum, AttrUnit::Range), 
                    ("Max Distance", stats.warp_distance_maximum, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |chance,stats,duration_frames,_,_| format!("{}% Chance to Warp Cats\n{} Range for {}", chance, fmt_compress(stats.warp_distance_minimum, stats.warp_distance_maximum), fmt_time(duration_frames)),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Unknown",
        fallback: "Unkwn",
        icon: AbilityIcon::Custom(CustomIcon::Unknown),
        group: DisplayGroup::Body2,
        schema: &[],
        get_attributes: |stats| if stats.has_unknown_abilities > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "This Enemy has an undefined ability\nThe App may need to be updated".into(),
        minus_one_is_inf: false,
    },
    
    // --- FOOTER ---
    EnemyAbilityDef { 
        name: "Immune Wave", 
        fallback: "NoWav", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WAVE), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.wave_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Wave Attacks".into(),
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Surge", 
        fallback: "NoSrg", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SURGE), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.surge_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Surge Attacks".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Explosion", 
        fallback: "NoExp", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_EXPLOSION), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.explosion_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Explosions".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Weaken", 
        fallback: "NoWk", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WEAKEN), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.weaken_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Weaken".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Freeze", 
        fallback: "NoFrz", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_FREEZE), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.freeze_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Freeze".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Slow", 
        fallback: "NoSlw", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SLOW), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.slow_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Slow".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Knockback", 
        fallback: "NoKB", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_KNOCKBACK), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.knockback_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Knockback".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Curse", 
        fallback: "NoCur", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_CURSE), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.curse_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Curse".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef { 
        name: "Immune Warp", 
        fallback: "NoWrp", 
        icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WARP), 
        group: DisplayGroup::Footer, 
        schema: &[],
        get_attributes: |stats| if stats.warp_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "Immune to Warp".into(), 
        minus_one_is_inf: false,
    },
    EnemyAbilityDef {
        name: "Counter Surge",
        fallback: "C-Srg",
        icon: AbilityIcon::Standard(img015::ICON_COUNTER_SURGE),
        group: DisplayGroup::Headline2,
        schema: &[],
        get_attributes: |stats| if stats.counter_surge > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_,_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        minus_one_is_inf: false,
    },
];

// --- STATS REGISTRY ---
pub struct EnemyStatsDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub get_value: fn(&EnemyRaw, i32, Magnification) -> i32, 
    pub formatter: fn(i32) -> String,       
}

pub const ENEMY_STATS_REGISTRY: &[EnemyStatsDef] = &[
    EnemyStatsDef {
        name: "Hitpoints",
        display_name: "Hitpoints",
        get_value: |stats, _, magnification| (stats.hitpoints as f32 * (magnification.hitpoints as f32 / 100.0)).round() as i32,
        formatter: |hp| format!("{}", hp),
    },
    EnemyStatsDef {
        name: "Knockbacks",
        display_name: "Knockback",
        get_value: |stats, _, _| stats.knockbacks,
        formatter: |kbs| format!("{}", kbs),
    },
    EnemyStatsDef {
        name: "Speed",
        display_name: "Speed",
        get_value: |stats, _, _| stats.speed,
        formatter: |speed| format!("{}", speed),
    },
    EnemyStatsDef {
        name: "Range",
        display_name: "Range",
        get_value: |stats, _, _| stats.standing_range,
        formatter: |range| format!("{}", range),
    },
    EnemyStatsDef {
        name: "Attack",
        display_name: "Attack",
        get_value: |stats, _, magnification| {
            let magnification_factor = magnification.attack as f32 / 100.0;
            let damage_hit_1 = (stats.attack_1 as f32 * magnification_factor).round() as i32;
            let damage_hit_2 = (stats.attack_2 as f32 * magnification_factor).round() as i32;
            let damage_hit_3 = (stats.attack_3 as f32 * magnification_factor).round() as i32;
            damage_hit_1 + damage_hit_2 + damage_hit_3
        },
        formatter: |attack| format!("{}", attack),
    },
    EnemyStatsDef {
        name: "Dps",
        display_name: "DPS",
        get_value: |stats, animation_frames, magnification| {
            let magnification_factor = magnification.attack as f32 / 100.0;
            let damage_hit_1 = (stats.attack_1 as f32 * magnification_factor).round() as i32;
            let damage_hit_2 = (stats.attack_2 as f32 * magnification_factor).round() as i32;
            let damage_hit_3 = (stats.attack_3 as f32 * magnification_factor).round() as i32;
            let total_attack_damage = damage_hit_1 + damage_hit_2 + damage_hit_3;
            
            let mut effective_foreswing = stats.pre_attack_animation;
            if stats.attack_3 > 0 && stats.time_before_attack_3 > 0 {
                effective_foreswing = stats.time_before_attack_3;
            } else if stats.attack_2 > 0 && stats.time_before_attack_2 > 0 {
                effective_foreswing = stats.time_before_attack_2;
            }
            let cooldown_frames = stats.time_before_attack_1.saturating_sub(1);
            let attack_cycle = (effective_foreswing + cooldown_frames).max(animation_frames);

            if attack_cycle > 0 { ((total_attack_damage as f32 * 30.0) / attack_cycle as f32).round() as i32 } else { 0 }
        },
        formatter: |dps| format!("{}", dps),
    },
    EnemyStatsDef {
        name: "Atk Cycle",
        display_name: "Atk Cycle",
        get_value: |stats, animation_frames, _| {
            let mut effective_foreswing = stats.pre_attack_animation;
            if stats.attack_3 > 0 && stats.time_before_attack_3 > 0 {
                effective_foreswing = stats.time_before_attack_3;
            } else if stats.attack_2 > 0 && stats.time_before_attack_2 > 0 {
                effective_foreswing = stats.time_before_attack_2;
            }
            let cooldown_frames = stats.time_before_attack_1.saturating_sub(1);
            (effective_foreswing + cooldown_frames).max(animation_frames)
        },
        formatter: |cycle| format!("{}f", cycle), 
    },
    EnemyStatsDef {
        name: "Atk Type",
        display_name: "Atk Type",
        get_value: |stats, _, _| stats.area_attack,
        formatter: |atk_type| if atk_type == 0 { "Single".to_string() } else { "Area".to_string() },
    },
    EnemyStatsDef {
        name: "Endure",
        display_name: "Endure",
        get_value: |stats, _, magnification| {
            let hp = (stats.hitpoints as f32 * (magnification.hitpoints as f32 / 100.0)).round() as i32;
            if stats.knockbacks > 0 { (hp as f32 / stats.knockbacks as f32).round() as i32 } else { hp }
        },
        formatter: |endure| format!("{}", endure),
    },
    EnemyStatsDef {
        name: "Cash Drop",
        display_name: "Cash Drop",
        get_value: |stats, _, _| (stats.cash_drop as f32 * 3.95).floor() as i32,
        formatter: |cash| format!("{}¢", cash),
    },
];

// --- REGISTRY HELPER FUNCTIONS ---
pub fn get_enemy_stat(name: &str) -> &'static EnemyStatsDef {
    ENEMY_STATS_REGISTRY.iter().find(|s| s.name == name).expect("Stat not found in registry")
}

pub fn format_enemy_stat(name: &str, stats: &EnemyRaw, animation_frames: i32, magnification: Magnification) -> String {
    let def = get_enemy_stat(name);
    (def.formatter)((def.get_value)(stats, animation_frames, magnification))
}

pub fn get_fallback_by_icon(icon_id: usize) -> &'static str {
    ENEMY_ABILITY_REGISTRY.iter().find(|def| {
        if let AbilityIcon::Standard(id) = def.icon {
            id == icon_id
        } else {
            false
        }
    }).map(|def| def.fallback).unwrap_or("???")
}