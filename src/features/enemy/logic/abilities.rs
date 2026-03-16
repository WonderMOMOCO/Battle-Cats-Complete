use crate::features::settings::logic::Settings;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::registry::{self, DisplayGroup, AttrUnit};
use crate::global::game::abilities::AbilityItem;
use crate::global::game::img015;

pub fn collect_ability_data(
    stats: &EnemyRaw,
    _settings: &Settings,
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

        let attrs = (def.get_attributes)(stats);
        
        // If the vector isn't empty, the enemy has the ability
        if !attrs.is_empty() {
            // Grab the primary value and the duration (if it exists)
            let val = attrs.first().map(|(_, v, _)| *v).unwrap_or(0);
            let dur = attrs.iter().find(|(_, _, u)| *u == AttrUnit::Frames).map(|(_, v, _)| *v).unwrap_or(0);
            
            let text = (def.formatter)(val, stats, dur, magnification);

            let custom_icon = def.custom_icon;

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