use std::sync::Arc;

use nyanko::graphics::rig::{Animation, Model, Rig};
use nyanko::graphics::tools::part;

use super::authoring::Maanim;

const PROBE: i32 = 1024;
const SINGULAR: f32 = 1.0e-4;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Gizmo {
    #[default]
    Channel,
    Model,
}

impl Gizmo {
    pub const ALL: [Gizmo; 2] = [Gizmo::Channel, Gizmo::Model];

    pub fn label(self) -> &'static str {
        match self {
            Gizmo::Channel => "Channel",
            Gizmo::Model => "Model",
        }
    }
}

impl std::fmt::Display for Gizmo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spot {
    Pivot,
    Corner(usize),
}

pub struct Probe {
    rig: Arc<Rig>,
    anim: Option<Arc<Animation>>,
    frame: i32,
    offset: Option<usize>,
    part: usize,
    resting: Option<[f32; 8]>,
}

impl Probe {
    pub fn new(
        rig: Arc<Rig>,
        anim: Option<Arc<Animation>>,
        frame: i32,
        offset: Option<usize>,
        part: usize,
    ) -> Self {
        Self { rig, anim, frame, offset, part, resting: None }
    }

    pub fn seeded(mut self, resting: [f32; 8]) -> Self {
        self.resting = Some(resting);
        self
    }

    pub fn model(&self) -> &Model {
        &self.rig.model
    }

    fn quad(&self) -> Option<[f32; 8]> {
        part::resolve(&self.rig, self.anim.as_deref(), self.frame, self.offset)
            .ok()?
            .iter()
            .find(|entry| entry.part == self.part)
            .map(|found| found.frame.vertices)
    }

    fn resting(&mut self) -> Option<[f32; 8]> {
        if self.resting.is_none() {
            self.resting = self.quad();
        }

        self.resting
    }

    fn swing(&mut self, field: usize, held: i32, step: i32) -> Option<[f32; 8]> {
        set_field(&mut Arc::make_mut(&mut self.rig).model, self.part, field, held.wrapping_add(step));

        let moved = self.quad();

        set_field(&mut Arc::make_mut(&mut self.rig).model, self.part, field, held);

        moved
    }

    fn swung(&mut self, doc: &Maanim, kind: i32, held: i32, step: i32) -> Option<[f32; 8]> {
        let mut probed = doc.clone();

        probed.pose(self.part, kind, self.frame, held.wrapping_add(step), Some(&self.rig.model));

        let put_back = self.anim.replace(probed.shared());
        let moved = self.quad();

        self.anim = put_back;

        moved
    }

    pub fn rest_reach(&mut self, fields: (usize, usize), spots: &[Spot]) -> Option<Vec<[(f32, f32); 2]>> {
        let across = self.rest_sweep(fields.0, PROBE, spots)?;
        let down = self.rest_sweep(fields.1, PROBE, spots)?;

        Some(across.into_iter().zip(down).map(|(across, down)| [across, down]).collect())
    }

    pub fn rest_sweep(&mut self, field_at: usize, step: i32, spots: &[Spot]) -> Option<Vec<(f32, f32)>> {
        let held = field(self.model(), self.part, field_at)?;
        let resting = self.resting()?;
        let moved = self.swing(field_at, held, step)?;

        Some(self.column(&resting, &moved, step, spots))
    }

    pub fn channel_reach(
        &mut self,
        doc: &Maanim,
        kinds: (i32, i32),
        held: (i32, i32),
        spots: &[Spot],
    ) -> Option<Vec<[(f32, f32); 2]>> {
        let across = self.channel_sweep(doc, kinds.0, held.0, PROBE, spots)?;
        let down = self.channel_sweep(doc, kinds.1, held.1, PROBE, spots)?;

        Some(across.into_iter().zip(down).map(|(across, down)| [across, down]).collect())
    }

    pub fn channel_sweep(
        &mut self,
        doc: &Maanim,
        kind: i32,
        held: i32,
        step: i32,
        spots: &[Spot],
    ) -> Option<Vec<(f32, f32)>> {
        let resting = self.resting()?;
        let moved = self.swung(doc, kind, held, step)?;

        Some(self.column(&resting, &moved, step, spots))
    }

    fn column(&self, resting: &[f32; 8], moved: &[f32; 8], step: i32, spots: &[Spot]) -> Vec<(f32, f32)> {
        spots
            .iter()
            .map(|spot| {
                let (was, now) = (self.locate(resting, *spot), self.locate(moved, *spot));

                ((now.0 - was.0) / step as f32, (now.1 - was.1) / step as f32)
            })
            .collect()
    }

    fn locate(&self, quad: &[f32; 8], spot: Spot) -> (f32, f32) {
        let corner = |at: usize| (quad[at * 2], quad[at * 2 + 1]);

        let Spot::Corner(at) = spot else {
            let (across, down) = pivot_bias(&self.rig, self.part).unwrap_or((0.5, 0.5));
            let (top_left, bottom_left, top_right) = (corner(0), corner(1), corner(2));

            return (
                top_left.0 + across * (top_right.0 - top_left.0) + down * (bottom_left.0 - top_left.0),
                top_left.1 + across * (top_right.1 - top_left.1) + down * (bottom_left.1 - top_left.1),
            );
        };

        corner(at.min(3))
    }
}

pub fn pivot_bias(rig: &Rig, part: usize) -> Option<(f32, f32)> {
    let declared = rig.model.parts.get(part)?;
    let cut = rig.sheet.cuts.get(usize::try_from(declared.sprite).ok()?)?;

    (cut.width != 0 && cut.height != 0)
        .then(|| (declared.pivot_x as f32 / cut.width as f32, declared.pivot_y as f32 / cut.height as f32))
}

pub fn split(reach: [(f32, f32); 2], travel: (f32, f32)) -> Option<(f32, f32)> {
    let [(ax, ay), (bx, by)] = reach;
    let determinant = ax * by - ay * bx;

    if determinant.abs() < SINGULAR {
        return None;
    }

    Some((
        (travel.0 * by - travel.1 * bx) / determinant,
        (travel.1 * ax - travel.0 * ay) / determinant,
    ))
}

pub fn along(reach: (f32, f32), travel: (f32, f32)) -> Option<f32> {
    let span = reach.0 * reach.0 + reach.1 * reach.1;

    (span > SINGULAR).then(|| (travel.0 * reach.0 + travel.1 * reach.1) / span)
}

fn field(model: &Model, part: usize, at: usize) -> Option<i32> {
    let part = model.parts.get(part)?;

    Some(match at {
        0 => part.parent,
        1 => part.id,
        2 => part.sprite,
        3 => part.z,
        4 => part.x,
        5 => part.y,
        6 => part.pivot_x,
        7 => part.pivot_y,
        8 => part.scale_x,
        9 => part.scale_y,
        10 => part.angle,
        11 => part.opacity,
        _ => part.glow,
    })
}

fn set_field(model: &mut Model, part: usize, at: usize, value: i32) {
    let Some(part) = model.parts.get_mut(part) else {
        return;
    };

    let cell = match at {
        0 => &mut part.parent,
        1 => &mut part.id,
        2 => &mut part.sprite,
        3 => &mut part.z,
        4 => &mut part.x,
        5 => &mut part.y,
        6 => &mut part.pivot_x,
        7 => &mut part.pivot_y,
        8 => &mut part.scale_x,
        9 => &mut part.scale_y,
        10 => &mut part.angle,
        11 => &mut part.opacity,
        12 => &mut part.glow,
        _ => return,
    };

    *cell = value;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_travel_splits_back_into_the_deltas_that_made_it() {
        // The reach columns are whatever the engine reported; splitting a travel by them
        // has to hand back the deltas that produced it, whatever the part's rotation.
        let reach = [(2.0, 0.0), (0.0, 4.0)];

        assert_eq!(split(reach, (6.0, 8.0)), Some((3.0, 2.0)));

        let turned = [(0.0, 1.5), (-1.5, 0.0)];
        let (across, down) = split(turned, (-3.0, 3.0)).expect("the pair is invertible");

        assert!((across - 2.0).abs() < 1.0e-5 && (down - 2.0).abs() < 1.0e-5);
    }

    #[test]
    fn a_degenerate_pair_reports_nothing_rather_than_a_wild_answer() {
        // A part scaled to nothing, or two fields that move it the same way, has no
        // unique split. Dividing by that determinant would fling it across the screen.
        assert_eq!(split([(0.0, 0.0), (0.0, 0.0)], (5.0, 5.0)), None);
        assert_eq!(split([(1.0, 1.0), (2.0, 2.0)], (5.0, 5.0)), None);
    }

    #[test]
    fn a_single_field_travels_along_its_own_reach_and_ignores_the_rest() {
        // Dragging across a field that only moves the part vertically must read the
        // vertical component and discard the sideways one.
        assert_eq!(along((0.0, 2.0), (10.0, 4.0)), Some(2.0));
        assert_eq!(along((0.0, 0.0), (10.0, 4.0)), None);
    }
}
