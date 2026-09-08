use iced::advanced::text::{self, Paragraph as _, Renderer as _};
use iced::advanced::widget::{self, Tree};
use iced::advanced::{layout, mouse, renderer, Layout, Widget};
use iced::alignment::Vertical;
use iced::{Element, Length, Pixels, Rectangle, Renderer as IcedRenderer, Size, Theme};

use crate::common::glyphs;

const MAX_FONT_SIZE: f32 = 22.0;
const MIN_FONT_SIZE: f32 = 8.0;
const MAX_LINES: f32 = 2.0;
const UNBROKEN_LINES: f32 = 1.0;
const TRUNCATE_LINES: f32 = 5.0;
const ELLIPSIS: &str = "...";

type RendererParagraph = <IcedRenderer as text::Renderer>::Paragraph;

#[derive(Default)]
struct Fitted {
    key: Option<(String, f32)>,
    paragraph: RendererParagraph,
}

fn allowance(content: &str) -> f32 {
    match content.split_whitespace().nth(1) {
        Some(_) => MAX_LINES,
        None => UNBROKEN_LINES,
    }
}

fn lines(paragraph: &RendererParagraph, size: f32) -> f32 {
    let line_height = text::LineHeight::default().to_absolute(Pixels(size)).0;

    (paragraph.min_bounds().height / line_height).round()
}

fn middle(glyphs: &[char], keep: usize) -> String {
    if keep >= glyphs.len() {
        return glyphs.iter().collect();
    }

    let head = keep.div_ceil(2);
    let tail = keep - head;

    glyphs[..head]
        .iter()
        .copied()
        .chain(ELLIPSIS.chars())
        .chain(glyphs[glyphs.len() - tail..].iter().copied())
        .collect()
}

fn clipped(
    content: &str,
    size: f32,
    shape: &impl Fn(&str, f32) -> RendererParagraph,
) -> RendererParagraph {
    let glyphs: Vec<char> = content.chars().collect();
    let mut least = 0;
    let mut most = glyphs.len();

    while least < most {
        let keep = least.midpoint(most + 1);
        let candidate = shape(&middle(&glyphs, keep), size);

        match lines(&candidate, size) < TRUNCATE_LINES {
            true => least = keep,
            false => most = keep - 1,
        }
    }

    shape(&middle(&glyphs, least), size)
}

pub fn name_box<'a, Message: 'a>(name_text: impl Into<String>, width: f32, height: f32, wrap_width: f32) -> Element<'a, Message> {
    Element::new(NameBox { content: name_text.into(), width, height, wrap_width })
}

struct NameBox {
    content: String,
    width: f32,
    height: f32,
    wrap_width: f32,
}

impl<Message> Widget<Message, Theme, IcedRenderer> for NameBox {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<Fitted>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(Fitted::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.width), Length::Fixed(self.height))
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &IcedRenderer, limits: &layout::Limits) -> layout::Node {
        layout::sized(limits, Length::Fixed(self.width), Length::Fixed(self.height), |limits| {
            let bounds = limits.max();
            let state = tree.state.downcast_mut::<Fitted>();

            let fresh = state
                .key
                .as_ref()
                .is_some_and(|(held, wrapped)| held == &self.content && *wrapped == self.wrap_width);

            if fresh {
                return bounds;
            }

            let font = renderer.default_font();
            let shaping = glyphs::shaping(&self.content);
            let shape = |content: &str, size: f32| {
                RendererParagraph::with_text(text::Text {
                    content,
                    bounds: Size::new(self.wrap_width, f32::INFINITY),
                    size: Pixels(size),
                    line_height: text::LineHeight::default(),
                    font,
                    align_x: text::Alignment::Left,
                    align_y: Vertical::Center,
                    shaping,
                    wrapping: text::Wrapping::WordOrGlyph,
                })
            };

            let allowed = allowance(&self.content);

            let mut size = MAX_FONT_SIZE;
            let paragraph = loop {
                let candidate = shape(self.content.as_str(), size);
                let measured = candidate.min_bounds();
                let fits = lines(&candidate, size) <= allowed && measured.width <= self.wrap_width;

                if fits || size <= MIN_FONT_SIZE {
                    break candidate;
                }

                size -= 0.5;
            };

            let paragraph = match lines(&paragraph, size) < TRUNCATE_LINES {
                true => paragraph,
                false => clipped(&self.content, size, &shape),
            };

            state.paragraph = paragraph;
            state.key = Some((self.content.clone(), self.wrap_width));

            bounds
        })
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut IcedRenderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let fitted = tree.state.downcast_ref::<Fitted>();

        widget::text::draw(
            renderer,
            style,
            layout.bounds(),
            &fitted.paragraph,
            widget::text::Style { color: None },
            viewport,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{RendererParagraph, MAX_FONT_SIZE, MIN_FONT_SIZE};
    use crate::common::glyphs;
    use iced::advanced::text::{self, Paragraph as _};
    use iced::alignment::Vertical;
    use iced::widget::text::{Shaping, Wrapping};
    use iced::{Font, Pixels, Size};

    fn measured(content: &str, size: f32, wrap: f32, wrapping: text::Wrapping) -> Size {
        RendererParagraph::with_text(text::Text {
            content,
            bounds: Size::new(wrap, f32::INFINITY),
            size: Pixels(size),
            line_height: text::LineHeight::default(),
            font: Font::DEFAULT,
            align_x: text::Alignment::Left,
            align_y: Vertical::Center,
            shaping: glyphs::shaping(content),
            wrapping,
        })
        .min_bounds()
    }

    // The shrink-to-fit loop re-shapes the name at every candidate size, and it used to
    // ask for Advanced every time. A middle dot in a unit's name is enough to make that
    // the uncached font-database walk, paid this many times over, on every layout.
    #[test]
    fn a_decorative_dot_in_a_name_does_not_earn_advanced_shaping() {
        assert_eq!(glyphs::shaping("Kitaro Cat \u{00b7} Nezumi-Otoko Cat"), Shaping::Basic);
        assert_eq!(glyphs::shaping("\u{9b3c}\u{592a}\u{90ce}\u{30cd}\u{30b3}"), Shaping::Auto);
    }

    // A name with no break in it stayed on one line at full size, so the line-count
    // check called it a fit and it drew straight out of the box.
    #[test]
    fn an_unbreakable_name_is_wrapped_and_shrunk_rather_than_overflowing() {
        let wrap = 145.0;
        let name = "orbital anihhilator ragnarokssssssssssssssssss";

        let width = |wrapping| measured(name, MAX_FONT_SIZE, wrap, wrapping).width;

        assert!(
            width(Wrapping::Word) > wrap,
            "the old wrapping left the long token whole, which is what escaped the box",
        );
        assert!(width(Wrapping::WordOrGlyph) <= wrap, "glyph fallback keeps it inside the wrap width");
    }

    // Middle rather than end, because a unit's tail carries the form it belongs to,
    // and ASCII dots rather than one ellipsis glyph, which would cost Advanced shaping.
    #[test]
    fn a_name_too_long_to_shrink_keeps_both_ends() {
        let glyphs: Vec<char> = "Orbital Annihilator Ragnarok".chars().collect();

        let clipped = super::middle(&glyphs, 12);

        assert_eq!(clipped, "Orbita...gnarok");
        assert_eq!(glyphs::shaping(&clipped), Shaping::Basic, "the marker must stay cheap to shape");
    }

    #[test]
    fn a_name_that_already_fits_is_never_marked() {
        let glyphs: Vec<char> = "Cat Army".chars().collect();

        assert_eq!(super::middle(&glyphs, glyphs.len()), "Cat Army");
        assert_eq!(super::middle(&glyphs, 99), "Cat Army");
    }

    // A name with nothing to break on reads worse split mid-word than shrunk, so it is
    // held to one line and only falls back to a glyph break once it cannot shrink further.
    #[test]
    fn a_name_with_no_break_in_it_shrinks_instead_of_wrapping() {
        assert_eq!(super::allowance("ragnarokssssssssssssssssss"), super::UNBROKEN_LINES);
        assert_eq!(super::allowance("Orbital Annihilator"), super::MAX_LINES);
        assert_eq!(super::allowance(""), super::UNBROKEN_LINES);
    }

    #[test]
    fn the_fitting_loop_is_worth_caching() {
        let steps = ((MAX_FONT_SIZE - MIN_FONT_SIZE) / 0.5) as u32;

        assert!(steps > 20, "the fit costs {steps} shapes, so it must not run every layout");
    }
}
