use std::collections::HashMap;
use crate::features::cat::logic::stats::{CatRaw, CatLevelCurve};
use crate::features::cat::data::skillacquisition::TalentRaw;
use crate::global::context::GlobalContext;

#[derive(Clone, Copy)]
pub struct CatRenderContext<'a> {
    pub global: GlobalContext<'a>,
    pub base_stats: &'a CatRaw,
    pub final_stats: &'a CatRaw,
    pub current_level: i32,
    pub level_curve: Option<&'a CatLevelCurve>,
    pub talent_data: Option<&'a TalentRaw>,
    pub talent_levels: Option<&'a HashMap<u8, u8>>,
    pub is_conjure_unit: bool,
}