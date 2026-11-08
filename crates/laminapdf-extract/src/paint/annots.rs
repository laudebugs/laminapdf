//! The covers of a page's annotations: viewers draw an annotation's normal appearance over the
//! page (ISO 32000-2 §12.5.5), so its opaque areas cover every glyph under them. Annotations
//! flagged Hidden or NoView (§12.5.3), popups, those with `/CA` below 1 and those without an
//! appearance stream are left out. A malformed annotation or appearance is skipped (ADR 0003).

use laminapdf_content::{interpret_content, transformed_box};
use laminapdf_cos::{Result, absent_if_malformed};
use laminapdf_model::{Document, Matrix, Page, PageFrame, UserSpaceRect};
use laminapdf_syntax::{Dict, Object, Stream};

use super::{Collector, Cover};

/// `/F` bit 2: Hidden.
const HIDDEN: i64 = 1 << 1;
/// `/F` bit 6: NoView.
const NO_VIEW: i64 = 1 << 5;

/// The covers of `page`'s annotations, each drawn after the whole page.
///
/// # Errors
/// What [`absent_if_malformed`] passes on: a reference cycle, a limit, strict mode, pending
/// data or a source error.
pub(crate) fn covers(doc: &Document, page: &Page<'_>, frame: &PageFrame) -> Result<Vec<Cover>> {
    let mut out = Vec::new();
    let Some(annots) = absent_if_malformed(page.annots())? else {
        return Ok(out);
    };
    for annot in annots {
        if let Some(covers) = absent_if_malformed(annotation(doc, annot, frame))? {
            out.extend(covers);
        }
    }
    Ok(out)
}

/// `value` as numbers, each read through a reference; `None` unless all are numbers.
fn numbers(doc: &Document, value: Option<&Object>) -> Result<Option<Vec<f64>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let array = doc.deref(value)?;
    let Some(items) = array.as_array() else {
        return Ok(None);
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match doc.deref(item)?.as_f64() {
            Some(v) => out.push(v),
            None => return Ok(None),
        }
    }
    Ok(Some(out))
}

fn rect(values: &[f64]) -> Option<UserSpaceRect> {
    match *values {
        [x0, y0, x1, y1] => Some(UserSpaceRect::from_corners(x0, y0, x1, y1)),
        _ => None,
    }
}

/// The normal appearance stream (`/AP /N`, or its entry `/AS` names), when the annotation is
/// shown on screen at full opacity.
fn appearance(doc: &Document, dict: &Dict) -> Result<Option<Stream>> {
    let name = |key: &str| match dict.get(key) {
        Some(Object::Name(n)) => Some(n.as_bytes().to_vec()),
        _ => None,
    };
    if name("Subtype").as_deref() == Some(b"Popup") {
        return Ok(None);
    }
    let flags = match dict.get("F") {
        Some(f) => doc.deref(f)?.as_int().unwrap_or(0),
        None => 0,
    };
    if flags & (HIDDEN | NO_VIEW) != 0 {
        return Ok(None);
    }
    if let Some(ca) = dict.get("CA") {
        if doc.deref(ca)?.as_f64().is_none_or(|ca| ca < 1.0) {
            return Ok(None);
        }
    }
    let Some(ap) = dict.get("AP") else {
        return Ok(None);
    };
    let ap = doc.deref(ap)?;
    let Some(normal) = ap.as_dict().and_then(|ap| ap.get("N")) else {
        return Ok(None);
    };
    let normal = doc.deref(normal)?;
    let stream = match &*normal {
        Object::Stream(s) => Some(s.clone()),
        Object::Dict(states) => match name("AS").and_then(|state| states.get(&String::from_utf8_lossy(&state))) {
            Some(chosen) => match &*doc.deref(chosen)? {
                Object::Stream(s) => Some(s.clone()),
                _ => None,
            },
            None => None,
        },
        _ => None,
    };
    Ok(stream)
}

/// The matrix that draws an appearance whose `/BBox` is `bbox` and `/Matrix` is `matrix` into
/// the annotation's `rect` (§12.5.5): the matrix, then the scale and shift that fit the
/// transformed box onto the rectangle.
pub(crate) fn appearance_matrix(bbox: &UserSpaceRect, matrix: &Matrix, rect: &UserSpaceRect) -> Option<Matrix> {
    let boxed = transformed_box(bbox, matrix);
    let (w, h) = (boxed.x1 - boxed.x0, boxed.y1 - boxed.y0);
    if w <= 0.0 || h <= 0.0 || !w.is_finite() || !h.is_finite() {
        return None;
    }
    let fit = Matrix::translate(-boxed.x0, -boxed.y0)
        .then(&Matrix::scale((rect.x1 - rect.x0) / w, (rect.y1 - rect.y0) / h))
        .then(&Matrix::translate(rect.x0, rect.y0));
    Some(matrix.then(&fit))
}

fn annotation(doc: &Document, annot: laminapdf_syntax::ObjRef, frame: &PageFrame) -> Result<Vec<Cover>> {
    let object = doc.resolve(annot)?;
    let Some(dict) = object.as_dict() else {
        return Ok(Vec::new());
    };
    let Some(stream) = appearance(doc, dict)? else {
        return Ok(Vec::new());
    };
    let (Some(target), Some(bbox)) = (
        numbers(doc, dict.get("Rect"))?.as_deref().and_then(rect),
        numbers(doc, stream.dict.get("BBox"))?.as_deref().and_then(rect),
    ) else {
        return Ok(Vec::new());
    };
    let matrix = match numbers(doc, stream.dict.get("Matrix"))?.as_deref() {
        Some(&[a, b, c, d, e, f]) => Matrix::new([a, b, c, d, e, f]),
        _ => Matrix::IDENTITY,
    };
    let Some(outer) = appearance_matrix(&bbox, &matrix, &target) else {
        return Ok(Vec::new());
    };
    let content = doc.decode_stream(&stream)?;
    let resources = match stream.dict.get("Resources") {
        Some(r) => Some(doc.deref(r)?.into_arc()),
        None => None,
    };
    let mut collector = Collector::appearance(frame, outer, frame.rect_to_page(&target));
    interpret_content(doc, &content, resources, &mut collector)?;
    Ok(collector.finish().1)
}

#[cfg(test)]
mod tests {
    use laminapdf_model::{Matrix, UserSpaceRect};

    use super::appearance_matrix;

    fn mapped(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
        let (x, y) = m.apply(x, y);
        ((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0)
    }

    #[test]
    fn the_box_maps_onto_the_rect() {
        let bbox = UserSpaceRect::from_corners(0.0, 0.0, 10.0, 20.0);
        let rect = UserSpaceRect::from_corners(100.0, 200.0, 120.0, 220.0);
        let m = appearance_matrix(&bbox, &Matrix::IDENTITY, &rect).unwrap_or(Matrix::IDENTITY);
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
        let m = appearance_matrix(&bbox, &turn, &rect).unwrap_or(Matrix::IDENTITY);
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
        assert!(appearance_matrix(&flat, &Matrix::IDENTITY, &rect).is_none());
    }
}
