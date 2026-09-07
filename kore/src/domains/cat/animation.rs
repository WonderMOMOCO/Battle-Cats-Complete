use crate::systems::animation::{self, named_offsets, Clip, ClipSet, Loop, RigFiles};
use crate::Vfs;

use super::files;
use super::scanner::CatEntry;

const SPIRIT_ATTACK: usize = 2;

pub fn set_id(cat: &CatEntry, form: usize) -> String {
    let form_char = match form {
        0 => 'f',
        1 => 'c',
        2 => 's',
        _ => 'u',
    };

    format!("{:03}_{}", cat.id, form_char)
}

pub fn rig_files(cat: &CatEntry, form: usize, vfs: &Vfs) -> Option<RigFiles> {
    let egg_ids = cat.egg_ids.unwrap_or((-1, -1));
    let base = vec![files::anim_base_filename(cat.id, form, egg_ids)];

    animation::rig_files(vfs, &set_id(cat, form), &base)
}

pub fn clips(cat: &CatEntry, form: usize, vfs: &Vfs) -> ClipSet {
    let egg_ids = cat.egg_ids.unwrap_or((-1, -1));
    let base = vec![files::anim_base_filename(cat.id, form, egg_ids)];
    let id = set_id(cat, form);

    let mut clips = Vec::new();

    if let Some(rig) = animation::rigging(vfs, &id, &base) {
        for (suffix, path) in animation::maanims(vfs, &base) {
            let index = suffix.parse::<usize>().ok();
            let named = index.and_then(animation::standard);

            clips.push(Clip {
                name: named.map(|(name, _, _)| name.to_string()),
                slot: named.map(|(_, slot, _)| slot),
                role: named.map(|(_, _, role)| role),
                looping: if named.is_some_and(|(_, _, role)| role.loops()) { Loop::Exact } else { Loop::Frames },
                rig: rig.clone(),
                anim: Some(path),
            });
        }

        clips.push(Clip::model(rig));
    }

    if let Some(spirit) = spirit_clip(cat, form, vfs) {
        clips.push(spirit);
    }

    ClipSet { name: id, clips, offsets: named_offsets("Gacha") }
}

fn spirit_clip(cat: &CatEntry, form: usize, vfs: &Vfs) -> Option<Clip> {
    let conjure_id = cat.stats.get(form)?.as_ref()?.conjure_unit_id;

    if conjure_id <= 0 {
        return None;
    }

    let spirit_id = conjure_id as u32;
    let base = vec![files::anim_base_filename(spirit_id, 0, (-1, -1))];
    let rig = animation::rigging(vfs, &format!("spirit_{}", spirit_id), &base)?;
    let anim = vfs.find(&files::maanim_file(spirit_id, 0, (-1, -1), SPIRIT_ATTACK))?;

    Some(Clip {
        name: Some("Spirit".to_string()),
        slot: None,
        role: None,
        looping: Loop::Frames,
        rig,
        anim: Some(anim),
    })
}
