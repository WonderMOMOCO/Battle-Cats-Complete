use eframe::egui;
use crate::global::assets::CustomAssets;
use crate::global::game::img015::*;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum CustomIcon {
    #[default] None,
    Multihit,
    Kamikaze,
    BossWave,
    Dojo,
    StarredAlien,
    Burrow,
    Revive,
}

impl CustomIcon {
    pub fn get_texture<'a>(&self, assets: &'a CustomAssets) -> Option<&'a egui::TextureHandle> {
        match self {
            CustomIcon::Multihit => Some(&assets.multihit),
            CustomIcon::Kamikaze => Some(&assets.kamikaze),
            CustomIcon::BossWave => Some(&assets.boss_wave),
            CustomIcon::Dojo => Some(&assets.dojo),
            CustomIcon::StarredAlien => Some(&assets.starred_alien),
            CustomIcon::Burrow => Some(&assets.burrow),
            CustomIcon::Revive => Some(&assets.revive),
            CustomIcon::None => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AbilityItem {
    pub icon_id: usize,
    pub text: String,
    pub custom_icon: CustomIcon,
    pub border_id: Option<usize>,
}

// UI Spacing Constants
pub const ABILITY_X: f32 = 3.0;
pub const ABILITY_Y: f32 = 5.0;
pub const TRAIT_Y: f32 = 7.0;

// Trait Order
pub const UI_TRAIT_ORDER: &[usize] = &[
    ICON_TRAIT_RED,
    ICON_TRAIT_FLOATING,
    ICON_TRAIT_BLACK,
    ICON_TRAIT_METAL,
    ICON_TRAIT_ANGEL,
    ICON_TRAIT_ALIEN,
    ICON_TRAIT_ZOMBIE,
    ICON_TRAIT_RELIC,
    ICON_TRAIT_AKU,
    ICON_TRAIT_TRAITLESS,
];