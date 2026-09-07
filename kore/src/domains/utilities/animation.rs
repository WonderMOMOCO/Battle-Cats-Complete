use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::domains::settings::FrameCount;
use crate::systems::animation::{Clip, ClipSet, Rigging};

pub fn key(png: &Path, cut: &Path, model: &Path, anims: &[PathBuf], frames: FrameCount) -> String {
    let mut key = format!("{:?}|{}", frames, rig_id(png, cut, model));

    for anim in anims {
        key.push('|');
        key.push_str(&anim.to_string_lossy());
    }

    key
}

pub fn clips(png: &Path, cut: &Path, model: &Path, anims: &[PathBuf], frames: FrameCount) -> ClipSet {
    let rig = Arc::new(Rigging {
        id: rig_id(png, cut, model),
        png: png.to_path_buf(),
        cut: cut.to_path_buf(),
        model: model.to_path_buf(),
    });

    let mut clips: Vec<Clip> = anims
        .iter()
        .map(|anim| Clip {
            name: None,
            slot: None,
            role: None,
            looping: frames.looping(),
            rig: rig.clone(),
            anim: Some(anim.clone()),
        })
        .collect();

    clips.push(Clip::model(rig));

    ClipSet { name: stem_of(model), clips, offsets: Vec::new() }
}

fn rig_id(png: &Path, cut: &Path, model: &Path) -> String {
    format!("{}|{}|{}", png.display(), cut.display(), model.display())
}

fn stem_of(path: &Path) -> String {
    path.file_stem().map_or_else(|| "animation".to_string(), |stem| stem.to_string_lossy().to_string())
}
