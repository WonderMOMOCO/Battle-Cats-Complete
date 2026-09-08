use std::collections::HashMap;
use std::sync::Arc;

use nyanko::files::{GatyaItemBuy, GatyaItemName};
use serde::{Deserialize, Serialize};

use crate::domains::stage::files::{GATYA_ITEM_BUY, GATYA_ITEM_NAME};
use crate::Vfs;

use super::Slot;


#[derive(Default, Serialize, Deserialize)]
pub struct ItemStore {
    catalogue: Slot<Vec<GatyaItemBuy>>,
    lines: Slot<HashMap<u32, usize>>,
    names: Slot<Vec<GatyaItemName>>,
}

impl Clone for ItemStore {
    fn clone(&self) -> Self {
        Self {
            catalogue: super::snapshot(&self.catalogue),
            lines: super::snapshot(&self.lines),
            names: super::snapshot(&self.names),
        }
    }
}

impl ItemStore {
    pub fn catalogue(&self, vfs: &Vfs) -> Arc<Vec<GatyaItemBuy>> {
        super::cached(&self.catalogue, || {
            super::parsed(vfs, GATYA_ITEM_BUY, |bytes| GatyaItemBuy::parse(bytes, None)).unwrap_or_default()
        })
    }

    pub fn names(&self, vfs: &Vfs) -> Arc<Vec<GatyaItemName>> {
        super::cached(&self.names, || {
            let mut merged: Vec<GatyaItemName> = Vec::new();

            for bytes in super::layered(vfs, GATYA_ITEM_NAME) {
                let Ok(parsed) = GatyaItemName::parse(bytes, None) else {
                    continue;
                };

                if parsed.len() > merged.len() {
                    merged.resize_with(parsed.len(), GatyaItemName::default);
                }

                for (line, entry) in parsed.into_iter().enumerate() {
                    if let Some(slot) = merged.get_mut(line)
                        && slot.name.is_none()
                        && entry.name.is_some()
                    {
                        *slot = entry;
                    }
                }
            }

            merged
        })
    }

    fn lines(&self, vfs: &Vfs) -> Arc<HashMap<u32, usize>> {
        super::cached(&self.lines, || {
            self.catalogue(vfs)
                .iter()
                .enumerate()
                .filter_map(|(line, row)| u32::try_from(row.stage_drop_item_id).ok().map(|id| (id, line)))
                .collect()
        })
    }

    pub fn line(&self, vfs: &Vfs, item_id: u32) -> Option<usize> {
        self.lines(vfs).get(&item_id).copied()
    }

    pub fn name(&self, vfs: &Vfs, item_id: u32) -> Option<String> {
        let line = self.line(vfs, item_id)?;

        self.names(vfs).get(line).and_then(|entry| entry.name.clone()).filter(|name| !name.is_empty())
    }

    pub fn icon_index(&self, vfs: &Vfs, item_id: u32) -> Option<u32> {
        let line = self.line(vfs, item_id)?;

        self.catalogue(vfs).get(line).and_then(|row| u32::try_from(row.icon_index(line)).ok())
    }

    pub(super) fn evict(&self, filename: &str) {
        match filename {
            GATYA_ITEM_BUY => {
                super::reset(&self.catalogue);
                super::reset(&self.lines);
            }
            GATYA_ITEM_NAME => super::reset(&self.names),
            _ => (),
        }
    }

    pub(super) fn clear(&self) {
        super::reset(&self.catalogue);
        super::reset(&self.lines);
        super::reset(&self.names);
    }
}
