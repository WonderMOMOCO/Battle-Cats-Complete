use crate::features::settings::logic::Settings;
use crate::features::enemy::data::t_unit::EnemyRaw;
use crate::features::enemy::registry::{self, DisplayGroup, AttrUnit, AbilityIcon, Magnification};
use crate::global::game::abilities::{AbilityItem, CustomIcon};

pub fn collect_ability_data(
    stats: &EnemyRaw,
    _settings: &Settings,
    magnification: Magnification,
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
        
        if !attrs.is_empty() {
            let val = attrs.first().map(|(_, v, _)| *v).unwrap_or(0);
            let dur = attrs.iter().find(|(_, _, u)| *u == AttrUnit::Frames).map(|(_, v, _)| *v).unwrap_or(0);
            
            let text = (def.formatter)(val, stats, dur, magnification);

            let (final_icon, custom_icon) = match def.icon {
                AbilityIcon::Standard(id) => (Some(id), CustomIcon::None),
                AbilityIcon::Custom(icon) => (None, icon),
            };

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