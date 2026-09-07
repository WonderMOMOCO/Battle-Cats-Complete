use iced::mouse;
use iced::widget::canvas as canvas_widget;
use iced::widget::canvas::{self, Geometry, Path, Stroke};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, Vector};

use crate::systems::animation::{Posed, ZOOM_SCROLL_STRENGTH};
use crate::widget::{smooth_scroll, LINE_PIXELS};

use super::Message;

const HANDLE: f32 = 7.0;
const GRAB: f32 = 10.0;
const INSET: f32 = HANDLE / 2.0;
const EDGE_SHARE: f32 = 1.0 / 3.0;
const NECK_SHARE: f32 = 0.45;
const NECK_FLOOR: f32 = 18.0;
const KNOB: f32 = 5.5;
const KNOB_GRAB: f32 = 11.0;
const DEGENERATE: f32 = 0.5;
const ORBIT: f32 = 8.0;
const BLEED: f32 = 0.02;
const SLACK: f32 = 1.5;
const SINGULAR: f32 = 1.0e-4;
const OUTLINE: f32 = 2.5;
const FLAT: f32 = 1.0;
const CLICK_SLOP: f32 = 3.0;
const OPACITY_STEP: f32 = 0.04;
const INK: Color = Color::from_rgb(0.64, 0.42, 0.94);
const LIVE_INK: Color = Color::from_rgb(0.79, 0.60, 1.0);
const TURN_INK: Color = Color::from_rgb(0.38, 0.20, 0.62);
const TURN_LIVE: Color = Color::from_rgb(0.55, 0.32, 0.85);
const ORBIT_INK: Color = Color::from_rgba(0.46, 0.26, 0.76, 0.85);
const ORBIT_WIDTH: f32 = 1.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grip {
    Move,
    Rotate,
    Scale { across: i8, down: i8 },
}

impl Grip {
    pub fn corners(self) -> Option<(usize, usize)> {
        let Grip::Scale { across, down } = self else {
            return None;
        };

        let seat = |right: bool, bottom: bool| usize::from(right) * 2 + usize::from(bottom);
        let (grabbed_right, grabbed_bottom) = (across > 0, down > 0);
        let flip = |side: i8, held: bool| if side == 0 { held } else { !held };

        Some((
            seat(grabbed_right, grabbed_bottom),
            seat(flip(across, grabbed_right), flip(down, grabbed_bottom)),
        ))
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Sweep {
    pub grip: Grip,
    pub travel: Vector,
    pub spun: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Turn {
    Halt,
    Pick(usize),
    Seize(usize),
    Zoom(f32),
    Begin(usize, Grip),
    Drag(Sweep),
    Drop,
    Fade(f32),
}

#[derive(Default)]
pub(super) struct State {
    shown: bool,
    held: Option<Grip>,
}

impl State {
    pub(super) fn show(&mut self, shown: bool) {
        self.shown = shown;

        if !shown {
            self.held = None;
        }
    }

    pub(super) fn seize(&mut self, held: Option<Grip>) {
        self.held = held;
    }

    pub(super) fn view<'a>(
        &self,
        picked: Option<usize>,
        posed: Vec<Posed>,
        camera: (Vector, f32),
    ) -> Element<'a, Message> {
        let face = canvas_widget(Gizmo { picked, shown: self.shown, posed, camera, held: self.held })
            .width(Length::Fill)
            .height(Length::Fill);

        smooth_scroll(face).strength(ZOOM_SCROLL_STRENGTH).into()
    }
}

#[derive(Default)]
pub(super) struct Track {
    grip: Option<Grip>,
    from: Point,
    lag: f32,
    idle: Option<Point>,
}

struct Gizmo {
    picked: Option<usize>,
    shown: bool,
    posed: Vec<Posed>,
    camera: (Vector, f32),
    held: Option<Grip>,
}

impl Gizmo {
    fn seat(&self, bounds: Rectangle) -> impl Fn(f32, f32) -> Point + '_ {
        let (pan, zoom) = self.camera;
        let centre = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        move |x, y| Point::new(centre.x + (x + pan.x) * zoom, centre.y + (y + pan.y) * zoom)
    }

    fn held(&self, bounds: Rectangle) -> Option<([Point; 4], Point)> {
        let seat = self.seat(bounds);
        let found = self.posed.iter().find(|entry| Some(entry.part) == self.picked)?;
        let quad = std::array::from_fn(|at| seat(found.quad[at * 2], found.quad[at * 2 + 1]));

        Some((quad, seat(found.origin.0, found.origin.1)))
    }

    fn claimed(&self, bounds: Rectangle, at: Point) -> Option<Grip> {
        let (quad, origin) = self.held(bounds)?;
        let grip = reachable(&quad, origin, at).then(|| grip_at(&quad, origin, at))?;

        if grip == Grip::Move && topmost(&self.posed, at, &self.seat(bounds)) != self.picked {
            return None;
        }

        Some(grip)
    }
}

pub(super) fn topmost(posed: &[Posed], at: Point, seat: &impl Fn(f32, f32) -> Point) -> Option<usize> {
    posed
        .iter()
        .rev()
        .find(|entry| {
            let quad: [Point; 4] = std::array::from_fn(|i| seat(entry.quad[i * 2], entry.quad[i * 2 + 1]));

            inside(&quad, at)
        })
        .map(|entry| entry.part)
}

pub(super) fn grip_at(quad: &[Point; 4], origin: Point, at: Point) -> Grip {
    if turning(quad, origin, at) {
        return Grip::Rotate;
    }

    let Some((across, down)) = coords(quad, at) else {
        return Grip::Move;
    };

    let side = |ratio: f32, reach: f32| {
        let band = INSET.min(reach * EDGE_SHARE);

        match (ratio * reach, (1.0 - ratio) * reach) {
            (near, _) if (-GRAB..=band).contains(&near) => -1,
            (_, far) if (-GRAB..=band).contains(&far) => 1,
            _ => 0,
        }
    };

    let (wide, tall) = sides(quad);

    match (side(across, wide), side(down, tall)) {
        (0, 0) => Grip::Move,
        (across, down) => Grip::Scale { across, down },
    }
}

pub(super) fn reachable(quad: &[Point; 4], origin: Point, at: Point) -> bool {
    if turning(quad, origin, at) {
        return true;
    }

    let Some((across, down)) = coords(quad, at) else {
        let centre = centroid(quad);

        return (at.x - centre.x).hypot(at.y - centre.y) <= GRAB;
    };

    let over = |ratio: f32, reach: f32| (if ratio < 0.0 { -ratio } else { ratio - 1.0 }).max(0.0) * reach;
    let (wide, tall) = sides(quad);

    over(across, wide) <= GRAB && over(down, tall) <= GRAB
}

pub(super) struct Ring {
    pub(super) origin: Point,
    pub(super) knob: Point,
    pub(super) lag: f32,
    pub(super) slack: f32,
}

impl Ring {
    pub(super) fn new(origin: Point, knob: Point, lag: f32, zoom: f32) -> Self {
        let reach = (knob.x - origin.x).hypot(knob.y - origin.y) / zoom.max(f32::EPSILON);

        Self { origin, knob, lag, slack: (SLACK / reach.max(1.0)).min(BLEED) }
    }
}

pub(super) fn swept(ring: &Ring, from: Point, at: Point, grip: Grip) -> Sweep {
    let away = |point: Point| (point.x - ring.origin.x).hypot(point.y - ring.origin.y) > ORBIT;

    Sweep {
        grip,
        travel: Vector::new(at.x - from.x, at.y - from.y),
        spun: match away(from) && away(at) {
            true => tracked(ring, from, at),
            false => 0.0,
        },
    }
}

fn seized(track: &mut Track, at: Point, part: Option<usize>) -> Turn {
    let Some(part) = part else {
        return Turn::Halt;
    };

    track.grip = Some(Grip::Move);
    track.from = at;
    track.lag = 0.0;

    Turn::Seize(part)
}

pub(super) fn lagging(origin: Point, at: Point, knob: Point) -> f32 {
    wrapped(turn_at(origin, knob) - turn_at(origin, at))
}

fn tracked(ring: &Ring, from: Point, at: Point) -> f32 {
    let seen = turn_at(ring.origin, at);
    let walked = wrapped(seen - turn_at(ring.origin, from));
    let standing = wrapped(seen + ring.lag - turn_at(ring.origin, ring.knob));
    let adrift = standing - walked;

    walked + (adrift.abs() - ring.slack).clamp(0.0, BLEED) * adrift.signum()
}

fn turn_at(centre: Point, at: Point) -> f32 {
    (at.y - centre.y).atan2(at.x - centre.x)
}

fn wrapped(mut angle: f32) -> f32 {
    while angle > std::f32::consts::PI {
        angle -= std::f32::consts::TAU;
    }

    while angle < -std::f32::consts::PI {
        angle += std::f32::consts::TAU;
    }

    angle
}

pub(super) fn turning(quad: &[Point; 4], origin: Point, at: Point) -> bool {
    let knob = lever(quad, origin).1;

    (at.x - knob.x).hypot(at.y - knob.y) <= KNOB_GRAB
}

pub(super) fn neck(quad: &[Point; 4]) -> f32 {
    let [top_left, bottom_left, top_right, _] = *quad;
    let across = (top_right.x - top_left.x).hypot(top_right.y - top_left.y);
    let down = (bottom_left.x - top_left.x).hypot(bottom_left.y - top_left.y);

    ((across + down) / 2.0 * NECK_SHARE).max(NECK_FLOOR)
}

pub(super) fn lever(quad: &[Point; 4], origin: Point) -> (Point, Point) {
    let [top_left, bottom_left, top_right, _] = *quad;
    let up = Point::new((top_left.x + top_right.x) / 2.0, (top_left.y + top_right.y) / 2.0);
    let aim = Vector::new(up.x - origin.x, up.y - origin.y);
    let reach = aim.x.hypot(aim.y);
    let lead = neck(quad);

    if reach < DEGENERATE {
        return (up, Point::new(up.x, up.y - lead));
    }

    let seat = |across: f32, down: f32| {
        Point::new(
            top_left.x + across * (top_right.x - top_left.x) + down * (bottom_left.x - top_left.x),
            top_left.y + across * (top_right.y - top_left.y) + down * (bottom_left.y - top_left.y),
        )
    };

    let Some(start) = coords(quad, origin) else {
        return (up, Point::new(up.x + aim.x / reach * lead, up.y + aim.y / reach * lead));
    };

    let step = (0.5 - start.0, -start.1);
    let leaving = |from: f32, by: f32| match by.abs() < SINGULAR {
        true => f32::INFINITY,
        false => ((if by > 0.0 { 1.0 } else { 0.0 }) - from) / by,
    };

    let out = leaving(start.0, step.0).min(leaving(start.1, step.1)).max(1.0);
    let exit = seat(start.0 + out * step.0, start.1 + out * step.1);

    (exit, Point::new(exit.x + aim.x / reach * lead, exit.y + aim.y / reach * lead))
}

pub(super) fn fade(delta: f32) -> f32 {
    delta * OPACITY_STEP * 1000.0
}

pub(super) fn coords(quad: &[Point; 4], at: Point) -> Option<(f32, f32)> {
    let [top_left, bottom_left, top_right, _] = *quad;
    let across = Vector::new(top_right.x - top_left.x, top_right.y - top_left.y);
    let down = Vector::new(bottom_left.x - top_left.x, bottom_left.y - top_left.y);
    let turned = across.x * down.y - across.y * down.x;

    if turned.abs() <= FLAT {
        return None;
    }

    let reach = Vector::new(at.x - top_left.x, at.y - top_left.y);

    Some((
        (reach.x * down.y - reach.y * down.x) / turned,
        (across.x * reach.y - across.y * reach.x) / turned,
    ))
}

fn sides(quad: &[Point; 4]) -> (f32, f32) {
    let [top_left, bottom_left, top_right, _] = *quad;

    (
        (top_right.x - top_left.x).hypot(top_right.y - top_left.y),
        (bottom_left.x - top_left.x).hypot(bottom_left.y - top_left.y),
    )
}

fn inside(quad: &[Point; 4], at: Point) -> bool {
    coords(quad, at)
        .is_some_and(|(across, down)| (0.0..=1.0).contains(&across) && (0.0..=1.0).contains(&down))
}

fn centroid(quad: &[Point; 4]) -> Point {
    let sum = quad.iter().fold((0.0, 0.0), |held, point| (held.0 + point.x, held.1 + point.y));

    Point::new(sum.0 / 4.0, sum.1 / 4.0)
}

impl canvas::Program<Message> for Gizmo {
    type State = Track;

    fn draw(
        &self,
        _state: &Track,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let Some((quad, origin)) = self.held(bounds).filter(|_| self.shown) else {
            return Vec::new();
        };

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let live = self.held.is_some();
        let ink = if live { LIVE_INK } else { INK };

        let [top_left, bottom_left, top_right, bottom_right] = quad;
        let outline = Path::new(|builder| {
            builder.move_to(top_left);
            builder.line_to(top_right);
            builder.line_to(bottom_right);
            builder.line_to(bottom_left);
            builder.close();
        });

        frame.stroke(&outline, Stroke::default().with_color(ink).with_width(OUTLINE));

        let turn = if live { TURN_LIVE } else { TURN_INK };
        let (edge, knob) = lever(&quad, origin);

        if self.held == Some(Grip::Rotate) {
            let orbit = (knob.x - origin.x).hypot(knob.y - origin.y);

            frame.stroke(
                &Path::circle(origin, orbit),
                Stroke::default().with_color(ORBIT_INK).with_width(ORBIT_WIDTH),
            );
            frame.fill(&Path::circle(origin, ORBIT_WIDTH * 2.0), ORBIT_INK);
        }

        frame.stroke(&Path::line(edge, knob), Stroke::default().with_color(turn).with_width(OUTLINE));
        frame.fill(&Path::circle(knob, KNOB), turn);
        frame.stroke(&Path::circle(knob, KNOB), Stroke::default().with_color(Color::BLACK).with_width(1.0));

        let midpoint = |a: Point, b: Point| Point::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
        let handles = [
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            midpoint(top_left, top_right),
            midpoint(bottom_left, bottom_right),
            midpoint(top_left, bottom_left),
            midpoint(top_right, bottom_right),
        ];

        for handle in handles {
            let box_at = Path::rectangle(
                Point::new(handle.x - HANDLE / 2.0, handle.y - HANDLE / 2.0),
                iced::Size::new(HANDLE, HANDLE),
            );

            frame.fill(&box_at, ink);
            frame.stroke(&box_at, Stroke::default().with_color(Color::BLACK).with_width(1.0));
        }

        vec![frame.into_geometry()]
    }

    fn update(
        &self,
        track: &mut Track,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let iced::Event::Mouse(mouse) = event else {
            return None;
        };

        match mouse {
            mouse::Event::ButtonPressed(button) => {
                let Some(at) = cursor.position_in(bounds) else {
                    return self
                        .picked
                        .map(|_| canvas::Action::publish(Message::Gizmo(Turn::Halt)));
                };

                if *button == mouse::Button::Left {
                    track.idle = Some(at);

                    return None;
                }

                if *button != mouse::Button::Right {
                    return None;
                }

                let seat = self.seat(bounds);

                let Some((quad, origin)) = self.held(bounds) else {
                    let turn = seized(track, at, topmost(&self.posed, at, &seat));

                    return Some(canvas::Action::publish(Message::Gizmo(turn)).and_capture());
                };

                match (self.claimed(bounds, at), self.picked) {
                    (Some(grip), Some(part)) => {
                        track.grip = Some(grip);
                        track.from = at;
                        track.lag = lagging(origin, at, lever(&quad, origin).1);

                        Some(canvas::Action::publish(Message::Gizmo(Turn::Begin(part, grip))).and_capture())
                    }
                    _ => {
                        let turn = seized(track, at, topmost(&self.posed, at, &seat));

                        Some(canvas::Action::publish(Message::Gizmo(turn)).and_capture())
                    }
                }
            }
            mouse::Event::ButtonReleased(button) => {
                if track.grip.take().is_some() {
                    return Some(canvas::Action::publish(Message::Gizmo(Turn::Drop)).and_capture());
                }

                if *button != mouse::Button::Left {
                    return None;
                }

                let from = track.idle.take()?;
                let at = cursor.position_in(bounds)?;

                if (at.x - from.x).hypot(at.y - from.y) > CLICK_SLOP {
                    return None;
                }

                let seat = self.seat(bounds);
                let turn = topmost(&self.posed, at, &seat).map_or(Turn::Halt, Turn::Pick);

                Some(canvas::Action::publish(Message::Gizmo(turn)))
            }
            mouse::Event::CursorLeft => {
                track.idle = None;
                track.grip.take()?;

                Some(canvas::Action::publish(Message::Gizmo(Turn::Drop)))
            }
            mouse::Event::CursorMoved { position } => {
                let grip = track.grip?;
                let at = Point::new(position.x - bounds.x, position.y - bounds.y);
                let (quad, origin) = self.held(bounds)?;
                let ring = Ring::new(origin, lever(&quad, origin).1, track.lag, self.camera.1);
                let sweep = swept(&ring, track.from, at, grip);

                track.from = at;

                Some(canvas::Action::publish(Message::Gizmo(Turn::Drag(sweep))).and_capture())
            }
            mouse::Event::WheelScrolled { delta } if track.grip.is_some() && self.held.is_some() => {
                let step = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };

                Some(canvas::Action::publish(Message::Gizmo(Turn::Fade(fade(step)))).and_capture())
            }
            mouse::Event::WheelScrolled { delta } => {
                track.grip = None;
                cursor.position_in(bounds)?;

                let pixels = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * LINE_PIXELS,
                    mouse::ScrollDelta::Pixels { y, .. } => *y,
                };

                (pixels != 0.0)
                    .then(|| canvas::Action::publish(Message::Gizmo(Turn::Zoom(pixels))).and_capture())
            }
            _ => None,
        }
    }

    fn mouse_interaction(
        &self,
        _state: &Track,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let Some(at) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };

        let Some(grip) = self.claimed(bounds, at).filter(|_| self.shown) else {
            return mouse::Interaction::None;
        };

        match grip {
            Grip::Move => mouse::Interaction::Move,
            Grip::Rotate => mouse::Interaction::Grab,
            Grip::Scale { .. } => mouse::Interaction::ResizingDiagonallyDown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad() -> [Point; 4] {
        [Point::new(0.0, 0.0), Point::new(0.0, 100.0), Point::new(100.0, 0.0), Point::new(100.0, 100.0)]
    }

    fn middle() -> Point {
        Point::new(50.0, 50.0)
    }

    fn seen(posed: [f32; 8]) -> Posed {
        Posed { part: 3, quad: posed, origin: (0.0, 0.0) }
    }

    #[test]
    fn the_scale_band_bleeds_outward_from_the_edge_not_inward() {
        // The handles sit on the edge, so the band reaches `GRAB` px out and only far
        // enough in to cover the half of the handle that overlaps the part.
        let (box_at, mid) = (quad(), middle());

        assert_eq!(grip_at(&box_at, mid, Point::new(-6.0, 50.0)), Grip::Scale { across: -1, down: 0 });
        assert_eq!(grip_at(&box_at, mid, Point::new(2.0, 50.0)), Grip::Scale { across: -1, down: 0 });
        assert_eq!(grip_at(&box_at, mid, Point::new(10.0, 50.0)), Grip::Move);
        assert_eq!(grip_at(&box_at, mid, Point::new(50.0, 50.0)), Grip::Move);
        assert_eq!(grip_at(&box_at, mid, Point::new(103.0, 50.0)), Grip::Scale { across: 1, down: 0 });
        assert_eq!(grip_at(&box_at, mid, Point::new(2.0, 103.0)), Grip::Scale { across: -1, down: 1 });
    }

    #[test]
    fn a_part_too_small_to_spare_the_band_keeps_a_middle_to_move_by() {
        let small = [Point::new(0.0, 0.0), Point::new(0.0, 16.0), Point::new(16.0, 0.0), Point::new(16.0, 16.0)];

        assert_eq!(grip_at(&small, Point::new(8.0, 8.0), Point::new(8.0, 8.0)), Grip::Move);
        assert_eq!(grip_at(&small, Point::new(8.0, 8.0), Point::new(1.0, 8.0)), Grip::Scale { across: -1, down: 0 });
    }

    #[test]
    fn the_knob_leaves_the_box_where_the_origin_axis_crosses_the_edge() {
        // The lever runs from the part's own origin through the top edge's midpoint and
        // out. A centred origin leaves through the top; only the crossing matters.
        let (exit, knob) = lever(&quad(), middle());

        assert_eq!(exit, Point::new(50.0, 0.0));
        assert_eq!(knob, Point::new(50.0, -neck(&quad())));
        assert_eq!(grip_at(&quad(), middle(), knob), Grip::Rotate);
        assert!(reachable(&quad(), middle(), knob));
    }

    #[test]
    fn an_origin_outside_the_box_still_leaves_through_a_real_edge() {
        // Anything with the origin off its own sprite used to fling the knob into space.
        // Approaching from below or from the side, the ray leaves by the top edge it aims at.
        assert_eq!(lever(&quad(), Point::new(50.0, 400.0)).0, Point::new(50.0, 0.0));
        assert_eq!(lever(&quad(), Point::new(400.0, 50.0)).0, Point::new(50.0, 0.0));
    }

    #[test]
    fn entering_the_box_does_not_count_only_leaving_it_does() {
        // With the origin above the part the axis crosses the top edge on the way in,
        // so the knob belongs on the far side, not on the first edge it meets.
        let (exit, knob) = lever(&quad(), Point::new(50.0, -400.0));

        assert_eq!(exit, Point::new(50.0, 100.0));
        assert_eq!(knob, Point::new(50.0, 100.0 + neck(&quad())));
    }

    #[test]
    fn an_origin_sitting_on_the_axis_falls_back_rather_than_dividing_by_nothing() {
        let (exit, knob) = lever(&quad(), Point::new(50.0, 0.0));

        assert_eq!(exit, Point::new(50.0, 0.0));
        assert_eq!(knob, Point::new(50.0, -neck(&quad())));
    }

    #[test]
    fn a_long_thin_sheared_part_is_hittable_along_its_whole_length() {
        // Anubis part 68, the staff shaft: 421 long, 10 wide, and pixel rounding leaves the
        // two edges 1.5 degrees off square. Projecting onto each edge separately leaks the
        // long axis into the short one, so only the near half of the part answered a click.
        let staff = [
            Point::new(-42.5, -427.0),
            Point::new(-52.5, -426.0),
            Point::new(-11.5, -6.0),
            Point::new(-21.5, -5.0),
        ];

        let seat = |across: f32, down: f32| {
            Point::new(
                staff[0].x + across * (staff[2].x - staff[0].x) + down * (staff[1].x - staff[0].x),
                staff[0].y + across * (staff[2].y - staff[0].y) + down * (staff[1].y - staff[0].y),
            )
        };

        for along in [0.05, 0.25, 0.5, 0.75, 0.95] {
            assert!(inside(&staff, seat(along, 0.5)), "along {along}");
        }

        assert!(!inside(&staff, seat(0.5, 1.6)));
        assert!(!inside(&staff, seat(1.4, 0.5)));

        let (across, down) = coords(&staff, seat(0.75, 0.25)).expect("the quad has area");

        assert!((across - 0.75).abs() < 1.0e-3 && (down - 0.25).abs() < 1.0e-3);
    }

    #[test]
    fn a_collapsed_part_claims_its_own_pixels_rather_than_the_whole_canvas() {
        // Scaling a part to zero used to make every span read 0.5 and swallow every click,
        // which is what left the viewer feeling frozen. It stays grabbable up close so
        // the part can be dragged back out, and picking it never sees it at all.
        let squashed = [Point::new(50.0, 50.0); 4];

        assert!(!inside(&squashed, Point::new(400.0, 12.0)));
        assert!(!reachable(&squashed, middle(), Point::new(400.0, 12.0)));
        assert!(reachable(&squashed, middle(), Point::new(56.0, 50.0)));
        assert_eq!(topmost(&[seen([50.0; 8])], Point::new(50.0, 50.0), &|x, y| Point::new(x, y)), None);
    }

    #[test]
    fn the_sweep_is_the_angle_the_hand_walked_around_the_origin() {
        // Rotation tracks the cursor one for one; the probe only decides which way the
        // angle field has to move to follow it.
        let (from, at) = (Point::new(150.0, 50.0), Point::new(50.0, 150.0));
        let quarter = std::f32::consts::FRAC_PI_2;

        // A knob sitting where the hand left it last event has nothing standing against it,
        // so the hand's own walk is the whole answer.
        let kept_up = swept(&Ring::new(middle(), from, 0.0, 1.0), from, at, Grip::Rotate);

        assert!((kept_up.spun - quarter).abs() < 1.0e-5);

        let stuck = swept(&Ring::new(middle(), at, 0.0, 1.0), middle(), Point::new(50.2, 50.0), Grip::Rotate);

        assert_eq!(stuck.spun, 0.0);
    }

    #[test]
    fn a_knob_that_has_fallen_behind_the_hand_is_bled_back_onto_it() {
        // Any per frame mismatch between applied angle and rendered angle accumulates over
        // a long drag until the ring visibly trails the cursor. The standing error is
        // folded back in at `BLEED` per event, so it converges without ever lurching.
        let (from, at) = (Point::new(150.0, 50.0), Point::new(50.0, 150.0));
        let quarter = std::f32::consts::FRAC_PI_2;

        let behind = Point::new(137.8, 2.057);
        let caught = swept(&Ring::new(middle(), behind, 0.0, 1.0), from, at, Grip::Rotate);

        assert!(caught.spun > quarter && caught.spun <= quarter + BLEED + 1.0e-4);

        // Overshooting the other way bleeds back by the same bounded step.
        let ahead = Point::new(137.8, 97.94);
        let pulled = swept(&Ring::new(middle(), ahead, 0.0, 1.0), from, at, Grip::Rotate);

        assert!(pulled.spun < quarter && pulled.spun >= quarter - BLEED - 1.0e-4);

        // Under the noise floor nothing is corrected at all, so a knob whose rendered angle
        // is only wobbling by a rounded pixel cannot make the part jitter.
        let nudged = Point::new(150.0, 49.6);
        let ignored = swept(&Ring::new(middle(), nudged, 0.0, 1.0), from, at, Grip::Rotate);

        assert!((ignored.spun - quarter).abs() < 1.0e-5);
    }

    #[test]
    fn the_grabbed_corner_and_its_anchor_sit_opposite_each_other() {
        // Corners are [top left, bottom left, top right, bottom right].
        assert_eq!(Grip::Scale { across: 1, down: 1 }.corners(), Some((3, 0)));
        assert_eq!(Grip::Scale { across: -1, down: -1 }.corners(), Some((0, 3)));
        assert_eq!(Grip::Scale { across: -1, down: 0 }.corners(), Some((0, 2)));
        assert_eq!(Grip::Move.corners(), None);
    }

    #[test]
    fn the_neck_grows_with_the_part_on_screen_and_never_shrinks_past_reach() {
        // The quad is already seated in screen space, so a share of it scales with zoom.
        let doubled: [Point; 4] = std::array::from_fn(|at| Point::new(quad()[at].x * 2.0, quad()[at].y * 2.0));

        assert_eq!(neck(&quad()), 100.0 * NECK_SHARE);
        assert_eq!(neck(&doubled), 200.0 * NECK_SHARE);

        let speck = [Point::new(0.0, 0.0), Point::new(0.0, 4.0), Point::new(4.0, 0.0), Point::new(4.0, 4.0)];

        assert_eq!(neck(&speck), NECK_FLOOR);
    }

    #[test]
    fn the_topmost_part_wins_where_two_overlap() {
        // `part::resolve` hands parts back in draw order, so the last one that
        // contains the cursor is the one drawn on top.
        let posed = vec![
            Posed { part: 3, quad: [0.0, 0.0, 0.0, 100.0, 100.0, 0.0, 100.0, 100.0], origin: (0.0, 0.0) },
            Posed { part: 7, quad: [50.0, 50.0, 50.0, 150.0, 150.0, 50.0, 150.0, 150.0], origin: (0.0, 0.0) },
        ];
        let seat = |x: f32, y: f32| Point::new(x, y);

        assert_eq!(topmost(&posed, Point::new(75.0, 75.0), &seat), Some(7));
        assert_eq!(topmost(&posed, Point::new(10.0, 10.0), &seat), Some(3));
        assert_eq!(topmost(&posed, Point::new(400.0, 400.0), &seat), None);
    }
}
