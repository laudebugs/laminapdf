//! `DuplicatePages`: copies of pages within the document (E2 plan §2.2), sharing their source's
//! content streams and resources, with their own annotations, fields and structure.

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{PageId, PagePosition};

use super::copies::copy_annotations;
use super::tags::{Pair, clone_tags};
use super::{checked, page_ids, pages_before, relabel};
use crate::error::Result;
use crate::op::Effect;
use crate::wire;

/// Copies `pages` (each once, in the order given) to `at`. A copy shares its source's content
/// streams and resources and shows what it shows, inherited attributes included (ISO 32000-2
/// §7.7.3.4). Its annotations are new objects with a new `/NM`; a form field's widget becomes
/// a new field named `<partial>#<n>` with its own value, and a signature's widget is not copied
/// (both in [`Applied::fields`](crate::Applied::fields)). In a tagged document the copy gets
/// its own `/StructParents` key and clones of the elements whose content lies only on its
/// source; content of elements that span pages is left untagged and the copy is listed in
/// [`Applied::tags_incomplete`](crate::Applied::tags_incomplete). Page labels continue the range
/// each copy lands in.
///
/// On the wire: `{ "op": "duplicatePages", "args": { "pages": ["p7.0"], "at": { "after": "p7.0" } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DuplicatePages {
    /// The pages to copy.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// Where the copies go.
    #[serde(with = "wire::position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "\"start\" | \"end\" | { before: string } | { after: string }")
    )]
    pub at: PagePosition,
}

impl DuplicatePages {
    /// The op that copies `pages` to `at`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(pages: Vec<PageId>, at: PagePosition) -> Self {
        Self { pages, at }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        // The page-tree editor lists only page objects, so a page stored inside its parent
        // would read as unknown: name it for what it is first.
        if let Some(direct) = self.pages.iter().find(|p| p.as_lamina_id().obj_ref().is_none()) {
            return Err(crate::Error::DirectPage(direct.clone()));
        }
        let before = pages_before(doc)?;
        let pages = checked(&before, &self.pages)?;
        if pages.is_empty() {
            return Ok(Effect::default());
        }
        let sources = read_sources(doc, &pages)?;
        let mut copies: Vec<PageId> = Vec::with_capacity(pages.len());
        {
            let mut editor = doc.edit_page_tree()?;
            let mut at = self.at.clone();
            for source in &sources {
                let id = editor.insert_new(source.dict.clone(), &at)?;
                at = PagePosition::After(id.clone());
                copies.push(id);
            }
            editor.finish()?;
        }
        let mut names = names_in_use(doc, &before)?;
        let mut fields = Vec::new();
        let mut per_page = Vec::with_capacity(pages.len());
        for (source, copy) in sources.iter().zip(&copies) {
            let Some(c) = copy.as_lamina_id().obj_ref() else {
                continue;
            };
            let annots = copy_annotations(doc, &source.id, &source.annots, c, copy, &mut names)?;
            fields.extend(annots.fields);
            per_page.push((source.obj, c, annots.map, source.shares_tags));
        }
        let pairs: Vec<Pair<'_>> = per_page
            .iter()
            .map(|(source, copy, annots, shares_tags)| Pair {
                source: *source,
                copy: *copy,
                annots,
                shares_tags: *shares_tags,
            })
            .collect();
        let incomplete = clone_tags(doc, &pairs)?;
        let tags_incomplete = copies
            .iter()
            .filter(|c| c.as_lamina_id().obj_ref().is_some_and(|r| incomplete.contains(&r)))
            .cloned()
            .collect();
        let after = page_ids(doc)?;
        relabel(doc, &before, &after)?;
        Ok(Effect {
            pages: copies,
            page_order: Some(after),
            fields,
            tags_incomplete,
            ..Effect::default()
        })
    }
}

/// The keys a copy doesn't take from its source: its place in the tree, what the copy gets
/// anew (annotations, structure key), and article beads, which belong to their thread's pages.
const NOT_COPIED: [&str; 5] = ["Parent", "Annots", "StructParents", "B", "Type"];

/// What a copy is made from, read in one walk of the page tree.
struct Source {
    id: PageId,
    obj: laminapdf_cos::ObjRef,
    /// The dictionary the copy starts from ([`copy_dict`]).
    dict: Dict,
    /// The source's annotations (`/Annots`).
    annots: Vec<laminapdf_cos::ObjRef>,
    /// Whether it draws a Form XObject with its own `/StructParents`, whose key the copy
    /// would share (ISO 32000-2 §14.7.5.4).
    shares_tags: bool,
}

/// Each page of `pages` as a [`Source`]. A page stored inside its parent (a direct dictionary)
/// has no object of its own for annotations and structure to name, so duplicating it is
/// [`Error::DirectPage`](crate::Error::DirectPage).
fn read_sources(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<Vec<Source>> {
    let tree = doc.pages()?;
    let mut out = Vec::with_capacity(pages.len());
    for id in pages {
        let Some(obj) = id.as_lamina_id().obj_ref() else {
            return Err(crate::Error::DirectPage(id.clone()));
        };
        let Some(page) = tree.by_id(id) else {
            return Err(crate::Error::UnknownPage(id.clone()));
        };
        out.push(Source {
            id: id.clone(),
            obj,
            dict: copy_dict(&page),
            annots: page.annots()?,
            shares_tags: draws_tagged_form(doc, &page)?,
        });
    }
    Ok(out)
}

/// The dictionary `page`'s copy starts from: its own entries but those of [`NOT_COPIED`], and
/// the attributes it inherits (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`) set on it, so
/// the copy shows the same wherever it lands. Values are copied as they are, so references
/// keep sharing the source's objects.
fn copy_dict(page: &laminapdf_model::Page<'_>) -> Dict {
    let mut dict = Dict::new();
    for (key, value) in page.dict().iter() {
        if !NOT_COPIED.iter().any(|k| key.0 == k.as_bytes()) {
            dict.insert(key.clone(), value.clone());
        }
    }
    for (key, value) in page.cos().inherited.entries() {
        if let Some(value) = value {
            dict.insert(key, Object::clone(value));
        }
    }
    dict
}

/// Whether `page` draws (from its own `/XObject` resources) a Form XObject with
/// `/StructParents`: its marked content belongs to the source's elements, and a copy sharing
/// the stream can't have its own.
fn draws_tagged_form(doc: &laminapdf_model::Document, page: &laminapdf_model::Page<'_>) -> Result<bool> {
    let Some(xobjects) = page.resources().and_then(|r| r.get("XObject")) else {
        return Ok(false);
    };
    let Some(xobjects) = laminapdf_cos::tolerant(doc.deref(xobjects))?.and_then(|o| o.as_dict().cloned()) else {
        return Ok(false);
    };
    for (_, xobject) in xobjects.iter() {
        let tagged = laminapdf_cos::tolerant(doc.deref(xobject))?
            .is_some_and(|o| o.as_dict().is_some_and(|d| d.contains_key("StructParents")));
        if tagged {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The fully qualified names any field has: those the form lists, and those of every page's
/// widgets, since a field the form's `/Fields` doesn't reach still names a value viewers share.
fn names_in_use(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<laminapdf_model::FieldNames> {
    let mut names = doc.field_names()?;
    for page in pages {
        for w in doc.page_widgets(page)? {
            if let Some(name) = doc.field_name(w.field)? {
                names.insert(name);
            }
        }
    }
    Ok(names)
}
