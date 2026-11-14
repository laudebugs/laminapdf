//! A page moved onto a new media box through one matrix (E2 plan §2.1): what `ResizePages` and
//! `SetOrientation { swapBox }` share. The matrix maps the page's old user space to its new one,
//! so everything drawn or placed in user space follows it: the content (wrapped and given a
//! `cm`, through `laminapdf-content`'s append API), the bleed, trim and art boxes, and the
//! annotations' geometry (ISO 32000-2 §12.5.2 and the subtypes' entries). An annotation's
//! appearance is fitted to its `/Rect` (§12.5.5), so a scale needs nothing more; a quarter turn
//! also turns each appearance stream's `/Matrix`.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{Matrix, PageFrame, PageId, Rotation, UserSpaceRect};
use laminapdf_syntax::format_real;

use super::cleanup::{edit_dict, edit_object};
use crate::error::{Error, Result};

/// Where a page goes: its new displayed size, in points, and how its displayed content moves
/// (a page-space matrix: x right, y down from the top-left of the page as shown).
#[derive(Debug, Clone, Copy)]
pub(crate) struct Reshape {
    /// The new page as displayed, in points.
    pub(crate) width: f64,
    /// The new page as displayed, in points.
    pub(crate) height: f64,
    /// The displayed content's move, in page space.
    pub(crate) page_space: Matrix,
}

/// What a reshape did to one page: the matrix from its old user space to its new one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Reshaped {
    pub(crate) user_space: Matrix,
}

/// Annotation flag `NoRotate` (§12.5.3): such an annotation keeps its appearance upright.
const NO_ROTATE: i64 = 1 << 4;

/// The page's frame, its own dictionary and its object.
fn page_parts(doc: &laminapdf_model::Document, id: &PageId) -> Result<(PageFrame, laminapdf_cos::Page)> {
    let pages = doc.pages()?;
    let page = pages.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
    Ok((page.frame()?, page.cos().clone()))
}

/// The user-space matrix that moves page `id` as `shape` says, and its new media box.
pub(crate) fn plan(doc: &laminapdf_model::Document, id: &PageId, shape: &Reshape) -> Result<(Matrix, UserSpaceRect)> {
    let (old, _) = page_parts(doc, id)?;
    let unit = old.user_unit();
    let quarter = matches!(old.rotate(), Rotation::Clockwise90 | Rotation::Clockwise270);
    let (w, h) = if quarter {
        (shape.height / unit, shape.width / unit)
    } else {
        (shape.width / unit, shape.height / unit)
    };
    let media = UserSpaceRect::from_corners(0.0, 0.0, w, h);
    let new = PageFrame::new(media, old.rotate(), unit);
    let to_page = old
        .to_user_space_matrix()
        .invert()
        .ok_or(Error::InvalidOp("the page's frame can't be inverted"))?;
    Ok((to_page.then(&shape.page_space).then(&new.to_user_space_matrix()), media))
}

/// Moves page `id` as `shape` says: content, boxes and annotations. `appearances` holds the
/// appearance streams already turned by this op, so one shared by two annotations turns once.
pub(crate) fn reshape(
    doc: &mut laminapdf_model::Document,
    id: &PageId,
    shape: &Reshape,
    appearances: &mut BTreeSet<ObjRef>,
) -> Result<Reshaped> {
    let (m, media) = plan(doc, id, shape)?;
    let (_, cos_page) = page_parts(doc, id)?;
    let r = id
        .as_lamina_id()
        .obj_ref()
        .ok_or_else(|| Error::DirectPage(id.clone()))?;
    {
        let mut writer = laminapdf_content::PageContentWriter::new(doc.cos_mut().edit(), &cos_page)?;
        writer.wrap_existing()?;
        writer.prepend(cm(&m).as_bytes())?;
    }
    let annots = doc.resolve(r)?.as_dict().and_then(|d| d.get("Annots").cloned());
    let turns = m.b.abs() > EPS || m.c.abs() > EPS;
    // Direct annotation dictionaries live in the array, held by the page or as their own object.
    let mut annot_refs = Vec::new();
    match annots {
        Some(Object::Ref(array)) => {
            let items = doc
                .resolve(array)?
                .as_array()
                .map(<[Object]>::to_vec)
                .unwrap_or_default();
            annot_refs.extend(items.iter().filter_map(Object::as_ref));
            edit_object(doc, array, |o| {
                if let Object::Array(items) = o {
                    for item in items.iter_mut() {
                        if let Object::Dict(d) = item {
                            map_annotation(d, &m);
                        }
                    }
                }
            })?;
        }
        Some(Object::Array(items)) => annot_refs.extend(items.iter().filter_map(Object::as_ref)),
        _ => {}
    }
    edit_dict(doc, r, |d| {
        d.insert("MediaBox", rect_object(&media));
        d.remove("CropBox");
        for key in ["BleedBox", "TrimBox", "ArtBox"] {
            let mapped = d
                .get(key)
                .and_then(rect_of)
                .and_then(|b| map_rect(&b, &m).intersect(&media));
            match mapped.filter(UserSpaceRect::has_area) {
                Some(b) => d.insert(key, rect_object(&b)),
                None => {
                    d.remove(key);
                }
            }
        }
        if let Some(Object::Array(items)) = d.get_mut("Annots") {
            for item in items.iter_mut() {
                if let Object::Dict(a) = item {
                    map_annotation(a, &m);
                }
            }
        }
    })?;
    for annot in annot_refs {
        let mut turned = Vec::new();
        edit_dict(doc, annot, |d| {
            map_annotation(d, &m);
            let upright = d.get("F").and_then(Object::as_int).is_some_and(|f| f & NO_ROTATE != 0);
            if turns && !upright {
                turned = appearance_streams(d);
            }
        })?;
        for stream in turned {
            if appearances.insert(stream) {
                turn_appearance(doc, stream, &m)?;
            }
        }
    }
    Ok(Reshaped { user_space: m })
}

const EPS: f64 = 1e-9;

/// `a b c d e f cm`, with a newline.
fn cm(m: &Matrix) -> String {
    let v = m.to_array().map(format_real);
    format!("{} {} {} {} {} {} cm\n", v[0], v[1], v[2], v[3], v[4], v[5])
}

/// A rectangle array as a rectangle (any corner order), `None` unless it is four numbers.
pub(crate) fn rect_of(o: &Object) -> Option<UserSpaceRect> {
    let n = numbers(o)?;
    match n.as_slice() {
        [x0, y0, x1, y1] => Some(UserSpaceRect::from_corners(*x0, *y0, *x1, *y1)),
        _ => None,
    }
}

/// An array of numbers, `None` if anything else is in it.
fn numbers(o: &Object) -> Option<Vec<f64>> {
    o.as_array()?.iter().map(Object::as_f64).collect()
}

/// `rect` through `m`, normalised (its four corners' bounding box).
pub(crate) fn map_rect(rect: &UserSpaceRect, m: &Matrix) -> UserSpaceRect {
    let corners = [
        m.apply(rect.x0, rect.y0),
        m.apply(rect.x1, rect.y0),
        m.apply(rect.x0, rect.y1),
        m.apply(rect.x1, rect.y1),
    ];
    let xs = corners.map(|c| c.0);
    let ys = corners.map(|c| c.1);
    let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
    let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
    UserSpaceRect::from_corners(min(xs), min(ys), max(xs), max(ys))
}

pub(crate) fn rect_object(rect: &UserSpaceRect) -> Object {
    Object::Array(rect.to_array().iter().map(|&v| Object::number(v)).collect())
}

/// A flat list of points (`x1 y1 x2 y2 …`) through `m`; anything that isn't one is left alone.
fn map_points(list: &mut Object, m: &Matrix) {
    let Some(values) = numbers(list) else { return };
    let (pairs, rest) = values.as_chunks::<2>();
    if !rest.is_empty() {
        return;
    }
    let mapped: Vec<Object> = pairs
        .iter()
        .flat_map(|&[x, y]| {
            let (x, y) = m.apply(x, y);
            [Object::number(x), Object::number(y)]
        })
        .collect();
    *list = Object::Array(mapped);
}

/// An annotation's geometry through `m`: `/Rect`, and the point lists of the subtypes that have
/// them (`/QuadPoints`, `/InkList`'s paths, `/Vertices`, `/CL`, `/L`).
fn map_annotation(d: &mut Dict, m: &Matrix) {
    if let Some(rect) = d.get("Rect").and_then(rect_of) {
        d.insert("Rect", rect_object(&map_rect(&rect, m)));
    }
    for key in ["QuadPoints", "Vertices", "CL", "L"] {
        if let Some(points) = d.get_mut(key) {
            map_points(points, m);
        }
    }
    if let Some(Object::Array(paths)) = d.get_mut("InkList") {
        for path in paths.iter_mut() {
            map_points(path, m);
        }
    }
}

/// The appearance streams an annotation's `/AP` names directly (`/N`, `/R`, `/D`, each a stream
/// or a dictionary of streams by state).
fn appearance_streams(d: &Dict) -> Vec<ObjRef> {
    let Some(Object::Dict(ap)) = d.get("AP") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for key in ["N", "R", "D"] {
        match ap.get(key) {
            Some(Object::Ref(r)) => out.push(*r),
            Some(Object::Dict(states)) => out.extend(states.iter().filter_map(|(_, v)| v.as_ref())),
            _ => {}
        }
    }
    out
}

/// Turns appearance stream `r` with the turn in `m` (its linear part; the scale is undone by
/// fitting the appearance to the new `/Rect`, and the translation means nothing there). A
/// reference to something that isn't a stream (a states dictionary given indirectly) is left
/// alone.
fn turn_appearance(doc: &mut laminapdf_model::Document, r: ObjRef, m: &Matrix) -> Result<()> {
    let linear = Matrix::new([m.a, m.b, m.c, m.d, 0.0, 0.0]);
    edit_object(doc, r, |o| {
        if let Object::Stream(s) = o {
            let current = s
                .dict
                .get("Matrix")
                .and_then(numbers)
                .and_then(|v| Matrix::from_slice(&v))
                .unwrap_or(Matrix::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]));
            let turned = current.then(&linear);
            s.dict.insert(
                "Matrix",
                Object::Array(turned.to_array().iter().map(|&v| Object::number(v)).collect()),
            );
        }
    })
}
