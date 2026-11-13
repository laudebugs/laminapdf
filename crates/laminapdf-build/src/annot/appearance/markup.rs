//! Text markup (ISO 32000-2 §12.5.6.10): a highlight fills each quad and multiplies with the text
//! under it, so the text stays legible; underline, strike-out and squiggly lines follow each
//! quad's baseline, so they lean with rotated text.

use laminapdf_model::UserSpacePoint;
#[cfg(test)]
use laminapdf_model::UserSpaceRect;
use laminapdf_ops::MarkupKind;

use super::{DeviceColor, Drawn, Paint, bounds};
use crate::surface::n;

/// Acrobat's default colour for `kind`, used when the op gives none.
const fn default_color(kind: MarkupKind) -> DeviceColor {
    match kind {
        MarkupKind::Highlight => DeviceColor::Rgb(1.0, 1.0, 0.0),
        MarkupKind::Underline => DeviceColor::Rgb(0.0, 0.6, 0.0),
        MarkupKind::StrikeOut => DeviceColor::Rgb(1.0, 0.0, 0.0),
        MarkupKind::Squiggly => DeviceColor::Rgb(0.0, 0.4, 1.0),
    }
}

/// One quad as Acrobat orders it: upper left, upper right, lower left, lower right.
type Quad = [UserSpacePoint; 4];

/// The appearance of a text-markup annotation over `quads`; `None` when there are none.
pub(crate) fn markup(kind: MarkupKind, quads: &[Quad], paint: &Paint) -> Option<Drawn> {
    let bbox = bounds(quads.iter().flatten().copied(), 0.0)?;
    let color = paint.color_or(default_color(kind));
    let mut drawn = Drawn::new(bbox, paint.opacity, kind == MarkupKind::Highlight);
    let c = &mut drawn.content;
    match kind {
        MarkupKind::Highlight => {
            color.fill(c);
            for [ul, ur, ll, lr] in quads {
                c.move_to(n(ll.x), n(ll.y));
                c.line_to(n(lr.x), n(lr.y));
                c.line_to(n(ur.x), n(ur.y));
                c.line_to(n(ul.x), n(ul.y));
                c.close_path();
            }
            c.fill_nonzero();
        }
        MarkupKind::Underline | MarkupKind::StrikeOut | MarkupKind::Squiggly => {
            color.stroke(c);
            for quad in quads {
                line(c, kind, &Frame::of(quad));
            }
        }
    }
    Some(drawn)
}

/// A quad as a baseline: its lower-left corner, its length along the text and its height,
/// with unit vectors along and up.
struct Frame {
    origin: UserSpacePoint,
    along: (f64, f64),
    up: (f64, f64),
    length: f64,
    height: f64,
}

impl Frame {
    fn of([ul, _, ll, lr]: &Quad) -> Self {
        let unit = |dx: f64, dy: f64| {
            let len = dx.hypot(dy);
            if len > 0.0 {
                ((dx / len, dy / len), len)
            } else {
                ((0.0, 0.0), 0.0)
            }
        };
        let (along, length) = unit(lr.x - ll.x, lr.y - ll.y);
        let (up, height) = unit(ul.x - ll.x, ul.y - ll.y);
        Self {
            origin: *ll,
            along,
            up,
            length,
            height,
        }
    }

    /// The point `s` along the baseline and `t` above it.
    fn at(&self, s: f64, t: f64) -> (f32, f32) {
        (
            n(self.origin.x + self.along.0 * s + self.up.0 * t),
            n(self.origin.y + self.along.1 * s + self.up.1 * t),
        )
    }
}

/// Underline, strike-out or squiggly along one quad, its width a fourteenth of the text's
/// height (at least half a point), kept inside the quad.
fn line(content: &mut pdf_writer::Content, kind: MarkupKind, frame: &Frame) {
    let (c, f) = (content, frame);
    let width = (f.height / 14.0).max(0.5);
    c.set_line_width(n(width));
    match kind {
        MarkupKind::Underline | MarkupKind::StrikeOut => {
            let above = if kind == MarkupKind::Underline {
                width / 2.0
            } else {
                f.height / 2.0
            };
            let (x0, y0) = f.at(0.0, above);
            let (x1, y1) = f.at(f.length, above);
            c.move_to(x0, y0);
            c.line_to(x1, y1);
        }
        MarkupKind::Squiggly | MarkupKind::Highlight => {
            // A zigzag in the quad's lowest sixth, one wave per third of the text's height.
            let wave = (f.height / 3.0).max(1.0);
            let (low, high) = (width / 2.0, f.height / 6.0);
            let (x0, y0) = f.at(0.0, low);
            c.move_to(x0, y0);
            let mut along = 0.0;
            let mut up = true;
            while along < f.length {
                along = (along + wave / 2.0).min(f.length);
                let (x1, y1) = f.at(along, if up { high } else { low });
                c.line_to(x1, y1);
                up = !up;
            }
        }
    }
    c.stroke();
}

/// The `/Rect` a markup over `quads` gets: their bounds.
#[cfg(test)]
fn rect_of(quads: &[Quad]) -> Option<UserSpaceRect> {
    bounds(quads.iter().flatten().copied(), 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> UserSpacePoint {
        UserSpacePoint::new(x, y)
    }

    fn word() -> Quad {
        [p(10.0, 114.0), p(60.0, 114.0), p(10.0, 100.0), p(60.0, 100.0)]
    }

    const PAINT: Paint = Paint {
        color: None,
        opacity: 1.0,
        intent: None,
    };

    fn text(kind: MarkupKind, quads: &[Quad]) -> String {
        String::from_utf8(markup(kind, quads, &PAINT).unwrap().content()).unwrap()
    }

    #[test]
    fn a_highlight_fills_each_quad_in_yellow() {
        assert_eq!(
            text(MarkupKind::Highlight, &[word()]),
            "/GS0 gs\n1 1 0 rg\n10 100 m\n60 100 l\n60 114 l\n10 114 l\nh\nf"
        );
    }

    #[test]
    fn an_underline_sits_on_the_baseline_inside_the_quad() {
        assert_eq!(
            text(MarkupKind::Underline, &[word()]),
            "0 0.6 0 RG\n1 w\n10 100.5 m\n60 100.5 l\nS"
        );
    }

    #[test]
    fn a_strike_out_crosses_the_middle_of_the_quad() {
        assert!(text(MarkupKind::StrikeOut, &[word()]).contains("10 107 m\n60 107 l"));
    }

    #[test]
    fn a_strike_out_follows_text_turned_a_quarter() {
        // Text running up the page: the baseline is the quad's right edge.
        let turned = [p(100.0, 10.0), p(100.0, 60.0), p(114.0, 10.0), p(114.0, 60.0)];
        assert!(text(MarkupKind::StrikeOut, &[turned]).contains("107 10 m\n107 60 l"));
    }

    #[test]
    fn a_squiggly_line_ends_at_the_end_of_the_text() {
        assert!(text(MarkupKind::Squiggly, &[word()]).ends_with(" l\nS"));
        assert!(text(MarkupKind::Squiggly, &[word()]).contains("\n60 "));
    }

    #[test]
    fn the_rect_bounds_every_quad() {
        let second = [p(10.0, 94.0), p(30.0, 94.0), p(10.0, 80.0), p(30.0, 80.0)];
        assert_eq!(
            rect_of(&[word(), second]),
            Some(UserSpaceRect::from_corners(10.0, 80.0, 60.0, 114.0))
        );
    }

    #[test]
    fn no_quads_draw_nothing() {
        assert!(markup(MarkupKind::Highlight, &[], &PAINT).is_none());
    }

    #[test]
    fn a_translucent_underline_sets_its_opacity_once() {
        let paint = Paint { opacity: 0.5, ..PAINT };
        let drawn = markup(MarkupKind::Underline, &[word()], &paint).unwrap();
        assert_eq!(
            drawn
                .gs
                .as_ref()
                .and_then(|gs| gs.get("CA"))
                .and_then(laminapdf_cos::Object::as_f64),
            Some(0.5)
        );
    }
}
