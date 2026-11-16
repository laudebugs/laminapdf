//! Drawing on a page as annotations (ISO 32000-2 §12.5.6.7–§12.5.6.13): freehand ink, squares
//! and circles, lines and arrows, polygons and polylines, built from points and boxes in page
//! space (ADR 0027), as a viewer has them from a pointer. Each method gives the edit
//! ([`AddAnnot`]) with a 1-point red outline and nothing filled; set the spec's fields (its
//! border, interior colour, endings) or [`AnnotCommon`](laminapdf_ops::AnnotCommon)'s colour and
//! opacity before [`Document::apply`] applies it. Like the points, the widths these methods set
//! are in points and written in the page's user space, so a page with a `/UserUnit` gets them
//! divided by it; widths set on the spec afterwards are in user space.
//!
//! Every mark gets an appearance stream drawn by `laminapdf-build`, which this crate gives every
//! document it opens.

use laminapdf_model::PageFrame;
use laminapdf_model::{PointPt, RectPt};
use laminapdf_ops::{
    AddAnnot, AnnotSpec, Border, Ink, InkStroke, LineEnding, LineMark, Poly, PolyKind, Shape, ShapeKind,
};

use crate::document::Document;
use crate::error::Result;
use crate::links::user_rect;

impl Document {
    /// The edit that draws freehand ink on page `page` (0-based): one stroke per list of points
    /// in page space, drawn smoothed through them by a pen `width` points wide. For a
    /// pressure-sensitive pen, give each stroke of the edit's [`Ink`] spec its widths, one per
    /// point ([`InkStroke::with_widths`]), before applying it.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.5.0.
    pub fn ink_at(&self, page: u32, strokes: &[Vec<PointPt>], width: f64) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let strokes = strokes
            .iter()
            .map(|points| InkStroke::new(points.iter().map(|p| frame.to_user_space(*p)).collect()))
            .collect();
        Ok(AddAnnot::new(
            id,
            AnnotSpec::Ink(Ink::new(strokes).with_width(width / frame.user_unit())),
        ))
    }

    /// The edit that draws a square or circle on page `page` (0-based) along `area`, a box in
    /// page space: a rectangle, or the ellipse inside the box.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.5.0.
    pub fn shape_at(&self, page: u32, kind: ShapeKind, area: RectPt) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        Ok(AddAnnot::new(
            id,
            AnnotSpec::Shape(Shape::new(kind, user_rect(&frame, &[area])).with_border(outline(&frame))),
        ))
    }

    /// The edit that draws a line on page `page` (0-based) from `from` to `to` in page space,
    /// with `ends` at its start and its end: an arrow is
    /// `[LineEnding::None, LineEnding::ClosedArrow]`.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree).
    ///
    /// Added in 0.5.0.
    pub fn line_at(&self, page: u32, from: PointPt, to: PointPt, ends: [LineEnding; 2]) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let [start, end] = ends;
        let line = LineMark::new(frame.to_user_space(from), frame.to_user_space(to))
            .with_ends(start, end)
            .with_border(outline(&frame));
        Ok(AddAnnot::new(id, AnnotSpec::Line(line)))
    }

    /// The edit that draws a polygon (closed) or polyline (open) on page `page` (0-based)
    /// through `vertices` in page space: at least three for a polygon, two for a polyline.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors (the page tree). Too few vertices fail when
    /// the edit is applied (`invalid-op`).
    ///
    /// Added in 0.5.0.
    pub fn poly_at(&self, page: u32, kind: PolyKind, vertices: &[PointPt]) -> Result<AddAnnot> {
        let (id, frame) = self.page_frame(page)?;
        let vertices = vertices.iter().map(|p| frame.to_user_space(*p)).collect();
        Ok(AddAnnot::new(
            id,
            AnnotSpec::Poly(Poly::new(kind, vertices).with_border(outline(&frame))),
        ))
    }
}

/// A solid 1-point outline in the user space of the page `frame` describes.
fn outline(frame: &PageFrame) -> Border {
    Border::solid(1.0 / frame.user_unit())
}
