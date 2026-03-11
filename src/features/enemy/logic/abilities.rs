use crate::features::settings::logic::Settings;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::registry::{self, DisplayGroup};
use crate::global::game::abilities::{AbilityItem, CustomIcon};
use crate::global::game::img015;

pub fn collect_ability_data(
    stats: &EnemyRaw,
    settings: &Settings,
    magnification: i32,
) -> (Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>, Vec<AbilityItem>) {
    
    let mut group_trait = Vec::new();
    let mut group_headline_1 = Vec::new();
    let mut group_headline_2 = Vec::new();
    let mut group_body_1 = Vec::new();
    let mut group_body_2 = Vec::new();
    let mut group_footer = Vec::new();

    for def in registry::ENEMY_ABILITY_REGISTRY {
        if def.group == DisplayGroup::Hidden { continue; } 

        let val = (def.getter)(stats);
        if val > 0 || val == -1 {
            let dur = if let Some(d_get) = def.duration_getter { d_get(stats) } else { 0 };
            let text = (def.formatter)(val, stats, dur, magnification);

            let mut custom_icon = def.custom_icon;
            if def.name == "Multi-Hit" && settings.general.game_language == "--" {
                custom_icon = CustomIcon::None;
            }

            let mut final_icon = def.icon_id;
            if def.name == "Wave Attack" && stats.mini_wave > 0 { final_icon = img015::ICON_MINI_WAVE; }
            else if def.name == "Surge Attack" && stats.mini_surge > 0 { final_icon = img015::ICON_MINI_SURGE; }

            let item = AbilityItem { icon_id: final_icon, text, custom_icon, border_id: None };

            match def.group {
                DisplayGroup::Type => group_trait.push(item),
                DisplayGroup::Headline1 => group_headline_1.push(item),
                DisplayGroup::Headline2 => group_headline_2.push(item),
                DisplayGroup::Body1 => group_body_1.push(item),
                DisplayGroup::Body2 => group_body_2.push(item),
                DisplayGroup::Footer => group_footer.push(item),
                DisplayGroup::Hidden => {}
            }
        }
    }

    (group_trait, group_headline_1, group_headline_2, group_body_1, group_body_2, group_footer)
}