use serde::{Deserialize, Serialize};

use crate::common::io::cache;

use super::{CatStore, EnemyStore, ItemStore, StageStore, Vds};

struct ContentCache;

impl cache::CacheSpec for ContentCache {
    type Data = ContentStore;
    const FILE: &'static str = "virtual_data_store.bin";
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ContentStore {
    cats: CatStore,
    enemies: EnemyStore,
    items: ItemStore,
    stages: StageStore,
}

impl ContentStore {
    pub fn capture(vds: &Vds) -> Self {
        Self {
            cats: vds.cats.clone(),
            enemies: vds.enemies.clone(),
            items: vds.items.clone(),
            stages: vds.stages.clone(),
        }
    }

    pub fn apply(self, vds: &mut Vds) {
        vds.cats = self.cats;
        vds.enemies = self.enemies;
        vds.items = self.items;
        vds.stages = self.stages;
    }

    pub fn save(&self, hash: u64) {
        cache::write::<ContentCache>(hash, self);
    }

    pub fn purge() {
        cache::purge::<ContentCache>();
    }

    pub fn hydrate() -> Option<Self> {
        cache::read::<ContentCache>().map(|(_, content)| content)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use nyanko::cat::unit::LevelCurve;
    use serde::{Deserialize, Serialize};

    use super::ContentStore;

    #[derive(Serialize, Deserialize)]
    struct Payload<T> {
        build_stamp: String,
        hash: u64,
        data: T,
    }

    #[derive(Serialize)]
    struct OldTalent {
        id: u16,
        type_id: u16,
        groups: Vec<u8>,
    }

    #[derive(Default, Serialize)]
    struct OldCatStore {
        talents: Option<HashMap<u16, OldTalent>>,
        talent_costs: Option<u8>,
        descriptions: Option<u8>,
        unitbuy: Option<u8>,
        evolve: Option<u8>,
        curves: Option<Vec<LevelCurve>>,
        combos: Option<u8>,
        combo_names: Option<u8>,
        combo_effects: Option<u8>,
        combo_bands: Option<u8>,
        combo_filters: Option<u8>,
        combo_params: Option<u8>,
    }

    #[derive(Default, Serialize)]
    struct NewCatStore {
        talents: Option<u8>,
        talent_costs: Option<u8>,
        descriptions: Option<u8>,
        unitbuy: Option<u8>,
        evolve: Option<u8>,
        curves: Option<HashMap<u32, LevelCurve>>,
        combos: Option<u8>,
        combo_names: Option<u8>,
        combo_effects: Option<u8>,
        combo_bands: Option<u8>,
        combo_filters: Option<u8>,
        combo_params: Option<u8>,
    }

    #[derive(Default, Serialize)]
    struct OldEnemyStore {
        stats: Option<u8>,
        names: Option<u8>,
        descriptions: Option<u8>,
    }

    #[derive(Default, Serialize)]
    struct OldStageStore {
        map_names: Option<u8>,
        map_options: Option<u8>,
        stage_options: Option<u8>,
        charagroups: Option<u8>,
        drop_items: Option<u8>,
        score_bonuses: Option<u8>,
        special_rules: Option<u8>,
        special_rule_options: Option<u8>,
        ex_options: Option<u8>,
        difficulties: Option<u8>,
        fixed_formations: Option<u8>,
    }

    #[derive(Default, Serialize)]
    struct OldItemStore {
        catalogue: Option<u8>,
        lines: Option<u8>,
        names: Option<u8>,
    }

    #[derive(Default, Serialize)]
    struct ContentStoreLike<C> {
        cats: C,
        enemies: OldEnemyStore,
        items: OldItemStore,
        stages: OldStageStore,
    }

    fn encode<C: Default + Serialize>(cats: C) -> Vec<u8> {
        let payload = Payload {
            build_stamp: "0123456789abcdef".to_string(),
            hash: 42,
            data: ContentStoreLike { cats, ..Default::default() },
        };

        postcard::to_allocvec(&payload).unwrap()
    }

    fn decode(bytes: &[u8]) -> Result<(), postcard::Error> {
        postcard::from_bytes::<Payload<ContentStore>>(bytes).map(|_| ())
    }

    fn sample_curves() -> Vec<LevelCurve> {
        (0..64)
            .map(|index| LevelCurve { increments: (0..10).map(|step| index * 10 + step).collect() })
            .collect()
    }

    #[test]
    fn mirror_structs_match_the_real_content_store_encoding() {
        let curves = sample_curves().into_iter().enumerate().map(|(id, curve)| (id as u32, curve)).collect();
        let bytes = encode(NewCatStore { curves: Some(curves), ..Default::default() });

        assert!(
            decode(&bytes).is_ok(),
            "the mirror structs do not encode like ContentStore, so the pre-migration test below proves nothing"
        );
    }

    #[test]
    fn old_vec_curves_cache_is_rejected_not_silently_accepted() {
        let bytes = encode(OldCatStore { curves: Some(sample_curves()), ..Default::default() });

        assert!(
            decode(&bytes).is_err(),
            "a pre-migration cache decoded cleanly under the new HashMap<u32, LevelCurve> shape, \
             so ContentStore::hydrate would apply corrupted curves instead of rebuilding"
        );
    }

    #[test]
    fn old_u16_talent_keys_decode_identically() {
        let talents = (0u16..32)
            .map(|id| (id, OldTalent { id, type_id: 1, groups: Vec::new() }))
            .collect();

        let bytes = encode(OldCatStore { talents: Some(talents), ..Default::default() });

        assert!(decode(&bytes).is_ok());
    }
}
