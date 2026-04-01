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

fn fmt_range(min_range: i32, max_range: i32) -> String {
    if min_range == max_range { format!("at {}", min_range) } else { format!("between {}~{}", min_range, max_range) }
}

fn get_dur_val(value_1: i32, value_2: i32) -> i32 {
    if value_1 != 0 { value_1 } else { value_2 }
}

fn fmt_effective_range(stats: &CatRaw) -> String {
    // Standing distance is ALWAYS dictated by Hit 1
    let primary_anchor = if stats.long_distance_1_anchor != 0 { 
        stats.long_distance_1_anchor 
    } else { 
        stats.standing_range 
    };

    let mut range_strings = Vec::new();
    
    // Does the unit have LD or Omni on ANY hit?
    let has_ld_or_omni = (stats.long_distance_1_span != 0 || stats.long_distance_1_anchor != 0) ||
                         (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span != 0 || stats.long_distance_2_anchor != 0)) ||
                         (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span != 0 || stats.long_distance_3_anchor != 0));

    if has_ld_or_omni {
        let hit_data = [
            (true, stats.long_distance_1_anchor, stats.long_distance_1_span, 1),
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
                    // Standard hit fallback! (Using standard Cat 320 collision offset)
                    range_strings.push(format!("-320~{}", stats.standing_range));
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
    format!("{} {}\nStands at {} Range relative to Enemy Base", label_prefix, range_strings.join(" / "), primary_anchor)
}

fn fmt_multihit(stats: &CatRaw) -> String {
    let damage_hit_1 = stats.attack_1;
    let damage_hit_2 = stats.attack_2;
    let damage_hit_3 = stats.attack_3;
    let ability_flag_1 = if stats.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if stats.attack_2_abilities > 0 { "True" } else { "False" };
    let ability_flag_3 = if stats.attack_3 > 0 { if stats.attack_3_abilities > 0 { " / True" } else { " / False" } } else { "" };
    let damage_string = if stats.attack_3 > 0 { 
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
        get_attributes: |stats| if stats.area_attack == 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
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
        get_attributes: |stats| if stats.area_attack == 1 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
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
        get_attributes: |stats| if stats.target_red > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Red Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_red = 1),
    },
    CatAbilityDef {
        name: "Target Float",
        fallback: "Float",
        icon_id: img015::ICON_TRAIT_FLOATING,
        talent_id: 34,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_floating > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Floating Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_floating = 1),
    },
    CatAbilityDef {
        name: "Target Black",
        fallback: "Black",
        icon_id: img015::ICON_TRAIT_BLACK,
        talent_id: 35,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_black > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Black Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_black = 1),
    },
    CatAbilityDef {
        name: "Target Metal",
        fallback: "Metal",
        icon_id: img015::ICON_TRAIT_METAL,
        talent_id: 36,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_metal > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Metal Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_metal = 1),
    },
    CatAbilityDef {
        name: "Target Angel",
        fallback: "Angel",
        icon_id: img015::ICON_TRAIT_ANGEL,
        talent_id: 37,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_angel > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Angel Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_angel = 1),
    },
    CatAbilityDef {
        name: "Target Alien",
        fallback: "Alien",
        icon_id: img015::ICON_TRAIT_ALIEN,
        talent_id: 38,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_alien > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Alien Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_alien = 1),
    },
    CatAbilityDef {
        name: "Target Zombie",
        fallback: "Zomb",
        icon_id: img015::ICON_TRAIT_ZOMBIE,
        talent_id: 39,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_zombie > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Zombie Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_zombie = 1),
    },
    CatAbilityDef {
        name: "Target Relic",
        fallback: "Relic",
        icon_id: img015::ICON_TRAIT_RELIC,
        talent_id: 40,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_relic > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Relic Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_relic = 1),
    },
    CatAbilityDef {
        name: "Target Aku",
        fallback: "Aku",
        icon_id: img015::ICON_TRAIT_AKU,
        talent_id: 57,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_aku > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Aku Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_aku = 1),
    },
    CatAbilityDef {
        name: "Target White",
        fallback: "White",
        icon_id: img015::ICON_TRAIT_TRAITLESS,
        talent_id: 41,
        group: DisplayGroup::Trait,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.target_traitless > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Targets Traitless Enemies".into(),
        apply_func: Some(|stats,_,_,_| stats.target_traitless = 1),
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
        get_attributes: |stats| if stats.attack_only > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Only damages {}", target),
        apply_func: Some(|stats, _, _, _| stats.attack_only = 1),
    },
    CatAbilityDef {
        name: "Strong Against",
        fallback: "Strng",
        icon_id: img015::ICON_STRONG_AGAINST,
        talent_id: 5,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.strong_against > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", target),
        apply_func: Some(|stats, _, _, _| stats.strong_against = 1),
    },
    CatAbilityDef {
        name: "Massive Damage",
        fallback: "Massv",
        icon_id: img015::ICON_MASSIVE_DAMAGE,
        talent_id: 7,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.massive_damage > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Deals 3×~4× Damage to {}", target),
        apply_func: Some(|stats, _, _, _| stats.massive_damage = 1),
    },
    CatAbilityDef {
        name: "Insane Damage",
        fallback: "InsDmg",
        icon_id: img015::ICON_INSANE_DAMAGE,
        talent_id: 7,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.insane_damage > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
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
        get_attributes: |stats| if stats.resist > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, target, _| format!("Takes 1/4×~1/5× Damage from {}", target),
        apply_func: Some(|stats, _, _, _| stats.resist = 1),
    },
    CatAbilityDef {
        name: "Insanely Tough",
        fallback: "InsRes",
        icon_id: img015::ICON_INSANELY_TOUGH,
        talent_id: 6,
        group: DisplayGroup::Headline1,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.insanely_tough > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
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
        get_attributes: |stats| if stats.metal > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Damage taken is reduced to 1 for Non-Critical attacks".into(),
        apply_func: Some(|stats,_,_,_| stats.metal = 1),
    },
    CatAbilityDef {
        name: "Base Destroyer",
        fallback: "Base",
        icon_id: img015::ICON_BASE_DESTROYER,
        talent_id: 12,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.base_destroyer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Deals 4× Damage to the Enemy Base".into(),
        apply_func: Some(|stats, _, _, _| stats.base_destroyer = 1),
    },
    
    CatAbilityDef {
        name: "Double Bounty",
        fallback: "2×$",
        icon_id: img015::ICON_DOUBLE_BOUNTY,
        talent_id: 16,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.double_bounty > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Receives 2× Cash from Enemies".into(),
        apply_func: Some(|stats, _, _, _| stats.double_bounty = 1),
    },
    CatAbilityDef {
        name: "Zombie Killer",
        fallback: "Zkill",
        icon_id: img015::ICON_ZOMBIE_KILLER,
        talent_id: 14,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.zombie_killer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Prevents Zombies from reviving".into(),
        apply_func: Some(|stats, _, _, _| stats.zombie_killer = 1),
    },
    CatAbilityDef {
        name: "Soulstrike",
        fallback: "SolStk",
        icon_id: img015::ICON_SOULSTRIKE,
        talent_id: 59,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.soulstrike == 2 || (stats.soulstrike > 0 && stats.zombie_killer > 0) { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Will attack Zombie corpses".into(),
        apply_func: Some(|stats, _, _, _| stats.soulstrike = 2),
    },
    CatAbilityDef {
        name: "Colossus Slayer",
        fallback: "Colos",
        icon_id: img015::ICON_COLOSSUS_SLAYER,
        talent_id: 63,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.colossus_slayer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into(),
        apply_func: Some(|stats, _, _, _| stats.colossus_slayer = 1),
    },
    CatAbilityDef {
        name: "Sage Slayer",
        fallback: "Sage",
        icon_id: img015::ICON_SAGE_SLAYER,
        talent_id: 66,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.sage_slayer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "Deals 1.2× Damage to and takes 0.5× Damage from Sage Enemies\nCrowd Control effects originating from Sage Enemies reduced by 70%".into(),
        apply_func: Some(|stats, _, _, _| stats.sage_slayer = 1),
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
        get_attributes: |stats| {
            if stats.behemoth_slayer > 0 {
                if stats.behemoth_dodge_chance > 0 {
                    vec![
                        ("Active", 1, AttrUnit::None), 
                        ("Dodge Chance", stats.behemoth_dodge_chance, AttrUnit::Percent), 
                        ("Dodge Duration", stats.behemoth_dodge_duration, AttrUnit::Frames),
                    ]
                } else {
                    vec![("Active", 1, AttrUnit::None)]
                }
            } else {
                vec![]
            }
        },
        formatter: |_, stats, _, _| {
            let mut formatted_text = "Deals 2.5× Damage to and takes 0.6× Damage from Behemoth Enemies".to_string();
            if stats.behemoth_dodge_chance > 0 {
                formatted_text.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", stats.behemoth_dodge_chance, fmt_time(stats.behemoth_dodge_duration)));
            }
            formatted_text
        },
        apply_func: Some(|stats, value_1, value_2, _| {
            stats.behemoth_slayer = 1;
            stats.behemoth_dodge_chance = if value_1 > 0 { value_1 } else { 5 };
            stats.behemoth_dodge_duration = if value_2 > 0 { value_2 } else { 30 };
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
        get_attributes: |stats| if stats.eva_killer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into(),
        apply_func: Some(|stats,_,_,_| stats.eva_killer = 1),
    },
    CatAbilityDef {
        name: "Witch Killer",
        fallback: "Witch",
        icon_id: img015::ICON_WITCH_KILLER,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.witch_killer > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Deals 5× Damage to and takes 0.1× Damage from Witches".into(),
        apply_func: Some(|stats,_,_,_| stats.witch_killer = 1),
    },
    CatAbilityDef {
        name: "Wave Block",
        fallback: "W-Blk",
        icon_id: img015::ICON_WAVE_BLOCK,
        talent_id: 0,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.wave_block > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, _, _, _| "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(),
        apply_func: Some(|stats, _, _, _| stats.wave_block = 1),
    },
    CatAbilityDef {
        name: "Counter Surge",
        fallback: "C-Srg",
        icon_id: img015::ICON_COUNTER_SURGE,
        talent_id: 68,
        group: DisplayGroup::Headline2,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.counter_surge > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        apply_func: Some(|stats,_,_,_| stats.counter_surge = 1),
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
        get_attributes: |stats| {
            if stats.attack_count_total > -1 && stats.attack_count_state == 2 { 
                vec![("Attacks", stats.attack_count_total, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, _, _| {
            let limit_suffix = match value_1 {
                0 => "immediately".to_string(),
                1 => "after 1 attack".to_string(),
                n => format!("after {} attacks", n),
            };
            format!("Unit disappears {}", limit_suffix)
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
        get_attributes: |stats| {
            if stats.attack_count_total > -1 && stats.attack_count_state == 0 { 
                vec![("Attacks", stats.attack_count_total, AttrUnit::None)] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, _, _| {
            let limit_suffix = match value_1 {
                0 => "immediately".to_string(),
                1 => "after 1 attack".to_string(),
                n => format!("after {} attacks", n),
            };
            format!("Unit stops moving {}", limit_suffix)
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
        get_attributes: |stats| if stats.attack_2 > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_, stats, _, _| fmt_multihit(stats),
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
        get_attributes: |stats| {
            // Check if ANY hit is Omni
            let has_omni = (stats.long_distance_1_span < 0 || (stats.long_distance_1_span == 0 && stats.long_distance_1_anchor != 0)) ||
                           (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span < 0 || (stats.long_distance_2_span == 0 && stats.long_distance_2_anchor != 0))) ||
                           (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span < 0 || (stats.long_distance_3_span == 0 && stats.long_distance_3_anchor != 0)));
            
            // Check if ANY hit is LD
            let has_ld = (stats.long_distance_1_span > 0) || 
                         (stats.long_distance_2_flag > 0 && stats.long_distance_2_span > 0) || 
                         (stats.long_distance_3_flag > 0 && stats.long_distance_3_span > 0);
            
            // ONLY show the Long Distance icon if it has LD and DOES NOT have Omni
            if has_ld && !has_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_, stats, _, _| fmt_effective_range(stats),
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
        get_attributes: |stats| {
            // Check if ANY hit is Omni
            let has_omni = (stats.long_distance_1_span < 0 || (stats.long_distance_1_span == 0 && stats.long_distance_1_anchor != 0)) ||
                           (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span < 0 || (stats.long_distance_2_span == 0 && stats.long_distance_2_anchor != 0))) ||
                           (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span < 0 || (stats.long_distance_3_span == 0 && stats.long_distance_3_anchor != 0)));
            
            if has_omni { vec![("Active", 1, AttrUnit::None)] } else { vec![] }
        },
        formatter: |_, stats, _, _| fmt_effective_range(stats),
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
        get_attributes: |stats| if stats.conjure_unit_id > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
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
        get_attributes: |stats| {
            if stats.metal_killer_percent > 0 { 
                vec![
                    ("Damage", stats.metal_killer_percent, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1,_,_,_| format!("Reduces Metal enemies current HP by {}% upon hit", value_1),
        apply_func: Some(|stats,value_1,_,_| stats.metal_killer_percent = value_1),
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
        get_attributes: |stats| {
            if stats.mini_wave_flag == 0 && stats.wave_chance > 0 { 
                let maximum_reach = (332.5 + ((stats.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", stats.wave_chance, AttrUnit::Percent), 
                    ("Level", stats.wave_level, AttrUnit::None),
                    ("Max Reach", maximum_reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, stats, _, _| {
            let maximum_reach = 332.5 + ((stats.wave_level - 1) as f32 * 200.0);
            format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", value_1, stats.wave_level, maximum_reach)
        },
        apply_func: Some(|stats, value_1, value_2, _| { stats.wave_chance += value_1; stats.wave_level = value_2; }),
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
        get_attributes: |stats| {
            if stats.mini_wave_flag > 0 && stats.wave_chance > 0 { 
                let maximum_reach = (332.5 + ((stats.wave_level - 1) as f32 * 200.0)).round() as i32;
                vec![
                    ("Chance", stats.wave_chance, AttrUnit::Percent), 
                    ("Level", stats.wave_level, AttrUnit::None),
                    ("Max Reach", maximum_reach, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, stats, _, _| {
             let maximum_reach = 332.5 + ((stats.wave_level - 1) as f32 * 200.0);
             format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", value_1, stats.wave_level, maximum_reach)
        },
        apply_func: Some(|stats, value_1, value_2, _| { stats.mini_wave_flag = 1; stats.wave_chance += value_1; stats.wave_level = value_2; }),
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
        get_attributes: |stats| {
            if stats.mini_surge_flag == 0 && stats.surge_chance > 0 { 
                vec![
                    ("Chance", stats.surge_chance, AttrUnit::Percent), 
                    ("Level", stats.surge_level, AttrUnit::None), 
                    ("Min Range", stats.surge_spawn_anchor, AttrUnit::Range), 
                    ("Max Range", stats.surge_spawn_anchor + stats.surge_spawn_span, AttrUnit::Range),
                    ("Width", stats.surge_spawn_span, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, stats, _, _| {
            let start_bound = stats.surge_spawn_anchor;
            let end_bound = stats.surge_spawn_anchor + stats.surge_spawn_span;
            let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
            format!("{}% Chance to create a Level {} Surge\n{} Range", value_1, stats.surge_level, fmt_range(minimum_range, maximum_range))
        },
        apply_func: Some(|stats, value_1, value_2, group_data| { 
            stats.surge_chance += value_1; stats.surge_level = value_2; 
            stats.surge_spawn_anchor = group_data.min_3 as i32 / 4;
            stats.surge_spawn_span = group_data.min_4 as i32 / 4;
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
        get_attributes: |stats| {
            if stats.mini_surge_flag > 0 && stats.surge_chance > 0 { 
                vec![
                    ("Chance", stats.surge_chance, AttrUnit::Percent), 
                    ("Level", stats.surge_level, AttrUnit::None), 
                    ("Min Range", stats.surge_spawn_anchor, AttrUnit::Range), 
                    ("Max Range", stats.surge_spawn_anchor + stats.surge_spawn_span, AttrUnit::Range),
                    ("Width", stats.surge_spawn_span, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, stats, _, _| {
            let start_bound = stats.surge_spawn_anchor;
            let end_bound = stats.surge_spawn_anchor + stats.surge_spawn_span;
            let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
            format!("{}% Chance to create a Level {} Mini-Surge\n{} Range", value_1, stats.surge_level, fmt_range(minimum_range, maximum_range))
        },
        apply_func: Some(|stats, value_1, value_2, group_data| { 
            stats.mini_surge_flag = 1; stats.surge_chance += value_1; stats.surge_level = value_2; 
            stats.surge_spawn_anchor = group_data.min_3 as i32 / 4;
            stats.surge_spawn_span = group_data.min_4 as i32 / 4;
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
        get_attributes: |stats| {
            if stats.explosion_chance > 0 { 
                vec![
                    ("Chance", stats.explosion_chance, AttrUnit::Percent), 
                    ("Min Range", stats.explosion_spawn_anchor, AttrUnit::Range), 
                    ("Max Range", stats.explosion_spawn_anchor + stats.explosion_spawn_span, AttrUnit::Range),
                    ("Width", stats.explosion_spawn_span, AttrUnit::Range),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, stats, _, _| {
             let start_bound = stats.explosion_spawn_anchor;
             let end_bound = stats.explosion_spawn_anchor + stats.explosion_spawn_span;
             let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
             format!("{}% Chance to create an Explosion {} Range", value_1, fmt_range(minimum_range, maximum_range))
        },
        apply_func: Some(|stats, value_1, _, group_data| {
            stats.explosion_chance += value_1;
            stats.explosion_spawn_anchor = group_data.min_2 as i32 / 4;
            stats.explosion_spawn_span = group_data.min_3 as i32 / 4;
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
        formatter: |value_1, stats, _, _| {
            format!("{}% Chance to Savage Blow\ndealing +{}% Damage", value_1, stats.savage_blow_boost)
        },
        apply_func: Some(|stats, value_1, value_2, _| { stats.savage_blow_chance += value_1; if value_2 > 0 { stats.savage_blow_boost = value_2; } }),
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
        get_attributes: |stats| {
            if stats.critical_chance > 0 { 
                vec![
                    ("Chance", stats.critical_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, _, _| format!("{}% Chance to Critical Hit dealing +100% Damage\nCritcal Hits bypass Metal resistance", value_1),
        apply_func: Some(|stats, value_1, _, _| stats.critical_chance += value_1),
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
        formatter: |_, stats, _, _| format!("When reduced to or below {}% HP\nDamage dealt increases by +{}%", stats.strengthen_threshold, stats.strengthen_boost),
        apply_func: Some(|stats, value_1, value_2, _| {
             if stats.strengthen_boost == 0 {
                 stats.strengthen_threshold = 100 - value_1; 
                 stats.strengthen_boost = value_2;
             } else {
                 stats.strengthen_boost += if value_1 != 0 { value_1 } else { value_2 };
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
        get_attributes: |stats| {
            if stats.survive > 0 { 
                vec![
                    ("Chance", stats.survive, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, _, _| format!("{}% Chance to Survive a lethal strike", value_1),
        apply_func: Some(|stats, value_1, _, _| stats.survive += value_1),
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
        get_attributes: |stats| {
            if stats.barrier_breaker_chance > 0 { 
                vec![
                    ("Chance", stats.barrier_breaker_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, _, _| format!("{}% Chance to break enemy Barriers", value_1),
        apply_func: Some(|stats, value_1, _, _| stats.barrier_breaker_chance += value_1),
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
        get_attributes: |stats| {
            if stats.shield_pierce_chance > 0 { 
                vec![
                    ("Chance", stats.shield_pierce_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, _, _| format!("{}% Chance to pierce enemy Shields", value_1),
        apply_func: Some(|stats, value_1, _, _| stats.shield_pierce_chance += value_1),
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
        formatter: |value_1, _, target, duration_frames| format!("{}% Chance to Dodge {} for {}", value_1, target, fmt_time(duration_frames)),
        apply_func: Some(|stats, value_1, value_2, _| { stats.dodge_chance += value_1; stats.dodge_duration += value_2; }),
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
        get_attributes: |stats| {
            if stats.weaken_chance > 0 { 
                vec![
                    ("Chance", stats.weaken_chance, AttrUnit::Percent), 
                    ("Reduced To", stats.weaken_to, AttrUnit::Percent), 
                    ("Duration", stats.weaken_duration, AttrUnit::Frames),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, stats, target, duration_frames| format!("{}% Chance to weaken {}\nto {}% Attack Power for {}", value_1, target, stats.weaken_to, fmt_time(duration_frames)),
        apply_func: Some(|stats, value_1, value_2, group_data| {
            if stats.weaken_chance == 0 {
                 stats.weaken_chance = value_1; stats.weaken_duration = value_2; stats.weaken_to = (100 - group_data.min_3) as i32; 
            } else if group_data.text_id == 42 { stats.weaken_duration += get_dur_val(value_1, value_2); } 
            else { stats.weaken_chance += value_1; stats.weaken_duration += value_2; }
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
        formatter: |value_1, _, target, duration_frames| format!("{}% Chance to Freeze {} for {}", value_1, target, fmt_time(duration_frames)),
        apply_func: Some(|stats, value_1, value_2, group_data| {
            if stats.freeze_chance == 0 { stats.freeze_chance = value_1; stats.freeze_duration = value_2; } 
            else if group_data.text_id == 74 { stats.freeze_chance += value_1; } 
            else { stats.freeze_duration += get_dur_val(value_1, value_2); }
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
        formatter: |value_1, _, target, duration_frames| format!("{}% Chance to Slow {} for {}", value_1, target, fmt_time(duration_frames)),
        apply_func: Some(|stats, value_1, value_2, group_data| {
            if stats.slow_chance == 0 { stats.slow_chance = value_1; stats.slow_duration = value_2; } 
            else if group_data.text_id == 63 { stats.slow_chance += value_1; } 
            else { stats.slow_duration += get_dur_val(value_1, value_2); }
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
        get_attributes: |stats| {
            if stats.knockback_chance > 0 { 
                vec![
                    ("Chance", stats.knockback_chance, AttrUnit::Percent),
                ] 
            } else { 
                vec![] 
            }
        },
        formatter: |value_1, _, target, _| format!("{}% Chance to Knockback {}", value_1, target),
        apply_func: Some(|stats, value_1, _, _| stats.knockback_chance += value_1),
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
        formatter: |value_1, _, target, duration_frames| format!("{}% Chance to Curse {} for {}", value_1, target, fmt_time(duration_frames)),
        apply_func: Some(|stats, value_1, value_2, group_data| {
             if stats.curse_chance == 0 { stats.curse_chance = value_1; stats.curse_duration = value_2; } 
             else if group_data.text_id == 93 { stats.curse_duration += get_dur_val(value_1, value_2); } 
             else { stats.curse_chance += value_1; }
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
        formatter: |value_1, stats, target, duration_frames| format!("{}% Chance to Warp {} {}~{} Range for {}", value_1, target, stats.warp_distance_minimum, stats.warp_distance_maximum, fmt_time(duration_frames)),
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
        get_attributes: |stats| if stats.wave_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Wave Attacks".into(),
        apply_func: Some(|stats,_,_,_| stats.wave_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Surge",
        fallback: "NoSrg",
        icon_id: img015::ICON_IMMUNE_SURGE,
        talent_id: 55,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.surge_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Surge Attacks".into(),
        apply_func: Some(|stats,_,_,_| stats.surge_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Explosion",
        fallback: "NoExp",
        icon_id: img015::ICON_IMMUNE_EXPLOSION,
        talent_id: 116,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.explosion_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Explosions".into(),
        apply_func: Some(|stats,_,_,_| stats.explosion_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Weaken",
        fallback: "NoWk",
        icon_id: img015::ICON_IMMUNE_WEAKEN,
        talent_id: 44,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.weaken_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Weaken".into(),
        apply_func: Some(|stats,_,_,_| stats.weaken_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Freeze",
        fallback: "NoFrz",
        icon_id: img015::ICON_IMMUNE_FREEZE,
        talent_id: 45,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.freeze_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Freeze".into(),
        apply_func: Some(|stats,_,_,_| stats.freeze_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Slow",
        fallback: "NoSlw",
        icon_id: img015::ICON_IMMUNE_SLOW,
        talent_id: 46,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.slow_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Slow".into(),
        apply_func: Some(|stats,_,_,_| stats.slow_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Knockback",
        fallback: "NoKB",
        icon_id: img015::ICON_IMMUNE_KNOCKBACK,
        talent_id: 47,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.knockback_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Knockback".into(),
        apply_func: Some(|stats,_,_,_| stats.knockback_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Curse",
        fallback: "NoCur",
        icon_id: img015::ICON_IMMUNE_CURSE,
        talent_id: 29,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.curse_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Curse".into(),
        apply_func: Some(|stats,_,_,_| stats.curse_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Toxic",
        fallback: "NoTox",
        icon_id: img015::ICON_IMMUNE_TOXIC,
        talent_id: 53,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.toxic_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Toxic".into(),
        apply_func: Some(|stats,_,_,_| stats.toxic_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Warp",
        fallback: "NoWrp",
        icon_id: img015::ICON_IMMUNE_WARP,
        talent_id: 49,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |stats| if stats.warp_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Warp".into(),
        apply_func: Some(|stats,_,_,_| stats.warp_immune = 1),
    },
    CatAbilityDef {
        name: "Immune Boss Wave",
        fallback: "NoBos",
        icon_id: img015::ICON_IMMUNE_BOSS_WAVE,
        talent_id: 0,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::BossWave,
        schema: &[],
        get_attributes: |stats| if stats.boss_wave_immune > 0 { vec![("Active", 1, AttrUnit::None)] } else { vec![] },
        formatter: |_,_,_,_| "Immune to Boss Shockwaves".into(),
        apply_func: Some(|stats,_,_,_| stats.boss_wave_immune = 1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Weaken ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Freeze ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Slow ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Knockback ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Wave ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Warp ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Curse ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Toxic ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |value_1,_,_,_| format!("Resist Surge ({}%)", value_1),
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
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| stats.eoc1_cost = stats.eoc1_cost.saturating_sub(value_1)),
    },
    CatAbilityDef {
        name: "Recover Speed Up",
        fallback: "Rec+",
        icon_id: img015::ICON_RECOVER_SPEED_UP,
        talent_id: 26,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| stats.cooldown = stats.cooldown.saturating_sub(value_1)),
    },
    CatAbilityDef {
        name: "Move Speed Up",
        fallback: "Spd",
        icon_id: img015::ICON_MOVE_SPEED,
        talent_id: 27,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| stats.speed += value_1),
    },
    CatAbilityDef {
        name: "Attack Buff",
        fallback: "Atk+",
        icon_id: img015::ICON_ATTACK_BUFF,
        talent_id: 31,
        group: DisplayGroup::Footer,
        custom_icon: CustomIcon::None,
        schema: &[],
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| {
            let percentage_factor = (100 + value_1) as f32 / 100.0;
            stats.attack_1 = (stats.attack_1 as f32 * percentage_factor).round() as i32;
            stats.attack_2 = (stats.attack_2 as f32 * percentage_factor).round() as i32;
            stats.attack_3 = (stats.attack_3 as f32 * percentage_factor).round() as i32;
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
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| {
            let percentage_factor = (100 + value_1) as f32 / 100.0;
            stats.hitpoints = (stats.hitpoints as f32 * percentage_factor).round() as i32;
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
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| {
             let time_reduction = (stats.time_before_attack_1 as f32 * value_1 as f32 / 100.0).round() as i32;
             stats.time_before_attack_1 = stats.time_before_attack_1.saturating_sub(time_reduction);
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
        get_attributes: |_stats| vec![],
        formatter: |_,_,_,_| "".into(),
        apply_func: Some(|stats, value_1, _, _| stats.knockbacks += value_1),
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
        get_value: |stats, _| stats.hitpoints,
        formatter: |value_1| format!("{}", value_1),
        linked_talent_id: Some(32),
        talent_modifier_fmt: Some(|value_1, _| format!("(+{}%)", value_1)),
    },
    CatStatsDef {
        name: "Knockbacks",
        display_name: "Knockback",
        get_value: |stats, _| stats.knockbacks,
        formatter: |value_1| format!("{}", value_1),
        linked_talent_id: Some(28),
        talent_modifier_fmt: Some(|value_1, _| format!("(+{})", value_1)),
    },
    CatStatsDef {
        name: "Speed",
        display_name: "Speed",
        get_value: |stats, _| stats.speed,
        formatter: |value_1| format!("{}", value_1),
        linked_talent_id: Some(27),
        talent_modifier_fmt: Some(|value_1, _| format!("(+{})", value_1)),
    },
    CatStatsDef {
        name: "Range",
        display_name: "Range",
        get_value: |stats, _| stats.standing_range,
        formatter: |value_1| format!("{}", value_1),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Attack",
        display_name: "Attack",
        get_value: |stats, _| stats.attack_1 + stats.attack_2 + stats.attack_3,
        formatter: |value_1| format!("{}", value_1),
        linked_talent_id: Some(31),
        talent_modifier_fmt: Some(|value_1, _| format!("(+{}%)", value_1)),
    },
    CatStatsDef {
        name: "Dps",
        display_name: "DPS",
        get_value: |stats, animation_frames| {
            let total_attack_damage = stats.attack_1 + stats.attack_2 + stats.attack_3;
            let mut effective_foreswing = stats.pre_attack_animation;
            if stats.attack_3 > 0 && stats.time_before_attack_3 > 0 { effective_foreswing = stats.time_before_attack_3; } 
            else if stats.attack_2 > 0 && stats.time_before_attack_2 > 0 { effective_foreswing = stats.time_before_attack_2; }
            let cooldown_frames = stats.time_before_attack_1.saturating_sub(1);
            let attack_cycle = (effective_foreswing + cooldown_frames).max(animation_frames);
            if attack_cycle > 0 { ((total_attack_damage as f32 * 30.0) / attack_cycle as f32).round() as i32 } else { 0 }
        },
        formatter: |value_1| format!("{}", value_1),
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Atk Cycle",
        display_name: "Atk Cycle",
        get_value: |stats, animation_frames| {
            let mut effective_foreswing = stats.pre_attack_animation;
            if stats.attack_3 > 0 && stats.time_before_attack_3 > 0 { effective_foreswing = stats.time_before_attack_3; } 
            else if stats.attack_2 > 0 && stats.time_before_attack_2 > 0 { effective_foreswing = stats.time_before_attack_2; }
            let cooldown_frames = stats.time_before_attack_1.saturating_sub(1);
            (effective_foreswing + cooldown_frames).max(animation_frames)
        },
        formatter: |value_1| format!("{}f", value_1), 
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Atk Type",
        display_name: "Atk Type",
        get_value: |stats, _| stats.area_attack,
        formatter: |value_1| if value_1 == 0 { "Single".to_string() } else { "Area".to_string() },
        linked_talent_id: None,
        talent_modifier_fmt: None,
    },
    CatStatsDef {
        name: "Cost",
        display_name: "Cost",
        get_value: |stats, _| (stats.eoc1_cost as f32 * 1.5).round() as i32,
        formatter: |value_1| format!("{}¢", value_1),
        linked_talent_id: Some(25),
        talent_modifier_fmt: Some(|value_1, _| format!("(-{}¢)", (value_1 as f32 * 1.5).round() as i32)),
    },
    CatStatsDef {
        name: "Cooldown",
        display_name: "Cooldown",
        get_value: |stats, _| (stats.cooldown - 264).max(60),
        formatter: |value_1| format!("{:.2}s^{}f", value_1 as f32 / 30.0, value_1),
        linked_talent_id: Some(26),
        talent_modifier_fmt: Some(|value_1, _| format!("(-{}f)", value_1)),
    },
    CatStatsDef {
        name: "TBA",
        display_name: "TBA",
        get_value: |stats, _| stats.time_before_attack_1,
        formatter: |value_1| format!("{}f", value_1),
        linked_talent_id: Some(61),
        talent_modifier_fmt: Some(|value_1, _| format!("(-{}%)", value_1)),
    },
];

// --- REGISTRY HELPER FUNCTIONS ---

pub fn get_cat_stat(name: &str) -> &'static CatStatsDef {
    CAT_STATS_REGISTRY.iter().find(|stat_definition| stat_definition.name == name).expect("CRITICAL: Hardcoded stat name was not found in CAT_STATS_REGISTRY")
}

pub fn format_cat_stat(name: &str, stats: &CatRaw, animation_frames: i32) -> String {
    let stat_definition = get_cat_stat(name);
    (stat_definition.formatter)((stat_definition.get_value)(stats, animation_frames))
}

pub fn get_by_talent_id(id: u8) -> Option<&'static CatAbilityDef> {
    CAT_ABILITY_REGISTRY.iter().find(|definition| definition.talent_id == id)
}

pub fn get_fallback_by_icon(icon_id: usize) -> &'static str {
    CAT_ABILITY_REGISTRY.iter().find(|definition| definition.icon_id == icon_id).map(|definition| definition.fallback).unwrap_or("???")
}