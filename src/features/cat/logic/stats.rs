pub use crate::features::cat::data::unitid::CatRaw;
pub use crate::features::cat::data::unitid::load_from_id;
pub use crate::features::cat::data::unitid::ICON_SIZE;
pub use crate::features::cat::data::unitlevel::CatLevelCurve;

use crate::features::cat::data::skillacquisition::TalentRaw;
use std::collections::HashMap;

pub fn apply_level(base_stats: &CatRaw, curve: Option<&CatLevelCurve>, level: i32) -> CatRaw {
    let mut s = base_stats.clone();
    if let Some(c) = curve {
        s.hitpoints = c.calculate_stat(s.hitpoints, level);
        s.attack_1 = c.calculate_stat(s.attack_1, level);
        s.attack_2 = c.calculate_stat(s.attack_2, level);
        s.attack_3 = c.calculate_stat(s.attack_3, level);
    }
    s
}

pub fn get_final_stats(
    base_stats: &CatRaw, 
    curve: Option<&CatLevelCurve>, 
    level: i32, 
    talent_data: Option<&TalentRaw>, 
    talent_levels: Option<&HashMap<u8, u8>>
) -> CatRaw {
    let leveled = apply_level(base_stats, curve, level);
    if let (Some(t_data), Some(levels)) = (talent_data, talent_levels) {
        crate::features::cat::logic::talents::apply_talent_stats(&leveled, t_data, levels)
    } else {
        leveled
    }
}