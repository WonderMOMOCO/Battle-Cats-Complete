use crate::{ItemStore, Vfs};

use super::files;
use super::treasure::ResolvedDrop;

pub const MAT_IDS: [u32; 16] = [
    85, 86, 87, 88, 89, 90, 91, 140,
    187, 188, 189, 190, 191, 192, 193, 194,
];

pub fn resolve(vfs: &Vfs, idx: usize, amt: u32, items: &ItemStore) -> ResolvedDrop {
    let Some(&item_id) = MAT_IDS.get(idx) else {
        return ResolvedDrop {
            name: format!("ID {}", idx),
            image_path: None,
            amount_display: amt.to_string(),
        };
    };

    ResolvedDrop {
        name: items.name(vfs, item_id).unwrap_or_else(|| format!("ID {}", item_id)),
        image_path: items
            .icon_index(vfs, item_id)
            .and_then(|index| vfs.find(&files::gatya_item_img(index))),
        amount_display: amt.to_string(),
    }
}