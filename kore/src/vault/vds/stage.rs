use std::collections::HashMap;
use std::sync::Arc;

use nyanko::chapter::map::{
    DropItem, DropItemEntry, ExOption, MapName, MapOption, MapOptionEntry, ScoreBonusMap,
    ScoreBonusMapEntry, SpecialRulesMap, SpecialRulesMapEntry, SpecialRulesMapOption,
    SpecialRulesMapOptionEntry,
};
use nyanko::chapter::stage::{
    CharaGroup, CharaGroupEntry, DifficultyLevel, FixedFormation, FixedFormationEntry, StageOption,
    StageOptionEntry,
};
use serde::{Deserialize, Serialize};

use crate::domains::stage::files::{DROP_ITEM, MAP_NAME};
use crate::Vfs;

use super::Slot;

const MAP_OPTION: &str = "Map_option.csv";
const STAGE_OPTION: &str = "Stage_option.csv";
const CHARA_GROUP: &str = "Charagroup.csv";
const SCORE_BONUS: &str = "ScoreBonusMap.json";
const SPECIAL_RULES: &str = "SpecialRulesMap.json";
const SPECIAL_RULE_OPTIONS: &str = "SpecialRulesMapOption.json";
const EX_OPTION: &str = "EX_option.csv";
const DIFFICULTY: &str = "difficulty_level.tsv";
const FIXED_FORMATION: &str = "fixed_formation.csv";

#[derive(Default, Serialize, Deserialize)]
pub struct StageStore {
    map_names: Slot<HashMap<u32, String>>,
    map_options: Slot<HashMap<u32, MapOptionEntry>>,
    stage_options: Slot<HashMap<u32, Vec<StageOptionEntry>>>,
    charagroups: Slot<HashMap<u32, CharaGroupEntry>>,
    drop_items: Slot<HashMap<u32, DropItemEntry>>,
    score_bonuses: Slot<HashMap<u32, ScoreBonusMapEntry>>,
    special_rules: Slot<HashMap<u32, SpecialRulesMapEntry>>,
    special_rule_options: Slot<HashMap<u8, SpecialRulesMapOptionEntry>>,
    ex_options: Slot<HashMap<u32, u32>>,
    difficulties: Slot<HashMap<u32, Vec<u16>>>,
    fixed_formations: Slot<HashMap<(u32, u8, u32), FixedFormationEntry>>,
}

impl Clone for StageStore {
    fn clone(&self) -> Self {
        Self {
            map_names: super::snapshot(&self.map_names),
            map_options: super::snapshot(&self.map_options),
            stage_options: super::snapshot(&self.stage_options),
            charagroups: super::snapshot(&self.charagroups),
            drop_items: super::snapshot(&self.drop_items),
            score_bonuses: super::snapshot(&self.score_bonuses),
            special_rules: super::snapshot(&self.special_rules),
            special_rule_options: super::snapshot(&self.special_rule_options),
            ex_options: super::snapshot(&self.ex_options),
            difficulties: super::snapshot(&self.difficulties),
            fixed_formations: super::snapshot(&self.fixed_formations),
        }
    }
}

impl StageStore {
    pub fn map_names(&self, vfs: &Vfs) -> Arc<HashMap<u32, String>> {
        super::cached(&self.map_names, || {
            let mut merged = HashMap::new();

            for bytes in super::layered(vfs, MAP_NAME).into_iter().rev() {
                if let Ok(parsed) = MapName::parse(bytes, None) {
                    merged.extend(parsed.names);
                }
            }

            merged
        })
    }

    pub fn map_options(&self, vfs: &Vfs) -> Arc<HashMap<u32, MapOptionEntry>> {
        super::cached(&self.map_options, || {
            super::parsed(vfs, MAP_OPTION, |bytes| MapOption::parse(bytes, None))
                .map(|parsed| parsed.entries)
                .unwrap_or_default()
        })
    }

    pub fn stage_options(&self, vfs: &Vfs) -> Arc<HashMap<u32, Vec<StageOptionEntry>>> {
        super::cached(&self.stage_options, || {
            super::parsed(vfs, STAGE_OPTION, |bytes| StageOption::parse(bytes, None))
                .map(|parsed| parsed.entries)
                .unwrap_or_default()
        })
    }

    pub fn charagroups(&self, vfs: &Vfs) -> Arc<HashMap<u32, CharaGroupEntry>> {
        super::cached(&self.charagroups, || {
            super::parsed(vfs, CHARA_GROUP, |bytes| CharaGroup::parse(bytes, None))
                .map(|parsed| parsed.groups)
                .unwrap_or_default()
        })
    }

    pub fn drop_items(&self, vfs: &Vfs) -> Arc<HashMap<u32, DropItemEntry>> {
        super::cached(&self.drop_items, || {
            super::parsed(vfs, DROP_ITEM, |bytes| DropItem::parse(bytes, None))
                .map(|parsed| parsed.map_drops)
                .unwrap_or_default()
        })
    }

    pub fn score_bonuses(&self, vfs: &Vfs) -> Arc<HashMap<u32, ScoreBonusMapEntry>> {
        super::cached(&self.score_bonuses, || {
            super::parsed(vfs, SCORE_BONUS, ScoreBonusMap::parse)
                .map(|parsed| parsed.entries)
                .unwrap_or_default()
        })
    }

    pub fn special_rules(&self, vfs: &Vfs) -> Arc<HashMap<u32, SpecialRulesMapEntry>> {
        super::cached(&self.special_rules, || {
            super::parsed(vfs, SPECIAL_RULES, SpecialRulesMap::parse)
                .map(|parsed| parsed.entries)
                .unwrap_or_default()
        })
    }

    pub fn special_rule_options(&self, vfs: &Vfs) -> Arc<HashMap<u8, SpecialRulesMapOptionEntry>> {
        super::cached(&self.special_rule_options, || {
            super::parsed(vfs, SPECIAL_RULE_OPTIONS, SpecialRulesMapOption::parse)
                .map(|parsed| parsed.entries)
                .unwrap_or_default()
        })
    }

    pub fn ex_options(&self, vfs: &Vfs) -> Arc<HashMap<u32, u32>> {
        super::cached(&self.ex_options, || {
            super::parsed(vfs, EX_OPTION, |bytes| ExOption::parse(bytes, None))
                .map(|parsed| parsed.map_to_ex_map)
                .unwrap_or_default()
        })
    }

    pub fn difficulties(&self, vfs: &Vfs) -> Arc<HashMap<u32, Vec<u16>>> {
        super::cached(&self.difficulties, || {
            super::parsed(vfs, DIFFICULTY, |bytes| DifficultyLevel::parse(bytes, None))
                .map(|parsed| parsed.map_difficulties)
                .unwrap_or_default()
        })
    }

    pub fn fixed_formations(&self, vfs: &Vfs) -> Arc<HashMap<(u32, u8, u32), FixedFormationEntry>> {
        super::cached(&self.fixed_formations, || {
            super::parsed(vfs, FIXED_FORMATION, |bytes| FixedFormation::parse(bytes, None))
                .map(|parsed| parsed.formations)
                .unwrap_or_default()
        })
    }

    pub(super) fn evict(&self, filename: &str) {
        match filename {
            MAP_NAME => super::reset(&self.map_names),
            MAP_OPTION => super::reset(&self.map_options),
            STAGE_OPTION => super::reset(&self.stage_options),
            CHARA_GROUP => super::reset(&self.charagroups),
            DROP_ITEM => super::reset(&self.drop_items),
            SCORE_BONUS => super::reset(&self.score_bonuses),
            SPECIAL_RULES => super::reset(&self.special_rules),
            SPECIAL_RULE_OPTIONS => super::reset(&self.special_rule_options),
            EX_OPTION => super::reset(&self.ex_options),
            DIFFICULTY => super::reset(&self.difficulties),
            FIXED_FORMATION => super::reset(&self.fixed_formations),
            _ => (),
        }
    }

    pub(super) fn clear(&self) {
        super::reset(&self.map_names);
        super::reset(&self.map_options);
        super::reset(&self.stage_options);
        super::reset(&self.charagroups);
        super::reset(&self.drop_items);
        super::reset(&self.score_bonuses);
        super::reset(&self.special_rules);
        super::reset(&self.special_rule_options);
        super::reset(&self.ex_options);
        super::reset(&self.difficulties);
        super::reset(&self.fixed_formations);
    }
}
