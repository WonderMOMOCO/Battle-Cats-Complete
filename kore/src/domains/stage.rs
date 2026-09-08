pub mod authoring;
pub mod cost;
pub mod filter;
pub mod fixedlineup;
pub mod materials;
pub mod names;
pub mod navigate;
pub mod files;
pub(crate) mod patterns;
pub mod restrictions;
pub mod scanner;
pub mod treasure;
pub mod waiter;

use std::collections::HashMap;

use nyanko::cat::unit::UnitBuy;
use nyanko::chapter::Category;
use nyanko::chapter::map::LockSkipDataEntry;
use nyanko::chapter::stage::ScatCpuSetting;
pub use nyanko::chapter::Map;
pub use nyanko::chapter::Stage;
use serde::{Deserialize, Serialize};
use tracing::{instrument, trace};

use crate::domains::enemy::scanner::EnemyEntry;

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct GlobalMapId {
    pub category: Category,
    pub map: u32,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub struct GlobalStageId {
    pub category: Category,
    pub map: u32,
    pub stage: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct StageRegistry {
    pub maps: HashMap<GlobalMapId, Map>,
    pub stages: HashMap<GlobalStageId, Stage>,
    pub grounds: HashMap<GlobalStageId, Box<str>>,
    pub addresses: HashMap<GlobalMapId, MapAddress>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MapAddress {
    pub global: Option<u32>,
    pub name_file: Option<Box<str>>,
    pub data_file: Option<Box<str>>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct StageDataState {
    #[serde(skip)] pub registry: StageRegistry,
    pub search_query: String,
    pub selected_category: Option<Category>,
    pub selected_map: Option<GlobalMapId>,
    pub selected_stage: Option<GlobalStageId>,

    #[serde(skip)] pub enemy_registry: HashMap<u32, EnemyEntry>,
    #[serde(skip)] pub enemy_name_registry: Vec<String>,
    #[serde(skip)] pub drop_chara_registry: HashMap<u32, u32>,
    #[serde(skip)] pub unit_buy_registry: HashMap<u32, UnitBuy>,
    #[serde(skip)] pub cat_name_registry: HashMap<u32, Vec<String>>,
    #[serde(skip)] pub lock_skip_registry: HashMap<u32, LockSkipDataEntry>,
    #[serde(skip)] pub scat_cpu_setting: ScatCpuSetting,
}

impl StageDataState {
    #[instrument(level = "trace", skip(self, extracted))]
    pub fn sync_enemies(&mut self, extracted: &[EnemyEntry]) {
        trace!("Syncing {} enemies to stage registry", extracted.len());

        self.enemy_registry = extracted
            .iter()
            .map(|enemy| (enemy.id, enemy.clone()))
            .collect();
    }
}