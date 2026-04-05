use crate::features::settings::logic::Settings;
use crate::global::game::param::Param;
use crate::global::assets::CustomAssets;

#[derive(Clone, Copy)]
pub struct GlobalContext<'a> {
    pub settings: &'a Settings,
    pub param: &'a Param,
    pub assets: &'a CustomAssets,
}