use iced::mouse;
use iced::widget::canvas::{self, Geometry, Path, Stroke};
use iced::widget::canvas as canvas_widget;
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Theme, Vector};

use nyanko::graphics::animate::{resolve_frame, FrameData};
use nyanko::graphics::rig::Rig;
use nyanko::graphics::tools::part;

use kore::domains::settings::{Overlays, Tier};

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

        let anchor_of = |wanted: usize| {
            placed.iter().find(|(index, _, _)| *index == Some(wanted)).map(|(_, _, origin)| *origin)
        };

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
