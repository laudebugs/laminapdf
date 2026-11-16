//! An annotation's normal appearance and where it draws (ISO 32000-2 §12.5.5): the stream
//! `/AP /N` holds, or the entry of its state dictionary that `/AS` names, and the matrix that
//! maps the stream's `/BBox`, transformed by its `/Matrix`, onto the annotation's `/Rect`.
//! The extractor reads appearances through these to find what an annotation covers, and the
//! flattening op to draw them into the page. Both read tolerantly (ADR 0003): a value that
//! can't be read counts as absent.

use laminapdf_cos::{ObjRef, Object, Result, Stream};

use crate::document::Document;
use crate::geometry::UserSpaceRect;
use crate::graphics::Matrix;
use crate::page::read_rect;

/// An annotation's normal appearance stream, with the object that holds it.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct NormalAppearance {
    /// The Form XObject.
    pub stream: Stream,
    /// Its object; `None` for a stream written directly in the dictionary, which Table 31
    /// doesn't allow.
    pub object: Option<ObjRef>,
}

impl Document {
    /// The normal appearance of the annotation `annot`: `/AP /N` when it is a stream, or the
    /// entry of its state dictionary that `/AS` names (a check box's `/On`, a stamp's state).
    /// `None` when the annotation has no `/AP`, its `/N` is missing, or a state dictionary has
    /// no entry for `/AS` (or there is no `/AS`).
    ///
    /// # Errors
    /// Only what must propagate (strict mode, pending data, a source error).
    ///
    /// Added in 0.6.0.
    pub fn normal_appearance(&self, annot: &laminapdf_cos::Dict) -> Result<Option<NormalAppearance>> {
        let Some(ap) = annot.get("AP") else {
            return Ok(None);
        };
        let Some(normal) = self
            .read(ap)?
            .and_then(|ap| ap.as_dict().and_then(|d| d.get("N")).cloned())
        else {
            return Ok(None);
        };
        let chosen = match &*self.read(&normal)?.unwrap_or_default() {
            Object::Stream(_) => normal,
            Object::Dict(states) => {
                let state = match annot.get("AS") {
                    Some(state) => self.read(state)?,
                    None => None,
                };
                let entry = match state.as_deref() {
                    Some(Object::Name(name)) => states.get(&String::from_utf8_lossy(name.as_bytes())),
                    _ => None,
                };
                match entry {
                    Some(entry) => entry.clone(),
                    None => return Ok(None),
                }
            }
            _ => return Ok(None),
        };
        let object = chosen.as_ref();
        Ok(match &*self.read(&chosen)?.unwrap_or_default() {
            Object::Stream(stream) => Some(NormalAppearance {
                stream: stream.clone(),
                object,
            }),
            _ => None,
        })
    }

    /// Where `appearance` draws in the annotation `annot` (§12.5.5): from the stream's `/BBox`,
    /// its `/Matrix` (the identity when absent) and the annotation's `/Rect`, as
    /// [`AppearancePlacement::new`] computes it. `None` when either box is missing or malformed,
    /// or the transformed box has no area.
    ///
    /// # Errors
    /// As [`normal_appearance`](Self::normal_appearance).
    ///
    /// Added in 0.6.0.
    pub fn appearance_placement(
        &self,
        annot: &laminapdf_cos::Dict,
        appearance: &Stream,
    ) -> Result<Option<AppearancePlacement>> {
        let rect = self.annotation_rect(annot)?;
        let bbox = match appearance.dict.get("BBox") {
            Some(bbox) => read_rect(self, bbox)?,
            None => None,
        };
        let (Some(rect), Some(bbox)) = (rect, bbox) else {
            return Ok(None);
        };
        let matrix = match appearance.dict.get("Matrix") {
            Some(m) => self.matrix(m)?.unwrap_or(Matrix::IDENTITY),
            None => Matrix::IDENTITY,
        };
        Ok(AppearancePlacement::new(&bbox, &matrix, &rect))
    }

    /// The annotation's `/Rect`, read through references; `None` unless it is four numbers.
    ///
    /// # Errors
    /// As [`normal_appearance`](Self::normal_appearance).
    ///
    /// Added in 0.6.0.
    pub fn annotation_rect(&self, annot: &laminapdf_cos::Dict) -> Result<Option<UserSpaceRect>> {
        match annot.get("Rect") {
            Some(rect) => read_rect(self, rect),
            None => Ok(None),
        }
    }

    /// Six numbers as a matrix, each read through a reference.
    fn matrix(&self, value: &Object) -> Result<Option<Matrix>> {
        let Some(array) = self.read(value)? else {
            return Ok(None);
        };
        let Some(items) = array.as_array() else {
            return Ok(None);
        };
        let mut values = Vec::with_capacity(items.len());
        for item in items {
            match crate::page::number(self, item)? {
                Some(v) => values.push(v),
                None => return Ok(None),
            }
        }
        Ok(Matrix::from_slice(&values))
    }
}

/// Where an appearance draws (§12.5.5, Algorithm 8.1): the form's own `/Matrix`, then the
/// scale and shift `fit` that map the box it transforms onto the annotation's `/Rect`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct AppearancePlacement {
    /// The form's `/Matrix`, which drawing the form with `Do` applies itself.
    pub matrix: Matrix,
    /// The matrix `A` that fits the transformed box onto `/Rect`: what a content stream that
    /// draws the form with `Do` sets with `cm` first.
    pub fit: Matrix,
}

impl AppearancePlacement {
    /// The placement of an appearance whose `/BBox` is `bbox` and `/Matrix` is `matrix` in the
    /// annotation's `rect`; `None` when the transformed box has no area.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn new(bbox: &UserSpaceRect, matrix: &Matrix, rect: &UserSpaceRect) -> Option<Self> {
        let boxed = bbox.transformed(matrix);
        let (w, h) = (boxed.x1 - boxed.x0, boxed.y1 - boxed.y0);
        if w <= 0.0 || h <= 0.0 || !w.is_finite() || !h.is_finite() {
            return None;
        }
        let fit = Matrix::translate(-boxed.x0, -boxed.y0)
            .then(&Matrix::scale((rect.x1 - rect.x0) / w, (rect.y1 - rect.y0) / h))
            .then(&Matrix::translate(rect.x0, rect.y0));
        Some(Self { matrix: *matrix, fit })
    }

    /// The whole mapping from the form's space to user space: its `/Matrix`, then `fit`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn to_user_space(&self) -> Matrix {
        self.matrix.then(&self.fit)
    }
}

#[cfg(test)]
mod tests {
    use super::AppearancePlacement;
    use crate::{Matrix, UserSpaceRect};

    fn mapped(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = m.apply(x, y);
        ((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0)
    }

    #[test]
    fn the_box_maps_onto_the_rect() {
        let bbox = UserSpaceRect::from_corners(0.0, 0.0, 10.0, 20.0);
        let rect = UserSpaceRect::from_corners(100.0, 200.0, 120.0, 220.0);
        let m =
            AppearancePlacement::new(&bbox, &Matrix::IDENTITY, &rect).map_or(Matrix::IDENTITY, |p| p.to_user_space());
        assert_eq!(
            (mapped(&m, 0.0, 0.0), mapped(&m, 10.0, 20.0)),
            ((100.0, 200.0), (120.0, 220.0))
        );
    }

    #[test]
    fn a_quarter_turned_matrix_still_fills_the_rect() {
        let bbox = UserSpaceRect::from_corners(0.0, 0.0, 10.0, 20.0);
        // 90° counter-clockwise: the 10 × 20 box becomes 20 × 10, x from −20 to 0.
        let turn = Matrix::new([0.0, 1.0, -1.0, 0.0, 0.0, 0.0]);
        let rect = UserSpaceRect::from_corners(100.0, 200.0, 140.0, 220.0);
        let m = AppearancePlacement::new(&bbox, &turn, &rect).map_or(Matrix::IDENTITY, |p| p.to_user_space());
        // (0, 0) turns to (0, 0), the box's right edge: x 140. (10, 20) turns to (−20, 10).
        assert_eq!(
            (mapped(&m, 0.0, 0.0), mapped(&m, 10.0, 20.0)),
            ((140.0, 200.0), (100.0, 220.0))
        );
    }

    #[test]
    fn a_box_of_no_area_maps_nothing() {
        let flat = UserSpaceRect::from_corners(0.0, 0.0, 10.0, 0.0);
        let rect = UserSpaceRect::from_corners(0.0, 0.0, 10.0, 10.0);
        assert!(AppearancePlacement::new(&flat, &Matrix::IDENTITY, &rect).is_none());
    }
}
