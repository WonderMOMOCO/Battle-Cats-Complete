use std::cell::RefCell;
use std::collections::HashMap;

use iced::alignment::Horizontal;
use iced::widget::image::Handle;
use iced::widget::{button, column, container, image as iced_image, row, text, tooltip};
use iced::{Alignment, Element, Length, Theme};
use nyanko::chapter::map::{BonusType, RuleType, ScoreBonusMapEntry, SpecialRulesMapEntry};
use nyanko::chapter::stage::{BossType, EnemyAmount};
use nyanko::common::{strip_html_tags, BreakHandling};
use tracing::warn;

use kore::common::context::GlobalContext;
use kore::domains::enemy::scanner::EnemyEntry;
use kore::domains::stage::{restrictions, Map, Stage};

use crate::app::theme;
use crate::editor;
use crate::common::item_icon;
use crate::widget::{section, subsection};

const MAX_ICON_SIZE: f32 = 32.0;
const HEADER_TEXT_SIZE: f32 = 12.0;
const CELL_TEXT_SIZE: f32 = 12.0;
const CELL_PADDING: [u16; 2] = [4, 6];
const COLUMN_SPACING: f32 = 6.0;
const BODY_SPACING: f32 = 10.0;
const LINE_SPACING: f32 = 2.0;

const ENEMY_COLUMN: u16 = 4;
const COUNT_COLUMN: u16 = 5;
const MAG_COLUMN: u16 = 9;
const MAG_SPLIT_COLUMN: u16 = 9;
const BASE_COLUMN: u16 = 6;
const SPAWN_COLUMN: u16 = 6;
const RESPAWN_COLUMN: u16 = 9;
const LAYER_COLUMN: u16 = 6;
const BOSS_COLUMN: u16 = 8;
const SCORE_COLUMN: u16 = 5;
const KILLS_COLUMN: u16 = 5;

fn format_enemy_amount(spawn_amount: &EnemyAmount) -> String {
    match spawn_amount {
        EnemyAmount::Infinite => "∞".to_string(),
        EnemyAmount::Limit(limit_amount) => limit_amount.to_string(),
    }
}

fn format_spawn_timer(time_flag: u32, start_frame: u32, base_hp_perc: u32) -> String {
    if base_hp_perc != 100 && time_flag == 0 {
        return "-".to_string();
    }
    format!("{}f", start_frame)
}

fn format_enemy_respawn(spawn_amount: &EnemyAmount, respawn_min_frames: u32, respawn_max_frames: u32) -> String {
    if spawn_amount == &EnemyAmount::Limit(1) {
        return "-".to_string();
    }

    if respawn_min_frames == respawn_max_frames {
        return format!("{}f", respawn_min_frames);
    }

    format!("{}f ~ {}f", respawn_min_frames, respawn_max_frames)
}

fn format_layer(layer_min: i32, layer_max: i32) -> String {
    if layer_min == layer_max {
        return layer_min.to_string();
    }
    format!("{} ~ {}", layer_min, layer_max)
}

fn format_boss_type(boss_type: &BossType) -> String {
    match boss_type {
        BossType::None => "-".to_string(),
        BossType::Boss => "Yes".to_string(),
        BossType::ScreenShake => "Yes (Shake)".to_string(),
        BossType::Unknown(_) => "Unknown".to_string(),
    }
}

fn format_kill_count(kill_count: u32) -> String {
    if kill_count == 0 { "-".to_string() } else { kill_count.to_string() }
}

fn format_score(score: u32) -> String {
    if score == 0 { "-".to_string() } else { score.to_string() }
}

fn format_base_hp_percentage(base_hp_perc: u32, is_dojo_mechanic: bool) -> String {
    if is_dojo_mechanic {
        return base_hp_perc.to_string();
    }
    if base_hp_perc == 100 {
        return "-".to_string();
    }
    format!("{}%", base_hp_perc)
}

fn format_special_rule(rule: &SpecialRulesMapEntry, global_ctx: &GlobalContext) -> String {
    let explanation_key = if !rule.explanation_label.is_empty() {
        rule.explanation_label.clone()
    } else if let Some(prefix) = rule.name_label.trim().strip_suffix("Name") {
        format!("{prefix}Explanation")
    } else {
        String::new()
    };

    let raw_description = if !explanation_key.is_empty() {
        global_ctx.localizable.lookup(&explanation_key).unwrap_or_default()
    } else {
        &String::new()
    };

    let mut description = strip_html_tags(raw_description, BreakHandling::Space);

    if description.is_empty() {
        warn!(key = %explanation_key, name_label = %rule.name_label, "missing localization for special rule explanation, falling back to raw enum parsing");

        let mut fallback = String::new();
        for target_rule in &rule.rules {
            let formatted_rule = match target_rule {
                RuleType::TrustFund(params) => format!("Trust Fund {:?}", params),
                RuleType::CooldownEquality(params) => format!("Cooldown Equality {:?}", params),
                RuleType::RarityLimit(params) => format!("Rarity Limit {:?}", params),
                RuleType::CheapLabor(params) => format!("Cheap Labor {:?}", params),
                RuleType::CatCost(params) => format!("Cat Cost {:?}", params),
                RuleType::CatProduction(params) => format!("Cat Production {:?}", params),
                RuleType::TotalDeployLimit(params) => format!("Total Deploy Limit {:?}", params),
                RuleType::MoreThanOne(params) => format!("More Than One {:?}", params),
                RuleType::MegaCatCannon(params) => format!("Mega Cat Cannon {:?}", params),
                RuleType::UniformMotion(params) => format!("Uniform Motion {:?}", params),
                RuleType::CompoundingCost(params) => format!("Compounding Cost {:?}", params),
                RuleType::Unknown(id, params) => format!("Unknown Rule {} {:?}", id, params),
            };
            fallback.push_str(&formatted_rule);
            fallback.push('\n');
        }
        description = fallback.trim().to_string();
    } else {
        for target_rule in &rule.rules {
            let parameters = match target_rule {
                RuleType::TrustFund(params) => params,
                RuleType::CooldownEquality(params) => params,
                RuleType::RarityLimit(params) => params,
                RuleType::CheapLabor(params) => params,
                RuleType::CatCost(params) => params,
                RuleType::CatProduction(params) => params,
                RuleType::TotalDeployLimit(params) => params,
                RuleType::MoreThanOne(params) => params,
                RuleType::MegaCatCannon(params) => params,
                RuleType::UniformMotion(params) => params,
                RuleType::CompoundingCost(params) => params,
                RuleType::Unknown(_, params) => params,
            };

            for param in parameters {
                description = description.replacen("%d", &param.to_string(), 1);
            }
        }
    }

    description
}

fn format_score_bonus(score_bonus: &ScoreBonusMapEntry, global_ctx: &GlobalContext) -> String {
    let lookup_key = if !score_bonus.explanation_label.is_empty() {
        &score_bonus.explanation_label
    } else {
        &score_bonus.name_label
    };

    let raw_description = global_ctx.localizable.lookup(lookup_key).unwrap_or_default();
    let mut description = strip_html_tags(raw_description, BreakHandling::Space);

    if description.is_empty() {
        description = format!("【{}】 Localization data missing.", lookup_key);
    } else {
        for bonus in &score_bonus.bonuses {
            let parameters = match bonus {
                BonusType::Weaken(params) => params,
                BonusType::Freeze(params) => params,
                BonusType::Slow(params) => params,
                BonusType::Knockback(params) => params,
                BonusType::StrongAttack(params) => params,
                BonusType::MassiveDamage(params) => params,
                BonusType::StrongDefense(params) => params,
                BonusType::Resist(params) => params,
                BonusType::Unknown(_, params) => params,
            };

            for param in parameters {
                description = description.replacen("%d", &param.to_string(), 1);
            }
        }
    }

    description
}

fn header_cell<'a>(label: &'a str, portion: u16) -> Element<'a, super::Message> {
    theme::table_cell_text(label, Length::FillPortion(portion)).size(HEADER_TEXT_SIZE).into()
}

fn value_cell<'a>(label: impl ToString, portion: u16) -> Element<'a, super::Message> {
    container(theme::centered_text(label.to_string()).size(CELL_TEXT_SIZE))
        .width(Length::FillPortion(portion))
        .align_x(Horizontal::Center)
        .into()
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<u32, Handle>>,
}

impl State {
    pub fn forget(&self, id: u32) {
        self.icon_cache.borrow_mut().remove(&id);
    }

    pub fn clear_icons(&self) {
        self.icon_cache.borrow_mut().clear();
    }

    fn icon(&self, id: u32, path: &std::path::Path) -> Option<Handle> {
        if let Some(cached) = self.icon_cache.borrow().get(&id) {
            return Some(cached.clone());
        }

        let handle = item_icon::load_scaled(path, MAX_ICON_SIZE as u32)?;
        self.icon_cache.borrow_mut().insert(id, handle.clone());
        Some(handle)
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        map: &'a Map,
        selected_crown: u8,
        enemy_registry: &'a HashMap<u32, EnemyEntry>,
        enemy_name_registry: &'a [String],
        global_ctx: GlobalContext<'a>,
    ) -> Element<'a, super::Message> {
        let mut content = column![].spacing(BODY_SPACING);

        let restriction_lines = restrictions::parse_restrictions(stage, selected_crown as i8, global_ctx);

        if !restriction_lines.is_empty() {
            let mut restriction_col = column![].spacing(LINE_SPACING);
            for line in &restriction_lines {
                restriction_col = restriction_col.push(text(line.clone()));
            }
            content = content.push(subsection("Restrictions", restriction_col));
        }

        if let Some(rule) = &map.special_rules {
            let mut rules_col = column![text(format_special_rule(rule, &global_ctx))].spacing(LINE_SPACING);
            if !map.invalid_combos.is_empty() {
                rules_col = rules_col.push(text(format!("Disabled Combos: {} total", map.invalid_combos.len())));
            }
            content = content.push(subsection("Rules", rules_col));
        }

        if let Some(score_bonus) = &map.score_bonuses {
            content = content.push(subsection("Score Bonus", text(format_score_bonus(score_bonus, &global_ctx))));
        }

        if stage.enemies.is_empty() {
            return editor::target(
                section("Battleground", Length::Fixed(super::CONTENT_WIDTH), content.push(text("No enemies defined for this stage."))),
                editor::Target::StageGround,
            );
        }

        let crown_mag = stage.crown_magnification(selected_crown.saturating_add(1)).unwrap_or(100);

        let show_score_column = stage.enemies.iter().any(|enemy| enemy.score > 0);
        let is_dojo_mechanic = stage.enemies.iter().any(|enemy| enemy.base_hp_perc > 100);
        let has_split_magnification = stage.enemies.iter().any(|enemy| {
            let final_hp_mag = (enemy.magnification * crown_mag) / 100;
            let final_atk_mag = (enemy.atk_magnification * crown_mag) / 100;
            final_hp_mag != final_atk_mag
        });

        let mag_column = if has_split_magnification { MAG_SPLIT_COLUMN } else { MAG_COLUMN };
        let mag_header = if has_split_magnification { "Magnification %\n(HP% / ATK%)" } else { "Magnification %" };
        let base_header = if is_dojo_mechanic { "Dmg #" } else { "Base %" };

        let mut header_row = row![
            header_cell("Enemy", ENEMY_COLUMN),
            header_cell("Count", COUNT_COLUMN),
            header_cell(mag_header, mag_column),
            header_cell(base_header, BASE_COLUMN),
            header_cell("Spawn", SPAWN_COLUMN),
            header_cell("Respawn", RESPAWN_COLUMN),
            header_cell("Layer", LAYER_COLUMN),
            header_cell("Boss", BOSS_COLUMN),
        ].spacing(COLUMN_SPACING).align_y(Alignment::Center);

        if show_score_column {
            header_row = header_row.push(header_cell("Score", SCORE_COLUMN));
        }
        header_row = header_row.push(header_cell("Kills", KILLS_COLUMN));

        let mut grid = column![
            container(header_row).style(theme::zebra_table_header).padding(CELL_PADDING).width(Length::Fill)
        ].width(Length::Fixed(super::CONTENT_WIDTH));

        for (index, spawn) in stage.enemies.iter().enumerate() {
            let resolved_name = enemy_name_registry
                .get(spawn.enemy_id as usize)
                .filter(|name| !name.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{:03}-E", spawn.enemy_id));

            let final_hp_mag = (spawn.magnification * crown_mag) / 100;
            let final_atk_mag = (spawn.atk_magnification * crown_mag) / 100;
            let same_mag = final_hp_mag == final_atk_mag;

            let icon_content: Element<'a, super::Message> = enemy_registry.get(&spawn.enemy_id)
                .and_then(|entry| entry.icon_path.as_ref())
                .and_then(|path| self.icon(spawn.enemy_id, path))
                .map_or_else(
                    || text(format!("{:03}", spawn.enemy_id)).size(11).into(),
                    |handle| iced_image(handle).width(Length::Fixed(MAX_ICON_SIZE)).height(Length::Fixed(MAX_ICON_SIZE)).into(),
                );

            let mag_input = if same_mag { final_hp_mag.to_string() } else { format!("{}/{}", final_hp_mag, final_atk_mag) };

            let clickable_icon = button(icon_content)
                .padding(0)
                .on_press(super::Message::JumpToEnemy(spawn.enemy_id, mag_input))
                .style(|_theme: &Theme, _status| button::Style::default());

            let icon_element: Element<'a, super::Message> = tooltip(
                clickable_icon,
                container(text(resolved_name.clone())).padding(6).style(container::bordered_box),
                tooltip::Position::Top,
            ).into();

            let formatted_mag = if same_mag {
                format!("{}%", final_hp_mag)
            } else {
                format!("{}% / {}%", final_hp_mag, final_atk_mag)
            };

            let mut value_row = row![
                container(icon_element).width(Length::FillPortion(ENEMY_COLUMN)).align_x(Horizontal::Center).align_y(Alignment::Center),
                value_cell(format_enemy_amount(&spawn.amount), COUNT_COLUMN),
                value_cell(formatted_mag, mag_column),
                value_cell(format_base_hp_percentage(spawn.base_hp_perc, is_dojo_mechanic), BASE_COLUMN),
                value_cell(format_spawn_timer(spawn.time_flag, spawn.start_frame, spawn.base_hp_perc), SPAWN_COLUMN),
                value_cell(format_enemy_respawn(&spawn.amount, spawn.respawn_min, spawn.respawn_max), RESPAWN_COLUMN),
                value_cell(format_layer(spawn.layer_min, spawn.layer_max), LAYER_COLUMN),
                value_cell(format_boss_type(&spawn.boss_type), BOSS_COLUMN),
            ].spacing(COLUMN_SPACING).align_y(Alignment::Center);

            if show_score_column {
                value_row = value_row.push(value_cell(format_score(spawn.score), SCORE_COLUMN));
            }
            value_row = value_row.push(value_cell(format_kill_count(spawn.kill_count), KILLS_COLUMN));

            grid = grid.push(
                container(value_row)
                    .style(move |theme: &Theme| theme::zebra_table_row(theme, index))
                    .padding(CELL_PADDING)
                    .width(Length::Fill),
            );
        }

        editor::target(
            section("Battleground", Length::Fixed(super::CONTENT_WIDTH), content.push(grid)),
            editor::Target::StageGround,
        )
    }
}
