//! Destinations (§12.3.2), URI actions (§12.6.4.8) and named destinations (`/Names /Dests`,
//! §7.7.4, §7.9.6).

use std::collections::BTreeMap;

use lamina_cos::{Dict, Name, ObjRef, Object, PdfString};
use lamina_model::{PageFrame, PointPt, Pt, RectPt};

use crate::error::{Error, Result};
use crate::page::{number, rect_object};

/// How a destination shows its page (Table 149). Positions are page space ([ADR 0027]):
/// points from the page's displayed top-left; `None` leaves that coordinate as the viewer
/// has it (`null`).
///
/// [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Fit {
    /// `/Fit`: the whole page in the window.
    Page,
    /// `/FitH`: the page's width, with `top` at the top of the window.
    Width {
        /// The position at the window's top edge.
        top: Option<Pt>,
    },
    /// `/FitV`: the page's height, with `left` at the window's left edge.
    Height {
        /// The position at the window's left edge.
        left: Option<Pt>,
    },
    /// `/XYZ`: `(left, top)` at the window's top-left corner, at `zoom` (1 = 100 %; `None`
    /// keeps the current zoom).
    Xyz {
        /// The x at the window's left edge.
        left: Option<Pt>,
        /// The y at the window's top edge.
        top: Option<Pt>,
        /// The zoom factor.
        zoom: Option<f64>,
    },
    /// `/FitR`: the rectangle in the window.
    Rect(RectPt),
    /// `/FitB`: the page's bounding box of content.
    Bounds,
}

/// Where a link or outline item goes.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum LinkTarget {
    /// A URI action (`/S /URI`, §12.6.4.8). Characters outside printable ASCII are
    /// percent-encoded, as `/URI` is a 7-bit ASCII string.
    Uri(String),
    /// A page of this document by its 0-based index (resolved when the build finishes, so a
    /// page added later can be named), shown as `fit` says.
    Page {
        /// The page's 0-based index in the document.
        index: u32,
        /// How the page is shown.
        fit: Fit,
    },
    /// A named destination ([`Builder::destination`](crate::Builder::destination)).
    Named(String),
}

impl LinkTarget {
    /// A page target showing the whole page.
    #[must_use]
    pub const fn page(index: u32) -> Self {
        Self::Page { index, fit: Fit::Page }
    }
}

/// `/URI`: printable ASCII as it is, every other byte percent-encoded (RFC 3987 §3.1).
fn uri_bytes(uri: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(uri.len());
    for &b in uri.as_bytes() {
        if (0x21..0x7f).contains(&b) {
            out.push(b);
        } else {
            out.extend_from_slice(format!("%{b:02X}").as_bytes());
        }
    }
    out
}

/// A URI action dictionary.
pub(crate) fn uri_action(uri: &str) -> Dict {
    let mut d = Dict::new();
    d.insert("S", Name::from("URI"));
    d.insert("URI", PdfString::literal(uri_bytes(uri)));
    d
}

fn optional(v: Option<f64>) -> Object {
    v.map_or(Object::Null, number)
}

/// An explicit destination array for `page` (§12.3.2.2), its coordinates in the page's user
/// space.
pub(crate) fn dest_array(page: ObjRef, frame: &PageFrame, fit: &Fit) -> Object {
    // Page-space x is user-space x (or y) depending on whether the page is turned sideways.
    let upright = frame.rotate().degrees().is_multiple_of(180);
    let point = |x: Option<Pt>, y: Option<Pt>| {
        let p = frame.to_user_space(PointPt::new(x.unwrap_or(Pt::ZERO), y.unwrap_or(Pt::ZERO)));
        let (along_x, along_y) = if upright { (x, y) } else { (y, x) };
        (along_x.map(|_| p.x), along_y.map(|_| p.y))
    };
    let mut array = vec![Object::Ref(page)];
    match fit {
        Fit::Page => array.push(Object::name("Fit")),
        Fit::Bounds => array.push(Object::name("FitB")),
        Fit::Width { top } => {
            let (ux, uy) = point(None, *top);
            array.push(Object::name("FitH"));
            array.push(optional(if upright { uy } else { ux }));
        }
        Fit::Height { left } => {
            let (ux, uy) = point(*left, None);
            array.push(Object::name("FitV"));
            array.push(optional(if upright { ux } else { uy }));
        }
        Fit::Xyz { left, top, zoom } => {
            let (ux, uy) = point(*left, *top);
            array.push(Object::name("XYZ"));
            array.push(optional(ux));
            array.push(optional(uy));
            array.push(optional(*zoom));
        }
        Fit::Rect(r) => {
            array.push(Object::name("FitR"));
            if let Object::Array(corners) = rect_object(&frame.rect_to_user_space(r)) {
                array.extend(corners);
            }
        }
    }
    Object::Array(array)
}

/// Whether every coordinate of `fit` is finite.
pub(crate) fn fit_is_finite(fit: &Fit) -> bool {
    let finite = |v: Option<Pt>| v.is_none_or(|v| v.get().is_finite());
    match fit {
        Fit::Page | Fit::Bounds => true,
        Fit::Width { top } => finite(*top),
        Fit::Height { left } => finite(*left),
        Fit::Xyz { left, top, zoom } => finite(*left) && finite(*top) && zoom.is_none_or(f64::is_finite),
        Fit::Rect(r) => r.is_finite(),
    }
}

/// What link targets resolve against when the build finishes: the document's pages and,
/// for a new document, its named destinations (an open document's names aren't checked).
pub(crate) struct Targets<'a> {
    pub(crate) pages: &'a [(ObjRef, PageFrame)],
    pub(crate) names: Option<&'a BTreeMap<String, (u32, Fit)>>,
}

impl Targets<'_> {
    /// The destination array for page `index`.
    pub(crate) fn page(&self, index: u32, fit: &Fit) -> Result<Object> {
        if !fit_is_finite(fit) {
            return Err(Error::InvalidGeometry {
                what: "a destination coordinate is not a finite number",
            });
        }
        let (page, frame) =
            usize::try_from(index)
                .ok()
                .and_then(|i| self.pages.get(i))
                .ok_or(Error::PageIndexOutOfRange {
                    index,
                    count: self.pages.len(),
                })?;
        Ok(dest_array(*page, frame, fit))
    }

    /// Puts `target` into `dict`: `/A` for a URI, `/Dest` for a page or a name.
    pub(crate) fn apply(&self, dict: &mut Dict, target: &LinkTarget) -> Result<()> {
        match target {
            LinkTarget::Uri(uri) => dict.insert("A", uri_action(uri)),
            LinkTarget::Page { index, fit } => dict.insert("Dest", self.page(*index, fit)?),
            LinkTarget::Named(name) => {
                if self.names.is_some_and(|names| !names.contains_key(name)) {
                    return Err(Error::UnknownDestination { name: name.clone() });
                }
                dict.insert("Dest", PdfString::literal(name.as_bytes()));
            }
        }
        Ok(())
    }

    /// The `/Dests` name tree (a single leaf: `/Names` sorted by key bytes, as a `BTreeMap`
    /// of UTF-8 strings is).
    pub(crate) fn dests_tree(&self) -> Result<Option<Dict>> {
        let Some(names) = self.names.filter(|n| !n.is_empty()) else {
            return Ok(None);
        };
        let mut entries = Vec::with_capacity(names.len() * 2);
        for (name, (index, fit)) in names {
            entries.push(Object::String(PdfString::literal(name.as_bytes())));
            entries.push(self.page(*index, fit)?);
        }
        let mut tree = Dict::new();
        tree.insert("Names", Object::Array(entries));
        Ok(Some(tree))
    }
}

#[cfg(test)]
mod tests {
    use lamina_model::{Rotation, UserSpaceRect};

    use super::*;

    fn letter(rotate: Rotation) -> PageFrame {
        PageFrame::new(UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0), rotate, 1.0)
    }

    #[test]
    fn xyz_maps_the_page_space_top_left_to_user_space() {
        let fit = Fit::Xyz {
            left: Some(Pt::new(10.0)),
            top: Some(Pt::new(20.0)),
            zoom: None,
        };
        let array = dest_array(ObjRef::new(3, 0), &letter(Rotation::None), &fit);
        assert_eq!(
            array,
            Object::Array(vec![
                Object::Ref(ObjRef::new(3, 0)),
                Object::name("XYZ"),
                Object::Integer(10),
                Object::Integer(772),
                Object::Null
            ])
        );
    }

    #[test]
    fn fit_h_on_a_page_turned_sideways_uses_the_user_x() {
        let fit = Fit::Width {
            top: Some(Pt::new(0.0)),
        };
        let Object::Array(a) = dest_array(ObjRef::new(3, 0), &letter(Rotation::Clockwise90), &fit) else {
            panic!("an array");
        };
        assert_eq!(a.get(1..), Some(&[Object::name("FitH"), Object::Integer(0)][..]));
    }

    #[test]
    fn uris_are_percent_encoded_outside_printable_ascii() {
        assert_eq!(uri_bytes("https://e.x/a b/ü"), b"https://e.x/a%20b/%C3%BC");
    }
}
