use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use nyanko::cat::unit::{ComboSlot, NyancomboData, UnitBuy, UnitExplanation};
use tracing::trace;

use crate::common::context::GlobalContext;
use crate::domains::cat::files;
use crate::domains::cat::waiter::unitexplanation;
use crate::{Vault, Vfs};

pub struct ComboMember {
    pub id: Option<u32>,
    pub form: usize,
    pub name: Option<String>,
    pub icon: Option<PathBuf>,
    pub unresolved: bool,
}

impl ComboMember {
    pub fn label(&self) -> Option<String> {
        let id = self.id.filter(|_| self.unresolved)?;

        Some(format!("{:03}-{}", id, self.form + 1))
    }
}

pub struct CatCombo {
    pub line: usize,
    pub name: String,
    pub effect: String,
    pub restriction: Option<String>,
    pub members: [ComboMember; 5],
}

fn joins(row: &NyancomboData, cat_id: u32, form: usize) -> bool {
    let (Ok(target), Ok(reached)) = (i32::try_from(cat_id), i32::try_from(form)) else {
        return false;
    };

    row.is_active() && row.members().any(|slot| slot.unit_id == target && slot.form <= reached)
}

pub fn combo_lines(vault: &Vault, cat_id: u32, form: usize) -> Vec<usize> {
    vault
        .vds
        .cats
        .combos(&vault.vfs)
        .iter()
        .enumerate()
        .filter(|(_, row)| joins(row, cat_id, form))
        .map(|(line, _)| line)
        .collect()
}

pub fn combos(ctx: GlobalContext<'_>, cat_id: u32, form: usize) -> Vec<CatCombo> {
    trace!(cat_id = cat_id, form = form, "resolving the combos a cat takes part in");

    let vfs = &ctx.vault.vfs;
    let vds = &ctx.vault.vds;

    let rows = vds.cats.combos(vfs);
    let names = vds.cats.combo_names(vfs);
    let effects = vds.cats.combo_effects(vfs);
    let bands = vds.cats.combo_bands(vfs);
    let unitbuy = vds.cats.unitbuy(vfs);
    let groups = vds.stages.charagroups(vfs);

    let joined: Vec<(usize, &NyancomboData)> =
        rows.iter().enumerate().filter(|(_, row)| joins(row, cat_id, form)).collect();

    if joined.is_empty() {
        return Vec::new();
    }

    let empty_icon = vfs.find(files::EMPTY_ICON);
    let mut explanations: HashMap<u32, UnitExplanation> = HashMap::new();

    joined
        .into_iter()
        .map(|(line, row)| {
            let name = names
                .get(line)
                .cloned()
                .flatten()
                .unwrap_or_else(|| format!("Combo {}-{}", row.series, row.combo_id));

            let effect = text_at(&effects, row.effect_type).unwrap_or_default();
            let band = text_at(&bands, row.effect_level).unwrap_or_default();

            let restriction = u32::try_from(row.charagroup_id)
                .ok()
                .and_then(|id| groups.get(&id))
                .and_then(|group| group.text_id.as_deref())
                .map(|key| ctx.localizable.lookup(key).unwrap_or(key).to_string());

            let members = row.slots().map(|slot| {
                member(vfs, &unitbuy, &mut explanations, slot, empty_icon.as_deref())
            });

            CatCombo { line, name, effect: format!("{effect}{band}"), restriction, members }
        })
        .collect()
}

fn text_at(table: &[Option<String>], index: i32) -> Option<&str> {
    usize::try_from(index).ok().and_then(|index| table.get(index)).and_then(Option::as_deref)
}

fn member(
    vfs: &Vfs,
    unitbuy: &HashMap<u32, UnitBuy>,
    explanations: &mut HashMap<u32, UnitExplanation>,
    slot: ComboSlot,
    empty_icon: Option<&Path>,
) -> ComboMember {
    let unit = slot.is_occupied().then(|| u32::try_from(slot.unit_id).ok()).flatten();

    let Some(id) = unit else {
        return ComboMember {
            id: None,
            form: 0,
            name: None,
            icon: empty_icon.map(Path::to_path_buf),
            unresolved: false,
        };
    };

    let form = usize::try_from(slot.form).unwrap_or(0).min(files::FORM_COUNT - 1);
    let egg_ids = unitbuy.get(&id).map_or((-1, -1), |row| (row.egg_id_normal, row.egg_id_evolved));

    let icon = vfs.find(&files::icon_file(id, form, egg_ids));
    let unresolved = icon.is_none();

    let explanation = explanations.entry(id).or_insert_with(|| unitexplanation(vfs, id));
    let name = explanation
        .names
        .get(form)
        .and_then(Option::as_ref)
        .or_else(|| explanation.names.iter().flatten().next())
        .filter(|name| !name.is_empty())
        .cloned();

    ComboMember {
        id: Some(id),
        form,
        name,
        icon: icon.or_else(|| empty_icon.map(Path::to_path_buf)),
        unresolved,
    }
}

#[derive(Default)]
pub struct ComboEffects {
    pub groups: Vec<Vec<i32>>,
    pub labels: HashMap<i32, String>,
    pub units: HashMap<i32, HashSet<u32>>,
}

pub fn effects(vault: &Vault) -> ComboEffects {
    let vfs = &vault.vfs;
    let rows = vault.vds.cats.combos(vfs);
    let names = vault.vds.cats.combo_effects(vfs);
    let tabs = vault.vds.cats.combo_filters(vfs);

    let mut units: HashMap<i32, HashSet<u32>> = HashMap::new();

    for row in rows.iter().filter(|row| row.is_active()) {
        let members = units.entry(row.effect_type).or_default();

        for slot in row.members() {
            if let Ok(id) = u32::try_from(slot.unit_id) {
                members.insert(id);
            }
        }
    }

    units.retain(|_, members| !members.is_empty());

    let labels: HashMap<i32, String> = units
        .keys()
        .filter_map(|effect| {
            let name = text_at(&names, *effect)?;

            (!name.trim().is_empty()).then(|| (*effect, name.trim().to_owned()))
        })
        .collect();

    units.retain(|effect, _| labels.contains_key(effect));

    let mut grouped: HashSet<i32> = HashSet::new();
    let mut groups: Vec<Vec<i32>> = Vec::new();

    for tab in tabs.iter().skip(1) {
        let group: Vec<i32> = tab
            .effect_types
            .iter()
            .flatten()
            .copied()
            .filter(|effect| labels.contains_key(effect) && grouped.insert(*effect))
            .collect();

        if !group.is_empty() {
            groups.push(group);
        }
    }

    let mut ungrouped: Vec<i32> = labels.keys().copied().filter(|effect| !grouped.contains(effect)).collect();
    ungrouped.sort_unstable();

    if !ungrouped.is_empty() {
        groups.push(ungrouped);
    }

    trace!(groups = groups.len(), effects = labels.len(), "resolved the combo effect groups");

    ComboEffects { groups, labels, units }
}
