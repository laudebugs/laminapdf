//! Structure elements that outlive their page: opened on one page with
//! [`Surface::open_element`](crate::Surface::open_element), given content on that page and later
//! ones with [`Surface::resume`](crate::Surface::resume), and written by
//! [`Builder::close_element`](crate::Builder::close_element).
//!
//! An open element's object number is reserved when its first page is written. Content on that
//! page joins its `/K` as plain MCIDs, since its `/Pg` is that page; content on later pages joins
//! as marked-content references with their own `/Pg` (§14.7.5.2, Table 357), and links there as
//! `OBJR`s with their own `/Pg` (§14.7.5.3). Each page's `ParentTree` entries name the element
//! like any other.

use std::sync::atomic::{AtomicU32, Ordering};

use laminapdf_cos::{Dict, ObjRef, Object};

use super::Tag;

/// Builder identities, unique within the process. Only compared, never written to output.
static NEXT_BUILDER: AtomicU32 = AtomicU32::new(0);

/// A new builder identity.
pub(crate) fn next_builder() -> u32 {
    NEXT_BUILDER.fetch_add(1, Ordering::Relaxed)
}

/// A structure element that can continue on later pages, from
/// [`Surface::open_element`](crate::Surface::open_element). It belongs to the builder whose page
/// opened it, and stays open until [`Builder::close_element`](crate::Builder::close_element).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ElementId {
    builder: u32,
    /// Which page builder opened it, counted from 0 over the build (a page that fails keeps its
    /// number, so its elements are never mistaken for another page's).
    page: u32,
    /// Which element of that page, in opening order.
    index: u32,
}

/// What a page's surface needs to name the elements it opens.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ElementIds {
    builder: u32,
    page: u32,
    next: u32,
}

impl ElementIds {
    pub(crate) const fn new(builder: u32, page: u32) -> Self {
        Self { builder, page, next: 0 }
    }

    /// The next element's identity.
    pub(crate) const fn next(&mut self) -> ElementId {
        let id = ElementId {
            builder: self.builder,
            page: self.page,
            index: self.next,
        };
        self.next = self.next.saturating_add(1);
        id
    }
}

/// An element opened on a written page and not yet closed.
#[derive(Debug, Clone)]
struct OpenElem {
    id: ElementId,
    object: ObjRef,
    tag: Tag,
    /// Its entries, with `/K` empty until it closes.
    dict: Dict,
    kids: Vec<Object>,
    /// Its index in the build's list of every element.
    info: usize,
}

/// Where the table was, to go back to if writing a page fails.
#[derive(Debug, Clone, Default)]
pub(crate) struct OpenMark {
    elems: usize,
    kids: Vec<usize>,
}

/// The elements open now, in opening order.
#[derive(Debug, Clone, Default)]
pub(crate) struct OpenElems {
    elems: Vec<OpenElem>,
}

impl OpenElems {
    pub(crate) fn mark(&self) -> OpenMark {
        OpenMark {
            elems: self.elems.len(),
            kids: self.elems.iter().map(|e| e.kids.len()).collect(),
        }
    }

    /// Forgets the elements opened and the content added since `mark`.
    pub(crate) fn rollback(&mut self, mark: &OpenMark) {
        self.elems.truncate(mark.elems);
        for (e, &len) in self.elems.iter_mut().zip(&mark.kids) {
            e.kids.truncate(len);
        }
    }

    /// Each open element and its tag, for the next page's marking.
    pub(crate) fn tags(&self) -> Vec<(ElementId, Tag)> {
        self.elems.iter().map(|e| (e.id, e.tag.clone())).collect()
    }

    /// How many are open.
    pub(crate) fn len(&self) -> usize {
        self.elems.len()
    }

    fn get(&self, id: ElementId) -> Option<&OpenElem> {
        self.elems.iter().find(|e| e.id == id)
    }

    /// The object and element index of open element `id`.
    pub(crate) fn find(&self, id: ElementId) -> Option<(ObjRef, usize)> {
        self.get(id).map(|e| (e.object, e.info))
    }

    /// Adds an element opened on the page being written, with its content there.
    pub(crate) fn open(&mut self, id: ElementId, object: ObjRef, tag: Tag, dict: Dict, kids: Vec<Object>, info: usize) {
        self.elems.push(OpenElem {
            id,
            object,
            tag,
            dict,
            kids,
            info,
        });
    }

    /// Adds content from a later page to open element `id`.
    pub(crate) fn extend(&mut self, id: ElementId, kids: Vec<Object>) {
        if let Some(e) = self.elems.iter_mut().find(|e| e.id == id) {
            e.kids.extend(kids);
        }
    }

    /// Removes open element `id`, returning its object and complete dictionary.
    pub(crate) fn close(&mut self, id: ElementId) -> Option<(ObjRef, Dict)> {
        let at = self.elems.iter().position(|e| e.id == id)?;
        let e = self.elems.remove(at);
        let mut dict = e.dict;
        dict.insert("K", Object::Array(e.kids));
        Some((e.object, dict))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TagKind;

    fn id(index: u32) -> ElementId {
        ElementId {
            builder: 0,
            page: 0,
            index,
        }
    }

    fn table() -> OpenElems {
        let mut open = OpenElems::default();
        open.open(
            id(0),
            ObjRef::new(5, 0),
            Tag::new(TagKind::P),
            Dict::new(),
            vec![Object::Integer(0)],
            2,
        );
        open
    }

    #[test]
    fn ids_of_one_page_count_from_zero() {
        let mut ids = ElementIds::new(3, 7);
        let [a, b] = [ids.next(), ids.next()];
        assert_eq!((a.index, b.index, b.page, b.builder), (0, 1, 7, 3));
    }

    #[test]
    fn closing_writes_every_kid_in_order() {
        let mut open = table();
        open.extend(id(0), vec![Object::Integer(9)]);
        let k = open.close(id(0)).and_then(|(_, d)| d.get("K").cloned());
        assert_eq!(k, Some(Object::Array(vec![Object::Integer(0), Object::Integer(9)])));
    }

    #[test]
    fn a_closed_element_is_no_longer_open() {
        let mut open = table();
        open.close(id(0));
        assert_eq!((open.find(id(0)), open.close(id(0)).is_none()), (None, true));
    }

    #[test]
    fn rolling_back_forgets_later_content_and_elements() {
        let mut open = table();
        let mark = open.mark();
        open.extend(id(0), vec![Object::Integer(9)]);
        open.open(
            id(1),
            ObjRef::new(6, 0),
            Tag::new(TagKind::P),
            Dict::new(),
            Vec::new(),
            3,
        );
        open.rollback(&mark);
        let k = open.close(id(0)).and_then(|(_, d)| d.get("K").cloned());
        assert_eq!((open.len(), k), (0, Some(Object::Array(vec![Object::Integer(0)]))));
    }
}
