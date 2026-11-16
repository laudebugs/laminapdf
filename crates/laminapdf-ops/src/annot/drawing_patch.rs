//! The changes an [`UpdateAnnot`](super::UpdateAnnot) can make to a drawing mark: new geometry,
//! endings, fill or border, and for ink, strokes added or taken away (an eraser, or strokes
//! joined into one mark). Each field is absent to keep the value; an optional value is `null` to
//! remove it.

use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use serde::{Deserialize, Serialize};

use super::drawing::{Border, Ink, InkStroke, Leader, LineEnding, LineMark, Poly, Shape};
use super::geom::AnnotColor;
use crate::wire;

/// A change to an ink mark. `remove` applies first, then `add`; `strokes` replaces every stroke
/// and can't come with either.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct InkPatch {
    /// Every stroke, replacing the mark's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub strokes: Option<Vec<InkStroke>>,
    /// The strokes to take away, by their 0-based index in the mark.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub remove: Vec<u32>,
    /// Strokes to add after the mark's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub add: Vec<InkStroke>,
    /// The pen's width.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub width: Option<f64>,
    /// Whether the strokes are drawn smoothed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub smoothing: Option<bool>,
}

/// A change to a square or circle.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ShapePatch {
    /// The box.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_user_space_rect")]
    #[cfg_attr(
        feature = "tsify",
        tsify(optional, type = "{ x0: number; y0: number; x1: number; y1: number }")
    )]
    pub rect: Option<UserSpaceRect>,
    /// The fill; `null` for none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "[number] | [number, number, number] | [number, number, number, number] | null"
        )
    )]
    pub interior: Option<Option<AnnotColor>>,
    /// The outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub border: Option<Border>,
}

/// A change to a line.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct LinePatch {
    /// Where it starts.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "wire::opt_user_space_point"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "{ x: number; y: number }"))]
    pub from: Option<UserSpacePoint>,
    /// Where it ends.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "wire::opt_user_space_point"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "{ x: number; y: number }"))]
    pub to: Option<UserSpacePoint>,
    /// The endings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub ends: Option<[LineEnding; 2]>,
    /// The colour closed endings are filled with; `null` for none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "[number] | [number, number, number] | [number, number, number, number] | null"
        )
    )]
    pub interior: Option<Option<AnnotColor>>,
    /// The stroke.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub border: Option<Border>,
    /// The leader lines; `null` for none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "Leader | null"))]
    pub leader: Option<Option<Leader>>,
}

/// A change to a polygon or polyline.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct PolyPatch {
    /// The vertices.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::points::opt")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "{ x: number; y: number }[]"))]
    pub vertices: Option<Vec<UserSpacePoint>>,
    /// A polyline's endings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub ends: Option<[LineEnding; 2]>,
    /// The fill; `null` for none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "[number] | [number, number, number] | [number, number, number, number] | null"
        )
    )]
    pub interior: Option<Option<AnnotColor>>,
    /// The outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub border: Option<Border>,
}

impl InkPatch {
    /// `ink` changed; `None` when `strokes` comes with `remove` or `add`, or `remove` names a
    /// stroke the mark doesn't have.
    pub(crate) fn applied_to(&self, ink: &Ink) -> Option<Ink> {
        let strokes = match &self.strokes {
            Some(strokes) if self.remove.is_empty() && self.add.is_empty() => strokes.clone(),
            Some(_) => return None,
            None => {
                let count = ink.strokes.len();
                if self
                    .remove
                    .iter()
                    .any(|&i| usize::try_from(i).map_or(true, |i| i >= count))
                {
                    return None;
                }
                ink.strokes
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !self.remove.iter().any(|&r| usize::try_from(r).is_ok_and(|r| r == *i)))
                    .map(|(_, s)| s.clone())
                    .chain(self.add.iter().cloned())
                    .collect()
            }
        };
        Some(Ink {
            strokes,
            width: self.width.unwrap_or(ink.width),
            smoothing: self.smoothing.unwrap_or(ink.smoothing),
        })
    }
}

impl ShapePatch {
    pub(crate) fn applied_to(&self, shape: &Shape) -> Shape {
        Shape {
            kind: shape.kind,
            rect: self.rect.unwrap_or(shape.rect),
            interior: self.interior.unwrap_or(shape.interior),
            border: self.border.clone().unwrap_or_else(|| shape.border.clone()),
        }
    }
}

impl LinePatch {
    pub(crate) fn applied_to(&self, line: &LineMark) -> LineMark {
        LineMark {
            from: self.from.unwrap_or(line.from),
            to: self.to.unwrap_or(line.to),
            ends: self.ends.unwrap_or(line.ends),
            interior: self.interior.unwrap_or(line.interior),
            border: self.border.clone().unwrap_or_else(|| line.border.clone()),
            leader: self.leader.unwrap_or(line.leader),
        }
    }
}

impl PolyPatch {
    pub(crate) fn applied_to(&self, poly: &Poly) -> Poly {
        Poly {
            kind: poly.kind,
            vertices: self.vertices.clone().unwrap_or_else(|| poly.vertices.clone()),
            ends: self.ends.unwrap_or(poly.ends),
            interior: self.interior.unwrap_or(poly.interior),
            border: self.border.clone().unwrap_or_else(|| poly.border.clone()),
        }
    }
}
