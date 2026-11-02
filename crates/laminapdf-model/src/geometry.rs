//! Coordinates and units ([ADR 0027]): page space in typed points ([`Pt`], [`PointPt`],
//! [`RectPt`]: the top-left of the displayed `CropBox`, y down, `/UserUnit` applied), raw user
//! space only in `UserSpace…` types ([`UserSpacePoint`], [`UserSpaceRect`]), and the
//! [`PageFrame`] that converts between the two.
//!
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md

use std::ops::{Add, Div, Mul, Neg, Sub};

use crate::page::Rotation;

/// A length in points: 1/72 inch, physical (`/UserUnit` applied). Page-space geometry is in
/// points; [`Pt::from_mm`] and [`Pt::from_in`] convert other units.
///
/// ```
/// use laminapdf_model::Pt;
///
/// assert_eq!(Pt::from_in(1.0), Pt::new(72.0));
/// assert!((Pt::from_mm(210.0).get() - 595.2756).abs() < 1e-4); // A4's width
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Pt(f64);

impl Pt {
    /// Zero points.
    pub const ZERO: Self = Self(0.0);

    /// `pt` points.
    #[must_use]
    pub const fn new(pt: f64) -> Self {
        Self(pt)
    }

    /// `mm` millimetres (`mm * 72 / 25.4`).
    #[must_use]
    pub const fn from_mm(mm: f64) -> Self {
        Self(mm * 72.0 / 25.4)
    }

    /// `inches` inches (`inches * 72`).
    #[must_use]
    pub const fn from_in(inches: f64) -> Self {
        Self(inches * 72.0)
    }

    /// The value in points.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Add for Pt {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Pt {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Mul<f64> for Pt {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self {
        Self(self.0 * rhs)
    }
}

impl Div<f64> for Pt {
    type Output = Self;

    fn div(self, rhs: f64) -> Self {
        Self(self.0 / rhs)
    }
}

impl Neg for Pt {
    type Output = Self;

    fn neg(self) -> Self {
        Self(-self.0)
    }
}

/// A point in page space: from the top-left of the displayed `CropBox`, x right, y down, in
/// points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointPt {
    /// Distance from the left edge.
    pub x: Pt,
    /// Distance from the top edge (down is positive).
    pub y: Pt,
}

impl PointPt {
    /// The point `(x, y)`.
    #[must_use]
    pub const fn new(x: Pt, y: Pt) -> Self {
        Self { x, y }
    }

    /// The point `(x, y)` given in points.
    #[must_use]
    pub const fn from_points(x: f64, y: f64) -> Self {
        Self { x: Pt(x), y: Pt(y) }
    }
}

/// A rectangle in page space, corners normalised: `x0 <= x1`, `y0 <= y1`, and `(x0, y0)` is
/// the top-left corner as displayed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RectPt {
    /// Left.
    pub x0: Pt,
    /// Top.
    pub y0: Pt,
    /// Right.
    pub x1: Pt,
    /// Bottom.
    pub y1: Pt,
}

impl RectPt {
    /// The rectangle with corners `(x0, y0)` and `(x1, y1)`, in any order. A NaN or
    /// infinite coordinate is kept (never replaced by the other corner), so
    /// [`RectPt::is_finite`] reports it.
    #[must_use]
    pub fn new(x0: Pt, y0: Pt, x1: Pt, y1: Pt) -> Self {
        // Not f64::min/max: they return the other operand for NaN.
        let order = |a: Pt, b: Pt| if b.0 < a.0 { (b, a) } else { (a, b) };
        let (x0, x1) = order(x0, x1);
        let (y0, y1) = order(y0, y1);
        Self { x0, y0, x1, y1 }
    }

    /// Whether every coordinate is finite.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        [self.x0, self.y0, self.x1, self.y1].iter().all(|v| v.0.is_finite())
    }

    /// The rectangle whose top-left corner is `(x, y)`, `width` wide and `height` tall
    /// (negative sizes extend left or up).
    #[must_use]
    pub fn from_xywh(x: Pt, y: Pt, width: Pt, height: Pt) -> Self {
        Self::new(x, y, x + width, y + height)
    }

    /// [`RectPt::from_xywh`] with every value given in points: the rectangle whose top-left
    /// corner is `(x, y)`, `width` wide and `height` tall.
    ///
    /// ```
    /// use laminapdf_model::{Pt, RectPt};
    ///
    /// let r = RectPt::from_points(72.0, 72.0, 200.0, 50.0);
    /// assert_eq!((r.width(), r.height()), (Pt::new(200.0), Pt::new(50.0)));
    /// ```
    #[must_use]
    pub fn from_points(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self::from_xywh(Pt(x), Pt(y), Pt(width), Pt(height))
    }

    /// `x1 - x0`.
    #[must_use]
    pub fn width(&self) -> Pt {
        self.x1 - self.x0
    }

    /// `y1 - y0`.
    #[must_use]
    pub fn height(&self) -> Pt {
        self.y1 - self.y0
    }
}

/// A point in raw user space (ISO 32000-2 §8.3.2.3): as the content stream sees it, y up, in
/// user-space units (not scaled by `/UserUnit`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UserSpacePoint {
    /// x.
    pub x: f64,
    /// y (up is positive).
    pub y: f64,
}

impl UserSpacePoint {
    /// The point `(x, y)`.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A rectangle in raw user space with its corners normalised (§7.9.5): `x0 <= x1` and
/// `y0 <= y1`, so `(x0, y0)` is the lower-left corner. Page boxes and annotation `/Rect`s as
/// stored are in this space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UserSpaceRect {
    /// Lower-left x.
    pub x0: f64,
    /// Lower-left y.
    pub y0: f64,
    /// Upper-right x.
    pub x1: f64,
    /// Upper-right y.
    pub y1: f64,
}

impl UserSpaceRect {
    /// The rectangle with corners `(a, b)` and `(c, d)`, in any order.
    #[must_use]
    pub const fn from_corners(a: f64, b: f64, c: f64, d: f64) -> Self {
        Self {
            x0: a.min(c),
            y0: b.min(d),
            x1: a.max(c),
            y1: b.max(d),
        }
    }

    /// `x1 - x0`.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }

    /// `y1 - y0`.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.y1 - self.y0
    }

    /// Whether the rectangle has a positive width and height.
    #[must_use]
    pub fn has_area(&self) -> bool {
        self.width() > 0.0 && self.height() > 0.0
    }

    /// The overlap of two rectangles, `None` when they don't overlap.
    #[must_use]
    pub const fn intersect(&self, other: &Self) -> Option<Self> {
        let r = Self {
            x0: self.x0.max(other.x0),
            y0: self.y0.max(other.y0),
            x1: self.x1.min(other.x1),
            y1: self.y1.min(other.y1),
        };
        if r.x0 <= r.x1 && r.y0 <= r.y1 { Some(r) } else { None }
    }

    /// `[x0 y0 x1 y1]`.
    #[must_use]
    pub const fn to_array(&self) -> [f64; 4] {
        [self.x0, self.y0, self.x1, self.y1]
    }
}

/// US Letter, `[0 0 612 792]`: the frame's box when a page has no usable box (ADR 0027 §3,
/// as pdf.js and PDFium do).
pub const LETTER: UserSpaceRect = UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0);

/// A page's frame ([ADR 0027] §3): the reference box (the effective `CropBox`), the rotation and
/// `/UserUnit`, which together map raw user space to page space (the top-left of the `CropBox`
/// as displayed, y down, points) and back. [`crate::Page::frame`] computes it once per page.
///
/// ```
/// use laminapdf_model::{PageFrame, Pt, PointPt, Rotation, UserSpacePoint, UserSpaceRect};
///
/// let letter = UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0);
/// let frame = PageFrame::new(letter, Rotation::None, 1.0);
/// // 1 inch from the left and the top of the page.
/// assert_eq!(
///     frame.to_page(UserSpacePoint::new(72.0, 720.0)),
///     PointPt::new(Pt::new(72.0), Pt::new(72.0))
/// );
/// let turned = PageFrame::new(letter, Rotation::Clockwise90, 1.0);
/// assert_eq!((turned.width(), turned.height()), (Pt::new(792.0), Pt::new(612.0)));
/// ```
///
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PageFrame {
    crop_box: UserSpaceRect,
    rotate: Rotation,
    user_unit: f64,
    letter_fallback: bool,
}

impl PageFrame {
    /// The frame of a page with this effective `CropBox`, rotation and `/UserUnit` (a unit that
    /// is not a positive finite number counts as 1).
    #[must_use]
    pub fn new(crop_box: UserSpaceRect, rotate: Rotation, user_unit: f64) -> Self {
        let user_unit = if user_unit.is_finite() && user_unit > 0.0 {
            user_unit
        } else {
            1.0
        };
        Self {
            crop_box,
            rotate,
            user_unit,
            letter_fallback: false,
        }
    }

    pub(crate) const fn with_letter_fallback(mut self) -> Self {
        self.letter_fallback = true;
        self
    }

    /// The reference box: the effective `CropBox` in user space (or [`LETTER`] when the page
    /// had no usable box, [`PageFrame::is_letter_fallback`]).
    #[must_use]
    pub const fn crop_box(&self) -> UserSpaceRect {
        self.crop_box
    }

    /// The rotation applied.
    #[must_use]
    pub const fn rotate(&self) -> Rotation {
        self.rotate
    }

    /// The `/UserUnit` applied.
    #[must_use]
    pub const fn user_unit(&self) -> f64 {
        self.user_unit
    }

    /// Whether the page had no box with an area, so the frame is US Letter (JSON's
    /// `coordinates.box` is `"default"`).
    #[must_use]
    pub const fn is_letter_fallback(&self) -> bool {
        self.letter_fallback
    }

    /// The displayed width in points (the box's height for 90° and 270°).
    #[must_use]
    pub fn width(&self) -> Pt {
        let b = &self.crop_box;
        let side = if self.quarter_turned() { b.height() } else { b.width() };
        Pt(side * self.user_unit)
    }

    /// The displayed height in points.
    #[must_use]
    pub fn height(&self) -> Pt {
        let b = &self.crop_box;
        let side = if self.quarter_turned() { b.width() } else { b.height() };
        Pt(side * self.user_unit)
    }

    const fn quarter_turned(&self) -> bool {
        matches!(self.rotate, Rotation::Clockwise90 | Rotation::Clockwise270)
    }

    /// A user-space point in page space (ADR 0027 §3's table).
    #[must_use]
    pub fn to_page(&self, p: UserSpacePoint) -> PointPt {
        let UserSpaceRect { x0, y0, x1, y1 } = self.crop_box;
        let u = self.user_unit;
        let (x, y) = match self.rotate {
            Rotation::None => (p.x - x0, y1 - p.y),
            Rotation::Clockwise90 => (p.y - y0, p.x - x0),
            Rotation::Half => (x1 - p.x, p.y - y0),
            Rotation::Clockwise270 => (y1 - p.y, x1 - p.x),
        };
        PointPt::new(Pt(x * u), Pt(y * u))
    }

    /// A page-space point in user space: the inverse of [`PageFrame::to_page`].
    #[must_use]
    #[expect(clippy::many_single_char_names, reason = "a–f are the matrix entries of §8.3.4")]
    pub fn to_user_space(&self, p: PointPt) -> UserSpacePoint {
        let [a, b, c, d, e, f] = self.to_user_space_matrix();
        let (x, y) = (p.x.get(), p.y.get());
        UserSpacePoint::new(a * x + c * y + e, b * x + d * y + f)
    }

    /// [`PageFrame::to_user_space`] as a PDF matrix `[a b c d e f]` (§8.3.4): a page-space
    /// point `(x, y)` in points maps to `(a·x + c·y + e, b·x + d·y + f)` in user space.
    #[must_use]
    pub fn to_user_space_matrix(&self) -> [f64; 6] {
        let UserSpaceRect { x0, y0, x1, y1 } = self.crop_box;
        let s = 1.0 / self.user_unit;
        match self.rotate {
            Rotation::None => [s, 0.0, 0.0, -s, x0, y1],
            Rotation::Clockwise90 => [0.0, s, s, 0.0, x0, y0],
            Rotation::Half => [-s, 0.0, 0.0, s, x1, y0],
            Rotation::Clockwise270 => [0.0, -s, -s, 0.0, x1, y1],
        }
    }

    /// A page-space rectangle in user space (normalised: rotation and the y flip swap
    /// corners).
    #[must_use]
    pub fn rect_to_user_space(&self, r: &RectPt) -> UserSpaceRect {
        let a = self.to_user_space(PointPt::new(r.x0, r.y0));
        let b = self.to_user_space(PointPt::new(r.x1, r.y1));
        UserSpaceRect::from_corners(a.x, a.y, b.x, b.y)
    }

    /// A user-space rectangle in page space.
    #[must_use]
    pub fn rect_to_page(&self, r: &UserSpaceRect) -> RectPt {
        let a = self.to_page(UserSpacePoint::new(r.x0, r.y0));
        let b = self.to_page(UserSpacePoint::new(r.x1, r.y1));
        RectPt::new(a.x, a.y, b.x, b.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_points_matches_from_xywh_and_normalises_negative_sizes() {
        let pt = Pt::new;
        assert_eq!(
            RectPt::from_points(10.0, 20.0, 30.0, 40.0),
            RectPt::from_xywh(pt(10.0), pt(20.0), pt(30.0), pt(40.0))
        );
        assert_eq!(
            RectPt::from_points(10.0, 20.0, -5.0, -5.0),
            RectPt::new(pt(5.0), pt(15.0), pt(10.0), pt(20.0))
        );
    }

    const LETTER_RECT: UserSpaceRect = UserSpaceRect::from_corners(72.0, 692.0, 272.0, 720.0);

    fn frame(rotate: Rotation) -> PageFrame {
        PageFrame::new(LETTER, rotate, 1.0)
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
        RectPt::new(Pt(x0), Pt(y0), Pt(x1), Pt(y1))
    }

    // ADR 0027 §3's worked example and table, one test per /Rotate value.
    #[test]
    fn rotate_0_puts_the_heading_one_inch_from_the_top_left() {
        assert_eq!(
            frame(Rotation::None).rect_to_page(&LETTER_RECT),
            rect(72.0, 72.0, 272.0, 100.0)
        );
    }

    #[test]
    fn rotate_90_maps_user_space_bottom_left_to_the_displayed_top_left() {
        let f = frame(Rotation::Clockwise90);
        assert_eq!(f.rect_to_page(&LETTER_RECT), rect(692.0, 72.0, 720.0, 272.0));
        assert_eq!(
            f.to_page(UserSpacePoint::new(0.0, 0.0)),
            PointPt::new(Pt::ZERO, Pt::ZERO)
        );
    }

    #[test]
    fn rotate_180_maps_user_space_bottom_right_to_the_displayed_top_left() {
        let f = frame(Rotation::Half);
        assert_eq!(
            f.to_page(UserSpacePoint::new(612.0, 0.0)),
            PointPt::new(Pt::ZERO, Pt::ZERO)
        );
        assert_eq!(f.rect_to_page(&LETTER_RECT), rect(340.0, 692.0, 540.0, 720.0));
    }

    #[test]
    fn rotate_270_maps_user_space_top_right_to_the_displayed_top_left() {
        let f = frame(Rotation::Clockwise270);
        assert_eq!(
            f.to_page(UserSpacePoint::new(612.0, 792.0)),
            PointPt::new(Pt::ZERO, Pt::ZERO)
        );
        assert_eq!(f.rect_to_page(&LETTER_RECT), rect(72.0, 340.0, 100.0, 540.0));
        assert_eq!((f.width(), f.height()), (Pt(792.0), Pt(612.0)));
    }

    #[test]
    fn to_user_space_inverts_to_page_for_every_rotation_and_offset_box() {
        let crop = UserSpaceRect::from_corners(10.0, 20.0, 310.0, 420.0);
        for rotate in [
            Rotation::None,
            Rotation::Clockwise90,
            Rotation::Half,
            Rotation::Clockwise270,
        ] {
            let f = PageFrame::new(crop, rotate, 2.0);
            let p = UserSpacePoint::new(55.0, 77.0);
            assert_eq!(f.to_user_space(f.to_page(p)), p, "{rotate:?}");
        }
    }

    #[test]
    fn user_unit_scales_page_space_to_physical_points() {
        let f = PageFrame::new(UserSpaceRect::from_corners(0.0, 0.0, 100.0, 50.0), Rotation::None, 10.0);
        assert_eq!((f.width(), f.height()), (Pt(1000.0), Pt(500.0)));
        assert_eq!(
            f.to_page(UserSpacePoint::new(10.0, 50.0)),
            PointPt::new(Pt(100.0), Pt::ZERO)
        );
    }

    #[test]
    fn an_invalid_user_unit_counts_as_one() {
        assert_eq!(PageFrame::new(LETTER, Rotation::None, -3.0).user_unit(), 1.0);
    }

    #[test]
    fn rect_pt_keeps_a_nan_coordinate() {
        let r = RectPt::new(Pt(f64::NAN), Pt(0.0), Pt(10.0), Pt(10.0));
        assert!(!r.is_finite());
    }

    #[test]
    fn rect_pt_normalises_its_corners() {
        let r = RectPt::from_xywh(Pt(10.0), Pt(10.0), Pt(-5.0), Pt(20.0));
        assert_eq!(r, rect(5.0, 10.0, 10.0, 30.0));
    }
}
