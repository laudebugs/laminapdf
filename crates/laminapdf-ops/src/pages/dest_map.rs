//! The destinations and structure boxes that follow a page moved through a matrix (E2 plan
//! §2.1): every explicit destination naming the page, wherever it is held (outline items,
//! annotations' `/Dest` and `GoTo` actions, the `/Names /Dests` tree, the PDF 1.1 `/Dests`
//! dictionary and `/OpenAction`), and the Layout `/BBox` attribute of each structure element
//! whose `/Pg` is the page (ISO 32000-2 §14.8.5.4.3).

use std::collections::BTreeMap;

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{Matrix, PageId, ReferenceKind, Via};

use super::cleanup::{
    Holder, NameStoreKey, edit_dict, edit_object, named, resolve, set_old_destination, set_tree_destinations,
    write_dest,
};
use super::reshape::{map_rect, rect_object, rect_of};
use crate::error::Result;

/// How far from zero a matrix entry may be and still count as zero: the ops' matrices are
/// scales and quarter turns, whose zeros may come out of `sin` as 1e-17.
const EPS: f64 = 1e-9;

/// A name's bytes, `None` for anything else.
fn name_of(o: &Object) -> Option<&[u8]> {
    match o {
        Object::Name(n) => Some(n.as_bytes()),
        _ => None,
    }
}

/// One coordinate of a destination: a number, or `null` (the viewer keeps its own).
type Coord = Option<f64>;

/// `x` through the axis-aligned matrix `m`'s x part, `null` staying `null`.
fn along(c: Coord, scale: f64, offset: f64) -> Object {
    c.map_or(Object::Null, |v| Object::number(scale.mul_add(v, offset)))
}

/// A destination operand: a number, `null`, or (for anything else) `None`.
fn coord(o: Option<&Object>) -> Option<Coord> {
    match o {
        None | Some(Object::Null) => Some(None),
        Some(o) => o.as_f64().map(Some),
    }
}

/// Explicit destination `dest` (`[page /Kind …]`) with its coordinates mapped through `m`
/// (ISO 32000-2 §12.3.2.2): a point for `/XYZ` (zoom kept), the top for `/FitH` and `/FitBH`,
/// the left for `/FitV` and `/FitBV`, the rectangle, normalised, for `/FitR`; `/Fit` and
/// `/FitB` name no coordinate. A quarter turn swaps the axes, so a `/FitH` becomes a `/FitV`
/// and the other way round. `null` coordinates stay `null`. A destination it can't read, or a
/// matrix that is neither axis-aligned nor a quarter turn, leaves it as it was (`None`).
pub(crate) fn map_dest(dest: &[Object], m: &Matrix) -> Option<Vec<Object>> {
    let (page, kind) = (dest.first()?.clone(), name_of(dest.get(1)?)?);
    let aligned = m.b.abs() < EPS && m.c.abs() < EPS;
    let turned = m.a.abs() < EPS && m.d.abs() < EPS;
    if !aligned && !turned {
        return None;
    }
    // x' and y' from x, or from y when the axes swap.
    let x_of = |x: Coord, y: Coord| {
        if aligned {
            along(x, m.a, m.e)
        } else {
            along(y, m.c, m.e)
        }
    };
    let y_of = |x: Coord, y: Coord| {
        if aligned {
            along(y, m.d, m.f)
        } else {
            along(x, m.b, m.f)
        }
    };
    let name = |n: &str| Object::name(n);
    let rest: Vec<Object> = match kind {
        b"XYZ" => {
            let (x, y) = (coord(dest.get(2))?, coord(dest.get(3))?);
            let zoom = dest.get(4).cloned().unwrap_or(Object::Null);
            vec![name("XYZ"), x_of(x, y), y_of(x, y), zoom]
        }
        b"FitH" | b"FitBH" => {
            let top = coord(dest.get(2))?;
            let bounded = kind == b"FitBH";
            if aligned {
                vec![name(if bounded { "FitBH" } else { "FitH" }), y_of(None, top)]
            } else {
                vec![name(if bounded { "FitBV" } else { "FitV" }), x_of(None, top)]
            }
        }
        b"FitV" | b"FitBV" => {
            let left = coord(dest.get(2))?;
            let bounded = kind == b"FitBV";
            if aligned {
                vec![name(if bounded { "FitBV" } else { "FitV" }), x_of(left, None)]
            } else {
                vec![name(if bounded { "FitBH" } else { "FitH" }), y_of(left, None)]
            }
        }
        b"FitR" => {
            let rect = rect_of(&Object::Array(dest.get(2..6)?.to_vec()))?;
            let mapped = map_rect(&rect, m);
            let mut out = vec![name("FitR")];
            out.extend(mapped.to_array().iter().map(|&v| Object::number(v)));
            out
        }
        b"Fit" | b"FitB" => return Some(dest.to_vec()),
        _ => return None,
    };
    let mut out = vec![page];
    out.extend(rest);
    Some(out)
}

/// The explicit destination array `value` holds: the array itself, or a `<< /D [...] >>`
/// destination dictionary's (one reference followed each time).
fn dest_array(doc: &laminapdf_model::Document, value: Option<Object>) -> Result<Option<Vec<Object>>> {
    Ok(match resolve(doc, value)? {
        Some(Object::Array(items)) => Some(items),
        Some(Object::Dict(d)) => match resolve(doc, d.get("D").cloned())? {
            Some(Object::Array(items)) => Some(items),
            _ => None,
        },
        _ => None,
    })
}

/// The destination the holder keeps where `via` says: its `/Dest`, its `GoTo` action's `/D`,
/// or (for the catalog) `/OpenAction` or that action's `/D`.
fn held(doc: &laminapdf_model::Document, holder: Holder, via: &Via) -> Result<Option<Vec<Object>>> {
    let dict = match holder {
        Holder::Catalog => doc.catalog()?,
        Holder::Object(r) => match doc.resolve(r)?.as_dict() {
            Some(d) => d.clone(),
            None => return Ok(None),
        },
    };
    let value = match (holder, via) {
        (Holder::Catalog, Via::Dest { .. }) => dict.get("OpenAction").cloned(),
        (Holder::Object(_), Via::Dest { .. }) => dict.get("Dest").cloned(),
        (_, Via::GoTo { action: Some(a), .. }) => {
            return dest_array(doc, doc.resolve(*a)?.as_dict().and_then(|d| d.get("D").cloned()));
        }
        (Holder::Catalog, Via::GoTo { action: None, .. }) => match resolve(doc, dict.get("OpenAction").cloned())? {
            Some(Object::Dict(action)) => action.get("D").cloned(),
            _ => None,
        },
        (Holder::Object(_), Via::GoTo { action: None, .. }) => match resolve(doc, dict.get("A").cloned())? {
            Some(Object::Dict(action)) => action.get("D").cloned(),
            _ => None,
        },
        _ => None,
    };
    dest_array(doc, value)
}

/// The value named destination `name` has in `store`.
fn named_value(doc: &laminapdf_model::Document, name: &[u8], store: NameStoreKey) -> Result<Option<Object>> {
    Ok(match store {
        NameStoreKey::NameTree => match doc.name_tree("Dests")? {
            Some(tree) => tree.entries()?.into_iter().find(|(n, _)| n == name).map(|(_, v)| v),
            None => None,
        },
        NameStoreKey::DestsDict => match resolve(doc, doc.catalog()?.get("Dests").cloned())? {
            Some(Object::Dict(d)) => d.iter().find(|(k, _)| k.0 == name).map(|(_, v)| v.clone()),
            _ => None,
        },
    })
}

/// Maps every destination and structure `/BBox` that names a page of `moved` through that
/// page's matrix (the matrix from its old user space to its new one).
pub(crate) fn follow(doc: &mut laminapdf_model::Document, moved: &BTreeMap<PageId, Matrix>) -> Result<()> {
    if moved.is_empty() {
        return Ok(());
    }
    let pages: Vec<PageId> = moved.keys().cloned().collect();
    let references = doc.page_references(&pages)?;
    let mut tree: BTreeMap<Vec<u8>, Option<Object>> = BTreeMap::new();
    for reference in references.entries() {
        let Some(m) = moved.get(&reference.page) else { continue };
        let holder = match &reference.kind {
            ReferenceKind::OutlineItem { item, via } if named(via).is_none() => {
                item.as_lamina_id().obj_ref().map(|r| (Holder::Object(r), via))
            }
            ReferenceKind::Link { annot, via, .. } if named(via).is_none() => Some((Holder::Object(*annot), via)),
            ReferenceKind::OpenAction { via } if named(via).is_none() => Some((Holder::Catalog, via)),
            ReferenceKind::NamedDestination { name, store } => {
                let store = NameStoreKey::from(*store);
                let mapped = dest_array(doc, named_value(doc, name, store)?)?.and_then(|d| map_dest(&d, m));
                if let Some(dest) = mapped {
                    match store {
                        NameStoreKey::NameTree => {
                            tree.insert(name.clone(), Some(Object::Array(dest)));
                        }
                        NameStoreKey::DestsDict => set_old_destination(doc, name, Some(&Object::Array(dest)))?,
                    }
                }
                None
            }
            ReferenceKind::StructElem { elem: Some(elem) } => {
                map_layout_boxes(doc, *elem, m)?;
                None
            }
            _ => None,
        };
        if let Some((holder, via)) = holder
            && let Some(dest) = held(doc, holder, via)?.and_then(|d| map_dest(&d, m))
        {
            write_dest(doc, holder, via, Some(Object::Array(dest)), &[])?;
        }
    }
    set_tree_destinations(doc, &tree)
}

/// The Layout attributes' `/BBox` of structure element `elem` through `m`: its `/A` is one
/// attribute dictionary or an array of them (with revision numbers between), each direct or
/// indirect.
fn map_layout_boxes(doc: &mut laminapdf_model::Document, elem: ObjRef, m: &Matrix) -> Result<()> {
    let mut indirect: Vec<ObjRef> = Vec::new();
    edit_dict(doc, elem, |d| {
        let Some(attributes) = d.get_mut("A") else { return };
        let mut each = |a: &mut Object| match a {
            Object::Dict(attr) => map_layout(attr, m),
            Object::Ref(r) => indirect.push(*r),
            _ => {}
        };
        match attributes {
            Object::Array(items) => items.iter_mut().for_each(&mut each),
            other => each(other),
        }
    })?;
    for r in indirect {
        edit_object(doc, r, |o| {
            if let Object::Dict(attr) = o {
                map_layout(attr, m);
            }
        })?;
    }
    Ok(())
}

/// A Layout attribute dictionary's `/BBox` through `m`; any other owner's is left alone.
fn map_layout(attr: &mut laminapdf_cos::Dict, m: &Matrix) {
    let layout = attr.get("O").and_then(name_of) == Some(b"Layout".as_slice());
    if let (true, Some(rect)) = (layout, attr.get("BBox").and_then(rect_of)) {
        attr.insert("BBox", rect_object(&map_rect(&rect, m)));
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

    use laminapdf_cos::{ObjRef, Object};
    use laminapdf_model::Matrix;

    use super::map_dest;

    fn dest(kind: &str, operands: &[Option<f64>]) -> Vec<Object> {
        let mut out = vec![Object::Ref(ObjRef::new(3, 0)), Object::name(kind)];
        out.extend(operands.iter().map(|o| o.map_or(Object::Null, Object::number)));
        out
    }

    /// Twice as large, then 10 right and 20 up.
    fn scale() -> Matrix {
        Matrix::new([2.0, 0.0, 0.0, 2.0, 10.0, 20.0])
    }

    /// A quarter turn: (x, y) → (-y + 100, x).
    fn turn() -> Matrix {
        Matrix::new([0.0, 1.0, -1.0, 0.0, 100.0, 0.0])
    }

    #[test]
    fn a_scale_maps_each_kind_of_destination() {
        let m = scale();
        let cases = [
            (
                dest("XYZ", &[Some(72.0), Some(700.0), Some(0.0)]),
                dest("XYZ", &[Some(154.0), Some(1420.0), Some(0.0)]),
            ),
            (
                dest("XYZ", &[None, Some(700.0), None]),
                dest("XYZ", &[None, Some(1420.0), None]),
            ),
            (dest("FitH", &[Some(500.0)]), dest("FitH", &[Some(1020.0)])),
            (dest("FitBV", &[Some(36.0)]), dest("FitBV", &[Some(82.0)])),
            (
                dest("FitR", &[Some(110.0), Some(10.0), Some(10.0), Some(60.0)]),
                dest("FitR", &[Some(30.0), Some(40.0), Some(230.0), Some(140.0)]),
            ),
            (dest("Fit", &[]), dest("Fit", &[])),
        ];
        for (before, after) in cases {
            assert_eq!(map_dest(&before, &m), Some(after));
        }
    }

    #[test]
    fn a_quarter_turn_swaps_the_axes() {
        let m = turn();
        assert_eq!(
            (
                map_dest(&dest("XYZ", &[Some(10.0), Some(30.0), None]), &m),
                map_dest(&dest("FitH", &[Some(30.0)]), &m),
                map_dest(&dest("FitV", &[Some(10.0)]), &m),
            ),
            (
                Some(dest("XYZ", &[Some(70.0), Some(10.0), None])),
                Some(dest("FitV", &[Some(70.0)])),
                Some(dest("FitH", &[Some(10.0)])),
            )
        );
    }

    #[test]
    fn an_unreadable_destination_is_left_alone() {
        let m = scale();
        assert_eq!(
            (
                map_dest(&[Object::Ref(ObjRef::new(3, 0))], &m),
                map_dest(&dest("Nowhere", &[Some(1.0)]), &m),
                map_dest(
                    &[Object::Ref(ObjRef::new(3, 0)), Object::name("XYZ"), Object::name("x")],
                    &m
                ),
            ),
            (None, None, None)
        );
    }
}
