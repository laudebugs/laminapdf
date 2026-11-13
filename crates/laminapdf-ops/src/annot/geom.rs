//! The values the annotation specs carry: points, rectangles and quads in user space, colours
//! and opacity. Each is checked when it is made and when it is read from JSON, so an op never
//! holds a number a PDF can't store (a NaN, an infinity, a colour component outside 0 to 1).

use laminapdf_cos::Object;
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use serde::{Deserialize, Serialize};

/// A point in user space (ISO 32000-2 §8.3.2.3: y up), in user-space units. Both coordinates
/// are finite.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Point {
    x: f64,
    y: f64,
}

impl Point {
    /// The point `(x, y)`; `None` unless both are finite.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Option<Self> {
        if x.is_finite() && y.is_finite() {
            Some(Self { x, y })
        } else {
            None
        }
    }

    /// x.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    /// y (up is positive).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }

    /// The same point as the model's type.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn to_user_space(self) -> UserSpacePoint {
        UserSpacePoint::new(self.x, self.y)
    }

    /// The model's point, when both coordinates are finite.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn from_user_space(p: UserSpacePoint) -> Option<Self> {
        Self::new(p.x, p.y)
    }
}

// Sound: both fields are finite, so no NaN breaks reflexivity.
impl Eq for Point {}

impl<'de> Deserialize<'de> for Point {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            x: f64,
            y: f64,
        }
        let w = Wire::deserialize(d)?;
        Self::new(w.x, w.y).ok_or_else(|| serde::de::Error::custom("a point must be finite"))
    }
}

/// A rectangle in user space with its corners normalised: `x0 <= x1` and `y0 <= y1`, every
/// coordinate finite. On the wire: `{ "x0": 72, "y0": 700, "x1": 90, "y1": 712 }`, corners in
/// any order.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Rect {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

impl Rect {
    /// The rectangle with corners `(a, b)` and `(c, d)`, in any order; `None` unless all four
    /// are finite.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn from_corners(a: f64, b: f64, c: f64, d: f64) -> Option<Self> {
        if a.is_finite() && b.is_finite() && c.is_finite() && d.is_finite() {
            Some(Self {
                x0: a.min(c),
                y0: b.min(d),
                x1: a.max(c),
                y1: b.max(d),
            })
        } else {
            None
        }
    }

    /// The same rectangle as the model's type.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn to_user_space(self) -> UserSpaceRect {
        UserSpaceRect::from_corners(self.x0, self.y0, self.x1, self.y1)
    }

    /// The model's rectangle, when its corners are finite.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn from_user_space(r: UserSpaceRect) -> Option<Self> {
        Self::from_corners(r.x0, r.y0, r.x1, r.y1)
    }

    /// Whether it has a positive width and height.
    pub(crate) fn has_area(self) -> bool {
        self.x1 > self.x0 && self.y1 > self.y0
    }
}

impl Eq for Rect {}

impl<'de> Deserialize<'de> for Rect {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            x0: f64,
            y0: f64,
            x1: f64,
            y1: f64,
        }
        let w = Wire::deserialize(d)?;
        Self::from_corners(w.x0, w.y0, w.x1, w.y1).ok_or_else(|| serde::de::Error::custom("a rectangle must be finite"))
    }
}

/// The four corners of a piece of marked text, in the order pdf.js, PDFium and Acrobat read
/// `/QuadPoints`: upper left, upper right, lower left, lower right, where "upper" is the side
/// of the text's ascenders (ISO 32000-2 Table 182 describes another order, which readers that
/// follow Acrobat's files draw wrongly). Rotated text gives a rotated quad. On the wire: an
/// array of four points.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Quad(pub [Point; 4]);

impl Quad {
    /// The quad of an upright rectangle: its top edge is the text's top.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_rect(r: Rect) -> Self {
        let p = |x, y| Point { x, y };
        Self([p(r.x0, r.y1), p(r.x1, r.y1), p(r.x0, r.y0), p(r.x1, r.y0)])
    }

    /// The quad read from four stored corners in either order: the standard's (counter-clockwise
    /// from the lower left) or Acrobat's. The standard's order goes around the quad, so its third
    /// corner is opposite the first; in Acrobat's order the fourth is.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_stored(c: [Point; 4]) -> Self {
        let dist = |a: Point, b: Point| (a.x - b.x).hypot(a.y - b.y);
        // Acrobat: ul ur ll lr, whose diagonal is 0–3. The standard: ll lr ur ul around the
        // quad, whose diagonal is 0–2.
        if dist(c[0], c[2]) > dist(c[0], c[3]) {
            Self([c[3], c[2], c[0], c[1]])
        } else {
            Self(c)
        }
    }

    /// The smallest upright rectangle holding the quad.
    #[must_use]
    pub(crate) fn bounds(&self) -> Rect {
        let xs = self.0.map(|p| p.x);
        let ys = self.0.map(|p| p.y);
        let (min, max) = (|v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min), |v: [f64; 4]| {
            v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
        });
        Rect {
            x0: min(xs),
            y0: min(ys),
            x1: max(xs),
            y1: max(ys),
        }
    }
}

/// An annotation's colour (`/C`, ISO 32000-2 Table 166): gray, RGB or CMYK, each component in
/// 0 to 1. On the wire it is `/C`'s own form, an array of one, three or four numbers.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(into = "Vec<f64>")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(type = "[number] | [number, number, number] | [number, number, number, number]"))]
#[non_exhaustive]
pub enum Color {
    /// `[gray]`.
    Gray(f64),
    /// `[r g b]`.
    Rgb(f64, f64, f64),
    /// `[c m y k]`.
    Cmyk(f64, f64, f64, f64),
}

impl Color {
    /// The colour of `/C`'s components, or `None` unless there are one, three or four, each
    /// in 0 to 1.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn from_components(c: &[f64]) -> Option<Self> {
        if !c.iter().all(|v| (0.0..=1.0).contains(v)) {
            return None;
        }
        match *c {
            [g] => Some(Self::Gray(g)),
            [r, g, b] => Some(Self::Rgb(r, g, b)),
            [c, m, y, k] => Some(Self::Cmyk(c, m, y, k)),
            _ => None,
        }
    }

    /// The components, as `/C` stores them.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn components(self) -> Vec<f64> {
        match self {
            Self::Gray(g) => vec![g],
            Self::Rgb(r, g, b) => vec![r, g, b],
            Self::Cmyk(c, m, y, k) => vec![c, m, y, k],
        }
    }

    /// `/C`'s array.
    pub(crate) fn to_object(self) -> Object {
        Object::Array(self.components().into_iter().map(Object::number).collect())
    }
}

impl Eq for Color {}

impl From<Color> for Vec<f64> {
    fn from(c: Color) -> Self {
        c.components()
    }
}

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let c = Vec::<f64>::deserialize(d)?;
        Self::from_components(&c)
            .ok_or_else(|| serde::de::Error::custom("a colour is one, three or four numbers from 0 to 1"))
    }
}

/// A constant opacity from 0 (invisible) to 1 (opaque).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(type = "number"))]
pub struct Opacity(f64);

impl Opacity {
    /// Fully opaque.
    ///
    /// Added in 0.2.0.
    pub const OPAQUE: Self = Self(1.0);

    /// The opacity `value`; `None` outside 0 to 1.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        (0.0..=1.0).contains(&value).then_some(Self(value))
    }

    /// The value, from 0 to 1.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// Whether it is below 1.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn is_translucent(self) -> bool {
        self.0 < 1.0
    }
}

impl Eq for Opacity {}

impl<'de> Deserialize<'de> for Opacity {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = f64::deserialize(d)?;
        Self::new(v).ok_or_else(|| serde::de::Error::custom("an opacity is a number from 0 to 1"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Point {
        Point::new(x, y).unwrap()
    }

    #[test]
    fn quads_read_in_either_order_come_out_in_acrobats() {
        let acrobat = [p(0.0, 10.0), p(50.0, 10.0), p(0.0, 0.0), p(50.0, 0.0)];
        let standard = [p(0.0, 0.0), p(50.0, 0.0), p(50.0, 10.0), p(0.0, 10.0)];
        assert_eq!(Quad::from_stored(acrobat), Quad(acrobat));
        assert_eq!(Quad::from_stored(standard), Quad(acrobat));
    }

    #[test]
    fn a_colour_outside_zero_to_one_or_of_two_components_is_refused() {
        assert_eq!(Color::from_components(&[1.0, 0.5, 0.0]), Some(Color::Rgb(1.0, 0.5, 0.0)));
        assert_eq!(Color::from_components(&[1.2]), None);
        assert_eq!(Color::from_components(&[0.0, 0.0]), None);
        assert!(serde_json::from_str::<Color>("[0, 1]").is_err());
    }

    #[test]
    fn a_point_that_isnt_finite_is_refused_on_the_wire() {
        assert!(serde_json::from_str::<Point>(r#"{ "x": 1, "y": 1e999 }"#).is_err());
        assert!(serde_json::from_str::<Opacity>("1.5").is_err());
    }
}
