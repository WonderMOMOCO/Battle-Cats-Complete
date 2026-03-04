#![allow(dead_code)]
use eframe::egui;
use crate::features::settings::logic::Settings;
use super::imgcut::SpriteSheet;
use crate::global_data::paths;

// Traits
pub const ICON_TRAIT_RED: usize = 219;
pub const ICON_TRAIT_FLOATING: usize = 220;
pub const ICON_TRAIT_BLACK: usize = 221;
pub const ICON_TRAIT_METAL: usize = 222;
pub const ICON_TRAIT_ANGEL: usize = 223;
pub const ICON_TRAIT_ALIEN: usize = 224;
pub const ICON_TRAIT_ZOMBIE: usize = 225;
pub const ICON_TRAIT_RELIC: usize = 226;
pub const ICON_TRAIT_AKU: usize = 294;      
pub const ICON_TRAIT_TRAITLESS: usize = 227;

// Range
pub const ICON_SINGLE_ATTACK: usize = 217;
pub const ICON_AREA_ATTACK: usize = 211;
pub const ICON_OMNI_STRIKE: usize = 112;
pub const ICON_LONG_DISTANCE: usize = 212;
pub const ICON_MULTIHIT: usize = 9999; // Mock ID

// Target Abilties
pub const ICON_ATTACK_ONLY: usize = 202;
pub const ICON_STRONG_AGAINST: usize = 203;
pub const ICON_RESIST: usize = 204;
pub const ICON_INSANELY_TOUGH: usize = 122;
pub const ICON_MASSIVE_DAMAGE: usize = 206;
pub const ICON_INSANE_DAMAGE: usize = 114;
pub const ICON_DODGE: usize = 231;

// Crowd Control
pub const ICON_WARP: usize = 266;
pub const ICON_CURSE: usize = 289;
pub const ICON_WEAKEN: usize = 195;
pub const ICON_FREEZE: usize = 197;
pub const ICON_SLOW: usize = 198;
pub const ICON_KNOCKBACK: usize = 207;

// Slayer Abilities
pub const ICON_EVA_KILLER: usize = 110;
pub const ICON_WITCH_KILLER: usize = 258;
pub const ICON_COLOSSUS_SLAYER: usize = 297;
pub const ICON_BEHEMOTH_SLAYER: usize = 302;
pub const ICON_SAGE_SLAYER: usize = 319;

// Passive Abilities
pub const ICON_STRENGTHEN: usize = 196;
pub const ICON_SURVIVE: usize = 199;
pub const ICON_BASE_DESTROYER: usize = 200;
pub const ICON_CRITICAL_HIT: usize = 201;
pub const ICON_DOUBLE_BOUNTY: usize = 205;
pub const ICON_WAVE: usize = 208;
pub const ICON_METAL: usize = 209;
pub const ICON_SAVAGE_BLOW: usize = 229;
pub const ICON_SURGE: usize = 239;
pub const ICON_ZOMBIE_KILLER: usize = 260;
pub const ICON_BARRIER_BREAKER: usize = 264;
pub const ICON_MINI_WAVE: usize = 293;
pub const ICON_SHIELD_PIERCER: usize = 296;
pub const ICON_SOULSTRIKE: usize = 300;
pub const ICON_MINI_SURGE: usize = 310;
pub const ICON_CONJURE: usize = 317;
pub const ICON_METAL_KILLER: usize = 321;
pub const ICON_EXPLOSION: usize = 335;
pub const ICON_KAMIKAZE: usize = 9998; // Mock ID

// Immunities
pub const ICON_IMMUNE_CURSE: usize = 116;
pub const ICON_IMMUNE_WAVE: usize = 210;
pub const ICON_IMMUNE_WEAKEN: usize = 213;
pub const ICON_IMMUNE_FREEZE: usize = 214;
pub const ICON_IMMUNE_SLOW: usize = 215;
pub const ICON_IMMUNE_KNOCKBACK: usize = 216;
pub const ICON_IMMUNE_TOXIC: usize = 237;
pub const ICON_IMMUNE_SURGE: usize = 243;
pub const ICON_IMMUNE_WARP: usize = 262;
pub const ICON_IMMUNE_EXPLOSION: usize = 337;
pub const ICON_IMMUNE_BOSS_WAVE: usize = 9997; // Mock ID

// Counters
pub const ICON_WAVE_BLOCK: usize = 218;
pub const ICON_COUNTER_SURGE: usize = 315;

// Talent Only
pub const ICON_MOVE_SPEED: usize = 96;
pub const ICON_IMPROVE_KNOCKBACK_COUNT: usize = 98;
pub const ICON_ATTACK_BUFF: usize = 118;
pub const ICON_HEALTH_BUFF: usize = 120;
pub const ICON_TBA_DOWN: usize = 305;
pub const ICON_COST_DOWN: usize = 92;
pub const ICON_RECOVER_SPEED_UP: usize = 94;

// Resist
pub const ICON_RESIST_WEAKEN: usize = 43;
pub const ICON_RESIST_FREEZE: usize = 45;
pub const ICON_RESIST_SLOW: usize = 47;
pub const ICON_RESIST_KNOCKBACK: usize = 49;
pub const ICON_RESIST_WAVE: usize = 51;
pub const ICON_RESIST_WARP: usize = 53;
pub const ICON_RESIST_CURSE: usize = 109;
pub const ICON_RESIST_TOXIC: usize = 235;
pub const ICON_SURGE_RESIST: usize = 241;

// Other
pub const ICON_EMPTY: usize = 270;
pub const BORDER_GOLD_SMALL: usize = 271;
pub const BORDER_RED: usize = 272;
pub const BORDER_GOLD: usize = 273;

pub fn ensure_loaded(ctx: &egui::Context, sheet: &mut SpriteSheet, settings: &Settings) {
    sheet.update(ctx);

    if settings.game_language == "--" {
        return; 
    }

    if sheet.texture_handle.is_some() || sheet.is_loading_active {
        return;
    }

    let base_dir = paths::img015_folder(std::path::Path::new(""));
    let current_language = &settings.game_language;
    
    let codes_to_try: Vec<String> = if current_language.is_empty() {
        crate::core::utils::LANGUAGE_PRIORITY
            .iter()
            .map(|language_code| language_code.to_string())
            .collect()
    } else {
        vec![current_language.clone()]
    };

    for code in codes_to_try {
        let (png_filename, imgcut_filename) = if code.is_empty() {
            ("img015.png".to_string(), "img015.imgcut".to_string())
        } else {
            (format!("img015_{}.png", code), format!("img015_{}.imgcut", code))
        };

        let (full_png_path, full_imgcut_path) = (base_dir.join(png_filename), base_dir.join(imgcut_filename));
        if full_png_path.exists() && full_imgcut_path.exists() {
            sheet.load(ctx, &full_png_path, &full_imgcut_path, "global_img015".to_string());
            break;
        }
    }
}