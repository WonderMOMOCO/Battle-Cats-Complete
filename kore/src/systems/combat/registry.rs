use std::cmp::Reverse;
use std::collections::HashMap;

use nyanko::cat::unit::UnitBuy;
use nyanko::combat::{AttrValue, Entity, Faction, Identity, REGISTRY};
use nyanko::files::{img015, Param};
use serde::{Deserialize, Serialize};

use crate::systems::combat::CustomIcon;

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
    Trait,
    Headline1,
    Headline2,
    Body1,
    Body2,
    Footer,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum AbilityIcon {
    Standard(usize),
    Custom(CustomIcon),
    None,
}

pub struct FormatContext<'a> {
    pub value: AttrValue,
    pub stats: &'a Entity,
    pub target: &'a str,
    pub duration: i32,
    pub magnification: Magnification,
    pub param: &'a Param,
}

pub struct AbilityDisplayDef {
    pub name: &'static str,
    pub fallback: &'static str,
    pub icon: AbilityIcon,
    pub group: DisplayGroup,
    pub formatter: fn(&FormatContext<'_>) -> String,
}

fn unmapped() -> AbilityDisplayDef {
    AbilityDisplayDef {
        name: "Unsupported",
        fallback: "Unsup",
        icon: AbilityIcon::None,
        group: DisplayGroup::Body2,
        formatter: |ctx| side(
            ctx,
            "This Cat has an ability this application does not recognize\nA newer version of this tool may support it",
            "This Enemy has an ability this application does not recognize\nA newer version of this tool may support it",
        ),
    }
}


fn pick<T>(faction: Faction, cat: T, enemy: T) -> T {
    if faction == Faction::Cat { cat } else { enemy }
}

fn side(ctx: &FormatContext<'_>, cat: &str, enemy: &str) -> String {
    pick(ctx.stats.faction, cat, enemy).to_string()
}

fn finite(value: AttrValue) -> i32 {
    match value {
        AttrValue::Finite(amount) => amount,
        AttrValue::Infinite => -1,
    }
}

fn fmt_time(frames: i32) -> String {
    format!("{:.2}s^{}f", frames as f32 / 30.0, frames)
}

fn fmt_range(min_range: i32, max_range: i32) -> String {
    if min_range == max_range {
        format!("at {}", min_range)
    } else {
        format!("between {}~{}", min_range, max_range)
    }
}

fn fmt_compress(min_val: i32, max_val: i32) -> String {
    if min_val == max_val {
        format!("{}", min_val)
    } else {
        format!("{}~{}", min_val, max_val)
    }
}

fn fmt_count(value: AttrValue) -> String {
    match value {
        AttrValue::Infinite => "infinitely".to_string(),
        AttrValue::Finite(1) => "1 time".to_string(),
        AttrValue::Finite(count) => format!("{} times", count),
    }
}

fn fmt_spawn_range(anchor: i32, span: i32) -> String {
    let start_bound = anchor;
    let end_bound = anchor + span;
    let (minimum_range, maximum_range) = if start_bound < end_bound { (start_bound, end_bound) } else { (end_bound, start_bound) };
    fmt_range(minimum_range, maximum_range)
}

fn fmt_attack_timing(stats: &Entity) -> String {
    let attack_cooldown = fmt_time(stats.attack_cooldown);

    if stats.attack_2_damage > 0 {
        return format!("Attack cooldown {}", attack_cooldown);
    }

    format!("Attack cooldown {}\nTime before hit {}", attack_cooldown, fmt_time(stats.time_until_attack_1))
}

fn wave_reach(stats: &Entity) -> f32 {
    let base_reach = pick(stats.faction, 332.5, 467.5);
    base_reach + ((stats.wave_level - 1) as f32 * 200.0)
}

fn fmt_effective_range(stats: &Entity) -> String {
    let primary_anchor = if stats.long_distance_1_anchor != 0 {
        stats.long_distance_1_anchor
    } else {
        stats.standing_range
    };

    let mut range_strings = Vec::new();

    let has_ld_or_omni = (stats.long_distance_1_span != 0 || stats.long_distance_1_anchor != 0) ||
        (stats.long_distance_2_flag > 0 && (stats.long_distance_2_span != 0 || stats.long_distance_2_anchor != 0)) ||
        (stats.long_distance_3_flag > 0 && (stats.long_distance_3_span != 0 || stats.long_distance_3_anchor != 0));

    if has_ld_or_omni {
        let hit_data = [
            (true, stats.long_distance_1_anchor, stats.long_distance_1_span, 1),
            (stats.attack_2_damage > 0, stats.long_distance_2_anchor, stats.long_distance_2_span, stats.long_distance_2_flag),
            (stats.attack_3_damage > 0, stats.long_distance_3_anchor, stats.long_distance_3_span, stats.long_distance_3_flag),
        ];

        for (is_active, anchor, span, flag) in hit_data {
            if !is_active { continue; }

            if flag > 0 && (span != 0 || anchor != 0) {
                let start = anchor;
                let end = anchor + span;
                let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                range_strings.push(fmt_compress(min_r, max_r));
            } else if stats.long_distance_1_span != 0 || stats.long_distance_1_anchor != 0 {
                let start = stats.long_distance_1_anchor;
                let end = stats.long_distance_1_anchor + stats.long_distance_1_span;
                let (min_r, max_r) = if start < end { (start, end) } else { (end, start) };
                range_strings.push(fmt_compress(min_r, max_r));
            } else {
                let near_bound = pick(stats.faction, -320, -stats.hitbox_width);
                range_strings.push(fmt_compress(near_bound, stats.standing_range));
            }
        }
    }

    if range_strings.len() > 1
        && let Some(first_string) = range_strings.first().cloned()
        && range_strings.iter().all(|entry| entry == &first_string) {
        range_strings.truncate(1);
    }

    let label_prefix = if range_strings.len() > 1 { "Range split" } else { "Effective Range" };
    let opposing_base = pick(stats.faction, "Enemy Base", "Cat Base");

    format!("{} {}\nStands at {} Range relative to {}", label_prefix, range_strings.join(" / "), primary_anchor, opposing_base)
}

fn fmt_multihit(stats: &Entity, magnification: Magnification) -> String {
    let magnification_factor = magnification.attack as f32 / 100.0;
    let scale = |damage: i32| (damage as f32 * magnification_factor).round() as i32;

    let ability_flag_1 = if stats.attack_1_abilities > 0 { "True" } else { "False" };
    let ability_flag_2 = if stats.attack_2_abilities > 0 { "True" } else { "False" };
    let ability_flag_3 = if stats.attack_3_damage == 0 {
        ""
    } else if stats.attack_3_abilities > 0 {
        " / True"
    } else {
        " / False"
    };

    let damage_string = if stats.attack_3_damage > 0 {
        format!("{} / {} / {}", scale(stats.attack_1_damage), scale(stats.attack_2_damage), scale(stats.attack_3_damage))
    } else {
        format!("{} / {}", scale(stats.attack_1_damage), scale(stats.attack_2_damage))
    };

    let timing_string = if stats.attack_3_damage > 0 {
        format!("{} / {} / {}", fmt_time(stats.time_until_attack_1), fmt_time(stats.time_until_attack_2), fmt_time(stats.time_until_attack_3))
    } else {
        format!("{} / {}", fmt_time(stats.time_until_attack_1), fmt_time(stats.time_until_attack_2))
    };

    format!("Damage split {}\nTiming split {}\nAbility split {} / {}{}", damage_string, timing_string, ability_flag_1, ability_flag_2, ability_flag_3)
}

fn fmt_resistance_groups(base_description: &str, groups: HashMap<i32, Vec<&str>>) -> String {
    if groups.len() == 1
        && let Some((percentage, _)) = groups.iter().next() {
        return format!("{} {}%", base_description, percentage);
    }

    let mut sorted_groups: Vec<_> = groups.into_iter().collect();
    sorted_groups.sort_by_key(|group| Reverse(group.0));

    let mut formatted_lines = Vec::new();

    for (percentage, effect_names) in sorted_groups {
        let formatted_effect_list = match effect_names.len() {
            1 => effect_names[0].to_string(),
            2 => format!("{} and {}", effect_names[0], effect_names[1]),
            _ => effect_names
                .split_last()
                .map_or_else(String::new, |(last_effect, leading_effects)| format!("{}, and {}", leading_effects.join(", "), last_effect)),
        };
        formatted_lines.push(format!("{}% for {}", percentage, formatted_effect_list));
    }

    format!("{}\n{}", base_description, formatted_lines.join("\n"))
}

fn fmt_sage_slayer(param: &Param) -> String {
    let mut groups: HashMap<i32, Vec<&str>> = HashMap::new();

    groups.entry(param.sage_slayer_resist_weaken).or_default().push("Weaken");
    groups.entry(param.sage_slayer_resist_freeze).or_default().push("Freeze");
    groups.entry(param.sage_slayer_resist_slow).or_default().push("Slow");
    groups.entry(param.sage_slayer_resist_curse).or_default().push("Curse");
    groups.entry(param.sage_slayer_resist_other).or_default().push("Knockback");
    groups.entry(param.sage_slayer_resist_other).or_default().push("Delay");
    groups.entry(param.sage_slayer_resist_warp).or_default().push("Warp");

    let base_description = format!(
        "Deals {:.1}× Damage to and takes {:.1}× Damage from Sage Enemies\nIgnores the Crowd Control resistance of Sage Enemies\nCrowd Control effects originating from Sage Enemies reduced by",
        param.sage_slayer_attack_multiplier as f32 / 1000.0,
        param.sage_slayer_defense_multiplier as f32 / 1000.0
    );

    fmt_resistance_groups(&base_description, groups)
}

fn fmt_sage_trait(param: &Param) -> String {
    let mut groups: HashMap<i32, Vec<&str>> = HashMap::new();

    groups.entry(param.sage_type_resist_weaken).or_default().push("Weaken");
    groups.entry(param.sage_type_resist_freeze).or_default().push("Freeze");
    groups.entry(param.sage_type_resist_slow).or_default().push("Slow");
    groups.entry(param.sage_type_resist_curse).or_default().push("Curse");
    groups.entry(param.sage_type_resist_knockback).or_default().push("Knockback");

    fmt_resistance_groups("Crowd Control effects inflicted upon Sage Enemies are reduced by", groups)
}


#[derive(Clone, Copy)]
pub struct Figure {
    pub label: &'static str,
    pub read: fn(&Entity) -> Option<String>,
    pub weigh: fn(&Entity) -> i32,
}

const ATTACK_BAND: Figure = Figure { label: "Effective Range", read: read_band, weigh: weigh_band };

const BASE_OFFSET: Figure = Figure { label: "VS Base", read: read_offset, weigh: weigh_offset };

const DAMAGE_SPLIT: Figure = Figure { label: "Damage split", read: read_damage, weigh: weigh_damage };

const TIMING_SPLIT: Figure = Figure { label: "Timing split", read: read_timing, weigh: weigh_timing };

const ABILITY_SPLIT: Figure = Figure { label: "Ability split", read: read_carriers, weigh: weigh_carriers };

pub fn is_trait(identity: Identity) -> bool {
    if matches!(get_display_def(identity).group, DisplayGroup::Trait) {
        return true;
    }

    matches!(
        identity,
        Identity::TraitDojo
            | Identity::TraitStarredAlien
            | Identity::TraitCatGod
            | Identity::TraitColossus
            | Identity::TraitBehemoth
            | Identity::TraitSage
            | Identity::TraitKaijin
    )
}

pub fn get_figures(identity: Identity) -> &'static [Figure] {
    match identity {
        Identity::OmniStrike | Identity::LongDistance => &const { [ATTACK_BAND, BASE_OFFSET] },
        Identity::MultiHit => &const { [DAMAGE_SPLIT, TIMING_SPLIT, ABILITY_SPLIT] },
        _ => &[],
    }
}

fn reach_slots(stats: &Entity) -> [(bool, i32, i32); 3] {
    [
        (true, stats.long_distance_1_anchor, stats.long_distance_1_span),
        (stats.long_distance_2_flag > 0, stats.long_distance_2_anchor, stats.long_distance_2_span),
        (stats.long_distance_3_flag > 0, stats.long_distance_3_anchor, stats.long_distance_3_span),
    ]
}

fn read_band(stats: &Entity) -> Option<String> {
    let reach: Vec<String> = reach_slots(stats)
        .into_iter()
        .filter(|(active, anchor, span)| *active && (*anchor != 0 || *span != 0))
        .map(|(_, anchor, span)| {
            let edge = anchor + span;

            fmt_compress(anchor.min(edge), anchor.max(edge))
        })
        .collect();

    if reach.is_empty() {
        let near = pick(stats.faction, -320, -stats.hitbox_width);

        return Some(fmt_compress(near, stats.standing_range));
    }

    Some(reach.join(", "))
}

fn anchor_range(stats: &Entity) -> i32 {
    if stats.long_distance_1_anchor != 0 {
        return stats.long_distance_1_anchor;
    }

    stats.standing_range
}

fn read_offset(stats: &Entity) -> Option<String> {
    Some(anchor_range(stats).to_string())
}

fn weigh_offset(stats: &Entity) -> i32 {
    anchor_range(stats)
}

fn weigh_band(stats: &Entity) -> i32 {
    reach_slots(stats).into_iter().filter(|(active, _, _)| *active).map(|(_, _, span)| span.abs()).sum()
}

fn hits<T>(stats: &Entity, read: impl Fn(usize) -> T) -> Vec<T> {
    let landed = if stats.attack_3_damage > 0 {
        3
    } else if stats.attack_2_damage > 0 {
        2
    } else {
        1
    };

    (0..landed).map(read).collect()
}

fn joined(parts: Vec<String>) -> Option<String> {
    (!parts.is_empty()).then(|| parts.join(" / "))
}

fn read_damage(stats: &Entity) -> Option<String> {
    let damage = [stats.attack_1_damage, stats.attack_2_damage, stats.attack_3_damage];

    joined(hits(stats, |hit| damage[hit].to_string()))
}

fn weigh_damage(stats: &Entity) -> i32 {
    stats.attack_1_damage + stats.attack_2_damage + stats.attack_3_damage
}

fn read_timing(stats: &Entity) -> Option<String> {
    let timing = [stats.time_until_attack_1, stats.time_until_attack_2, stats.time_until_attack_3];

    joined(hits(stats, |hit| format!("{}f", timing[hit])))
}

fn weigh_timing(stats: &Entity) -> i32 {
    -(stats.time_until_attack_1 + stats.time_until_attack_2 + stats.time_until_attack_3)
}

fn read_carriers(stats: &Entity) -> Option<String> {
    let carries = [stats.attack_1_abilities, stats.attack_2_abilities, stats.attack_3_abilities];

    joined(hits(stats, |hit| if carries[hit] > 0 { "True".to_string() } else { "False".to_string() }))
}

fn weigh_carriers(stats: &Entity) -> i32 {
    [stats.attack_1_abilities, stats.attack_2_abilities, stats.attack_3_abilities]
        .iter()
        .filter(|carries| **carries > 0)
        .count() as i32
}

pub fn get_display_def(identity: Identity) -> AbilityDisplayDef {
    match identity {
        Identity::TraitRed => AbilityDisplayDef {
            name: "Red",
            fallback: "Red",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_RED),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Red Enemies", "Red"),
        },
        Identity::TraitFloating => AbilityDisplayDef {
            name: "Floating",
            fallback: "Float",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_FLOATING),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Floating Enemies", "Floating"),
        },
        Identity::TraitDark => AbilityDisplayDef {
            name: "Dark",
            fallback: "Dark",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_BLACK),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Dark Enemies", "Dark"),
        },
        Identity::TraitMetal => AbilityDisplayDef {
            name: "Metal",
            fallback: "Metal",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_METAL),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Metal Enemies", "Metal"),
        },
        Identity::TraitTraitless => AbilityDisplayDef {
            name: "Traitless",
            fallback: "NoTrt",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_TRAITLESS),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Traitless Enemies", "Traitless"),
        },
        Identity::TraitAngel => AbilityDisplayDef {
            name: "Angel",
            fallback: "Angel",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ANGEL),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Angel Enemies", "Angel"),
        },
        Identity::TraitAlien => AbilityDisplayDef {
            name: "Alien",
            fallback: "Alien",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ALIEN),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Alien Enemies", "Alien"),
        },
        Identity::TraitZombie => AbilityDisplayDef {
            name: "Zombie",
            fallback: "Zomb",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_ZOMBIE),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Zombie Enemies", "Zombie"),
        },
        Identity::TraitRelic => AbilityDisplayDef {
            name: "Relic",
            fallback: "Relic",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_RELIC),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Relic Enemies", "Relic"),
        },
        Identity::TraitAku => AbilityDisplayDef {
            name: "Aku",
            fallback: "Aku",
            icon: AbilityIcon::Standard(img015::ICON_TRAIT_AKU),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Aku Enemies", "Aku"),
        },
        Identity::TraitWitch => AbilityDisplayDef {
            name: "Witch",
            fallback: "Witch",
            icon: AbilityIcon::Standard(img015::ICON_WITCH),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets Witch Enemies", "Witch Enemy"),
        },
        Identity::TraitEva => AbilityDisplayDef {
            name: "EVA Angel",
            fallback: "EVA",
            icon: AbilityIcon::Standard(img015::ICON_EVA),
            group: DisplayGroup::Trait,
            formatter: |ctx| side(ctx, "Targets EVA Angels", "EVA Angel"),
        },

        Identity::TraitDojo => AbilityDisplayDef {
            name: "Dojo",
            fallback: "Dojo",
            icon: AbilityIcon::Custom(CustomIcon::Dojo),
            group: DisplayGroup::Headline1,
            formatter: |_| "Dojo".into(),
        },
        Identity::TraitStarredAlien => AbilityDisplayDef {
            name: "Starred Alien",
            fallback: "Star",
            icon: AbilityIcon::Custom(CustomIcon::StarredAlien),
            group: DisplayGroup::Headline1,
            formatter: |_| "Starred Alien".into(),
        },
        Identity::TraitCatGod => AbilityDisplayDef {
            name: "Cat God",
            fallback: "God",
            icon: AbilityIcon::Custom(CustomIcon::God),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("CotC {} Cat God", finite(ctx.value) - 1),
        },
        Identity::TraitColossus => AbilityDisplayDef {
            name: "Colossus",
            fallback: "Colos",
            icon: AbilityIcon::Standard(img015::ICON_COLOSSUS),
            group: DisplayGroup::Headline1,
            formatter: |_| "Colossus Enemy".into(),
        },
        Identity::TraitBehemoth => AbilityDisplayDef {
            name: "Behemoth",
            fallback: "Behem",
            icon: AbilityIcon::Standard(img015::ICON_BEHEMOTH),
            group: DisplayGroup::Headline1,
            formatter: |_| "Behemoth Enemy".into(),
        },
        Identity::TraitSage => AbilityDisplayDef {
            name: "Sage",
            fallback: "Sage",
            icon: AbilityIcon::Standard(img015::ICON_SAGE),
            group: DisplayGroup::Headline1,
            formatter: |ctx| fmt_sage_trait(ctx.param),
        },
        Identity::TraitKaijin => AbilityDisplayDef {
            name: "Kaijin",
            fallback: "Villn",
            icon: AbilityIcon::Standard(img015::ICON_SUPERVILLIAN),
            group: DisplayGroup::Headline1,
            formatter: |_| "Kaijin Enemy".into(),
        },

        Identity::AttackOnly => AbilityDisplayDef {
            name: "Attack Only",
            fallback: "AtkOnly",
            icon: AbilityIcon::Standard(img015::ICON_ATTACK_ONLY),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("Only damages {}", ctx.target),
        },
        Identity::StrongAgainst => AbilityDisplayDef {
            name: "Strong Against",
            fallback: "Strng",
            icon: AbilityIcon::Standard(img015::ICON_STRONG_AGAINST),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from {}", ctx.target),
        },
        Identity::MassiveDamage => AbilityDisplayDef {
            name: "Massive Damage",
            fallback: "Massv",
            icon: AbilityIcon::Standard(img015::ICON_MASSIVE_DAMAGE),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("Deals 3×~4× Damage to {}", ctx.target),
        },
        Identity::InsaneDamage => AbilityDisplayDef {
            name: "Insane Damage",
            fallback: "InsDmg",
            icon: AbilityIcon::Standard(img015::ICON_INSANE_DAMAGE),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("Deals 5×~6× Damage to {}", ctx.target),
        },
        Identity::Resist => AbilityDisplayDef {
            name: "Resist",
            fallback: "Resist",
            icon: AbilityIcon::Standard(img015::ICON_RESIST),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("Takes 1/4×~1/5× Damage from {}", ctx.target),
        },
        Identity::InsanelyTough => AbilityDisplayDef {
            name: "Insanely Tough",
            fallback: "InsRes",
            icon: AbilityIcon::Standard(img015::ICON_INSANELY_TOUGH),
            group: DisplayGroup::Headline1,
            formatter: |ctx| format!("Takes 1/6×~1/7× Damage from {}", ctx.target),
        },

        Identity::IsMetal => AbilityDisplayDef {
            name: "Metal",
            fallback: "Metal",
            icon: AbilityIcon::Standard(img015::ICON_METAL),
            group: DisplayGroup::Headline2,
            formatter: |_| "Damage taken is reduced to 1 for Non-Critical attacks".into(),
        },
        Identity::BaseDestroyer => AbilityDisplayDef {
            name: "Base Destroyer",
            fallback: "Base",
            icon: AbilityIcon::Standard(img015::ICON_BASE_DESTROYER),
            group: DisplayGroup::Headline2,
            formatter: |ctx| side(ctx, "Deals 4× Damage to the Enemy Base", "Deals 4× Damage to the Cat Base"),
        },
        Identity::DoubleBounty => AbilityDisplayDef {
            name: "Double Bounty",
            fallback: "2×$",
            icon: AbilityIcon::Standard(img015::ICON_DOUBLE_BOUNTY),
            group: DisplayGroup::Headline2,
            formatter: |_| "Receives 2× Cash from Enemies".into(),
        },
        Identity::ZombieKiller => AbilityDisplayDef {
            name: "Zombie Killer",
            fallback: "Zkill",
            icon: AbilityIcon::Standard(img015::ICON_ZOMBIE_KILLER),
            group: DisplayGroup::Headline2,
            formatter: |_| "Prevents Zombies from reviving".into(),
        },
        Identity::Soulstrike => AbilityDisplayDef {
            name: "Soulstrike",
            fallback: "SolStk",
            icon: AbilityIcon::Standard(img015::ICON_SOULSTRIKE),
            group: DisplayGroup::Headline2,
            formatter: |_| "Will attack Zombie corpses".into(),
        },
        Identity::ColossusSlayer => AbilityDisplayDef {
            name: "Colossus Slayer",
            fallback: "Colos",
            icon: AbilityIcon::Standard(img015::ICON_COLOSSUS_SLAYER),
            group: DisplayGroup::Headline2,
            formatter: |_| "Deals 1.6× Damage to and takes 0.7× Damage from Colossus Enemies".into(),
        },
        Identity::SageSlayer => AbilityDisplayDef {
            name: "Sage Slayer",
            fallback: "Sage",
            icon: AbilityIcon::Standard(img015::ICON_SAGE_SLAYER),
            group: DisplayGroup::Headline2,
            formatter: |ctx| fmt_sage_slayer(ctx.param),
        },
        Identity::BehemothSlayer => AbilityDisplayDef {
            name: "Behemoth Slayer",
            fallback: "Behem",
            icon: AbilityIcon::Standard(img015::ICON_BEHEMOTH_SLAYER),
            group: DisplayGroup::Headline2,
            formatter: |ctx| {
                let mut formatted_text = format!(
                    "Deals {:.1}× Damage to and takes {:.1}× Damage from Behemoth Enemies",
                    ctx.param.behemoth_slayer_attack_multiplier as f32 / 1000.0,
                    ctx.param.behemoth_slayer_defense_multiplier as f32 / 1000.0
                );
                if ctx.stats.behemoth_dodge_chance > 0 {
                    formatted_text.push_str(&format!("\n{}% Chance to Dodge Behemoth Enemies for {}", ctx.stats.behemoth_dodge_chance, fmt_time(ctx.stats.behemoth_dodge_duration)));
                }
                formatted_text
            },
        },
        Identity::WitchKiller => AbilityDisplayDef {
            name: "Witch Killer",
            fallback: "Witch",
            icon: AbilityIcon::Standard(img015::ICON_WITCH_KILLER),
            group: DisplayGroup::Headline2,
            formatter: |_| "Deals 5× Damage to and takes 0.1× Damage from Witches".into(),
        },
        Identity::EvaKiller => AbilityDisplayDef {
            name: "Eva Killer",
            fallback: "Eva",
            icon: AbilityIcon::Standard(img015::ICON_EVA_KILLER),
            group: DisplayGroup::Headline2,
            formatter: |_| "Deals 5× Damage to and takes 0.2× Damage from Eva Angels".into(),
        },
        Identity::WaveBlock => AbilityDisplayDef {
            name: "Wave Block",
            fallback: "W-Blk",
            icon: AbilityIcon::Standard(img015::ICON_WAVE_BLOCK),
            group: DisplayGroup::Headline2,
            formatter: |_| "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into(),
        },
        Identity::CounterSurge => AbilityDisplayDef {
            name: "Counter Surge",
            fallback: "C-Srg",
            icon: AbilityIcon::Standard(img015::ICON_COUNTER_SURGE),
            group: DisplayGroup::Headline2,
            formatter: |_| "When hit with a Surge Attack, create a Surge of equal Type, Level, and Range".into(),
        },
        Identity::Kamikaze => AbilityDisplayDef {
            name: "Kamikaze",
            fallback: "Kamik",
            icon: AbilityIcon::Custom(CustomIcon::Kamikaze),
            group: DisplayGroup::Headline2,
            formatter: |ctx| match finite(ctx.value) {
                0 => "Unit disappears immediately".to_string(),
                1 => "Unit disappears after 1 attack".to_string(),
                attacks => format!("Unit disappears after {} attacks", attacks),
            },
        },
        Identity::TimeBeforeDeath => AbilityDisplayDef {
            name: "Time Before Death",
            fallback: "TBDth",
            icon: AbilityIcon::Custom(CustomIcon::DeathTimer),
            group: DisplayGroup::Headline2,
            formatter: |ctx| format!("Dies after {} when spawned", fmt_time(finite(ctx.value))),
        },
        Identity::Stop => AbilityDisplayDef {
            name: "Stop",
            fallback: "Stop",
            icon: AbilityIcon::Custom(CustomIcon::Stop),
            group: DisplayGroup::Headline2,
            formatter: |ctx| match finite(ctx.value) {
                0 => "Unit stops moving immediately".to_string(),
                1 => "Unit stops moving after 1 attack".to_string(),
                attacks => format!("Unit stops moving after {} attacks", attacks),
            },
        },

        Identity::SingleAttack => AbilityDisplayDef {
            name: "Single Attack",
            fallback: "Sngl",
            icon: AbilityIcon::Standard(img015::ICON_SINGLE_ATTACK),
            group: DisplayGroup::Body1,
            formatter: |ctx| fmt_attack_timing(ctx.stats),
        },
        Identity::AreaAttack => AbilityDisplayDef {
            name: "Area Attack",
            fallback: "Area",
            icon: AbilityIcon::Standard(img015::ICON_AREA_ATTACK),
            group: DisplayGroup::Body1,
            formatter: |ctx| fmt_attack_timing(ctx.stats),
        },
        Identity::MultiHit => AbilityDisplayDef {
            name: "Multi-Hit",
            fallback: "Multi",
            icon: AbilityIcon::Custom(CustomIcon::Multihit),
            group: DisplayGroup::Body1,
            formatter: |ctx| fmt_multihit(ctx.stats, ctx.magnification),
        },
        Identity::LongDistance => AbilityDisplayDef {
            name: "Long Distance",
            fallback: "LD",
            icon: AbilityIcon::Standard(img015::ICON_LONG_DISTANCE),
            group: DisplayGroup::Body1,
            formatter: |ctx| fmt_effective_range(ctx.stats),
        },
        Identity::OmniStrike => AbilityDisplayDef {
            name: "Omni Strike",
            fallback: "Omni",
            icon: AbilityIcon::Standard(img015::ICON_OMNI_STRIKE),
            group: DisplayGroup::Body1,
            formatter: |ctx| fmt_effective_range(ctx.stats),
        },
        Identity::Conjure => AbilityDisplayDef {
            name: "Conjure",
            fallback: "Spirit",
            icon: AbilityIcon::Standard(img015::ICON_CONJURE),
            group: DisplayGroup::Body1,
            formatter: |_| "Conjures a Spirit to the battlefield when tapped\nThis Cat may only be deployed one at a time".into(),
        },
        Identity::MetalKiller => AbilityDisplayDef {
            name: "Metal Killer",
            fallback: "MetKil",
            icon: AbilityIcon::Standard(img015::ICON_METAL_KILLER),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("Reduces Metal enemies current HP by {}% upon hit", finite(ctx.value)),
        },
        Identity::WaveAttack => AbilityDisplayDef {
            name: "Wave Attack",
            fallback: "Wave",
            icon: AbilityIcon::Standard(img015::ICON_WAVE),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to create a Level {} Wave\nWave reaches {} Range", finite(ctx.value), ctx.stats.wave_level, wave_reach(ctx.stats)),
        },
        Identity::MiniWave => AbilityDisplayDef {
            name: "Mini-Wave",
            fallback: "MiniW",
            icon: AbilityIcon::Standard(img015::ICON_MINI_WAVE),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to create a Level {} Mini-Wave\nMini-Wave reaches {} Range", finite(ctx.value), ctx.stats.wave_level, wave_reach(ctx.stats)),
        },
        Identity::SurgeAttack => AbilityDisplayDef {
            name: "Surge Attack",
            fallback: "Surge",
            icon: AbilityIcon::Standard(img015::ICON_SURGE),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!(
                "{}% Chance to create a Level {} Surge\n{} Range",
                finite(ctx.value),
                ctx.stats.surge_level,
                fmt_spawn_range(ctx.stats.surge_spawn_anchor, ctx.stats.surge_spawn_span)
            ),
        },
        Identity::MiniSurge => AbilityDisplayDef {
            name: "Mini-Surge",
            fallback: "MiniS",
            icon: AbilityIcon::Standard(img015::ICON_MINI_SURGE),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!(
                "{}% Chance to create a Level {} Mini-Surge\n{} Range",
                finite(ctx.value),
                ctx.stats.surge_level,
                fmt_spawn_range(ctx.stats.surge_spawn_anchor, ctx.stats.surge_spawn_span)
            ),
        },
        Identity::DeathSurge => AbilityDisplayDef {
            name: "Death Surge",
            fallback: "DSurg",
            icon: AbilityIcon::Standard(img015::ICON_DEATH_SURGE),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!(
                "{}% Chance to create a Level {} Surge\n{} Range upon death",
                finite(ctx.value),
                ctx.stats.death_surge_level,
                fmt_spawn_range(ctx.stats.death_surge_spawn_anchor, ctx.stats.death_surge_spawn_span)
            ),
        },
        Identity::Explosion => AbilityDisplayDef {
            name: "Explosion",
            fallback: "Expl",
            icon: AbilityIcon::Standard(img015::ICON_EXPLOSION),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!(
                "{}% Chance to create an Explosion {} Range",
                finite(ctx.value),
                fmt_spawn_range(ctx.stats.explosion_spawn_anchor, ctx.stats.explosion_spawn_span)
            ),
        },
        Identity::SavageBlow => AbilityDisplayDef {
            name: "Savage Blow",
            fallback: "Savge",
            icon: AbilityIcon::Standard(img015::ICON_SAVAGE_BLOW),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to Savage Blow\ndealing +{}% Damage", finite(ctx.value), ctx.stats.savage_blow_boost),
        },
        Identity::CriticalHit => AbilityDisplayDef {
            name: "Critical Hit",
            fallback: "Crit",
            icon: AbilityIcon::Standard(img015::ICON_CRITICAL_HIT),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to Critical Hit dealing +100% Damage\nCritcal Hits bypass Metal resistance", finite(ctx.value)),
        },
        Identity::Strengthen => AbilityDisplayDef {
            name: "Strengthen",
            fallback: "Str+",
            icon: AbilityIcon::Standard(img015::ICON_STRENGTHEN),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("When reduced to or below {}% HP\nDamage dealt increases by +{}%", ctx.stats.strengthen_threshold, ctx.stats.strengthen_boost),
        },
        Identity::Survive => AbilityDisplayDef {
            name: "Survive",
            fallback: "Surv",
            icon: AbilityIcon::Standard(img015::ICON_SURVIVE),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to Survive a lethal strike", finite(ctx.value)),
        },
        Identity::BarrierBreaker => AbilityDisplayDef {
            name: "Barrier Breaker",
            fallback: "Brkr",
            icon: AbilityIcon::Standard(img015::ICON_BARRIER_BREAKER),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to break enemy Barriers", finite(ctx.value)),
        },
        Identity::ShieldPiercer => AbilityDisplayDef {
            name: "Shield Piercer",
            fallback: "Spierc",
            icon: AbilityIcon::Standard(img015::ICON_SHIELD_PIERCER),
            group: DisplayGroup::Body1,
            formatter: |ctx| format!("{}% Chance to pierce enemy Shields", finite(ctx.value)),
        },

        Identity::Barrier => AbilityDisplayDef {
            name: "Barrier",
            fallback: "Barri",
            icon: AbilityIcon::Standard(img015::ICON_BARRIER),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("Has a Barrier with {} HP", finite(ctx.value)),
        },
        Identity::AkuShield => AbilityDisplayDef {
            name: "Aku Shield",
            fallback: "Shiel",
            icon: AbilityIcon::Standard(img015::ICON_SHIELD),
            group: DisplayGroup::Body2,
            formatter: |ctx| {
                let scaled_hp = (finite(ctx.value) as f32 * (ctx.magnification.hitpoints as f32 / 100.0)).round() as i32;
                if ctx.stats.shield_regen > 0 {
                    format!("Has a Shield with {} HP\nShield regenerates {}% HP when knocked back", scaled_hp, ctx.stats.shield_regen)
                } else {
                    format!("Has a Shield with {} HP", scaled_hp)
                }
            },
        },
        Identity::Burrow => AbilityDisplayDef {
            name: "Burrow",
            fallback: "Burro",
            icon: AbilityIcon::Custom(CustomIcon::Burrow),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("Burrows {} Range {}", ctx.stats.burrow_distance, fmt_count(ctx.value)),
        },
        Identity::Revive => AbilityDisplayDef {
            name: "Revive",
            fallback: "Reviv",
            icon: AbilityIcon::Custom(CustomIcon::Revive),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!(
                "Revives {} with {}% HP after {} \nDoesn't revive if Z-Killed",
                fmt_count(ctx.value),
                ctx.stats.revive_hp,
                fmt_time(ctx.stats.revive_time)
            ),
        },
        Identity::Toxic => AbilityDisplayDef {
            name: "Toxic",
            fallback: "Toxic",
            icon: AbilityIcon::Standard(img015::ICON_TOXIC),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("{}% Chance to deal {}% of a\nCat's Max HP in additional damage", finite(ctx.value), ctx.stats.toxic_damage),
        },
        Identity::Drain => AbilityDisplayDef {
            name: "Drain",
            fallback: "Drain",
            icon: AbilityIcon::Standard(img015::ICON_DRAIN),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("{}% Chance to extend\nongoing Cat cooldown by {}%", finite(ctx.value), ctx.stats.drain_percent),
        },
        Identity::Dodge => AbilityDisplayDef {
            name: "Dodge",
            fallback: "Dodge",
            icon: AbilityIcon::Standard(img015::ICON_DODGE),
            group: DisplayGroup::Body2,
            formatter: |ctx| {
                let dodged = pick(ctx.stats.faction, ctx.target, "attacks");
                format!("{}% Chance to Dodge {} for {}", finite(ctx.value), dodged, fmt_time(ctx.duration))
            },
        },
        Identity::Weaken => AbilityDisplayDef {
            name: "Weaken",
            fallback: "Weak",
            icon: AbilityIcon::Standard(img015::ICON_WEAKEN),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!(
                "{}% Chance to weaken {}\nto {}% Attack Power for {}",
                finite(ctx.value),
                ctx.target,
                ctx.stats.weaken_to,
                fmt_time(ctx.duration)
            ),
        },
        Identity::Freeze => AbilityDisplayDef {
            name: "Freeze",
            fallback: "Freez",
            icon: AbilityIcon::Standard(img015::ICON_FREEZE),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("{}% Chance to Freeze {} for {}", finite(ctx.value), ctx.target, fmt_time(ctx.duration)),
        },
        Identity::Slow => AbilityDisplayDef {
            name: "Slow",
            fallback: "Slow",
            icon: AbilityIcon::Standard(img015::ICON_SLOW),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("{}% Chance to Slow {} for {}", finite(ctx.value), ctx.target, fmt_time(ctx.duration)),
        },
        Identity::Knockback => AbilityDisplayDef {
            name: "Knockback",
            fallback: "KB",
            icon: AbilityIcon::Standard(img015::ICON_KNOCKBACK),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("{}% Chance to Knockback {}", finite(ctx.value), ctx.target),
        },
        Identity::Curse => AbilityDisplayDef {
            name: "Curse",
            fallback: "Curse",
            icon: AbilityIcon::Standard(img015::ICON_CURSE),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!("{}% Chance to Curse {} for {}", finite(ctx.value), ctx.target, fmt_time(ctx.duration)),
        },
        Identity::Warp => AbilityDisplayDef {
            name: "Warp",
            fallback: "Warp",
            icon: AbilityIcon::Standard(img015::ICON_WARP),
            group: DisplayGroup::Body2,
            formatter: |ctx| format!(
                "{}% Chance to Warp {}\n{} Range for {}",
                finite(ctx.value),
                ctx.target,
                fmt_compress(ctx.stats.warp_distance_anchor, ctx.stats.warp_distance_span),
                fmt_time(ctx.duration)
            ),
        },
        Identity::Unknown => AbilityDisplayDef {
            name: "Unknown",
            fallback: "Unkwn",
            icon: AbilityIcon::Custom(CustomIcon::Unknown),
            group: DisplayGroup::Body2,
            formatter: |ctx| side(
                ctx,
                "This Cat may have an unrecognized ability\nA newer version of this tool may support it",
                "This Enemy may have an unrecognized ability\nA newer version of this tool may support it",
            ),
        },

        Identity::ImmuneWave => AbilityDisplayDef {
            name: "Immune Wave",
            fallback: "NoWav",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WAVE),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Wave Attacks".into(),
        },
        Identity::ImmuneSurge => AbilityDisplayDef {
            name: "Immune Surge",
            fallback: "NoSrg",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SURGE),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Surge Attacks".into(),
        },
        Identity::ImmuneExplosion => AbilityDisplayDef {
            name: "Immune Explosion",
            fallback: "NoExp",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_EXPLOSION),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Explosions".into(),
        },
        Identity::ImmuneWeaken => AbilityDisplayDef {
            name: "Immune Weaken",
            fallback: "NoWk",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WEAKEN),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Weaken".into(),
        },
        Identity::ImmuneFreeze => AbilityDisplayDef {
            name: "Immune Freeze",
            fallback: "NoFrz",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_FREEZE),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Freeze".into(),
        },
        Identity::ImmuneSlow => AbilityDisplayDef {
            name: "Immune Slow",
            fallback: "NoSlw",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_SLOW),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Slow".into(),
        },
        Identity::ImmuneKnockback => AbilityDisplayDef {
            name: "Immune Knockback",
            fallback: "NoKB",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_KNOCKBACK),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Knockback".into(),
        },
        Identity::ImmuneCurse => AbilityDisplayDef {
            name: "Immune Curse",
            fallback: "NoCur",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_CURSE),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Curse".into(),
        },
        Identity::ImmuneDrain => AbilityDisplayDef {
            name: "Immune Drain",
            fallback: "NoDrn",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_DRAIN),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Drain".into(),
        },
        Identity::ImmuneWarp => AbilityDisplayDef {
            name: "Immune Warp",
            fallback: "NoWrp",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_WARP),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Warp".into(),
        },
        Identity::ImmuneToxic => AbilityDisplayDef {
            name: "Immune Toxic",
            fallback: "NoTox",
            icon: AbilityIcon::Standard(img015::ICON_IMMUNE_TOXIC),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Toxic".into(),
        },
        Identity::ImmuneBossWave => AbilityDisplayDef {
            name: "Immune Boss Wave",
            fallback: "NoBos",
            icon: AbilityIcon::Custom(CustomIcon::BossWave),
            group: DisplayGroup::Footer,
            formatter: |_| "Immune to Boss Shockwaves".into(),
        },

        Identity::ResistWeaken => AbilityDisplayDef {
            name: "Resist Weaken",
            fallback: "ReWkn",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_WEAKEN),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Weaken ({}%)", finite(ctx.value)),
        },
        Identity::ResistFreeze => AbilityDisplayDef {
            name: "Resist Freeze",
            fallback: "ReFrz",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_FREEZE),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Freeze ({}%)", finite(ctx.value)),
        },
        Identity::ResistSlow => AbilityDisplayDef {
            name: "Resist Slow",
            fallback: "ReSlw",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_SLOW),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Slow ({}%)", finite(ctx.value)),
        },
        Identity::ResistKnockback => AbilityDisplayDef {
            name: "Resist Knockback",
            fallback: "ReKB",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_KNOCKBACK),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Knockback ({}%)", finite(ctx.value)),
        },
        Identity::ResistWave => AbilityDisplayDef {
            name: "Resist Wave",
            fallback: "ReWav",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_WAVE),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Wave ({}%)", finite(ctx.value)),
        },
        Identity::ResistWarp => AbilityDisplayDef {
            name: "Resist Warp",
            fallback: "ReWrp",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_WARP),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Warp ({}%)", finite(ctx.value)),
        },
        Identity::ResistCurse => AbilityDisplayDef {
            name: "Resist Curse",
            fallback: "ReCur",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_CURSE),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Curse ({}%)", finite(ctx.value)),
        },
        Identity::ResistToxic => AbilityDisplayDef {
            name: "Resist Toxic",
            fallback: "ReTox",
            icon: AbilityIcon::Standard(img015::ICON_RESIST_TOXIC),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Toxic ({}%)", finite(ctx.value)),
        },
        Identity::ResistSurge => AbilityDisplayDef {
            name: "Resist Surge",
            fallback: "ReSrg",
            icon: AbilityIcon::Standard(img015::ICON_SURGE_RESIST),
            group: DisplayGroup::Footer,
            formatter: |ctx| format!("Resist Surge ({}%)", finite(ctx.value)),
        },

        Identity::CostDown => AbilityDisplayDef {
            name: "Cost Down",
            fallback: "Cost-",
            icon: AbilityIcon::Standard(img015::ICON_COST_DOWN),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        Identity::RecoverSpeedUp => AbilityDisplayDef {
            name: "Recover Speed Up",
            fallback: "Rec+",
            icon: AbilityIcon::Standard(img015::ICON_RECOVER_SPEED_UP),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        Identity::MoveSpeedUp => AbilityDisplayDef {
            name: "Move Speed Up",
            fallback: "Spd",
            icon: AbilityIcon::Standard(img015::ICON_MOVE_SPEED),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        Identity::AttackBuff => AbilityDisplayDef {
            name: "Attack Buff",
            fallback: "Atk+",
            icon: AbilityIcon::Standard(img015::ICON_ATTACK_BUFF),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        Identity::HealthBuff => AbilityDisplayDef {
            name: "Health Buff",
            fallback: "HP+",
            icon: AbilityIcon::Standard(img015::ICON_HEALTH_BUFF),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        Identity::TbaDown => AbilityDisplayDef {
            name: "TBA Down",
            fallback: "TBA-",
            icon: AbilityIcon::Standard(img015::ICON_TBA_DOWN),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        Identity::ImproveKnockbacks => AbilityDisplayDef {
            name: "Improve Knockbacks",
            fallback: "KB+",
            icon: AbilityIcon::Standard(img015::ICON_IMPROVE_KNOCKBACK_COUNT),
            group: DisplayGroup::Footer,
            formatter: |_| String::new(),
        },
        _ => unmapped(),
    }
}

pub fn get_fallback_by_icon(target_icon: AbilityIcon) -> &'static str {
    REGISTRY.iter()
        .map(|pure_definition| get_display_def(pure_definition.identity))
        .find(|display_definition| display_definition.icon == target_icon)
        .map_or("???", |display_definition| display_definition.fallback)
}


pub struct StatContext<'a> {
    pub stats: &'a Entity,
    pub animation_frames: i32,
    pub magnification: Magnification,
    pub unitbuy: Option<&'a UnitBuy>,
}

impl<'a> StatContext<'a> {
    pub fn cat(stats: &'a Entity, animation_frames: i32, unitbuy: Option<&'a UnitBuy>) -> Self {
        Self { stats, animation_frames, magnification: Magnification::default(), unitbuy }
    }

    pub fn enemy(stats: &'a Entity, animation_frames: i32, magnification: Magnification) -> Self {
        Self { stats, animation_frames, magnification, unitbuy: None }
    }

    fn scaled_attack(&self) -> i32 {
        let magnification_factor = self.magnification.attack as f32 / 100.0;
        let scale = |damage: i32| (damage as f32 * magnification_factor).round() as i32;

        scale(self.stats.attack_1_damage) + scale(self.stats.attack_2_damage) + scale(self.stats.attack_3_damage)
    }
}

pub struct StatsDef {
    pub name: &'static str,
    pub display_name: &'static str,
    pub get_value: fn(&StatContext<'_>) -> i32,
    pub formatter: fn(i32) -> String,
    pub linked_talent_id: Option<u8>,
    pub talent_modifier_fmt: Option<fn(i32, i32) -> String>,
}

pub const STAT_HITPOINTS: StatsDef = StatsDef {
    name: "Hitpoints",
    display_name: "Hitpoints",
    get_value: |ctx| (ctx.stats.hitpoints as f32 * (ctx.magnification.hitpoints as f32 / 100.0)).round() as i32,
    formatter: |hitpoints| format!("{}", hitpoints),
    linked_talent_id: Some(32),
    talent_modifier_fmt: Some(|percent, _| format!("(+{}%)", percent)),
};

pub const STAT_KNOCKBACKS: StatsDef = StatsDef {
    name: "Knockbacks",
    display_name: "Knockback",
    get_value: |ctx| ctx.stats.knockbacks,
    formatter: |knockbacks| format!("{}", knockbacks),
    linked_talent_id: Some(28),
    talent_modifier_fmt: Some(|count, _| format!("(+{})", count)),
};

pub const STAT_SPEED: StatsDef = StatsDef {
    name: "Speed",
    display_name: "Speed",
    get_value: |ctx| ctx.stats.speed,
    formatter: |speed| format!("{}", speed),
    linked_talent_id: Some(27),
    talent_modifier_fmt: Some(|speed, _| format!("(+{})", speed)),
};

pub const STAT_RANGE: StatsDef = StatsDef {
    name: "Range",
    display_name: "Range",
    get_value: |ctx| ctx.stats.standing_range,
    formatter: |range| format!("{}", range),
    linked_talent_id: None,
    talent_modifier_fmt: None,
};

pub const STAT_ATTACK: StatsDef = StatsDef {
    name: "Attack",
    display_name: "Attack",
    get_value: |ctx| ctx.scaled_attack(),
    formatter: |attack| format!("{}", attack),
    linked_talent_id: Some(31),
    talent_modifier_fmt: Some(|percent, _| format!("(+{}%)", percent)),
};

pub const STAT_DPS: StatsDef = StatsDef {
    name: "Dps",
    display_name: "DPS",
    get_value: |ctx| {
        let attack_cycle = ctx.stats.attack_cycle(ctx.animation_frames);

        if attack_cycle <= 0 { return 0; }

        ((ctx.scaled_attack() as f32 * 30.0) / attack_cycle as f32).round() as i32
    },
    formatter: |dps| format!("{}", dps),
    linked_talent_id: None,
    talent_modifier_fmt: None,
};

pub const STAT_ATK_CYCLE: StatsDef = StatsDef {
    name: "Atk Cycle",
    display_name: "Atk Cycle",
    get_value: |ctx| ctx.stats.attack_cycle(ctx.animation_frames),
    formatter: |frames| format!("{}f", frames),
    linked_talent_id: None,
    talent_modifier_fmt: None,
};

pub const STAT_RARITY: StatsDef = StatsDef {
    name: "Rarity",
    display_name: "Rarity",
    get_value: |ctx| ctx.unitbuy.map_or(-1, |unitbuy| unitbuy.rarity),
    formatter: |rarity| match rarity {
        0 => "Normal".to_string(),
        1 => "Special".to_string(),
        2 => "Rare".to_string(),
        3 => "Super Rare".to_string(),
        4 => "Uber Rare".to_string(),
        5 => "Legend Rare".to_string(),
        _ => "??".to_string(),
    },
    linked_talent_id: None,
    talent_modifier_fmt: None,
};

pub const STAT_COST: StatsDef = StatsDef {
    name: "Cost",
    display_name: "Cost",
    get_value: |ctx| (ctx.stats.eoc1_cost as f32 * 1.5).round() as i32,
    formatter: |cost| format!("{}¢", cost),
    linked_talent_id: Some(25),
    talent_modifier_fmt: Some(|reduction, _| format!("(-{}¢)", (reduction as f32 * 1.5).round() as i32)),
};

pub const STAT_COOLDOWN: StatsDef = StatsDef {
    name: "Cooldown",
    display_name: "Cooldown",
    get_value: |ctx| (ctx.stats.cooldown - 264).max(60),
    formatter: |cooldown| format!("{:.2}s^{}f", cooldown as f32 / 30.0, cooldown),
    linked_talent_id: Some(26),
    talent_modifier_fmt: Some(|frames, _| format!("(-{}f)", frames)),
};

const STAT_ATTACK_COOLDOWN: StatsDef = StatsDef {
    name: "Attack Cooldown",
    display_name: "Attack Cooldown",
    get_value: |ctx| ctx.stats.attack_cooldown,
    formatter: |attack_cooldown| format!("{}f", attack_cooldown),
    linked_talent_id: Some(61),
    talent_modifier_fmt: Some(|percent, _| format!("(-{}%)", percent)),
};

pub const STAT_CASH_DROP: StatsDef = StatsDef {
    name: "Cash Drop",
    display_name: "Cash Drop",
    get_value: |ctx| (ctx.stats.cash_drop as f32 * 3.95).floor() as i32,
    formatter: |cash| format!("{}¢", cash),
    linked_talent_id: None,
    talent_modifier_fmt: None,
};

pub(crate) const CAT_STATS_REGISTRY: &[StatsDef] = &[
    STAT_HITPOINTS,
    STAT_KNOCKBACKS,
    STAT_SPEED,
    STAT_RANGE,
    STAT_ATTACK,
    STAT_DPS,
    STAT_ATK_CYCLE,
    STAT_RARITY,
    STAT_COST,
    STAT_COOLDOWN,
    STAT_ATTACK_COOLDOWN,
];

pub(crate) const ENEMY_STATS_REGISTRY: &[StatsDef] = &[
    STAT_HITPOINTS,
    STAT_KNOCKBACKS,
    STAT_SPEED,
    STAT_RANGE,
    STAT_ATTACK,
    STAT_DPS,
    STAT_ATK_CYCLE,
    STAT_CASH_DROP,
];

pub fn format_stat(definition: &StatsDef, ctx: &StatContext<'_>) -> String {
    (definition.formatter)((definition.get_value)(ctx))
}

#[cfg(test)]
mod trait_tests {
    use super::{is_trait, DisplayGroup, Identity};

    // The subtraits sit in Headline1 beside abilities that are not traits at all, so the
    // group alone cannot answer this.
    #[test]
    fn the_subtraits_count_as_traits_and_their_neighbours_do_not() {
        for identity in [Identity::TraitStarredAlien, Identity::TraitBehemoth, Identity::TraitColossus, Identity::TraitSage] {
            assert!(is_trait(identity), "{:?} is a subtrait", identity);
            assert!(!matches!(super::get_display_def(identity).group, DisplayGroup::Trait));
        }

        for identity in [Identity::MassiveDamage, Identity::InsaneDamage, Identity::Resist, Identity::AttackOnly] {
            assert!(!is_trait(identity), "{:?} shares Headline1 but is no trait", identity);
        }
    }

    #[test]
    fn the_ordinary_traits_still_count() {
        assert!(is_trait(Identity::TraitRed));
        assert!(is_trait(Identity::TraitAku));
    }
}

#[cfg(test)]
mod tests {
    use nyanko::combat::REGISTRY;

    use super::get_display_def;

    #[test]
    fn every_registry_ability_has_a_display_def() {
        let unmapped: Vec<_> = REGISTRY
            .iter()
            .map(|ability| ability.identity)
            .filter(|identity| get_display_def(*identity).name == "Unsupported")
            .collect();

        assert!(
            unmapped.is_empty(),
            "these nyanko REGISTRY abilities have no arm in get_display_def, so they would render \
             as an iconless blank row in statblocks and a bare \"?\" chip in the filters: {:?}",
            unmapped
        );
    }
}
