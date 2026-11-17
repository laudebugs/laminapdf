//! Drawing a figure: its image or page, and the numbered marks over it, inside its `Figure`
//! element when the flow is tagged.

use laminapdf_model::{FillRule, PointPt, Pt, RectPt};

use super::emit::draw_line;
use super::measure::Source;
use super::model::{Figure, FigureMark, Graphic, ParagraphStyle};
use crate::error::Result;
use crate::surface::Surface;
use crate::surface::color::Color;
use crate::surface::style::StrokeStyle;
use crate::tag::{Tag, TagKind};

/// How far a mark's circle reaches past its label on each side, in ems.
const MARK_PADDING: f64 = 0.35;
/// The gray and width, in points, of the line around a drawn page.
const PAGE_EDGE_GRAY: f64 = 0.6;
const PAGE_EDGE_WIDTH: f64 = 0.5;

/// Draws `figure` over `rect`: a `Figure` element with its alternate text when `tagged`.
pub(super) fn draw_figure(s: &mut Surface, tagged: bool, figure: &Figure, rect: RectPt) -> Result<()> {
    let draw = |s: &mut Surface| -> Result<()> {
        let page = match &figure.graphic {
            Graphic::Image(image) if figure.marks.is_empty() => {
                s.image(image, rect);
                return Ok(());
            }
            Graphic::Image(image) => {
                s.save().image(image, rect);
                None
            }
            Graphic::Page(form) => Some(form),
        };
        // The marks' and the edge's colours stay inside the figure: the flow's text after it
        // sets none of its own.
        if let Some(form) = page {
            // A page's edge, which its own content may not draw.
            s.save().page_form(form, rect).stroke_color(Color::gray(PAGE_EDGE_GRAY));
            s.path(|p| {
                p.rect(rect);
            })
            .stroke(&StrokeStyle {
                width: Pt::new(PAGE_EDGE_WIDTH),
                ..StrokeStyle::default()
            });
        }
        let scale = rect.width().get() / figure.graphic.natural_width().max(f64::MIN_POSITIVE);
        let marked = figure.marks.iter().try_for_each(|mark| {
            let at = PointPt::from_points(
                rect.x0.get() + mark.at.x.get() * scale,
                rect.y0.get() + mark.at.y.get() * scale,
            );
            draw_mark(s, mark, at)
        });
        s.restore();
        marked
    };
    if tagged {
        s.tag(Tag::new(TagKind::Figure).with_alt(figure.alt.as_str()), draw)
    } else {
        draw(s)
    }
}

/// `mark`'s label, white, in a circle of its style's colour centred on `at`.
fn draw_mark(s: &mut Surface, mark: &FigureMark, at: PointPt) -> Result<()> {
    let style = mark.style.clone().with_color(Color::gray(1.0));
    let source = Source::single(&mark.label, &style, ParagraphStyle::new().with_space(0.0, 0.0));
    let Some(line) = source.lines(0, f64::from(u16::MAX))?.into_iter().next() else {
        return Ok(());
    };
    let size = mark.style.size;
    let radius = (line.line.width / 2.0 + size * MARK_PADDING).max(size * (0.5 + MARK_PADDING));
    s.fill_color(mark.style.color.clone().unwrap_or(Color::gray(0.0)));
    s.path(|p| {
        p.circle(at, Pt::new(radius));
    })
    .fill(FillRule::NonZero);
    // The line's box centred on the point: its baseline below the centre by half the
    // label's height above the baseline, less its depth.
    let x = at.x.get() - line.line.x - line.line.width / 2.0;
    let top = at.y.get() - line.baseline + size * 0.35;
    // Inside the figure's element: no element of its own, and the figure's text stands for it.
    let _strikes = draw_line(s, false, false, &source, &line, x, top);
    Ok(())
}
