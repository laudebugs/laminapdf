//! The document's structure elements, collected page by page, and the structure tree written
//! when the build finishes (§14.7.2): `/StructTreeRoot` with one `Document` element, each
//! element's `/S`, `/P`, `/Pg`, `/K` (MCIDs, `OBJR`s and child elements in reading order) and
//! attributes, the `ParentTree`, `/ParentTreeNextKey`, the `RoleMap`, and the catalog's
//! `/MarkInfo << /Marked true >>`.
//!
//! Each page's elements get their object numbers when the page is written, so their
//! dictionaries, and the page's `ParentTree` entries, are complete then; the build's end only
//! writes them out.

use laminapdf_cos::{CosMut, Dict, Name, ObjRef, Object, PdfString};

use super::mcid::{PageKid, PageMarks};
use super::parent_tree::ParentTree;
use super::rolemap::{is_standard, role_map};
use super::{ContentKind, Tag, TagDiagnostic};
use crate::error::Result;
use crate::text_string;

/// The structure tree root's and the `Document` element's object numbers.
#[derive(Debug, Clone, Copy)]
struct Roots {
    root: ObjRef,
    document: ObjRef,
}

/// The `/StructParents` key of a page, the `/StructParent` key of each of its links, and its
/// elements' object numbers.
#[derive(Debug, Default)]
pub(crate) struct PageKeys {
    pub(crate) page: Option<i64>,
    pub(crate) links: Vec<Option<i64>>,
    refs: Vec<ObjRef>,
}

/// One element's standard type and its parent's, in structure order (depth-first: elements
/// are opened parent first, siblings in order, pages in order), for the profiles' structure
/// checks.
///
/// The types are strings, not [`TagKind`](super::TagKind)s: a custom name that is a standard
/// type outside `TagKind` (`THead`, `TBody`, `H`, `H7`, `Formula`) is that type, and the
/// checks need it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ElemInfo {
    pub(crate) page: u32,
    /// The standard type: the name when it is one, else the role it maps to.
    pub(crate) kind: String,
    /// The parent's standard type; `None` under the `Document` element.
    pub(crate) parent: Option<String>,
    /// The parent's index in the list of every element; `None` under the `Document` element.
    pub(crate) parent_index: Option<usize>,
}

/// Where a page started, to forget it if writing it fails.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TagMark {
    tagged: bool,
    elems: usize,
    infos: usize,
    roots: usize,
    diagnostics: usize,
    roles: usize,
    ids: usize,
    tree: (usize, i64),
}

/// The structure collected so far.
#[derive(Debug, Clone, Default)]
pub(crate) struct TagState {
    /// Whether the document is tagged: asked for ([`BuildOptions::with_tagged`](crate::BuildOptions::with_tagged)),
    /// or a page used an element or artifact.
    pub(crate) tagged: bool,
    /// Reserved with the first page that has an element (kept if that page fails).
    roots_ref: Option<Roots>,
    /// Every element, complete.
    elems: Vec<(ObjRef, Dict)>,
    /// The `Document` element's children.
    roots: Vec<Object>,
    /// Every custom type used, in order.
    roles: Vec<Tag>,
    tree: ParentTree,
    diagnostics: Vec<TagDiagnostic>,
    infos: Vec<ElemInfo>,
    /// Each `Note` element's `/ID` and object, in structure order, for the `IDTree`.
    ids: Vec<(Vec<u8>, ObjRef)>,
}

impl TagState {
    pub(crate) fn new(tagged: bool) -> Self {
        Self {
            tagged,
            ..Self::default()
        }
    }

    pub(crate) fn mark(&self) -> TagMark {
        TagMark {
            tagged: self.tagged,
            elems: self.elems.len(),
            infos: self.infos.len(),
            roots: self.roots.len(),
            diagnostics: self.diagnostics.len(),
            roles: self.roles.len(),
            ids: self.ids.len(),
            tree: self.tree.mark(),
        }
    }

    /// Forgets everything recorded since `mark`.
    pub(crate) fn rollback(&mut self, mark: TagMark) {
        self.tagged = mark.tagged;
        self.elems.truncate(mark.elems);
        self.infos.truncate(mark.infos);
        self.roots.truncate(mark.roots);
        self.diagnostics.truncate(mark.diagnostics);
        self.roles.truncate(mark.roles);
        self.ids.truncate(mark.ids);
        self.tree.truncate(mark.tree);
    }

    /// What the tagged document lacks so far; nothing for an untagged one.
    pub(crate) fn diagnostics(&self) -> &[TagDiagnostic] {
        if self.tagged { &self.diagnostics } else { &[] }
    }

    /// Every element so far, in structure order.
    pub(crate) fn infos(&self) -> &[ElemInfo] {
        &self.infos
    }

    fn roots_ref(&mut self, doc: &laminapdf_cos::Document) -> Result<Roots> {
        if let Some(r) = self.roots_ref {
            return Ok(r);
        }
        let r = Roots {
            root: doc.reserve_number()?,
            document: doc.reserve_number()?,
        };
        self.roots_ref = Some(r);
        Ok(r)
    }

    /// Reserves the page's elements' numbers and adds its `ParentTree` entries: the page's
    /// (when it has MCIDs) and each of its links' inside an element.
    ///
    /// # Errors
    /// [`laminapdf_cos::Error`] when object numbers run out.
    pub(crate) fn keys(&mut self, marks: &PageMarks, doc: &laminapdf_cos::Document) -> Result<PageKeys> {
        let mut keys = PageKeys::default();
        if marks.elems.is_empty() {
            // No element, so no link inside one: every key is `None`.
            return Ok(keys);
        }
        self.roots_ref(doc)?;
        for _ in &marks.elems {
            keys.refs.push(doc.reserve_number()?);
        }
        let elem = |i: &usize| keys.refs.get(*i).map_or(Object::Null, |r| Object::Ref(*r));
        if !marks.mcids.is_empty() {
            let owners: Vec<Object> = marks.mcids.iter().map(elem).collect();
            keys.page = Some(self.tree.push(Object::Array(owners)));
        }
        for owner in &marks.links {
            let key = owner.as_ref().map(|o| self.tree.push(elem(o)));
            keys.links.push(key);
        }
        Ok(keys)
    }

    /// Adds page `index` (object `page`, whose link annotations are `annots`, in drawing
    /// order) with its elements, numbered by `keys`, and its diagnostics.
    pub(crate) fn add_page(&mut self, index: u32, page: ObjRef, marks: PageMarks, keys: &PageKeys, annots: &[ObjRef]) {
        self.tagged |= marks.used;
        for (content, count) in ContentKind::ALL.into_iter().zip(marks.untagged) {
            if count > 0 {
                self.diagnostics.push(TagDiagnostic::Untagged {
                    page: index,
                    content,
                    count,
                });
            }
        }
        let Some(roots) = self.roots_ref else { return };
        let elem = |i: usize| keys.refs.get(i).map_or(Object::Null, |r| Object::Ref(*r));
        let standard = |t: &Tag| {
            let name = t.name();
            if is_standard(name) {
                name.to_owned()
            } else {
                t.kind().as_str().to_owned()
            }
        };
        let first = self.infos.len();
        for pe in &marks.elems {
            let parent = pe.parent.and_then(|p| marks.elems.get(p)).map(|p| standard(&p.tag));
            self.infos.push(ElemInfo {
                page: index,
                kind: standard(&pe.tag),
                parent,
                parent_index: pe.parent.map(|p| first + p),
            });
        }
        for &r in &marks.roots {
            self.roots.push(elem(r));
        }
        for (pe, r) in marks.elems.into_iter().zip(&keys.refs) {
            let tag = pe.tag;
            if tag.lacks_alt() {
                self.diagnostics.push(TagDiagnostic::MissingAlt { page: index });
            }
            let mut kids = Vec::with_capacity(pe.kids.len());
            for kid in pe.kids {
                kids.push(match kid {
                    PageKid::Elem(i) => elem(i),
                    PageKid::Mcid(m) => Object::Integer(i64::from(m)),
                    PageKid::Link(l) => {
                        let Some(a) = annots.get(l) else { continue };
                        let mut objr = Dict::new();
                        objr.insert("Type", Name::from("OBJR"));
                        objr.insert("Obj", Object::Ref(*a));
                        Object::Dict(objr)
                    }
                });
            }
            let mut d = Dict::new();
            d.insert("Type", Name::from("StructElem"));
            d.insert("S", Name::from(tag.name()));
            d.insert("P", pe.parent.map_or(Object::Ref(roots.document), elem));
            d.insert("Pg", Object::Ref(page));
            d.insert("K", Object::Array(kids));
            for (key, value) in [
                ("Alt", tag.alt()),
                ("ActualText", tag.actual_text()),
                ("E", tag.expansion()),
                ("Lang", tag.lang()),
            ] {
                if let Some(v) = value {
                    d.insert(key, text_string(v));
                }
            }
            if let Some(scope) = tag.scope() {
                // The table attributes (§14.8.5.7).
                let mut a = Dict::new();
                a.insert("O", Name::from("Table"));
                a.insert("Scope", Name::from(scope.as_str()));
                d.insert("A", Object::Dict(a));
            }
            if standard(&tag) == "Note" {
                // PDF/UA-1 §7.9 (Matterhorn 19-003, 19-004): every note has a unique `/ID`;
                // numbered in structure order, so the output stays deterministic.
                let id = format!("Note{}", self.ids.len() + 1).into_bytes();
                d.insert("ID", PdfString::literal(id.clone()));
                self.ids.push((id, *r));
            }
            if tag.custom_name().is_some() {
                self.roles.push(tag);
            }
            self.elems.push((*r, d));
        }
    }

    /// Writes the structure tree of a tagged document and points the catalog at it.
    pub(crate) fn write(&mut self, cos: &mut CosMut<'_>, catalog: &mut Dict) -> Result<()> {
        if !self.tagged {
            return Ok(());
        }
        let Roots { root, document } = match self.roots_ref {
            Some(r) => r,
            None => Roots {
                root: cos.document().reserve_number()?,
                document: cos.document().reserve_number()?,
            },
        };
        for (r, d) in std::mem::take(&mut self.elems) {
            cos.set(r, Object::Dict(d))?;
        }
        let mut doc = Dict::new();
        doc.insert("Type", Name::from("StructElem"));
        doc.insert("S", Name::from("Document"));
        doc.insert("P", Object::Ref(root));
        doc.insert("K", Object::Array(std::mem::take(&mut self.roots)));
        cos.set(document, Object::Dict(doc))?;
        let mut tree = Dict::new();
        tree.insert("Type", Name::from("StructTreeRoot"));
        tree.insert("K", Object::Ref(document));
        tree.insert(
            "ParentTree",
            Object::Ref(cos.create(Object::Dict(self.tree.to_dict()))?),
        );
        tree.insert("ParentTreeNextKey", Object::Integer(self.tree.next_key()));
        if let Some(map) = role_map(&self.roles) {
            tree.insert("RoleMap", Object::Dict(map));
        }
        if !self.ids.is_empty() {
            // §14.7.2: the name tree from each `/ID` to its element, keys in byte order.
            let mut ids = std::mem::take(&mut self.ids);
            ids.sort_by(|a, b| a.0.cmp(&b.0));
            let names = ids
                .into_iter()
                .flat_map(|(id, r)| [Object::String(PdfString::literal(id)), Object::Ref(r)])
                .collect();
            let mut id_tree = Dict::new();
            id_tree.insert("Names", Object::Array(names));
            tree.insert("IDTree", Object::Dict(id_tree));
        }
        cos.set(root, Object::Dict(tree))?;
        let mut mark_info = Dict::new();
        mark_info.insert("Marked", Object::Bool(true));
        catalog.insert("MarkInfo", Object::Dict(mark_info));
        catalog.insert("StructTreeRoot", Object::Ref(root));
        Ok(())
    }
}
