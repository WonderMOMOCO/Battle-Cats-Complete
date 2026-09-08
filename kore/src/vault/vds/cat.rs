use std::collections::HashMap;
use std::sync::Arc;

use nyanko::cat::unit::{
    LevelCurve, NyancomboData, NyancomboFilter, NyancomboParam, SkillDescriptions, Talent, TalentCost,
    UnitBuy, UnitEvolve,
};
use serde::{Deserialize, Serialize};

use crate::domains::cat::files::{self, SKILL_DESCRIPTIONS, UNIT_EVOLVE};
use crate::domains::cat::waiter::{self, ComboText};
use crate::Vfs;

use super::Slot;


#[derive(Default, Serialize, Deserialize)]
pub struct CatStore {
    talents: Slot<HashMap<u32, Talent>>,
    talent_costs: Slot<HashMap<u8, TalentCost>>,
    descriptions: Slot<Vec<String>>,
    unitbuy: Slot<HashMap<u32, UnitBuy>>,
    evolve: Slot<HashMap<u32, UnitEvolve>>,
    curves: Slot<HashMap<u32, LevelCurve>>,
    combos: Slot<Vec<NyancomboData>>,
    combo_names: Slot<Vec<Option<String>>>,
    combo_effects: Slot<Vec<Option<String>>>,
    combo_bands: Slot<Vec<Option<String>>>,
    combo_filters: Slot<Vec<NyancomboFilter>>,
    combo_params: Slot<Vec<NyancomboParam>>,
}

impl Clone for CatStore {
    fn clone(&self) -> Self {
        Self {
            talents: super::snapshot(&self.talents),
            talent_costs: super::snapshot(&self.talent_costs),
            descriptions: super::snapshot(&self.descriptions),
            unitbuy: super::snapshot(&self.unitbuy),
            evolve: super::snapshot(&self.evolve),
            curves: super::snapshot(&self.curves),
            combos: super::snapshot(&self.combos),
            combo_names: super::snapshot(&self.combo_names),
            combo_effects: super::snapshot(&self.combo_effects),
            combo_bands: super::snapshot(&self.combo_bands),
            combo_filters: super::snapshot(&self.combo_filters),
            combo_params: super::snapshot(&self.combo_params),
        }
    }
}

impl CatStore {
    pub fn talents(&self, vfs: &Vfs) -> Arc<HashMap<u32, Talent>> {
        super::cached(&self.talents, || {
            super::parsed(vfs, files::SKILL_ACQUISITION, |bytes| Talent::parse(bytes, None)).unwrap_or_default()
        })
    }

    pub fn talent_costs(&self, vfs: &Vfs) -> Arc<HashMap<u8, TalentCost>> {
        super::cached(&self.talent_costs, || {
            super::parsed(vfs, files::SKILL_LEVEL, |bytes| TalentCost::parse(bytes, None)).unwrap_or_default()
        })
    }

    pub fn descriptions(&self, vfs: &Vfs) -> Arc<Vec<String>> {
        super::cached(&self.descriptions, || {
            let mut merged: Vec<String> = Vec::new();

            for (name, bytes) in super::named(vfs, SKILL_DESCRIPTIONS) {
                let separator = crate::common::region::text_separator(&name);

                let Ok(parsed) = SkillDescriptions::parse(bytes, Some(separator)) else {
                    continue;
                };

                if parsed.texts.len() > merged.len() {
                    merged.resize(parsed.texts.len(), String::new());
                }

                for (index, text) in parsed.texts.into_iter().enumerate() {
                    if let Some(slot) = merged.get_mut(index)
                        && slot.trim().is_empty()
                    {
                        *slot = text;
                    }
                }
            }

            merged
        })
    }

    pub fn unitbuy(&self, vfs: &Vfs) -> Arc<HashMap<u32, UnitBuy>> {
        super::cached(&self.unitbuy, || {
            super::parsed(vfs, files::UNIT_BUY, |bytes| UnitBuy::parse(bytes, None)).unwrap_or_default()
        })
    }

    pub fn curves(&self, vfs: &Vfs) -> Arc<HashMap<u32, LevelCurve>> {
        super::cached(&self.curves, || {
            super::parsed(vfs, files::UNIT_LEVEL, |bytes| LevelCurve::parse(bytes, None)).unwrap_or_default()
        })
    }

    pub fn evolve(&self, vfs: &Vfs) -> Arc<HashMap<u32, UnitEvolve>> {
        super::cached(&self.evolve, || {
            let mut merged: HashMap<u32, UnitEvolve> = HashMap::new();

            for bytes in super::layered(vfs, UNIT_EVOLVE) {
                let Ok(parsed) = UnitEvolve::parse(bytes, None) else {
                    continue;
                };

                for (cat_id, evolve) in parsed {
                    let entry = merged.entry(cat_id).or_default();

                    for index in 0..4 {
                        if entry.texts[index].is_none() {
                            entry.texts[index] = evolve.texts[index].clone();
                        }
                    }
                }
            }

            merged
        })
    }

    pub fn combos(&self, vfs: &Vfs) -> Arc<Vec<NyancomboData>> {
        super::cached(&self.combos, || waiter::nyancombodata(vfs))
    }

    pub fn combo_names(&self, vfs: &Vfs) -> Arc<Vec<Option<String>>> {
        super::cached(&self.combo_names, || waiter::nyancombo(vfs, ComboText::Name))
    }

    pub fn combo_effects(&self, vfs: &Vfs) -> Arc<Vec<Option<String>>> {
        super::cached(&self.combo_effects, || waiter::nyancombo(vfs, ComboText::Effect))
    }

    pub fn combo_bands(&self, vfs: &Vfs) -> Arc<Vec<Option<String>>> {
        super::cached(&self.combo_bands, || waiter::nyancombo(vfs, ComboText::Band))
    }

    pub(crate) fn combo_filters(&self, vfs: &Vfs) -> Arc<Vec<NyancomboFilter>> {
        super::cached(&self.combo_filters, || waiter::nyancombofilter(vfs))
    }

    pub fn combo_params(&self, vfs: &Vfs) -> Arc<Vec<NyancomboParam>> {
        super::cached(&self.combo_params, || waiter::nyancomboparam(vfs))
    }

    pub(super) fn evict(&self, filename: &str) {
        match filename {
            files::SKILL_ACQUISITION => super::reset(&self.talents),
            files::SKILL_LEVEL => super::reset(&self.talent_costs),
            SKILL_DESCRIPTIONS => super::reset(&self.descriptions),
            files::UNIT_BUY => super::reset(&self.unitbuy),
            files::UNIT_LEVEL => super::reset(&self.curves),
            UNIT_EVOLVE => super::reset(&self.evolve),
            files::NYANCOMBO_DATA => super::reset(&self.combos),
            files::NYANCOMBO_NAME => super::reset(&self.combo_names),
            files::NYANCOMBO_EFFECT => super::reset(&self.combo_effects),
            files::NYANCOMBO_BAND => super::reset(&self.combo_bands),
            files::NYANCOMBO_FILTER => super::reset(&self.combo_filters),
            files::NYANCOMBO_PARAM => super::reset(&self.combo_params),
            _ => (),
        }
    }

    pub(super) fn clear(&self) {
        super::reset(&self.talents);
        super::reset(&self.talent_costs);
        super::reset(&self.descriptions);
        super::reset(&self.unitbuy);
        super::reset(&self.evolve);
        super::reset(&self.curves);
        super::reset(&self.combos);
        super::reset(&self.combo_names);
        super::reset(&self.combo_effects);
        super::reset(&self.combo_bands);
        super::reset(&self.combo_filters);
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    use crate::Vfs;

    use super::CatStore;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!("bcc-skilldesc-{name}-{}", std::process::id()));

            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("scratch root");

            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    // The other localized tables merge row by row, and this one read a single file until
    // it was the odd one out. Japanese terminates its rows with a comma, everyone else
    // with a pipe, so the separator has to come from the file name, not from sniffing.
    #[test]
    fn an_untranslated_row_falls_through_to_the_region_below_it() {
        let scratch = Scratch::new("holes");
        let root = &scratch.0;

        fs::write(
            root.join("SkillDescriptions_en.csv"),
            "textID|text\n1|Gain the \"Weaken\" ability.\n2|\n",
        )
        .expect("seed en");
        fs::write(
            root.join("SkillDescriptions_ja.csv"),
            "textID,text\n1,\u{653b}\u{6483}\u{529b}\u{30c0}\u{30a6}\u{30f3}\n2,\u{52d5}\u{304d}\u{3092}\u{6b62}\u{3081}\u{308b}\n",
        )
        .expect("seed ja");

        let vfs = Vfs::with_priority(&[String::new(), "en".to_string(), "ja".to_string(), "--".to_string()]);
        vfs.create(root.as_path()).expect("mount the scratch dir");

        let store = CatStore::default();
        let merged = store.descriptions(&vfs);

        assert_eq!(merged.get(1).map(String::as_str), Some("Gain the \"Weaken\" ability."));
        assert_eq!(
            merged.get(2).map(String::as_str),
            Some("\u{52d5}\u{304d}\u{3092}\u{6b62}\u{3081}\u{308b}"),
            "the blank English row must fall through rather than showing empty",
        );
    }
}
