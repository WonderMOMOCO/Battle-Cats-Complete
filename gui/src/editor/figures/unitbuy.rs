use iced::alignment::Horizontal;
use iced::widget::{column, container, text};
use iced::{Element, Length};

use super::resolved::{Choice, Rule};
use super::{cards, Draft, Message};

pub(super) fn view<'a>(draft: &'a Draft, width: f32, query: &'a str, armed: bool) -> Element<'a, Message> {
    searched(draft, width, query, armed, None, 0.0)
}

pub(super) fn searched<'a>(
    draft: &'a Draft,
    width: f32,
    query: &'a str,
    armed: bool,
    notice: Option<&'a str>,
    notice_size: f32,
) -> Element<'a, Message> {
    bounded(draft, width, query, armed, notice, notice_size, draft.len())
}

pub(super) fn bounded<'a>(
    draft: &'a Draft,
    width: f32,
    query: &'a str,
    armed: bool,
    notice: Option<&'a str>,
    notice_size: f32,
    upto: usize,
) -> Element<'a, Message> {
    let needle = query.trim().to_lowercase();

    let shown: Vec<usize> = (0..upto.min(draft.len()))
        .filter(|index| needle.is_empty() || draft.label(*index).to_lowercase().contains(&needle))
        .collect();

    let search = cards::search(query, width, "Search Field...");

    let top = match notice {
        Some(notice) => cards::header(column![
            container(text(notice).size(notice_size).align_x(Horizontal::Center).style(text::secondary))
                .width(Length::Fill)
                .center_x(Length::Fill),
            search,
        ]
        .spacing(NOTICE_GAP)),
        None => search,
    };

    cards::shell(Some(top), cards::grid(draft, width, &shown, None), cards::footer(vec![cards::sync(armed)]))
}

const NOTICE_GAP: f32 = 6.0;

pub(super) fn rule(field: &str) -> Option<Rule> {
    match field {
        "rarity" => Some(Rule::Choice(&const {
            [
                Choice::new(0, "N"),
                Choice::new(1, "EX"),
                Choice::new(2, "RR"),
                Choice::new(3, "SR"),
                Choice::new(4, "UR"),
                Choice::new(5, "LR"),
            ]
        })),
        "chapter_unlock_requirement" => Some(Rule::Offset(1)),

        "guide_order" | "evolve_level_xp" | "true_form_unlock_level" | "ultra_form_unlock_level"
        | "level_cap_standard" | "version_added" | "egg_id_normal" | "egg_id_evolved" => Some(Rule::Floor(-1)),

        "currency_type" | "stage_unlock_requirement" | "purchase_cost" | "upgrade_cost_1" | "upgrade_cost_2"
        | "upgrade_cost_3" | "upgrade_cost_4" | "upgrade_cost_5" | "upgrade_cost_6" | "upgrade_cost_7"
        | "upgrade_cost_8" | "upgrade_cost_9" | "upgrade_cost_10"
        | "sell_xp_yield" | "level_cap_ch2"
        | "base_max_plus_level" | "level_cap_ch1" | "true_form_id" | "ultra_form_id"
        | "true_form_xp_cost"
        | "true_form_material_1_id" | "true_form_material_1_quantity" | "true_form_material_2_id"
        | "true_form_material_2_quantity" | "true_form_material_3_id" | "true_form_material_3_quantity"
        | "true_form_material_4_id" | "true_form_material_4_quantity" | "true_form_material_5_id"
        | "true_form_material_5_quantity" | "ultra_form_xp_cost" | "ultra_form_material_1_id"
        | "ultra_form_material_1_quantity" | "ultra_form_material_2_id" | "ultra_form_material_2_quantity"
        | "ultra_form_material_3_id" | "ultra_form_material_3_quantity" | "ultra_form_material_4_id"
        | "ultra_form_material_4_quantity" | "ultra_form_material_5_id" | "ultra_form_material_5_quantity"
        | "level_cap_catseye" | "level_cap_plus" | "normal_evolution_y_offset"
        | "evolved_evolution_y_offset" | "true_evolution_y_offset" | "ultra_evolution_y_offset"
        | "sell_np_yield" => Some(Rule::Plain),

        "unknown_17" | "unknown_21" | "unknown_56" | "unknown_59" | "unknown_60" => Some(Rule::Opaque),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::rule;
    use crate::editor::figures::schema::{self, Subject};

    #[test]
    fn every_buy_column_has_a_rule() {
        for entry in schema::of(Subject::Buy).order() {
            assert!(
                rule(entry.field).is_some(),
                "unitbuy: nyanko publishes {}, which no resolved rule arm names",
                entry.field,
            );
        }
    }
}
