#![allow(dead_code)]
use eframe::egui;
use crate::features::settings::logic::Settings;
use crate::global::formats::imgcut::SpriteSheet;
use crate::global::io::paths;

// Icon IDs Based On Positon
pub const ICON_RESIST_WEAKEN: usize = 43;
pub const ICON_RESIST_FREEZE: usize = 45;
pub const ICON_RESIST_SLOW: usize = 47;
pub const ICON_RESIST_KNOCKBACK: usize = 49;
pub const ICON_RESIST_WAVE: usize = 51;
pub const ICON_RESIST_WARP: usize = 53;
pub const ICON_COST_DOWN: usize = 92;
pub const ICON_RECOVER_SPEED_UP: usize = 94;
pub const ICON_MOVE_SPEED: usize = 96;
pub const ICON_IMPROVE_KNOCKBACK_COUNT: usize = 98;
pub const ICON_RESIST_CURSE: usize = 109;
pub const ICON_EVA_KILLER: usize = 110;
pub const ICON_OMNI_STRIKE: usize = 112;
pub const ICON_INSANE_DAMAGE: usize = 114;
pub const ICON_IMMUNE_CURSE: usize = 116;
pub const ICON_ATTACK_BUFF: usize = 118;
pub const ICON_HEALTH_BUFF: usize = 120;
pub const ICON_INSANELY_TOUGH: usize = 122;
pub const ICON_WEAKEN: usize = 195;
pub const ICON_STRENGTHEN: usize = 196;
pub const ICON_FREEZE: usize = 197;
pub const ICON_SLOW: usize = 198;
pub const ICON_SURVIVE: usize = 199;
pub const ICON_BASE_DESTROYER: usize = 200;
pub const ICON_CRITICAL_HIT: usize = 201;
pub const ICON_ATTACK_ONLY: usize = 202;
pub const ICON_STRONG_AGAINST: usize = 203;
pub const ICON_RESIST: usize = 204;
pub const ICON_DOUBLE_BOUNTY: usize = 205;
pub const ICON_MASSIVE_DAMAGE: usize = 206;
pub const ICON_KNOCKBACK: usize = 207;
pub const ICON_WAVE: usize = 208;
pub const ICON_METAL: usize = 209;
pub const ICON_IMMUNE_WAVE: usize = 210;
pub const ICON_AREA_ATTACK: usize = 211;
pub const ICON_LONG_DISTANCE: usize = 212;
pub const ICON_IMMUNE_WEAKEN: usize = 213;
pub const ICON_IMMUNE_FREEZE: usize = 214;
pub const ICON_IMMUNE_SLOW: usize = 215;
pub const ICON_IMMUNE_KNOCKBACK: usize = 216;
pub const ICON_SINGLE_ATTACK: usize = 217;
pub const ICON_WAVE_BLOCK: usize = 218;
pub const ICON_TRAIT_RED: usize = 219;
pub const ICON_TRAIT_FLOATING: usize = 220;
pub const ICON_TRAIT_BLACK: usize = 221;
pub const ICON_TRAIT_METAL: usize = 222;
pub const ICON_TRAIT_ANGEL: usize = 223;
pub const ICON_TRAIT_ALIEN: usize = 224;
pub const ICON_TRAIT_ZOMBIE: usize = 225;
pub const ICON_TRAIT_RELIC: usize = 226;
pub const ICON_TRAIT_TRAITLESS: usize = 227;
pub const ICON_SAVAGE_BLOW: usize = 229;
pub const ICON_DODGE: usize = 231;
pub const ICON_RESIST_TOXIC: usize = 235;
pub const ICON_IMMUNE_TOXIC: usize = 237;
pub const ICON_SURGE: usize = 239;
pub const ICON_SURGE_RESIST: usize = 241;
pub const ICON_IMMUNE_SURGE: usize = 243;
pub const ICON_WITCH_KILLER: usize = 258;
pub const ICON_ZOMBIE_KILLER: usize = 260;
pub const ICON_IMMUNE_WARP: usize = 262;
pub const ICON_BARRIER_BREAKER: usize = 264;
pub const ICON_WARP: usize = 266;
pub const ICON_EMPTY: usize = 270;
pub const BORDER_GOLD_SMALL: usize = 271;
pub const BORDER_RED: usize = 272;
pub const BORDER_GOLD: usize = 273;
pub const ICON_CURSE: usize = 289;
pub const ICON_MINI_WAVE: usize = 293;
pub const ICON_TRAIT_AKU: usize = 294;
pub const ICON_SHIELD_PIERCER: usize = 296;
pub const ICON_COLOSSUS_SLAYER: usize = 297;
pub const ICON_SOULSTRIKE: usize = 300;
pub const ICON_BEHEMOTH_SLAYER: usize = 302;
pub const ICON_TBA_DOWN: usize = 305;
pub const ICON_MINI_SURGE: usize = 310;
pub const ICON_COUNTER_SURGE: usize = 315;
pub const ICON_CONJURE: usize = 317;
pub const ICON_SAGE_SLAYER: usize = 319;
pub const ICON_METAL_KILLER: usize = 321;
pub const ICON_COLOSSUS: usize = 324;
pub const ICON_BEHEMOTH: usize = 325;
pub const ICON_SAGE: usize = 326;
pub const ICON_WITCH: usize = 327;
pub const ICON_EVA: usize = 328;
pub const ICON_TOXIC: usize = 329;
pub const ICON_BARRIER: usize = 330;
pub const ICON_SHIELD: usize = 331;
pub const ICON_DEATH_SURGE: usize = 332;
pub const ICON_EXPLOSION: usize = 335;
pub const ICON_IMMUNE_EXPLOSION: usize = 337;
pub const ICON_SUPERVILLIAN: usize = 384;
pub const ICON_RESIST_EXPLOSION: usize = 386;

// Mock IDs For Custom Assets
pub const ICON_DOJO: usize = 9993;
pub const ICON_STARRED_ALIEN: usize = 9994;
pub const ICON_BURROW: usize = 9995;
pub const ICON_REVIVE: usize = 9996;
pub const ICON_IMMUNE_BOSS_WAVE: usize = 9997;
pub const ICON_KAMIKAZE: usize = 9998;
pub const ICON_MULTIHIT: usize = 9999;

pub fn ensure_loaded(ctx: &egui::Context, sheet: &mut SpriteSheet, settings: &Settings) {
    sheet.update(ctx);

    if sheet.texture_handle.is_some() || sheet.is_loading_active {
        return;
    }

    let base_dir = paths::img015_folder(std::path::Path::new(""));
    
    let png_paths = crate::global::resolver::get(&base_dir, "img015.png", &settings.general.language_priority);
    let cut_paths = crate::global::resolver::get(&base_dir, "img015.imgcut", &settings.general.language_priority);

    if let (Some(png_path), Some(imgcut_path)) = (png_paths.into_iter().next(), cut_paths.into_iter().next()) {
        
        let key = png_path.file_stem().unwrap().to_string_lossy().into_owned();
        
        sheet.load(ctx, &png_path, &imgcut_path, key);
    }
}