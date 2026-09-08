use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::Category;
use nyanko::chapter::map::{DropItemEntry, LockSkipDataEntry, MapOptionEntry, RuleType, ScoreBonusMapEntry, SpecialRulesMapEntry, SpecialRulesMapOptionEntry};
use nyanko::chapter::stage::{CharaGroupEntry, CostType, FixedFormationEntry, MapStageData, ScatCpuSetting, StageNameEntry, StageOptionEntry, get_hardcoded_xp};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, trace, warn};

use crate::common::job::Ticker;
use crate::common::io::cache::{self, Scan};
use crate::domains::cat::waiter::unitexplanation;
use crate::domains::settings::ScannerConfig;
use crate::{Vfs, Vault};

use super::files;
use super::waiter::{
    battleground, certification_preset, drop_chara, lockskipdata, mapstagedata, scatcpusetting,
    stagename
};
use super::{GlobalMapId, GlobalStageId, Map, Stage, StageRegistry};

const MAP_STAGE_DATA: &str = "MapStageData";
const STAGE: &str = "stage";
const CSV: &str = ".csv";
const PREFIX_EC: &str = "EC";
const STORY_PREFIXES: [&str; 4] = [PREFIX_EC, "W", "Space", "Z"];
const EC_CHAPTERS: [u32; 3] = [0, 1, 2];
const INVASION: &str = "Invasion";

type Names = Arc<[Box<str>]>;

struct Globs<'a> {
    vfs: &'a Vfs,
    memo: RwLock<FxHashMap<Box<str>, Names>>,
}

impl<'a> Globs<'a> {
    fn new(vfs: &'a Vfs) -> Self {
        Self { vfs, memo: RwLock::new(FxHashMap::default()) }
    }

    fn get(&self, prefix: &str) -> Names {
        if let Ok(memo) = self.memo.read()
            && let Some(hit) = memo.get(prefix)
        {
            return Arc::clone(hit);
        }

        let names: Names = self.vfs.glob(prefix).into();

        if let Ok(mut memo) = self.memo.write() {
            memo.insert(prefix.into(), Arc::clone(&names));
        }

        names
    }
}

struct ScanContext<'a> {
    pub vfs: &'a Vfs,
    pub globs: Globs<'a>,
    pub map_names: Arc<HashMap<u32, String>>,
    pub map_options: Arc<HashMap<u32, MapOptionEntry>>,
    pub stage_options: Arc<HashMap<u32, Vec<StageOptionEntry>>>,
    pub charagroups: Arc<HashMap<u32, CharaGroupEntry>>,
    pub drop_items: Arc<HashMap<u32, DropItemEntry>>,
    pub score_bonuses: Arc<HashMap<u32, ScoreBonusMapEntry>>,
    pub special_rules: Arc<HashMap<u32, SpecialRulesMapEntry>>,
    pub special_rule_options: Arc<HashMap<u8, SpecialRulesMapOptionEntry>>,
    pub ex_options: Arc<HashMap<u32, u32>>,
    pub difficulties: Arc<HashMap<u32, Vec<u16>>>,
    pub fixed_formations: Arc<HashMap<(u32, u8, u32), FixedFormationEntry>>,
}

struct StageCache;

impl cache::CacheSpec for StageCache {
    type Data = StageBundle;
    const FILE: &'static str = "stages_cache.bin";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageDictionaries {
    pub enemy_name_registry: Vec<String>,
    pub drop_chara_registry: HashMap<u32, u32>,
    pub unit_buy_registry: HashMap<u32, UnitBuy>,
    pub cat_name_registry: HashMap<u32, Vec<String>>,
    pub lock_skip_registry: HashMap<u32, LockSkipDataEntry>,
    pub scat_cpu_setting: ScatCpuSetting,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StageBundle {
    pub registry: StageRegistry,
    pub dictionaries: StageDictionaries,
}

type StageHarvest = (Vec<u32>, Vec<(GlobalStageId, Stage)>, Vec<(GlobalStageId, Box<str>)>);

struct CategoryInfo {
    prefix: String,
    data_prefix: String,
    category: Category,
    stage_names: HashMap<u32, StageNameEntry>,
}

struct MapJob {
    category_index: usize,
    map_id: u32,
}

pub fn purge() {
    cache::purge::<StageCache>();
}

pub fn hydrate() -> Option<(u64, StageBundle)> {
    let (hash, bundle) = cache::read::<StageCache>()?;
    debug!(
        hash,
        maps = bundle.registry.maps.len(),
        stages = bundle.registry.stages.len(),
        "hydrated stages from cache"
    );

    Some((hash, bundle))
}

pub fn persist(payload: &[u8]) {
    cache::store::<StageCache>(payload);
}

pub fn load(config: ScannerConfig, vault: Arc<Vault>, progress: impl Fn(usize, usize) + Sync) -> Scan<StageBundle> {
    scan(&config, &vault, progress)
}

#[instrument(level = "debug", skip(vault))]
fn build_dictionaries(vault: &Vault) -> StageDictionaries {
    trace!("Loading auxiliary stage dictionaries");
    let vfs = &vault.vfs;

    let enemy_name_registry = vault.vds.enemies.names(vfs).as_ref().clone();

    let drop_chara_registry = drop_chara(vfs, "drop_chara.csv");
    let lock_skip_registry = lockskipdata(vfs, "LockSkipData.csv");
    let scat_cpu_setting = scatcpusetting(vfs, "ScatCPUsetting.csv");

    let unit_buy_registry = vault.vds.cats.unitbuy(vfs);

    let cat_name_registry: HashMap<u32, Vec<String>> = unit_buy_registry
        .par_iter()
        .map(|(&unit_id, _)| {
            let names = unitexplanation(vfs, unit_id)
                .names
                .into_iter()
                .flatten()
                .map(|name| name.to_lowercase())
                .collect();
            (unit_id, names)
        })
        .collect();

    StageDictionaries {
        enemy_name_registry,
        drop_chara_registry,
        unit_buy_registry: unit_buy_registry.as_ref().clone(),
        cat_name_registry,
        lock_skip_registry,
        scat_cpu_setting,
    }
}

fn scan(config: &ScannerConfig, vault: &Vault, progress: impl Fn(usize, usize) + Sync) -> Scan<StageBundle> {
    info!("--- STAGE SCANNER INITIATED ---");
    let dictionaries = build_dictionaries(vault);
    let registry = scan_all(vault, progress);
    info!("--- STAGE SCANNER COMPLETE: Found {} maps and {} stages ---", registry.maps.len(), registry.stages.len());

    let bundle = StageBundle { registry, dictionaries };

    if bundle.registry.maps.is_empty() {
        warn!("Registry is empty! Skipping cache save to prevent overwriting with blank data.");
        return Scan { data: bundle, key: None, payload: None };
    }

    let key = cache::content_hash(vault.vfs.fingerprint(), config);
    let payload = cache::encode::<StageCache>(key, &bundle);

    Scan { data: bundle, key: Some(key), payload }
}

#[instrument(skip_all)]
pub fn scan_single(vault: &Vault, category: &Category, map_id: u32) -> StageRegistry {
    let vfs = &vault.vfs;
    let ctx = context(vault);
    let prefix = category.map_prefix();

    let data_prefix = vfs
        .glob(MAP_STAGE_DATA)
        .iter()
        .filter_map(|name| split_map_data(name).map(|(prefix, _)| prefix))
        .find(|found| Category::from_prefix(found) == *category)
        .unwrap_or_else(|| prefix.clone());

    let mut stage_names = HashMap::new();

    for file in files::stage_name_targets(&prefix) {
        stage_names = stagename(vfs, &file);

        if !stage_names.is_empty() {
            break;
        }
    }

    let info = CategoryInfo { prefix, data_prefix, category: category.clone(), stage_names };
    let registry = Mutex::new(StageRegistry::default());

    process_map(&registry, &info, map_id, &ctx);

    registry.into_inner().unwrap_or_default()
}

fn context(vault: &Vault) -> ScanContext<'_> {
    let vfs = &vault.vfs;
    let stages = &vault.vds.stages;

    ScanContext {
        vfs,
        globs: Globs::new(vfs),
        map_names: stages.map_names(vfs),
        map_options: stages.map_options(vfs),
        stage_options: stages.stage_options(vfs),
        charagroups: stages.charagroups(vfs),
        drop_items: stages.drop_items(vfs),
        score_bonuses: stages.score_bonuses(vfs),
        special_rules: stages.special_rules(vfs),
        special_rule_options: stages.special_rule_options(vfs),
        ex_options: stages.ex_options(vfs),
        difficulties: stages.difficulties(vfs),
        fixed_formations: stages.fixed_formations(vfs),
    }
}

fn scan_all(vault: &Vault, progress: impl Fn(usize, usize) + Sync) -> StageRegistry {
    let reg_mtx = Mutex::new(StageRegistry::default());

    info!("Loading global table dictionaries into ScanContext...");
    let ctx = context(vault);

    let (categories, jobs) = enumerate_maps(&vault.vfs);

    let total = jobs.len();
    let done = AtomicUsize::new(0);
    let ticker = Ticker::default();

    jobs.par_iter().for_each(|job| {
        if let Some(info) = categories.get(job.category_index) {
            process_map(&reg_mtx, info, job.map_id, &ctx);
        }

        let finished = done.fetch_add(1, Ordering::Relaxed) + 1;

        if ticker.ready(finished, total) {
            progress(finished, total);
        }
    });

    reg_mtx.into_inner().unwrap_or_default()
}

fn enumerate_maps(vfs: &Vfs) -> (Vec<CategoryInfo>, Vec<MapJob>) {
    info!("Discovering categories and maps from the file index");

    let mut discovered: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();

    for name in vfs.glob(MAP_STAGE_DATA) {
        if let Some((data_prefix, map_id)) = split_map_data(&name) {
            discovered.entry(data_prefix).or_default().insert(map_id);
        }
    }

    for story in STORY_PREFIXES {
        let ids = story_map_ids(vfs, story);
        if !ids.is_empty() {
            discovered.entry(story.to_string()).or_default().extend(ids);
        }
    }

    info!("Found {} categories to evaluate", discovered.len());

    let mut categories = Vec::new();
    let mut jobs = Vec::new();

    for (data_prefix, map_ids) in discovered {
        let category = Category::from_prefix(&data_prefix);
        let prefix = category.map_prefix();

        debug!("Scanning category {} ({} maps)", prefix, map_ids.len());

        let mut stage_names = HashMap::new();

        for file in files::stage_name_targets(&prefix) {
            stage_names = stagename(vfs, &file);
            if !stage_names.is_empty() {
                break;
            }
        }

        let category_index = categories.len();
        categories.push(CategoryInfo { prefix, data_prefix, category, stage_names });

        for map_id in map_ids {
            jobs.push(MapJob { category_index, map_id });
        }
    }

    (categories, jobs)
}

fn split_map_data(name: &str) -> Option<(String, u32)> {
    let body = name.strip_prefix(MAP_STAGE_DATA)?.strip_suffix(CSV)?;
    let (prefix, map) = body.rsplit_once('_')?;

    if prefix.is_empty() {
        return None;
    }

    Some((prefix.to_string(), map.parse().ok()?))
}

fn split_stage(name: &str, prefix: &str, map_id: u32) -> Option<u32> {
    let body = name.strip_prefix(STAGE)?.strip_prefix(prefix)?.strip_suffix(CSV)?;
    let (map, stage) = body.split_once('_')?;

    if map.parse::<u32>().ok()? != map_id {
        return None;
    }

    stage.parse().ok()
}

fn split_chapter_stage(name: &str) -> Option<u32> {
    name.strip_prefix(STAGE)?.strip_suffix(CSV)?.parse().ok()
}

fn story_map_ids(vfs: &Vfs, prefix: &str) -> BTreeSet<u32> {
    if prefix == PREFIX_EC {
        let present = vfs.glob(STAGE).iter().any(|name| split_chapter_stage(name).is_some());
        return if present { EC_CHAPTERS.into_iter().collect() } else { BTreeSet::new() };
    }

    vfs.glob(&format!("{}{}", STAGE, prefix))
        .iter()
        .filter_map(|name| {
            let body = name.strip_prefix(STAGE)?.strip_prefix(prefix)?.strip_suffix(CSV)?;
            let (map, stage) = body.split_once('_')?;
            stage.parse::<u32>().ok()?;
            map.parse().ok()
        })
        .collect()
}

fn stage_prefixes(category: &Category) -> Vec<String> {
    let mut prefixes = category.stage_prefix();

    if let Category::Unknown(prefix) = category
        && !prefix.to_uppercase().starts_with('R')
    {
        let restricted = format!("R{prefix}");

        if !prefixes.iter().any(|candidate| candidate.eq_ignore_ascii_case(&restricted)) {
            prefixes.push(restricted);
        }
    }

    prefixes
}

fn invasion_battleground(globs: &Globs<'_>, category: &Category, map_id: u32) -> Option<Box<str>> {
    for prefix in stage_prefixes(category) {
        if prefix.is_empty() {
            continue;
        }

        let names = globs.get(&format!("{}{}", STAGE, prefix));

        let found = names.iter().find(|name| {
            let Some(body) = name
                .strip_prefix(STAGE)
                .and_then(|body| body.strip_prefix(prefix.as_str()))
                .and_then(|body| body.strip_suffix(CSV))
            else {
                return false;
            };

            let Some((map, rest)) = body.split_once('_') else {
                return false;
            };

            map.parse::<u32>().ok() == Some(map_id) && rest.starts_with(INVASION)
        });

        if found.is_some() {
            return found.cloned();
        }
    }

    None
}

fn battlegrounds(globs: &Globs<'_>, category: &Category, map_id: u32) -> Vec<(u32, Box<str>)> {
    let mut found: Vec<(u32, Box<str>)> = Vec::new();

    for prefix in stage_prefixes(category) {
        if prefix.is_empty() {
            let names = globs.get(STAGE);

            found.extend(names.iter().filter_map(|name| Some((split_chapter_stage(name)?, name.clone()))));
            continue;
        }

        let names = globs.get(&format!("{}{}", STAGE, prefix));

        found.extend(names.iter().filter_map(|name| Some((split_stage(name, &prefix, map_id)?, name.clone()))));
    }

    found.sort_unstable_by_key(|(stage_id, _)| *stage_id);
    found.dedup_by_key(|(stage_id, _)| *stage_id);
    found
}

fn process_map(reg_mtx: &Mutex<StageRegistry>, info: &CategoryInfo, map_id: u32, ctx: &ScanContext) {
    let category = &info.category;
    let cat_prefix = info.prefix.as_str();
    let mut global_map_id = category.global_map_id(map_id);

    if global_map_id.is_none() || global_map_id == Some(map_id) {
        let routed_id = match (cat_prefix, map_id) {
            ("EC", 0) | ("Z", 0) => Some(3000),
            ("EC", 1) | ("Z", 1) => Some(3001),
            ("EC", 2) | ("Z", 2) => Some(3002),
            ("W", 4)  | ("Z", 4) => Some(3003),
            ("W", 5)  | ("Z", 5) => Some(3004),
            ("W", 6)  | ("Z", 6) => Some(3005),
            ("Space", 7) | ("Z", 7) => Some(3006),
            ("Space", 8) | ("Z", 8) => Some(3007),
            ("Space", 9) | ("Z", 9) => Some(3008),
            _ => global_map_id,
        };
        if routed_id.is_some() && routed_id != global_map_id {
            global_map_id = routed_id;
        }
    }

    let mut proxy_stage_names = None;

    if cat_prefix == "Z" {
        let proxy_prefix = match map_id {
            0..=2 => "EC",
            4..=6 => "W",
            7..=9 => "Space",
            _ => "",
        };

        if !proxy_prefix.is_empty() {
            debug!("Fetching proxy names from {} for Z map {}", proxy_prefix, map_id);

            for file in files::stage_name_targets(proxy_prefix) {
                let names = stagename(ctx.vfs, &file);
                if names.is_empty() { continue; }

                proxy_stage_names = Some(names);
                break;
            }
        }
    }

    let active_stage_names = proxy_stage_names.as_ref().unwrap_or(&info.stage_names);

    let map_display_name = global_map_id
        .and_then(|id| ctx.map_names.get(&id))
        .filter(|name| !name.is_empty())
        .cloned()
        .unwrap_or_else(|| format!("{:03}", map_id));

    load_map(
        reg_mtx,
        info,
        map_id,
        &map_display_name,
        active_stage_names,
        ctx,
        global_map_id
    );
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
fn load_map(
    reg_mtx: &Mutex<StageRegistry>,
    info: &CategoryInfo,
    map_id: u32,
    map_display_name: &str,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>
) {
    let category = &info.category;
    let map_opt = global_map_id
        .and_then(|id| ctx.map_options.get(&id))
        .cloned()
        .unwrap_or_default();

    let map_key = GlobalMapId { category: category.clone(), map: map_id };

    let special_rules = global_map_id.and_then(|id| ctx.special_rules.get(&id)).cloned();
    let mut invalid_combos = Vec::new();

    if let Some(rule) = &special_rules {
        for target_rule in &rule.rules {
            let rule_id = match target_rule {
                RuleType::TrustFund(_) => 0,
                RuleType::CooldownEquality(_) => 1,
                RuleType::RarityLimit(_) => 3,
                RuleType::CheapLabor(_) => 4,
                RuleType::CatCost(_) => 5,
                RuleType::CatProduction(_) => 6,
                RuleType::TotalDeployLimit(_) => 7,
                RuleType::MoreThanOne(_) => 8,
                RuleType::MegaCatCannon(_) => 9,
                RuleType::UniformMotion(_) => 10,
                RuleType::CompoundingCost(_) => 11,
                RuleType::Unknown(id, _) => *id,
            };

            if let Some(opt) = ctx.special_rule_options.get(&rule_id) {
                invalid_combos.extend(&opt.invalid_combo_ids);
            }
        }
        invalid_combos.sort_unstable();
        invalid_combos.dedup();
    }

    let mut map_struct = Map {
        name: map_display_name.to_string(),
        category: category.clone(),
        map_id,
        stages: Vec::new(),
        max_crowns: map_opt.max_crowns,
        has_abyss: map_opt.has_abyss,
        crown_1_mag: map_opt.crown_1_mag,
        crown_2_mag: map_opt.crown_2_mag,
        crown_3_mag: map_opt.crown_3_mag,
        crown_4_mag: map_opt.crown_4_mag,
        reset_type: map_opt.reset_type,
        max_clears: map_opt.max_clears,
        cooldown_minutes: map_opt.cooldown_minutes,
        hidden_upon_clear: map_opt.hidden_upon_clear,
        comment: map_opt.comment,
        ex_invasion: global_map_id.and_then(|id| ctx.ex_options.get(&id)).cloned(),
        score_bonuses: global_map_id.and_then(|id| ctx.score_bonuses.get(&id)).cloned(),
        special_rules,
        invalid_combos,
        drop_items: global_map_id.and_then(|id| ctx.drop_items.get(&id)).cloned(),
    };

    let is_story = global_map_id
        .map(|id| (3000..=3008).contains(&id))
        .unwrap_or(true);

    let (stage_ids, stage_structs, ground_files) = if is_story {
        load_story_stages(&map_struct, stage_names, ctx, global_map_id)
    } else {
        load_legend_stages(info, &map_struct, stage_names, ctx, global_map_id)
    };

    if stage_ids.is_empty() {
        warn!("Map {:03} (Category: {:?}) returned zero parsed stages!", map_id, category);
    }

    map_struct.stages = stage_ids;

    if map_struct.stages.is_empty() {
        return;
    }

    map_struct.stages.sort();

    if let Ok(mut reg) = reg_mtx.lock() {
        reg.maps.insert(map_key, map_struct);
        reg.stages.extend(stage_structs);
        reg.grounds.extend(ground_files);
    }
}

fn load_story_stages(
    map: &Map,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> StageHarvest {
    let mut id_list = Vec::new();
    let mut stage_list = Vec::new();
    let mut ground_list = Vec::new();

    let story_file = if map.category.map_prefix() == "Z" {
        match global_map_id {
            Some(3000) => "stageNormal0_0_Z.csv",
            Some(3001) => "stageNormal0_1_Z.csv",
            Some(3002) => "stageNormal0_2_Z.csv",
            Some(3003) => "stageNormal1_0_Z.csv",
            Some(3004) => "stageNormal1_1_Z.csv",
            Some(3005) => "stageNormal1_2_Z.csv",
            Some(3006) => "stageNormal2_0_Z.csv",
            Some(3007) => "stageNormal2_1_Z.csv",
            Some(3008) => "stageNormal2_2_Z.csv",
            _ => "",
        }
    } else {
        match global_map_id {
            Some(3000..=3002) => "stageNormal0.csv",
            Some(3003) => "stageNormal1_0.csv",
            Some(3004) => "stageNormal1_1.csv",
            Some(3005) => "stageNormal1_2.csv",
            Some(3006) => "stageNormal2_0.csv",
            Some(3007) => "stageNormal2_1.csv",
            Some(3008) => "stageNormal2_2.csv",
            _ => "",
        }
    };

    let inv_story_file = if map.category.map_prefix() == "Z" {
        match global_map_id {
            Some(3008) => "stageNormal2_2_Invasion_Z.csv",
            _ => "",
        }
    } else {
        match global_map_id {
            Some(3008) => "stageNormal2_2_Invasion.csv",
            _ => "",
        }
    };

    let story_data = mapstagedata(ctx.vfs, story_file);
    let inv_story_data = mapstagedata(ctx.vfs, inv_story_file);

    for (stage_id, file_name) in battlegrounds(&ctx.globs, &map.category, map.map_id) {
        let is_ec_group = map.category.map_prefix() == PREFIX_EC || (map.category.map_prefix() == "Z" && map.map_id <= 2);

        if is_ec_group {
            if map.map_id == 0 && stage_id > 47 { continue; }
            if map.map_id == 1 && (stage_id == 47 || stage_id == 48 || stage_id > 49) { continue; }
            if map.map_id == 2 && (stage_id == 47 || stage_id == 48 || stage_id == 49 || stage_id > 50) { continue; }
        }

        let Some(raw_layout) = battleground(ctx.vfs, &file_name) else {
            warn!(stage = %file_name, "Failed to parse story battleground");
            continue;
        };

        let mut stage_struct = build_base_stage(
            map, stage_id, CostType::Energy, &raw_layout, stage_names, ctx, global_map_id
        );

        stage_struct.base_id = stage_id as i32;

        if let Some(entry) = story_data.entries.get(stage_id as usize) {
            stage_struct.cost = entry.cost;
            stage_struct.xp = global_map_id.map(|id| get_hardcoded_xp(id, stage_id as usize)).unwrap_or(0);
            stage_struct.init_track = entry.init_track;
            stage_struct.bgm_change_percent = entry.bgm_change_percent;
            stage_struct.boss_track = entry.boss_track;
        }

        let stage_key = GlobalStageId { category: map.category.clone(), map: map.map_id, stage: stage_id };
        ground_list.push((stage_key.clone(), file_name.clone()));
        stage_list.push((stage_key, stage_struct));
        id_list.push(stage_id);
    }

    if let Some(inv_file) = invasion_battleground(&ctx.globs, &map.category, map.map_id) {
        let inv_stage_id = id_list.iter().max().copied().unwrap_or(0) + 1;

        if let Some(raw_layout) = battleground(ctx.vfs, &inv_file) {
            let mut stage_struct = build_base_stage(
                map, inv_stage_id, CostType::Energy, &raw_layout, stage_names, ctx, global_map_id
            );

            stage_struct.name = "Invasion".to_string();
            stage_struct.base_id = inv_stage_id as i32;

            if let Some(entry) = inv_story_data.entries.first() {
                stage_struct.cost = entry.cost;
                stage_struct.xp = global_map_id.map(|id| get_hardcoded_xp(id, inv_stage_id as usize)).unwrap_or(0);
                stage_struct.init_track = entry.init_track;
                stage_struct.bgm_change_percent = entry.bgm_change_percent;
                stage_struct.boss_track = entry.boss_track;
            }

            let stage_key = GlobalStageId { category: map.category.clone(), map: map.map_id, stage: inv_stage_id };
            ground_list.push((stage_key.clone(), inv_file.clone()));
            stage_list.push((stage_key, stage_struct));
            id_list.push(inv_stage_id);
        } else {
            warn!(stage = %inv_file, "Failed to parse invasion battleground");
        }
    }

    (id_list, stage_list, ground_list)
}

fn load_legend_stages(
    info: &CategoryInfo,
    map: &Map,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> StageHarvest {
    let mut id_list = Vec::new();
    let mut stage_list = Vec::new();
    let mut ground_list = Vec::new();
    let mut map_data = MapStageData::default();

    for prefix in [info.data_prefix.as_str(), info.prefix.as_str()] {
        map_data = mapstagedata(ctx.vfs, &format!("{}{}_{:03}{}", MAP_STAGE_DATA, prefix, map.map_id, CSV));
        if !map_data.entries.is_empty() { break; }
    }

    if map_data.entries.is_empty() {
        map_data = mapstagedata(ctx.vfs, "stage.csv");
    }

    for (stage_id, file_name) in battlegrounds(&ctx.globs, &map.category, map.map_id) {
        let Some(raw_layout) = battleground(ctx.vfs, &file_name) else {
            warn!(stage = %file_name, "Failed to parse legend battleground");
            continue;
        };

        let mut stage_struct = build_base_stage(
            map, stage_id, map_data.header.cost_type, &raw_layout, stage_names, ctx, global_map_id
        );

        if let Some(entry) = map_data.entries.get(stage_id as usize) {
            stage_struct.cost = entry.cost;
            stage_struct.xp = entry.xp;
            stage_struct.init_track = entry.init_track;
            stage_struct.bgm_change_percent = entry.bgm_change_percent;
            stage_struct.boss_track = entry.boss_track;
            stage_struct.rewards = entry.rewards.clone();
        }

        let stage_key = GlobalStageId { category: map.category.clone(), map: map.map_id, stage: stage_id };
        ground_list.push((stage_key.clone(), file_name.clone()));
        stage_list.push((stage_key, stage_struct));
        id_list.push(stage_id);
    }

    (id_list, stage_list, ground_list)
}

fn build_base_stage(
    map: &Map,
    stage_id: u32,
    declared_cost_type: CostType,
    raw_layout: &nyanko::chapter::stage::Battleground,
    stage_names: &HashMap<u32, StageNameEntry>,
    ctx: &ScanContext,
    global_map_id: Option<u32>,
) -> Stage {
    let is_story_name = matches!(map.category.map_prefix().as_str(), "EC" | "W" | "Space" | "Z");

    let stage_display_name = if is_story_name {
        let mut lookup_id = stage_id;

        let is_ec = map.category.map_prefix() == "EC";
        let is_z_ec = map.category.map_prefix() == "Z" && map.map_id <= 2;

        if (is_ec || is_z_ec) && matches!(stage_id, 48..=50) {
            lookup_id = 47;
        }

        stage_names.get(&lookup_id)
            .and_then(|entry| entry.names.first())
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:02}", stage_id))
    } else {
        stage_names.get(&map.map_id)
            .and_then(|entry| entry.names.get(stage_id as usize))
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{:02}", stage_id))
    };

    let stage_opts: &[StageOptionEntry] = global_map_id
        .and_then(|id| ctx.stage_options.get(&id))
        .map_or(&[], Vec::as_slice);

    let mut final_opt = StageOptionEntry { target_crowns: -1, ..Default::default() };

    let valid_options = stage_opts.iter().filter(|o|
        (o.target_stage == -1 || o.target_stage == stage_id as i32) &&
            (o.target_crowns == -1 || o.target_crowns == 0)
    );

    for opt in valid_options {
        if opt.target_crowns != -1 { final_opt.target_crowns = opt.target_crowns; }
        if opt.rarity_mask != 0 { final_opt.rarity_mask = opt.rarity_mask; }
        if opt.deploy_limit != 0 { final_opt.deploy_limit = opt.deploy_limit; }
        if opt.allowed_rows != 0 { final_opt.allowed_rows = opt.allowed_rows; }
        if opt.min_cost != 0 { final_opt.min_cost = opt.min_cost; }
        if opt.max_cost != 0 { final_opt.max_cost = opt.max_cost; }
        if opt.charagroup_id != 0 { final_opt.charagroup_id = opt.charagroup_id; }
    }

    let stage_diff = global_map_id
        .and_then(|id| ctx.difficulties.get(&id))
        .and_then(|diff_list| diff_list.get(stage_id as usize))
        .copied()
        .unwrap_or(0);

    let current_charagroup = ctx.charagroups.get(&final_opt.charagroup_id).cloned();

    let mut loaded_fixed_lineups = HashMap::new();

    for crown_index in 0..map.max_crowns {
        let Some(map_id_val) = global_map_id else { continue; };
        let Some(formation_data) = ctx.fixed_formations.get(&(map_id_val, crown_index, stage_id)) else { continue; };
        let Some(preset_lineup_json) = certification_preset(
            ctx.vfs,
            &formation_data.preset_file_name
        ) else { continue; };

        loaded_fixed_lineups.insert(crown_index, preset_lineup_json);
    }

    Stage {
        name: stage_display_name,
        category: map.category.clone(),
        map_id: map.map_id,
        stage_id,
        base_id: raw_layout.base_id,
        anim_base_id: raw_layout.anim_base_id,
        width: raw_layout.width,
        base_hp: raw_layout.base_hp,
        min_spawn: raw_layout.min_spawn,
        max_spawn: raw_layout.max_spawn,
        background_id: raw_layout.background_id,
        max_enemies: raw_layout.max_enemies,
        time_limit: raw_layout.time_limit,
        is_no_continues: raw_layout.is_no_continues,
        is_base_indestructible: raw_layout.is_base_indestructible,
        unknown_value: raw_layout.unknown_value,
        enemies: raw_layout.entries.clone(),
        declared_cost_type,
        difficulty: stage_diff,
        max_crowns: map.max_crowns,
        target_crowns: final_opt.target_crowns,
        crown_1_mag: map.crown_1_mag,
        crown_2_mag: map.crown_2_mag,
        crown_3_mag: map.crown_3_mag,
        crown_4_mag: map.crown_4_mag,
        rarity_mask: final_opt.rarity_mask,
        deploy_limit: final_opt.deploy_limit,
        allowed_rows: final_opt.allowed_rows,
        min_cost: final_opt.min_cost,
        max_cost: final_opt.max_cost,
        charagroup: current_charagroup,
        fixed_lineups: loaded_fixed_lineups,
        ..Default::default()
    }
}
#[cfg(test)]
mod tests {
    use super::{split_stage, stage_prefixes};
    use nyanko::chapter::Category;

    // The event chapter declares its maps under RE and ships the battlegrounds under EX, and
    // "stageE" globs both, so the bare prefix must keep refusing the files EX owns.
    #[test]
    fn the_bare_event_prefix_never_claims_an_ex_battleground() {
        assert_eq!(split_stage("stageEX081_00.csv", "EX", 81), Some(0));
        assert_eq!(split_stage("stageEX081_00.csv", "E", 81), None);
    }

    #[test]
    fn an_unknown_category_gets_the_restricted_variant_added() {
        let category = Category::Unknown("PR".to_string());

        assert_eq!(stage_prefixes(&category), vec!["PR".to_string(), "RPR".to_string()]);
    }

    #[test]
    fn an_unknown_category_already_carrying_r_is_left_alone() {
        let category = Category::Unknown("RPR".to_string());

        assert_eq!(stage_prefixes(&category), category.stage_prefix());
    }

    #[test]
    fn a_named_category_is_left_exactly_as_nyanko_declares_it() {
        let category = Category::TowersAndCitadels;

        assert_eq!(stage_prefixes(&category), category.stage_prefix());
    }
}
