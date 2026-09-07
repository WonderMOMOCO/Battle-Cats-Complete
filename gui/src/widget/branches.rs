use iced::advanced::{layout, mouse, renderer, widget, Layout, Renderer as _, Widget};
use iced::widget::text;
use iced::{Color, Element, Length, Rectangle, Size, Theme};

use crate::common::fonts;

const LINE: f32 = 1.0;
const GUIDE_ALPHA: f32 = 0.55;
const OPEN_MARK_ALPHA: f32 = 0.7;
const LEVELS: u16 = 64;

fn guide_color(theme: &Theme) -> Color {
    Color { a: GUIDE_ALPHA, ..theme.palette().text }
}

pub(crate) fn open_mark(theme: &Theme) -> text::Style {
    text::Style { color: Some(Color { a: OPEN_MARK_ALPHA, ..theme.palette().text }) }
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Guide {
    pub(crate) trunks: u64,
    pub(crate) last: bool,
    pub(crate) first: bool,
}

#[derive(Default)]
pub(crate) struct Tracer {
    open: u64,
}

impl Tracer {
    pub(crate) fn back(&mut self, depth: u16, above: Option<u16>) -> Guide {
        let bit = 1u64 << depth.min(LEVELS - 1);

        let guide = Guide {
            trunks: self.open >> 1 & (bit - 1),
            last: self.open & bit == 0,
            first: above.is_some_and(|above| above + 1 == depth),
        };

        self.open = self.open & (bit - 1) | bit;

        guide
    }
}

fn fall(marker: f32) -> f32 {
    (fonts::TRIANGLE_REACH + fonts::TRIANGLE_GAP) * marker - LINE
}

fn snap(centre: f32) -> f32 {
    (centre - LINE * 0.5).round()
}

pub(crate) fn branches(guide: Guide, depth: u16, indent: f32, height: f32, marker: f32) -> Branches {
    Branches { guide, depth, indent, height, marker, reach: 0.0, opened: false }
}

pub(crate) struct Branches {
    guide: Guide,
    depth: u16,
    indent: f32,
    height: f32,
    marker: f32,
    reach: f32,
    opened: bool,
}

impl Branches {
    pub(crate) fn reach(mut self, reach: f32) -> Self {
        self.reach = reach;
        self
    }

    pub(crate) fn opened(mut self) -> Self {
        self.opened = true;
        self
    }

    fn rise(&self) -> f32 {
        if !self.guide.first {
            return 0.0;
        }

        (self.height * 0.5 - fall(self.marker)).max(0.0)
    }

    fn tip(&self) -> f32 {
        if self.reach > 0.0 {
            return self.reach - fonts::TRIANGLE_GAP * self.marker;
        }

        let edge = match self.opened {
            true => fonts::TRIANGLE_LEAD + fonts::TRIANGLE_REACH * 0.5,
            false => fonts::TRIANGLE_LEAD,
        };

        (edge - fonts::TRIANGLE_GAP) * self.marker
    }

    fn width(&self) -> f32 {
        self.indent * f32::from(self.depth) + self.reach
    }

    fn columns(&self) -> f32 {
        self.indent * f32::from(self.depth)
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Branches {
    fn size(&self) -> Size<Length> {
        Size { width: Length::Fixed(self.width()), height: Length::Fixed(self.height) }
    }

    fn layout(&mut self, _tree: &mut widget::Tree, _renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fixed(self.width()), Length::Fixed(self.height), Size::ZERO))
    }

    fn draw(
        &self,
        _tree: &widget::Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let color = guide_color(theme);
        let middle = snap(bounds.y + self.height * 0.5);
        let ink = fonts::TRIANGLE_MIDDLE * self.marker;
        let stem = |level: u16| snap(bounds.x + self.indent * f32::from(level) + ink);

        let mut paint = |bounds: Rectangle| {
            renderer.fill_quad(renderer::Quad { bounds, ..renderer::Quad::default() }, color);
        };

        if self.depth == 0 {
            return;
        }

        for level in 0..(self.depth - 1).min(LEVELS) {
            if self.guide.trunks & 1 << level == 0 {
                continue;
            }

            paint(Rectangle { x: stem(level), y: bounds.y, width: LINE, height: self.height });
        }

        let elbow = stem(self.depth - 1);
        let head = bounds.y - self.rise();
        let foot = if self.guide.last { middle + LINE } else { bounds.y + self.height };
        let close = bounds.x + self.columns() + self.tip();

        paint(Rectangle { x: elbow, y: head, width: LINE, height: foot - head });
        paint(Rectangle { x: elbow, y: middle, width: close - elbow, height: LINE });
    }
}

impl<'a, Message: 'a> From<Branches> for Element<'a, Message> {
    fn from(widget: Branches) -> Self {
        Element::new(widget)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(depths: &[u16]) -> Vec<Guide> {
        let mut tracer = Tracer::default();
        let mut traced: Vec<Guide> = (0..depths.len())
            .rev()
            .map(|index| tracer.back(depths[index], index.checked_sub(1).map(|prev| depths[prev])))
            .collect();

        traced.reverse();
        traced
    }

    // a
    // |- b
    // |  |- c
    // |  \- d
    // \- e
    // f
    #[test]
    fn a_walk_leaves_a_trunk_for_every_ancestor_still_going() {
        let read = trace(&[0, 1, 2, 2, 1, 0]);
        let trunks: Vec<bool> = read.iter().map(|guide| guide.trunks & 1 != 0).collect();
        let last: Vec<bool> = read.iter().map(|guide| guide.last).collect();
        let first: Vec<bool> = read.iter().map(|guide| guide.first).collect();

        assert_eq!(last, vec![false, false, false, true, true, true]);
        assert_eq!(trunks[2..4], [true, true], "b carries on past c and d, down to e");
        assert_eq!(first[1..], [true, true, false, false, false], "only b and c open a nest");
    }

    // A trunk belongs to the column its ancestor's own elbow sits in, one level
    // shallower than the ancestor's children. Reading it off by one broke the line
    // under any expanded folder that had an expanded sibling below it.
    //
    // A
    // |- B
    // |  \- b
    // \- C
    //    \- c
    #[test]
    fn a_sibling_below_keeps_the_trunk_running_past_a_nested_child() {
        let read = trace(&[0, 1, 2, 1, 2]);

        assert!(read[2].trunks & 1 != 0, "B still has C below it, so b keeps a trunk at column 0");
        assert!(read[4].trunks & 1 == 0, "C is the last child, so c has nothing to carry");
    }

    // The elbow has to meet the arrow at its middle, not below it. A 21px row centres
    // on 10.5, and rounding that outright put the line a whole pixel low against a
    // 7px triangle — the arrow looked like it was being pointed at from underneath.
    #[test]
    fn the_line_lands_on_the_centre_it_is_given() {
        for (centre, size) in [(10.5, 18.0), (12.0, 22.0)] {
            let drawn = snap(centre) + LINE * 0.5;

            assert!((drawn - centre).abs() <= LINE * 0.5, "{size}px marker: line at {drawn} for centre {centre}");
        }

        assert_eq!(snap(10.5), 10.0, "an odd row has a pixel dead on its centre");
    }

    // The open arrow tapers to a point, and because its ink box is square it is exactly
    // d pixels wide d pixels above its apex. So the tip carries less ink than the line
    // pointing at it until it is LINE wide; measuring the gap from there rather than from
    // the bare apex is what makes the riser read like the horizontal, which meets a solid
    // full-height edge instead of a taper.
    #[test]
    fn the_riser_measures_from_where_the_arrow_still_has_ink() {
        for marker in [20.571_f32, 18.0] {
            let apex = fonts::TRIANGLE_REACH * marker;
            let gap = fonts::TRIANGLE_GAP * marker;

            assert!((fall(marker) - (apex - LINE + gap)).abs() < 0.001, "{marker}px marker");
        }
    }

    #[test]
    fn a_closed_subtree_stops_carrying_its_own_trunks() {
        let mut tracer = Tracer::default();

        tracer.back(3, None);
        tracer.back(1, None);

        assert_eq!(tracer.back(2, None).trunks, 0b1);
    }
}
