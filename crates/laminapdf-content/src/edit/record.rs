//! Recording what each object draws while the interpreter reads the content (ADR 0022 §1).

use laminapdf_model::{Matrix, PageFrame, RectPt, UserSpaceRect};

use super::{ContentObjectKind, ContentOps};
use crate::lexer::Op;
use crate::state::{GraphicsState, TextAdvance, transformed_box};
use crate::visitor::{FormXObject, ImageDraw, Paint, PathPaint, ShadingPaint, TextShow, Visitor};

/// How far above and below the baseline a run of glyphs reaches, as fractions of the font size:
/// typical ascent and descent, since the interpreter doesn't read font metrics.
const ASCENT: f64 = 0.8;
const DESCENT: f64 = -0.2;

/// A [`Visitor`] that records, for each object of a [`ContentOps`], the graphics state before
/// its first operator, what a `Do` draws (an image or a form), and its box in page space: the
/// union of what its paths, images, shadings, glyphs or form paint.
///
/// Pass it to the interpreter over the same content the IR was read from, before any edit:
/// [`interpret_page`](crate::interpret_page) for a page read with
/// [`ContentOps::read_page`]. Once an edit has changed the IR, the recorder records nothing:
/// read the content again for states and boxes. A text show's box comes from the advance the visitor that
/// measures the glyphs returned ([`Visitor::shown_text`]); interpreted on its own, the
/// recorder measures none, so text objects get no width. `laminapdf-text`'s extractor
/// measures them while it passes the page's events on.
///
/// Added in 0.5.0.
#[derive(Debug)]
pub struct Recorder<'a> {
    ops: &'a mut ContentOps,
    frame: PageFrame,
    /// The object the operator running at depth 0 belongs to.
    current: Option<usize>,
    /// Form XObject nesting.
    depth: usize,
}

impl<'a> Recorder<'a> {
    pub(super) const fn new(ops: &'a mut ContentOps, frame: PageFrame) -> Self {
        Self {
            ops,
            frame,
            current: None,
            depth: 0,
        }
    }

    /// Adds `rect`, in the page's default user space, to the current object's box.
    fn cover(&mut self, rect: &UserSpaceRect) {
        let page = self.frame.rect_to_page(rect);
        if !page.is_finite() {
            return;
        }
        if let Some(object) = self.current.and_then(|i| self.ops.objects.get_mut(i)) {
            object.bounds = Some(object.bounds.map_or(page, |b: RectPt| b.union(&page)));
        }
    }

    fn set_kind(&mut self, kind: ContentObjectKind) {
        if let Some(object) = self.current.and_then(|i| self.ops.objects.get_mut(i))
            && object.kind == ContentObjectKind::XObject
        {
            object.kind = kind;
        }
    }
}

impl Visitor for Recorder<'_> {
    fn operator_state(&mut self, op: &Op<'_>, state: &GraphicsState, depth: usize) {
        // After an edit the operators no longer match the content the interpreter reads.
        if depth > 0 || self.ops.edited {
            return;
        }
        // The operators are in offset order; a written one has no span and isn't looked for.
        let Ok(index) = self
            .ops
            .ops
            .binary_search_by_key(&op.offset, |o| o.span.as_ref().map_or(usize::MAX, keyword_offset(o)))
        else {
            self.current = None;
            return;
        };
        let object = self
            .ops
            .objects
            .partition_point(|o| o.ops.start <= index)
            .checked_sub(1)
            .filter(|&i| self.ops.objects.get(i).is_some_and(|o| o.ops.contains(&index)));
        self.current = object;
        if let Some(object) = object.and_then(|i| self.ops.objects.get_mut(i)) {
            if object.ops.start == index {
                object.state = Some(Box::new(state.clone()));
            }
            if object.ops.end.checked_sub(1) == Some(index) {
                object.end_state = Some(Box::new(state.clone()));
            }
        }
    }

    fn shown_text(&mut self, show: &TextShow<'_>, advance: TextAdvance) {
        let text = &show.state.text;
        let size = text.font_size;
        let run = if advance.ty != 0.0 && advance.tx == 0.0 {
            // Vertical writing: the glyphs hang below the pen, centred on it.
            UserSpaceRect::from_corners(-size / 2.0, advance.ty, size / 2.0, 0.0)
        } else {
            UserSpaceRect::from_corners(0.0, text.rise + DESCENT * size, advance.tx, text.rise + ASCENT * size)
        };
        let to_user = show.tm.then(&show.state.ctm);
        self.cover(&transformed_box(&run, &to_user));
    }

    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        if let Some(bounds) = paint.bounds {
            let half = paint.state.line_width.abs() / 2.0 * scale(&paint.state.ctm);
            // A stroke reaches half the line width past the path.
            let grown = if matches!(paint.paint, Paint::Stroke { .. } | Paint::FillStroke { .. }) {
                UserSpaceRect::from_corners(bounds.x0 - half, bounds.y0 - half, bounds.x1 + half, bounds.y1 + half)
            } else {
                bounds
            };
            self.cover(&grown);
        }
    }

    fn image(&mut self, image: &ImageDraw<'_>) {
        self.set_kind(ContentObjectKind::Image);
        self.cover(&transformed_box(
            &UserSpaceRect::from_corners(0.0, 0.0, 1.0, 1.0),
            &image.state.ctm,
        ));
    }

    fn paint_shading(&mut self, shading: &ShadingPaint<'_>) {
        // `sh` fills the clip; with no clip, the page.
        let area = shading.state.clip_box.unwrap_or_else(|| self.frame.crop_box());
        self.cover(&area);
    }

    fn enter_xobject(&mut self, _form: &FormXObject<'_>) {
        if self.depth == 0 {
            self.set_kind(ContentObjectKind::Form);
        }
        self.depth += 1;
    }

    fn leave_xobject(&mut self, _form: &FormXObject<'_>) {
        self.depth = self.depth.saturating_sub(1);
    }
}

/// The keyword's offset of a source operator with this span: its span ends at the keyword's
/// end, or at `EI` for an inline image, which starts with `BI`.
fn keyword_offset(op: &super::ContentOp) -> impl Fn(&std::ops::Range<usize>) -> usize + '_ {
    move |span| match &op.inline_image {
        Some(image) => span.end - image.len(),
        None => span.end - op.keyword.len(),
    }
}

/// How much `m` scales lengths, roughly: the square root of its determinant's size.
fn scale(m: &Matrix) -> f64 {
    (m.a * m.d - m.b * m.c).abs().sqrt()
}
