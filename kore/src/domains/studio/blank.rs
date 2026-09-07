use image::{ImageFormat, Rgba, RgbaImage};

pub const SEED_SUFFIX: &str = "00";

const SHEET_SPAN: u32 = 128;
const BOX_SPAN: u32 = 64;
const BOX_INK: Rgba<u8> = Rgba([90, 160, 235, 255]);
const BOX_EDGE: Rgba<u8> = Rgba([235, 245, 255, 255]);
const EDGE_WIDTH: u32 = 4;

pub(super) fn sheet() -> Vec<u8> {
    let mut art = RgbaImage::new(SHEET_SPAN, SHEET_SPAN);

    for y in 0..BOX_SPAN {
        for x in 0..BOX_SPAN {
            let rim = x < EDGE_WIDTH
                || y < EDGE_WIDTH
                || x >= BOX_SPAN - EDGE_WIDTH
                || y >= BOX_SPAN - EDGE_WIDTH;

            art.put_pixel(x, y, if rim { BOX_EDGE } else { BOX_INK });
        }
    }

    let mut encoded = std::io::Cursor::new(Vec::new());

    if let Err(err) = art.write_to(&mut encoded, ImageFormat::Png) {
        tracing::warn!("Studio could not encode the seed atlas: {}", err);
    }

    encoded.into_inner()
}

pub(super) fn cuts(name: &str) -> String {
    format!("[imgcut]\n1\n{}.png\n1\n0,0,{},{},box\n", name, BOX_SPAN, BOX_SPAN)
}

pub(super) fn model() -> String {
    let half = BOX_SPAN / 2;

    format!(
        "[modelanim:model]\n3\n2\n-1,-1,0,0,0,0,0,0,1000,1000,0,1000,0,root\n0,0,0,1,0,-{half},{half},{half},1000,1000,0,1000,0,box\n1000,3600,1000\n2\n0,0,0,0,5,0,combat\n0,0,0,0,5,0,gacha\n"
    )
}

pub(super) fn track() -> String {
    "[modelanim:animation]\n1\n1\n1,11,-1,0,0,spin\n2\n0,0,0,0\n60,3600,0,0\n".to_owned()
}

#[cfg(test)]
mod tests {
    use nyanko::graphics::animate;
    use nyanko::graphics::rig::{Model, Rig};

    use super::*;
    use crate::systems::animation::authoring::{Imgcut, Maanim};

    #[test]
    fn every_seeded_document_parses_back() {
        // The seed is what "New Set" writes, so a malformed one is a broken button.
        let parsed = Imgcut::parse(cuts("Test").as_bytes()).expect("the cut list parses");
        assert_eq!(parsed.count(), 1);
        assert_eq!(parsed.sheet(), "Test.png");

        let parsed = Model::parse(model().as_bytes()).expect("the model parses");
        assert_eq!(parsed.parts.len(), 2);
        assert_eq!(parsed.alignment.len(), 2);

        let parsed = Maanim::parse(track().as_bytes()).expect("the animation parses");
        assert_eq!(parsed.tracks().len(), 1);
    }

    #[test]
    fn the_root_is_a_base_the_art_hangs_off_rather_than_a_part_of_its_own() {
        // 3287 of the 3398 shipped models open on an undrawn `-1,-1` root, and only a
        // handful of shipped tracks drive part zero. The seed teaches that shape: the
        // root carries no sprite and no animation, the box is its child.
        let model = Model::parse(model().as_bytes()).expect("the model parses");
        let (root, box_part) = (&model.parts[0], &model.parts[1]);

        assert_eq!((root.parent, root.id), (-1, -1), "the root draws nothing");
        assert_eq!(box_part.parent, 0, "and the only art hangs off it");
        assert!(model.alignment.iter().all(|row| row.part == 0), "placement measures against the root");

        let track = Maanim::parse(track().as_bytes()).expect("the animation parses");
        assert!(track.tracks().iter().all(|drive| drive.part != 0), "nothing moves the root");
    }

    #[test]
    fn the_seeded_entity_stands_on_the_ground_line() {
        // A box hanging below the origin is an authoring mistake, so the seed must not
        // ship one. Resolved through nyanko rather than by repeating its placement
        // arithmetic here, which would drift the moment the engine read changes.
        let rig = Rig::parse(sheet(), cuts("Test"), model()).expect("the rig parses");
        let placed = animate::resolve_frame(&rig, None, 0, Some(0));

        assert_eq!(placed.len(), 1, "the undrawn root contributes no quad");

        let box_part = placed.first().expect("the seed draws one part");

        let xs: Vec<f32> = box_part.vertices.iter().step_by(2).copied().collect();
        let ys: Vec<f32> = box_part.vertices.iter().skip(1).step_by(2).copied().collect();

        let span = |axis: &[f32]| {
            axis.iter().fold((f32::MAX, f32::MIN), |(low, high), at| (low.min(*at), high.max(*at)))
        };

        let (left, right) = span(&xs);
        let (top, bottom) = span(&ys);

        assert_eq!(bottom, 0.0, "its feet rest on the origin");
        assert_eq!(top, -(BOX_SPAN as f32));
        assert_eq!((left, right), (-(BOX_SPAN as f32) / 2.0, BOX_SPAN as f32 / 2.0), "centred on it");
    }

    #[test]
    fn the_seed_atlas_decodes_at_the_declared_span() {
        let decoded = image::load_from_memory(&sheet()).expect("the atlas decodes");

        assert_eq!((decoded.width(), decoded.height()), (SHEET_SPAN, SHEET_SPAN));
    }
}
