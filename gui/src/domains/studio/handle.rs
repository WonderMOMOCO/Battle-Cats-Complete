use super::*;

use iced::Vector;


const X_FIELD: usize = 4;
const Y_FIELD: usize = 5;
const SCALE_X_FIELD: usize = 8;
const SCALE_Y_FIELD: usize = 9;
const ANGLE_FIELD: usize = 10;
const OPACITY_FIELD: usize = 11;

const X_KIND: i32 = 4;
const Y_KIND: i32 = 5;
const SCALE_X_KIND: i32 = 9;
const SCALE_Y_KIND: i32 = 10;
const ANGLE_KIND: i32 = 11;
const OPACITY_KIND: i32 = 12;

const TURN_PROBE: i32 = 8;

type Step = (usize, i32, f32);

impl Session {
    pub(super) fn animated(&self) -> bool {
        self.viewer.animation().is_some()
    }

    pub(super) fn hand(&self, settings: &Settings) -> Gizmo {
        match self.animated() {
            true => settings.studio.gizmo,
            false => Gizmo::Model,
        }
    }

    pub(super) fn chosen_part(&self) -> Option<usize> {
        self.pose.as_ref().and_then(|pose| pose.part)
    }

    pub(super) fn grasp(&mut self, part: usize, grip: gizmo::Grip, hand: Gizmo) {
        self.gizmo.show(true);
        self.gizmo.seize(Some(grip));
        self.viewer.pause();
        self.drift.clear();

        if grip == gizmo::Grip::Rotate {
            self.winding = self.wound(part, hand).unwrap_or(1.0);
        }
    }

    pub(super) fn haul(&mut self, sweep: gizmo::Sweep, hand: Gizmo) -> Task<Message> {
        let entity = self.entity;

        let Some(part) = self.chosen_part() else {
            return Task::none();
        };

        let task = match sweep.grip {
            gizmo::Grip::Move => self.shove(part, sweep.travel, hand),
            gizmo::Grip::Scale { .. } => self.stretch(part, sweep, hand),
            gizmo::Grip::Rotate => self.spin(part, sweep.spun, hand),
        };

        self.reposed(entity);

        task
    }

    pub(super) fn rooted(&self, part: usize) -> bool {
        self.pose.as_ref().is_some_and(|pose| pose.doc.parent(part).is_none())
    }

    pub(super) fn pinned_notice(&self) -> Option<Notice> {
        if self.mode != Mode::Entity || self.focus != Focus::Curve {
            return None;
        }

        let draft = self.draft.as_ref()?;
        let held = draft.doc.track(draft.track?)?;

        if !matches!(held.kind, X_KIND | Y_KIND) {
            return None;
        }

        let part = usize::try_from(held.part).ok()?;

        self.pinned(part).then(|| (PINNED_NOTICE.to_owned(), None))
    }

    pub(super) fn pinned(&self, part: usize) -> bool {
        let Some(row) = self.viewer.offset() else {
            return false;
        };

        self.viewer
            .rig()
            .and_then(|rig| rig.model.alignment.get(row))
            .is_some_and(|align| usize::try_from(align.part).is_ok_and(|named| named == part))
    }

    pub(super) fn tint(&mut self, step: f32, hand: Gizmo) -> Task<Message> {
        let Some(part) = self.chosen_part() else {
            return Task::none();
        };

        self.apply(part, &[(OPACITY_FIELD, OPACITY_KIND, step)], hand)
    }

    fn shove(&mut self, part: usize, travel: Vector, hand: Gizmo) -> Task<Message> {
        if hand == Gizmo::Model && self.rooted(part) {
            return Task::done(Message::Refused(ROOT_MOVE_NOTICE));
        }

        if hand == Gizmo::Channel && self.pinned(part) {
            return Task::done(Message::Refused(PINNED_NOTICE));
        }

        let Some(travel) = self.worldly(travel) else {
            return Task::none();
        };

        let Some(reach) = self.pivot_reach(part, hand) else {
            return Task::none();
        };

        let Some((across, down)) = posing::split(reach, travel) else {
            return Task::none();
        };

        self.apply(part, &[(X_FIELD, X_KIND, across), (Y_FIELD, Y_KIND, down)], hand)
    }

    fn stretch(&mut self, part: usize, sweep: gizmo::Sweep, hand: Gizmo) -> Task<Message> {
        let gizmo::Grip::Scale { across, down } = sweep.grip else {
            return Task::none();
        };

        let (Some(travel), Some((grabbed, anchor))) = (self.worldly(sweep.travel), sweep.grip.corners())
        else {
            return Task::none();
        };

        let spots = [posing::Spot::Corner(grabbed), posing::Spot::Corner(anchor)];

        let Some(levers) = self.reach(part, (SCALE_X_FIELD, SCALE_Y_FIELD), (SCALE_X_KIND, SCALE_Y_KIND), hand, &spots)
        else {
            return Task::none();
        };

        let (Some(pulled), Some(held)) = (levers.first(), levers.get(1)) else {
            return Task::none();
        };

        let apart = [lessen(pulled[0], held[0]), lessen(pulled[1], held[1])];
        let grown = match (across != 0, down != 0) {
            (true, true) => posing::split(apart, travel),
            (true, false) => posing::along(apart[0], travel).map(|step| (step, 0.0)),
            (false, true) => posing::along(apart[1], travel).map(|step| (0.0, step)),
            (false, false) => None,
        };

        let Some((widen, heighten)) = grown else {
            return Task::none();
        };

        let strayed = (
            -(held[0].0 * widen + held[1].0 * heighten),
            -(held[0].1 * widen + held[1].1 * heighten),
        );

        let settled = self
            .pivot_reach(part, hand)
            .and_then(|reach| posing::split(reach, strayed))
            .unwrap_or((0.0, 0.0));

        let steps = [
            (SCALE_X_FIELD, SCALE_X_KIND, widen),
            (SCALE_Y_FIELD, SCALE_Y_KIND, heighten),
            (X_FIELD, X_KIND, settled.0),
            (Y_FIELD, Y_KIND, settled.1),
        ];

        self.apply(part, &steps, hand)
    }

    fn spin(&mut self, part: usize, spun: f32, hand: Gizmo) -> Task<Message> {
        let unit = self.viewer.rig().map_or(3600, |rig| rig.model.angle_unit).max(1) as f32;
        let step = spun / std::f32::consts::TAU * unit * self.winding;

        self.apply(part, &[(ANGLE_FIELD, ANGLE_KIND, step)], hand)
    }

    fn wound(&self, part: usize, hand: Gizmo) -> Option<f32> {
        let spots = [posing::Spot::Corner(0), posing::Spot::Corner(3)];
        let mut probe = self.probe(part)?;

        let swept = match hand {
            Gizmo::Model => probe.rest_sweep(ANGLE_FIELD, TURN_PROBE, &spots),
            Gizmo::Channel => {
                let doc = &self.draft.as_ref()?.doc;
                let held = self.held(part, ANGLE_FIELD, ANGLE_KIND, hand)?;

                probe.channel_sweep(doc, ANGLE_KIND, held, TURN_PROBE, &spots)
            }
        }?;

        let found = self.viewer.posed(Scope::Rig).into_iter().find(|entry| entry.part == part)?;
        let seat = |at: usize| (found.quad[at * 2], found.quad[at * 2 + 1]);

        let turning = [0, 3]
            .into_iter()
            .zip(swept)
            .map(|(corner, reach)| {
                let arm = span(found.origin, seat(corner));

                arm.0 * reach.1 - arm.1 * reach.0
            })
            .max_by(|turn, other| turn.abs().total_cmp(&other.abs()))?;

        (turning.abs() > f32::EPSILON).then(|| turning.signum())
    }

    fn worldly(&self, travel: Vector) -> Option<(f32, f32)> {
        let (_, zoom) = self.viewer.camera();

        (zoom.abs() > f32::EPSILON).then(|| (travel.x / zoom, travel.y / zoom))
    }

    fn probe(&self, part: usize) -> Option<Probe> {
        let rig = self.viewer.rig()?;

        Some(Probe::new(rig, self.viewer.animation(), self.viewer.frame(), self.viewer.offset(), part))
    }

    fn pivot_reach(&self, part: usize, hand: Gizmo) -> Option<[(f32, f32); 2]> {
        let spots = [posing::Spot::Pivot];

        self.reach(part, (X_FIELD, Y_FIELD), (X_KIND, Y_KIND), hand, &spots)?.first().copied()
    }

    fn reach(
        &self,
        part: usize,
        fields: (usize, usize),
        kinds: (i32, i32),
        hand: Gizmo,
        spots: &[posing::Spot],
    ) -> Option<Vec<[(f32, f32); 2]>> {
        let mut probe = self.probe(part)?;

        match hand {
            Gizmo::Model => probe.rest_reach(fields, spots),
            Gizmo::Channel => {
                let doc = &self.draft.as_ref()?.doc;
                let held = (
                    self.held(part, fields.0, kinds.0, hand)?,
                    self.held(part, fields.1, kinds.1, hand)?,
                );

                probe.channel_reach(doc, kinds, held, spots)
            }
        }
    }

    fn held(&self, part: usize, field: usize, kind: i32, hand: Gizmo) -> Option<i32> {
        let rest = || {
            let model = self.viewer.rig().map(|rig| &rig.model);

            model.map(|model| authoring::neutral_value(kind, part, Some(model)))
        };

        match hand {
            Gizmo::Model => self.pose.as_ref()?.doc.field(part, field).or_else(rest),
            Gizmo::Channel => {
                let frame = self.viewer.frame();

                self.draft.as_ref()?.doc.posed(part, kind, frame).or_else(rest)
            }
        }
    }

    fn carry(&mut self, field: usize, step: f32) -> i32 {
        let carried = self.drift.iter().find(|(at, _)| *at == field).map_or(0.0, |(_, held)| *held);
        let wanted = step + carried;
        let taken = wanted.round();

        match self.drift.iter_mut().find(|(at, _)| *at == field) {
            Some(slot) => slot.1 = wanted - taken,
            None => self.drift.push((field, wanted - taken)),
        }

        taken as i32
    }

    fn apply(&mut self, part: usize, steps: &[Step], hand: Gizmo) -> Task<Message> {
        let moved: Vec<(usize, i32, i32)> = steps
            .iter()
            .filter(|(_, _, step)| step.is_finite() && *step != 0.0)
            .filter_map(|(field, kind, step)| {
                let taken = self.carry(*field, *step);
                let held = self.held(part, *field, *kind, hand)?;

                (taken != 0).then(|| (*field, *kind, held.saturating_add(taken)))
            })
            .collect();

        if moved.is_empty() {
            return Task::none();
        }

        self.remember(Tag::Gizmo(part, hand));

        match hand {
            Gizmo::Model => self.reset_fields(part, &moved),
            Gizmo::Channel => self.key_fields(part, &moved),
        }
    }

    fn reset_fields(&mut self, part: usize, moved: &[(usize, i32, i32)]) -> Task<Message> {
        let Some(pose) = self.pose.as_mut() else {
            return Task::none();
        };

        pose.pick(part);

        for (field, _, value) in moved {
            pose.edit(*field, &value.to_string());
        }

        let task = pose.persist_if_dirty();

        self.settle_pose();

        task
    }

    fn key_fields(&mut self, part: usize, moved: &[(usize, i32, i32)]) -> Task<Message> {
        let frame = self.viewer.frame();
        let model = self.viewer.rig().map(|rig| rig.model.clone());

        let Some(draft) = self.draft.as_mut() else {
            return Task::none();
        };

        for (_, kind, value) in moved {
            draft.backing.dirty |= draft.doc.pose(part, *kind, frame, *value, model.as_ref());
        }

        draft.retrack_clamped();

        let task = draft.persist_if_dirty();
        let updated = draft.doc.shared();

        if let Some(showing) = self.viewer.selected_anim().cloned() {
            self.viewer.adopt_anim(&showing, updated);
        }

        self.relist();

        task
    }
}

fn lessen(pulled: (f32, f32), held: (f32, f32)) -> (f32, f32) {
    (pulled.0 - held.0, pulled.1 - held.1)
}

fn span(from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
    (to.0 - from.0, to.1 - from.1)
}
