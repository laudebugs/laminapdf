//! The values an annotation op writes besides its geometry, and the quad helpers: an
//! annotation's colour ([`AnnotColor`], `/C`) and opacity ([`Opacity`]), quads put in Acrobat's
//! order whichever order a file stores them in.
//!
//! Geometry itself is the model's: [`UserSpacePoint`] and [`UserSpaceRect`], and a quad is four
//! points, as links read them.

use laminapdf_cos::Object;
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use serde::{Deserialize, Serialize};

/// A quad: four corners in user space.
pub(crate) type Quad = [UserSpacePoint; 4];

/// The quad in Acrobat's order (upper left, upper right, lower left, lower right), which pdf.js,
/// PDFium and Acrobat all read, from either that order or the standard's (counter-clockwise
/// from the lower left, ISO 32000-2 Table 182). The standard's order goes around the quad, so its
/// third corner is opposite the first; in Acrobat's the fourth is.
pub(crate) fn acrobat_order(c: Quad) -> Quad {
    let dist = |a: UserSpacePoint, b: UserSpacePoint| (a.x - b.x).hypot(a.y - b.y);
    if dist(c[0], c[2]) > dist(c[0], c[3]) {
        [c[3], c[2], c[0], c[1]]
    } else {
        c
    }
}

/// Whether every coordinate of `points` is finite.
pub(crate) fn finite(points: impl IntoIterator<Item = UserSpacePoint>) -> bool {
    points.into_iter().all(|p| p.x.is_finite() && p.y.is_finite())
}

/// The two corners of `rect`.
pub(crate) const fn corners(r: &UserSpaceRect) -> [UserSpacePoint; 2] {
    [UserSpacePoint::new(r.x0, r.y0), UserSpacePoint::new(r.x1, r.y1)]
}

/// An annotation's colour (`/C`, ISO 32000-2 Table 166): gray, RGB or CMYK, each component from
/// 0 to 1. On the wire it is `/C`'s own form, an array of one, three or four numbers.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(into = "Vec<f64>")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(
    feature = "tsify",
    tsify(type = "[number] | [number, number, number] | [number, number, number, number]")
)]
#[non_exhaustive]
pub enum AnnotColor {
    /// `[gray]`.
    Gray(f64),
    /// `[r g b]`.
    Rgb(f64, f64, f64),
    /// `[c m y k]`.
    Cmyk(f64, f64, f64, f64),
}

impl AnnotColor {
    /// The colour of `/C`'s components; `None` unless there are one, three or four, each from 0
    /// to 1.
    ///
    /// Added in 0.3.0.
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
    /// Added in 0.3.0.
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
        Object::Array(self.components().into_iter().map(Object::Real).collect())
    }
}

// Sound: every component is in 0 to 1 (`from_components` and the deserialiser check), so no NaN
// breaks reflexivity.
impl Eq for AnnotColor {}

impl From<AnnotColor> for Vec<f64> {
    fn from(c: AnnotColor) -> Self {
        c.components()
    }
}

impl<'de> Deserialize<'de> for AnnotColor {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let c = Vec::<f64>::deserialize(d)?;
        Self::from_components(&c)
            .ok_or_else(|| serde::de::Error::custom("a colour is one, three or four numbers from 0 to 1"))
    }
}

/// A constant opacity from 0 (invisible) to 1 (opaque).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(feature = "tsify", tsify(type = "number"))]
pub struct Opacity(f64);

impl Opacity {
    /// Fully opaque.
    ///
    /// Added in 0.3.0.
    pub const OPAQUE: Self = Self(1.0);

    /// The opacity `value`; `None` outside 0 to 1.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        (0.0..=1.0).contains(&value).then_some(Self(value))
    }

    /// The value, from 0 to 1.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl Default for Opacity {
    fn default() -> Self {
        Self::OPAQUE
    }
}

// Sound: the value is in 0 to 1, never NaN.
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

    fn p(x: f64, y: f64) -> UserSpacePoint {
        UserSpacePoint::new(x, y)
    }

    #[test]
    fn quads_read_in_either_order_come_out_in_acrobats() {
        let acrobat = [p(0.0, 10.0), p(50.0, 10.0), p(0.0, 0.0), p(50.0, 0.0)];
        let standard = [p(0.0, 0.0), p(50.0, 0.0), p(50.0, 10.0), p(0.0, 10.0)];
        assert_eq!((acrobat_order(acrobat), acrobat_order(standard)), (acrobat, acrobat));
    }

    #[test]
    fn a_colour_outside_zero_to_one_or_of_two_components_is_refused() {
        assert_eq!(
            (
                AnnotColor::from_components(&[1.0, 0.5, 0.0]),
                AnnotColor::from_components(&[1.2]),
                AnnotColor::from_components(&[0.0, 0.0]),
            ),
            (Some(AnnotColor::Rgb(1.0, 0.5, 0.0)), None, None)
        );
    }

    #[test]
    fn a_colour_or_opacity_out_of_range_is_refused_on_the_wire() {
        assert!(
            serde_json::from_str::<AnnotColor>("[0, 1]").is_err() && serde_json::from_str::<Opacity>("1.5").is_err()
        );
    }
}
