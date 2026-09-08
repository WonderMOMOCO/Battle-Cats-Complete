use iced::mouse;
use iced::widget::canvas::{self, Geometry, Path, Stroke};
use iced::widget::canvas as canvas_widget;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, Vector};

use nyanko::graphics::animate::{resolve_frame, FrameData};
use nyanko::graphics::rig::{BoundingBox, Rig};
use nyanko::graphics::tools::part;

use kore::domains::settings::{Overlays, Scope, Tier};

use super::canvas as viewer;
use super::data;

const WORLD_COLOR: Color = Color::from_rgba(0.0, 1.0, 0.0, 0.5);
const ORIGIN_COLOR_MARK: Color = Color::from_rgb(0.0, 1.0, 0.0);
const WORLD_WIDTH: f32 = 1.5;
const ORIGIN_MARK_RADIUS: f32 = 5.0;

const PART_COLOR: Color = Color::from_rgba(1.0, 0.35, 0.35, 0.32);
const PICKED_COLOR: Color = Color::from_rgba(1.0, 0.18, 0.18, 0.95);
const AXIS_COLOR: Color = Color::from_rgba(1.0, 0.86, 0.15, 1.0);
const PARENT_COLOR: Color = Color::from_rgba(0.3, 0.9, 1.0, 1.0);
const ORIGIN_COLOR: Color = Color::from_rgba(0.3, 0.9, 1.0, 0.82);
const PICKED_ORIGIN_COLOR: Color = Color::from_rgba(0.3, 0.9, 1.0, 1.0);

const PART_WIDTH: f32 = 1.5;
const PICKED_WIDTH: f32 = 2.0;
const AXIS_WIDTH: f32 = 2.5;
const PARENT_WIDTH: f32 = 2.0;
const ORIGIN_RADIUS: f32 = 2.4;
const PICKED_RADIUS: f32 = 4.0;
const DEGENERATE: f32 = 1.5;

pub fn view<'a, M: 'a>(
    data: &'a data::State,
    state: &'a viewer::State,
    shown: Overlays,
    picked: Option<usize>,
) -> Element<'a, M> {
    let overlay = Parts {
        data,
        frame: state.current_frame,
        pan: state.pan,
        zoom: state.zoom,
        rig: shown.rig,
        selected: shown.selected,
        hierarchy: shown.hierarchy,
        origin: shown.origin.on(),
        world: shown.world.on(),
        picked,
    };

    canvas_widget(overlay).width(Length::Fill).height(Length::Fill).into()
}

struct Parts<'a> {
    data: &'a data::State,
    frame: f32,
    pan: Vector,
    zoom: f32,
    rig: Tier,
    selected: Tier,
    hierarchy: Tier,
    origin: bool,
    world: bool,
    picked: Option<usize>,
}

impl Parts<'_> {
    fn resolved(&self) -> Vec<(Option<usize>, FrameData)> {
        let Some(unit) = self.data.held_unit.as_ref() else {
            return Vec::new();
        };

        let frame = self.data.playback_frame(self.frame).floor() as i32;
        let anim = self.data.current_anim.as_deref();
        let offset = self.data.offset();

        if !self.rig.on() && !self.selected.on() && !self.hierarchy.on() {
            return resolve_frame(unit, anim, frame, offset).into_iter().map(|frame| (None, frame)).collect();
        }

        match part::resolve(unit, anim, frame, offset) {
            Ok(mapped) => mapped.into_iter().map(|entry| (Some(entry.part), entry.frame)).collect(),
            Err(_) => resolve_frame(unit, anim, frame, offset).into_iter().map(|frame| (None, frame)).collect(),
        }
    }

    fn level(&self, part: Option<usize>) -> u8 {
        let mut tier = self.rig;

        let Some(part) = part else {
            return tier.rank();
        };

        if self.picked == Some(part) {
            tier = tier.max(self.selected).max(self.hierarchy);
        } else if self.hierarchy.on() && self.picked == self.parent_of(part) {
            tier = tier.max(self.hierarchy);
        }

        tier.rank()
    }

    fn parent_of(&self, part: usize) -> Option<usize> {
        let model = &self.data.held_unit.as_ref()?.model;

        usize::try_from(model.parts.get(part)?.parent).ok()
    }

    fn anchor(&self, part: Option<usize>, geometry: &FrameData, quad: &[Point; 4]) -> Point {
        self.pivot(part, geometry, quad).unwrap_or_else(|| centroid(quad))
    }

    fn pivot(&self, part: Option<usize>, geometry: &FrameData, quad: &[Point; 4]) -> Option<Point> {
        pivot_of(self.data.held_unit.as_ref()?, part?, geometry, quad)
    }
}

fn corners(frame: &FrameData, to_screen: &impl Fn(f32, f32) -> Point) -> [Point; 4] {
    let at = |index: usize| to_screen(frame.vertices[index * 2], frame.vertices[index * 2 + 1]);

    [at(0), at(1), at(2), at(3)]
}

fn placed_corners(frame: &FrameData) -> [Point; 4] {
    corners(frame, &|x, y| Point::new(x, y))
}

pub(crate) fn pivot_of(unit: &Rig, part: usize, geometry: &FrameData, quad: &[Point; 4]) -> Option<Point> {
    let declared = unit.model.parts.get(part)?;
    let cut = unit.sheet.cuts.get(geometry.sprite_index)?;

    if cut.width == 0 || cut.height == 0 {
        return None;
    }

    let across = declared.pivot_x as f32 / cut.width as f32;
    let down = declared.pivot_y as f32 / cut.height as f32;
    let [top_left, bottom_left, top_right, _] = *quad;

    Some(Point::new(
        top_left.x + across * (top_right.x - top_left.x) + down * (bottom_left.x - top_left.x),
        top_left.y + across * (top_right.y - top_left.y) + down * (bottom_left.y - top_left.y),
    ))
}

pub fn anchor(data: &data::State, frame: f32, part: usize) -> Option<Point> {
    let unit = data.held_unit.as_ref()?;
    let at = data.playback_frame(frame).floor() as i32;
    let mapped = data.mapped(at)?;
    let found = mapped.iter().find(|entry| entry.part == part)?;
    let quad = placed_corners(&found.frame);

    pivot_of(unit, part, &found.frame, &quad).or_else(|| Some(centroid(&quad)))
}

fn centroid(corners: &[Point; 4]) -> Point {
    let sum = corners.iter().fold((0.0, 0.0), |acc, point| (acc.0 + point.x, acc.1 + point.y));

    Point::new(sum.0 / 4.0, sum.1 / 4.0)
}

fn outline(corners: &[Point; 4]) -> Path {
    let [top_left, bottom_left, top_right, bottom_right] = *corners;

    Path::new(|path| {
        path.move_to(top_left);
        path.line_to(top_right);
        path.line_to(bottom_right);
        path.line_to(bottom_left);
        path.close();
    })
}

impl<M> canvas::Program<M> for Parts<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        if !self.rig.on() && !self.selected.on() && !self.hierarchy.on() && !self.origin && !self.world {
            return Vec::new();
        }

        let parts = self.resolved();

        if parts.is_empty() {
            return Vec::new();
        }

        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = frame.center();
        let to_screen = |x: f32, y: f32| {
            Point::new(center.x + (x + self.pan.x) * self.zoom, center.y + (y + self.pan.y) * self.zoom)
        };

        let placed: Vec<(Option<usize>, [Point; 4], Point)> = parts
            .iter()
            .map(|(index, geometry)| {
                let quad = corners(geometry, &to_screen);
                let origin = self.anchor(*index, geometry, &quad);

                (*index, quad, origin)
            })
            .collect();

        let anchors: rustc_hash::FxHashMap<usize, Point> = placed
            .iter()
            .filter_map(|(index, _, origin)| Some(((*index)?, *origin)))
            .collect();

        let anchor_of = |wanted: usize| anchors.get(&wanted).copied();

        let ground = to_screen(0.0, 0.0);

        if let Some(reach) = self.world.then(|| self.data.bounds()).flatten() {
            let stroke = Stroke::default().with_color(WORLD_COLOR).with_width(WORLD_WIDTH);
            let left = to_screen(reach.min_x, 0.0).x;
            let right = to_screen(reach.max_x, 0.0).x;
            let top = to_screen(0.0, reach.min_y).y;

            frame.stroke(&Path::line(Point::new(left, ground.y), Point::new(right, ground.y)), stroke);

            if top < ground.y {
                frame.stroke(&Path::line(ground, Point::new(ground.x, top)), stroke);
            }
        }

        if self.origin {
            frame.fill(&Path::circle(ground, ORIGIN_MARK_RADIUS), ORIGIN_COLOR_MARK);
        }

        for (index, quad, origin) in &placed {
            let level = self.level(*index);

            if level == 0 {
                continue;
            }

            if level == 1 {
                frame.stroke(&outline(quad), Stroke::default().with_color(PART_COLOR).with_width(PART_WIDTH));
                frame.fill(&Path::circle(*origin, ORIGIN_RADIUS), ORIGIN_COLOR);

                continue;
            }

            if let Some(anchor) = index.and_then(|part| self.parent_of(part)).and_then(anchor_of) {
                frame.stroke(
                    &Path::line(*origin, anchor),
                    Stroke::default().with_color(PARENT_COLOR).with_width(PARENT_WIDTH),
                );
                frame.fill(&Path::circle(anchor, ORIGIN_RADIUS), PARENT_COLOR);
            }

            frame.stroke(&outline(quad), Stroke::default().with_color(PICKED_COLOR).with_width(PICKED_WIDTH));

            let up = Point::new((quad[0].x + quad[2].x) / 2.0, (quad[0].y + quad[2].y) / 2.0);

            if (up.x - origin.x).hypot(up.y - origin.y) >= DEGENERATE {
                frame.stroke(
                    &Path::line(*origin, up),
                    Stroke::default().with_color(AXIS_COLOR).with_width(AXIS_WIDTH),
                );
            }

            frame.fill(&Path::circle(*origin, PICKED_RADIUS), PICKED_ORIGIN_COLOR);
        }

        vec![frame.into_geometry()]
    }
}

pub(super) struct Shot {
    pub(super) overlays: Overlays,
    pub(super) scope: Scope,
    pub(super) picked: Option<usize>,
    pub(super) reach: Option<BoundingBox>,
}

impl Shot {
    fn level(&self, unit: &Rig, part: Option<usize>) -> u8 {
        let mut tier = self.overlays.rig;

        let Some(part) = part else {
            return tier.rank();
        };

        if self.picked == Some(part) {
            tier = tier.max(self.overlays.selected).max(self.overlays.hierarchy);
        } else if self.overlays.hierarchy.on() && self.picked == parent_of(unit, part) {
            tier = tier.max(self.overlays.hierarchy);
        }

        tier.rank()
    }

    fn silent(&self) -> bool {
        !self.overlays.rig.on()
            && !self.overlays.selected.on()
            && !self.overlays.hierarchy.on()
            && !self.overlays.origin.on()
            && !self.overlays.world.on()
    }
}

fn parent_of(unit: &Rig, part: usize) -> Option<usize> {
    usize::try_from(unit.model.parts.get(part)?.parent).ok()
}

struct Sheet<'a> {
    pixels: &'a mut [u8],
    width: i32,
    height: i32,
}

impl Sheet<'_> {
    fn blend(&mut self, x: i32, y: i32, color: Color, coverage: f32) {
        if x < 0 || y < 0 || x >= self.width || y >= self.height || coverage <= 0.0 {
            return;
        }

        let alpha = color.a * coverage.min(1.0);
        let flipped = self.height - 1 - y;
        let at = ((flipped * self.width + x) * 4) as usize;

        let Some(pixel) = self.pixels.get_mut(at..at + 4) else {
            return;
        };

        let over = |src: f32, dst: u8| {
            ((src * alpha + f32::from(dst) / 255.0 * (1.0 - alpha)) * 255.0).round().clamp(0.0, 255.0) as u8
        };

        pixel[0] = over(color.r, pixel[0]);
        pixel[1] = over(color.g, pixel[1]);
        pixel[2] = over(color.b, pixel[2]);
        pixel[3] = over(1.0, pixel[3]);
    }

    fn span(&mut self, from: Point, to: Point, color: Color, thickness: f32) {
        let reach = thickness / 2.0 + 0.5;
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        let run = dx.hypot(dy);

        for y in bracket(from.y.min(to.y) - reach, from.y.max(to.y) + reach, self.height) {
            for x in bracket(from.x.min(to.x) - reach, from.x.max(to.x) + reach, self.width) {
                let (px, py) = (x as f32 + 0.5 - from.x, y as f32 + 0.5 - from.y);

                let along = match run <= f32::EPSILON {
                    true => 0.0,
                    false => ((px * dx + py * dy) / (run * run)).clamp(0.0, 1.0),
                };

                let gap = (px - dx * along).hypot(py - dy * along);

                self.blend(x, y, color, reach - gap);
            }
        }
    }

    fn disc(&mut self, at: Point, radius: f32, color: Color) {
        for y in bracket(at.y - radius - 1.0, at.y + radius + 1.0, self.height) {
            for x in bracket(at.x - radius - 1.0, at.x + radius + 1.0, self.width) {
                let gap = (x as f32 + 0.5 - at.x).hypot(y as f32 + 0.5 - at.y);

                self.blend(x, y, color, radius + 0.5 - gap);
            }
        }
    }

    fn ring(&mut self, quad: &[Point; 4], color: Color, thickness: f32) {
        let [top_left, bottom_left, top_right, bottom_right] = *quad;

        for (from, to) in
            [(top_left, top_right), (top_right, bottom_right), (bottom_right, bottom_left), (bottom_left, top_left)]
        {
            self.span(from, to, color, thickness);
        }
    }
}

fn bracket(from: f32, to: f32, limit: i32) -> std::ops::Range<i32> {
    let first = (from.floor() as i32).max(0);
    let last = (to.ceil() as i32 + 1).min(limit);

    first..last.max(first)
}

pub(super) fn paint(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    unit: &Rig,
    parts: &[(Option<usize>, FrameData)],
    to_screen: impl Fn(f32, f32) -> Point,
    shot: &Shot,
) {
    if shot.silent() || parts.is_empty() {
        return;
    }

    let mut sheet =
        Sheet { pixels, width: width.min(i32::MAX as u32) as i32, height: height.min(i32::MAX as u32) as i32 };

    let placed: Vec<(Option<usize>, [Point; 4], Point)> = parts
        .iter()
        .map(|(index, geometry)| {
            let quad = corners(geometry, &to_screen);
            let origin = index
                .and_then(|part| pivot_of(unit, part, geometry, &quad))
                .unwrap_or_else(|| centroid(&quad));

            (*index, quad, origin)
        })
        .collect();

    let ground = to_screen(0.0, 0.0);

    if let Some(reach) = shot.overlays.world.on().then_some(shot.reach).flatten() {
        let left = to_screen(reach.min_x, 0.0).x;
        let right = to_screen(reach.max_x, 0.0).x;
        let top = to_screen(0.0, reach.min_y).y;

        sheet.span(Point::new(left, ground.y), Point::new(right, ground.y), WORLD_COLOR, WORLD_WIDTH);

        if top < ground.y {
            sheet.span(ground, Point::new(ground.x, top), WORLD_COLOR, WORLD_WIDTH);
        }
    }

    if shot.overlays.origin.on() {
        sheet.disc(ground, ORIGIN_MARK_RADIUS, ORIGIN_COLOR_MARK);
    }

    for (index, quad, origin) in &placed {
        match shot.level(unit, *index) {
            0 => continue,
            1 => {
                sheet.ring(quad, PART_COLOR, PART_WIDTH);
                sheet.disc(*origin, ORIGIN_RADIUS, ORIGIN_COLOR);
            }
            _ => {
                let anchor = index.and_then(|part| parent_of(unit, part)).and_then(|wanted| {
                    placed.iter().find(|(held, _, _)| *held == Some(wanted)).map(|(_, _, origin)| *origin)
                });

                if let Some(anchor) = anchor {
                    sheet.span(*origin, anchor, PARENT_COLOR, PARENT_WIDTH);
                    sheet.disc(anchor, ORIGIN_RADIUS, PARENT_COLOR);
                }

                sheet.ring(quad, PICKED_COLOR, PICKED_WIDTH);

                let up = Point::new((quad[0].x + quad[2].x) / 2.0, (quad[0].y + quad[2].y) / 2.0);

                if (up.x - origin.x).hypot(up.y - origin.y) >= DEGENERATE {
                    sheet.span(*origin, up, AXIS_COLOR, AXIS_WIDTH);
                }

                sheet.disc(*origin, PICKED_RADIUS, PICKED_ORIGIN_COLOR);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kore::domains::settings::Shown;

    fn sheet(pixels: &mut [u8], width: i32, height: i32) -> Sheet<'_> {
        Sheet { pixels, width, height }
    }

    #[test]
    fn the_painter_flips_rows_the_way_the_readback_does() {
        // The export buffer comes back bottom-up, so screen y 0 is the last row.
        // Getting this backwards draws a perfectly good overlay upside down.
        let mut pixels = vec![0u8; 4 * 2 * 3];

        sheet(&mut pixels, 2, 3).blend(0, 0, Color::WHITE, 1.0);

        assert_eq!(pixels[16..20], [255, 255, 255, 255], "screen y 0 belongs on the bottom row");
        assert_eq!(pixels[0..4], [0, 0, 0, 0], "and nothing landed on the top one");
    }

    #[test]
    fn painting_outside_the_frame_is_dropped_rather_than_wrapped() {
        let mut pixels = vec![0u8; 4 * 2 * 2];

        let mut held = sheet(&mut pixels, 2, 2);
        held.blend(-1, 0, Color::WHITE, 1.0);
        held.blend(0, -1, Color::WHITE, 1.0);
        held.blend(2, 0, Color::WHITE, 1.0);
        held.blend(0, 2, Color::WHITE, 1.0);

        assert!(pixels.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn a_disc_reaches_its_radius_and_stops() {
        let mut pixels = vec![0u8; 4 * 9 * 9];

        sheet(&mut pixels, 9, 9).disc(Point::new(4.5, 4.5), 2.0, Color::WHITE);

        let alpha = |x: usize, y: usize| pixels[((8 - y) * 9 + x) * 4 + 3];

        assert_eq!(alpha(4, 4), 255, "the middle is solid");
        assert!(alpha(4, 8) == 0 && alpha(0, 4) == 0, "and it does not bleed to the edges");
    }

    #[test]
    fn a_shot_is_a_snapshot_so_a_mid_export_reselection_cannot_reach_it() {
        // The job runs on its own thread off an owned copy. Every frame of one export
        // has to agree about what was selected, however the viewer moves meanwhile.
        let shot = Shot {
            overlays: Overlays { selected: Tier::Bold, ..Overlays::default() },
            scope: Scope::Selected,
            picked: Some(4),
            reach: None,
        };

        let held = std::thread::spawn(move || (shot.scope, shot.picked));

        assert_eq!(held.join().ok(), Some((Scope::Selected, Some(4))));
    }

    #[test]
    fn every_overlay_off_paints_nothing_at_all() {
        // "Include Debug" opts in, it never enables. With the viewer's overlays
        // all off the export has to come out exactly as it would untoggled.
        let shot =
            Shot { overlays: Overlays::default(), scope: Scope::Rig, picked: None, reach: None };

        assert!(shot.silent());

        let lit = Shot {
            overlays: Overlays { world: Shown::Visible, ..Overlays::default() },
            scope: Scope::Rig,
            picked: None,
            reach: None,
        };

        assert!(!lit.silent(), "one visible overlay is enough to draw");
    }
}
