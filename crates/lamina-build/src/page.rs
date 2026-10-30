//! Pages (ISO 32000-2 §7.7.3.3): size, boundary boxes (§14.11.2), rotation and `/UserUnit`.

use lamina_cos::{Dict, Name, Object, Stream};
use lamina_model::{PageFrame, PageId, Pt, RectPt, Rotation, UserSpaceRect};

use crate::builder::Target;
use crate::error::{Error, Result};
use crate::resources::PageResources;
use crate::surface::Surface;

/// A page size in points (physical: `/UserUnit` is applied when the page is written).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageSize {
    /// Width (before `/Rotate`).
    pub width: Pt,
    /// Height (before `/Rotate`).
    pub height: Pt,
}

impl PageSize {
    /// ISO 216 A0, 841 × 1189 mm.
    pub const A0: Self = Self::mm(841.0, 1189.0);
    /// ISO 216 A1, 594 × 841 mm.
    pub const A1: Self = Self::mm(594.0, 841.0);
    /// ISO 216 A2, 420 × 594 mm.
    pub const A2: Self = Self::mm(420.0, 594.0);
    /// ISO 216 A3, 297 × 420 mm.
    pub const A3: Self = Self::mm(297.0, 420.0);
    /// ISO 216 A4, 210 × 297 mm.
    pub const A4: Self = Self::mm(210.0, 297.0);
    /// ISO 216 A5, 148 × 210 mm.
    pub const A5: Self = Self::mm(148.0, 210.0);
    /// ISO 216 A6, 105 × 148 mm.
    pub const A6: Self = Self::mm(105.0, 148.0);
    /// US Letter, 8.5 × 11 in.
    pub const LETTER: Self = Self::new(Pt::new(612.0), Pt::new(792.0));
    /// US Legal, 8.5 × 14 in.
    pub const LEGAL: Self = Self::new(Pt::new(612.0), Pt::new(1008.0));
    /// Tabloid, 11 × 17 in.
    pub const TABLOID: Self = Self::new(Pt::new(792.0), Pt::new(1224.0));

    /// `width` × `height`.
    #[must_use]
    pub const fn new(width: Pt, height: Pt) -> Self {
        Self { width, height }
    }

    const fn mm(width: f64, height: f64) -> Self {
        Self::new(Pt::from_mm(width), Pt::from_mm(height))
    }

    /// The same size turned sideways (width and height swapped).
    #[must_use]
    pub const fn landscape(self) -> Self {
        Self::new(self.height, self.width)
    }
}

/// The boxes a page can set besides the `MediaBox` (§14.11.2).
#[derive(Debug, Clone, Copy, Default)]
struct Boxes {
    crop: Option<RectPt>,
    bleed: Option<RectPt>,
    trim: Option<RectPt>,
    art: Option<RectPt>,
}

/// A page being built: its geometry and its [`Surface`]. Nothing is written until
/// [`PageBuilder::finish`]; a page dropped without it is discarded.
///
/// Boxes are rectangles in the page space of the `MediaBox` as displayed (its top-left
/// corner after `/Rotate`, y down, points); the surface's page space starts at the `CropBox`'s
/// top-left (ADR 0027 §3), which is the `MediaBox`'s when no crop box is set.
#[derive(Debug)]
#[must_use = "a page is written only by PageBuilder::finish"]
pub struct PageBuilder<'b> {
    target: Target<'b>,
    size: PageSize,
    rotate: Rotation,
    user_unit: f64,
    boxes: Boxes,
    surface: Surface,
}

impl<'b> PageBuilder<'b> {
    pub(crate) fn new(target: Target<'b>, size: PageSize) -> Self {
        Self {
            target,
            size,
            rotate: Rotation::None,
            user_unit: 1.0,
            boxes: Boxes::default(),
            surface: Surface::default(),
        }
    }

    /// `/Rotate`: how the page is turned when displayed. The surface stays upright as
    /// displayed.
    pub fn rotate(&mut self, rotate: Rotation) -> &mut Self {
        self.rotate = rotate;
        self
    }

    /// `/UserUnit` (PDF 1.6, §14.11.2.1): the size of a user-space unit in points, for pages
    /// larger than 14,400 units. Sizes and coordinates stay physical points.
    pub fn user_unit(&mut self, unit: f64) -> &mut Self {
        self.user_unit = unit;
        self
    }

    /// `/CropBox`: the visible region (clipped to the `MediaBox` when written).
    pub fn crop_box(&mut self, rect: RectPt) -> &mut Self {
        self.boxes.crop = Some(rect);
        self
    }

    /// `/BleedBox`.
    pub fn bleed_box(&mut self, rect: RectPt) -> &mut Self {
        self.boxes.bleed = Some(rect);
        self
    }

    /// `/TrimBox`.
    pub fn trim_box(&mut self, rect: RectPt) -> &mut Self {
        self.boxes.trim = Some(rect);
        self
    }

    /// `/ArtBox`.
    pub fn art_box(&mut self, rect: RectPt) -> &mut Self {
        self.boxes.art = Some(rect);
        self
    }

    /// The page's drawing surface.
    pub fn surface(&mut self) -> &mut Surface {
        &mut self.surface
    }

    /// The `MediaBox` in user space and the frame the surface draws in.
    fn geometry(&self) -> Result<(UserSpaceRect, PageFrame)> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(self.user_unit) {
            return Err(Error::InvalidGeometry {
                what: "/UserUnit must be a positive number",
            });
        }
        if !positive(self.size.width.get()) || !positive(self.size.height.get()) {
            return Err(Error::InvalidGeometry {
                what: "the page width and height must be positive",
            });
        }
        let media = UserSpaceRect::from_corners(
            0.0,
            0.0,
            self.size.width.get() / self.user_unit,
            self.size.height.get() / self.user_unit,
        );
        let crop = match self.boxes.crop {
            Some(r) => self.media_rect(media, &r)?,
            None => media,
        };
        // As the reader computes it: the crop box clipped to the media box.
        let crop = crop.intersect(&media).filter(UserSpaceRect::has_area).unwrap_or(media);
        Ok((media, PageFrame::new(crop, self.rotate, self.user_unit)))
    }

    /// A box given in the `MediaBox`'s page space, in user space.
    fn media_rect(&self, media: UserSpaceRect, r: &RectPt) -> Result<UserSpaceRect> {
        let finite = [r.x0, r.y0, r.x1, r.y1].iter().all(|v| v.get().is_finite());
        if !finite {
            return Err(Error::InvalidGeometry {
                what: "a page box must have finite coordinates",
            });
        }
        Ok(PageFrame::new(media, self.rotate, self.user_unit).rect_to_user_space(r))
    }

    /// Writes the page: its content stream, the resources it uses (each object once per
    /// document) and the page dictionary, appended to the page tree. Returns its ID.
    ///
    /// # Errors
    /// [`Error::InvalidGeometry`] for a size, box or `/UserUnit` that isn't finite and
    /// positive; [`Error::Cos`] when object numbers run out
    /// ([`Limits::max_object_number`](lamina_cos::Limits::max_object_number)).
    pub fn finish(self) -> Result<PageId> {
        let (media, frame) = self.geometry()?;
        let mut page = Dict::new();
        page.insert("Type", Name::from("Page"));
        page.insert("Parent", self.target.state.pages_root);
        page.insert("MediaBox", rect_object(&media));
        let explicit = self.target.state.explicit_inherited;
        if self.boxes.crop.is_some() || explicit {
            page.insert("CropBox", rect_object(&frame.crop_box()));
        }
        for (key, rect) in [
            ("BleedBox", self.boxes.bleed),
            ("TrimBox", self.boxes.trim),
            ("ArtBox", self.boxes.art),
        ] {
            if let Some(r) = rect {
                page.insert(key, rect_object(&self.media_rect(media, &r)?));
            }
        }
        if self.rotate != Rotation::None || explicit {
            page.insert("Rotate", i64::from(self.rotate.degrees()));
        }
        if self.user_unit.to_bits() != 1.0f64.to_bits() {
            page.insert("UserUnit", self.user_unit);
        }
        let mut resources = PageResources::default();
        let content = self.surface.render(&frame, &mut resources);
        let state = self.target.state;
        let mut cos = self.target.doc.edit();
        page.insert("Resources", resources.write(&mut cos, &mut state.resources)?);
        let stream = if state.compress {
            Stream::flate(Dict::new(), &content)
        } else {
            Stream::new(Dict::new(), content)
        };
        page.insert("Contents", cos.create(Object::Stream(stream))?);
        let r = cos.create(Object::Dict(page))?;
        state.pages.push(r);
        Ok(PageId::object(r))
    }
}

/// `[x0 y0 x1 y1]`, integers where exact.
pub(crate) fn rect_object(r: &UserSpaceRect) -> Object {
    Object::Array(r.to_array().iter().map(|v| number(*v)).collect())
}

/// An integer object when `v` is integral (`612`, not `612.0`), else a real.
pub(crate) fn number(v: f64) -> Object {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "only integral values well inside i64 get here"
    )]
    if v.fract() == 0.0 && v.abs() < 1e15 {
        Object::Integer(v as i64)
    } else {
        Object::Real(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_210_by_297_mm() {
        assert_eq!(
            (PageSize::A4.width.get().round(), PageSize::A4.height.get().round()),
            (595.0, 842.0)
        );
    }

    #[test]
    fn landscape_swaps_the_sides() {
        assert_eq!(
            PageSize::LETTER.landscape(),
            PageSize::new(Pt::new(792.0), Pt::new(612.0))
        );
    }

    #[test]
    fn number_writes_integral_values_as_integers() {
        assert_eq!((number(612.0), number(0.5)), (Object::Integer(612), Object::Real(0.5)));
    }
}
