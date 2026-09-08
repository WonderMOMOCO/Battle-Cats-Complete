use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use iced::widget::image::Handle;
use iced::widget::{column, container, image as iced_image, row, rule};
use iced::{Alignment, Element, Length};
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;
use nyanko::chapter::Category;

use kore::domains::stage::{cost, files as stage_files, Map, Stage, StageDataState};
use kore::{ItemStore, Vfs};

use crate::app::theme;
use crate::common::item_icon;
use crate::editor;
use crate::widget::{grid_header, grid_value, section};


const MAP_IMG_HEIGHT: f32 = 50.0;
const STAGE_IMG_HEIGHT: f32 = 35.0;
const BANNER_GAP: f32 = 12.0;
const RULE_HEIGHT: f32 = 1.0;
const GRID_SPACING: f32 = 4.0;
const FALLBACK_NAME_SIZE: f32 = 32.0;

fn format_diff(difficulty_level: u16) -> String {
    if difficulty_level == 0 {
        return "-".to_string();
    }
    format!("★{}", difficulty_level)
}

fn format_base(anim_id: u32, standard_id: i32) -> (String, String) {
    if anim_id != 0 {
        let calculated_id = anim_id.saturating_sub(2);
        return ("Base Enemy".to_string(), format!("{:03}-E", calculated_id));
    }
    ("Base Img".to_string(), standard_id.to_string())
}

fn format_bool(status: bool, true_string: &str, false_string: &str) -> String {
    if status { true_string.to_string() } else { false_string.to_string() }
}

fn format_respawn(minimum_spawn: u32, maximum_spawn: u32) -> String {
    if minimum_spawn == maximum_spawn {
        return format!("{}f", minimum_spawn);
    }
    format!("{}f ~ {}f", minimum_spawn, maximum_spawn)
}

fn format_boss_bgm(boss_track: i16, init_track: u32, change_percent: u32) -> String {
    if boss_track < 0 || boss_track as u32 == init_track || change_percent == 100 {
        return "-".to_string();
    }
    boss_track.to_string()
}

fn format_time(time_limit: u32) -> String {
    if time_limit == 0 {
        return "-".to_string();
    }
    format!("{}m", time_limit)
}

fn get_skip_status(
    category: &Category,
    map_id: u32,
    lock_registry: &HashMap<u32, LockSkipDataEntry>,
    cpu_setting: &ScatCpuSetting,
) -> String {
    if let Some(global_id) = category.global_map_id(map_id)
        && let Some(entry) = lock_registry.get(&global_id)
        && entry.excluded_map_id == global_id
    {
        return "N/A".to_string();
    }

    if cpu_setting.super_cpu_consume_amount > 0 {
        return format!("{} CPUs", cpu_setting.super_cpu_consume_amount);
    }

    "-".to_string()
}

#[derive(Default)]
pub struct State {
    icon_cache: RefCell<HashMap<String, (Handle, u32, u32)>>,
}

impl State {
    pub fn clear_icons(&self) {
        self.icon_cache.borrow_mut().clear();
    }

    fn texture(&self, key: &str, path: &Path) -> Option<(Handle, u32, u32)> {
        if let Some(cached) = self.icon_cache.borrow().get(key) {
            return Some(cached.clone());
        }

        let loaded = item_icon::load_cropped(path)?;
        self.icon_cache.borrow_mut().insert(key.to_string(), loaded.clone());
        Some(loaded)
    }

    pub fn view<'a>(
        &'a self,
        stage: &'a Stage,
        map: &'a Map,
        vfs: &Vfs,
        data: &StageDataState,
        items: &ItemStore,
        selected_crown: u8,
    ) -> Element<'a, super::Message> {
        let image_prefix = stage.category.image_prefix();

        let map_key = format!("map_img_{:?}_{}", stage.category, stage.map_id);
        let stage_key = format!("stage_img_{:?}_{}_{}", stage.category, stage.map_id, stage.stage_id);

        let map_texture = vfs
            .find(&stage_files::map_banner_file(stage.map_id, &image_prefix))
            .and_then(|path| self.texture(&map_key, &path));

        let stage_texture = vfs
            .find(&stage_files::stage_banner_file(&stage.category, stage.map_id, stage.stage_id, &image_prefix))
            .and_then(|path| self.texture(&stage_key, &path));

        let banner_row = row![
            editor::target(
                banner_element(map_texture, MAP_IMG_HEIGHT, &map.name),
                editor::Target::MapBanner,
            ),
            editor::target(
                banner_element(stage_texture, STAGE_IMG_HEIGHT, &stage.name),
                editor::Target::StageBanner,
            ),
        ]
            .spacing(BANNER_GAP)
            .align_y(Alignment::End);

        let crown_row = super::crowns::view(stage, selected_crown);

        let crown_magnification = stage.crown_magnification(selected_crown.saturating_add(1)).unwrap_or(100);

        let final_hp = if stage.anim_base_id != 0 {
            (stage.base_hp * crown_magnification) / 100
        } else {
            stage.base_hp
        };

        let is_dojo = stage.category == Category::DojoRankingEvents || stage.category == Category::DojoHallOfInitiates;

        let hp_header = if is_dojo { "Time Limit" } else { "Base HP" };
        let hp_value = if is_dojo { format_time(stage.time_limit) } else { final_hp.to_string() };

        let cost = cost::resolve_cost(stage, items, vfs);
        let energy_header = cost.header.as_str();
        let energy_value = cost.value;

        let difficulty_value = format_diff(stage.difficulty);
        let continue_value = format_bool(stage.is_no_continues, "No", "Yes");
        let indestructible_value = format_bool(stage.is_base_indestructible, "Active", "-");
        let (base_header, base_value) = format_base(stage.anim_base_id, stage.base_id);
        let respawn_value = format_respawn(stage.min_spawn, stage.max_spawn);
        let boss_bgm_value = format_boss_bgm(stage.boss_track, stage.init_track, stage.bgm_change_percent);
        let skip_value = get_skip_status(&stage.category, stage.map_id, &data.lock_skip_registry, &data.scat_cpu_setting);

        let headers_row_1 = row![
            grid_header(hp_header),
            grid_header("Width"),
            grid_header(energy_header),
            grid_header("XP"),
            grid_header("Boss Guard"),
            grid_header("Max Enemy"),
            grid_header("Respawn"),
        ].spacing(GRID_SPACING);

        let width_value = stage.width.to_string();
        let xp_value = stage.xp.to_string();
        let max_enemies_value = stage.max_enemies.to_string();

        let values_row_1 = row![
            grid_value(hp_header, &hp_value),
            grid_value("Width", &width_value),
            grid_value(energy_header, &energy_value),
            grid_value("XP", &xp_value),
            grid_value("Boss Guard", &indestructible_value),
            grid_value("Max Enemy", &max_enemies_value),
            grid_value("Respawn", &respawn_value),
        ].spacing(GRID_SPACING);

        let headers_row_2 = row![
            grid_header(&base_header),
            grid_header("Background"),
            grid_header("Music"),
            grid_header("Boss Music"),
            grid_header("Difficulty"),
            grid_header("CPU Skip"),
            grid_header("Continues"),
        ].spacing(GRID_SPACING);

        let background_value = stage.background_id.to_string();
        let music_value = stage.init_track.to_string();

        let values_row_2 = row![
            grid_value(&base_header, &base_value),
            grid_value("Background", &background_value),
            grid_value("Music", &music_value),
            grid_value("Boss Music", &boss_bgm_value),
            grid_value("Difficulty", &difficulty_value),
            grid_value("CPU Skip", &skip_value),
            grid_value("Continues", &continue_value),
        ].spacing(GRID_SPACING);

        let grids = column![headers_row_1, values_row_1, headers_row_2, values_row_2].spacing(GRID_SPACING);

        column![
            banner_row,
            rule::horizontal(RULE_HEIGHT),
            crown_row,
            section("Information", Length::Fixed(super::CONTENT_WIDTH), grids),
        ]
            .spacing(8)
            .into()
    }
}

fn banner_element<'a, Message: 'a>(texture: Option<(Handle, u32, u32)>, target_height: f32, fallback_name: &str) -> Element<'a, Message> {
    texture.map_or_else(
        || container(theme::bold_text(fallback_name).size(FALLBACK_NAME_SIZE)).into(),
        |(handle, width, height)| {
            let display_width = width as f32 * (target_height / height as f32);
            iced_image(handle).width(Length::Fixed(display_width)).height(Length::Fixed(target_height)).into()
        },
    )
}
