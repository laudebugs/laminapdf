//! `InsertPagesFrom` and `ReplacePages`: pages copied in from a source document (E2 plan
//! §2.1, §2.2), added with [`Document::add_source`](crate::Document::add_source).

use laminapdf_model::{PageId, PagePosition};

use super::cleanup::{ReferenceChange, destinations};
use super::delete::{DeletePages, ReferencePolicy};
use super::duplicate::names_in_use;
use super::import::{Carry, Imported, Signatures, import};
use super::{checked, page_ids, pages_before, relabel};
use crate::SourceId;
use crate::document::Sources;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// Copies pages of a source document into this one at `at`, in the order given (each once).
/// The copies are new objects with new IDs, sharing nothing with the source. Their
/// annotations point at them and get a new `/NM`; a link to a page that didn't come along is
/// removed ([`Applied::links_removed`](crate::Applied::links_removed)), and the named
/// destinations the others use come along, renamed `<name>#<n>` on a clash. Form fields join
/// the form under their own names, or per [`Carry::fields`] when the name is taken
/// ([`Applied::fields`](crate::Applied::fields)); signature fields are left off. The copies
/// lose their tags ([`Applied::tags_dropped`](crate::Applied::tags_dropped)), page labels
/// continue the range each copy lands in, and the document's version rises to the source's
/// when that is newer.
///
/// On the wire: `{ "op": "insertPagesFrom", "args": { "source": "<64 hex digits>", "pages":
/// ["p3.0"], "at": "end" } }`, with `carry` optional.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct InsertPagesFrom {
    /// The source document.
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub source: SourceId,
    /// The source's pages to copy, by their IDs in the source.
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
    /// What comes along with the pages.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub carry: Carry,
}

impl InsertPagesFrom {
    /// The op that copies `pages` of `source` to `at`, with their annotations and fields.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(source: SourceId, pages: Vec<PageId>, at: PagePosition) -> Self {
        Self {
            source,
            pages,
            at,
            carry: Carry::default(),
        }
    }

    /// With `carry` deciding what comes along.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_carry(mut self, carry: Carry) -> Self {
        self.carry = carry;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, sources: &Sources) -> Result<Effect> {
        let src = sources.get(&self.source).ok_or(Error::UnknownSource(self.source))?;
        let before = pages_before(doc)?;
        let mut names = names_in_use(doc, &before)?;
        let imported = import(doc, src, &self.pages, self.carry, Signatures::Skip, &mut names)?;
        if imported.copies.is_empty() {
            return Ok(Effect::default());
        }
        let copies = insert(doc, &imported, |_| self.at.clone())?;
        let after = page_ids(doc)?;
        relabel(doc, &before, &after)?;
        Ok(effect(imported, copies, after))
    }
}

/// Puts each copy in the page tree in order: copy `i` before the page `at(i)` names when it
/// names one, else the first at `at(0)` and each other after the copy before it.
fn insert(
    doc: &mut laminapdf_model::Document,
    imported: &Imported,
    at: impl Fn(usize) -> PagePosition,
) -> Result<Vec<PageId>> {
    let mut editor = doc.edit_page_tree()?;
    let mut copies: Vec<PageId> = Vec::with_capacity(imported.copies.len());
    for (i, &copy) in imported.copies.iter().enumerate() {
        let place = match (i, copies.last()) {
            (0, _) | (_, None) => at(0),
            (i, Some(previous)) => match at(i) {
                PagePosition::Before(target) => PagePosition::Before(target),
                _ => PagePosition::After(previous.clone()),
            },
        };
        copies.push(editor.insert(copy, &place)?);
    }
    editor.finish()?;
    Ok(copies)
}

/// The effect of an import whose copies are now `copies`, the page list `after`.
fn effect(imported: Imported, copies: Vec<PageId>, after: Vec<PageId>) -> Effect {
    Effect {
        pages: copies,
        page_order: Some(after),
        fields: imported.fields,
        links_removed: imported.links_removed,
        tags_dropped: imported.tags_dropped,
        ..Effect::default()
    }
}

/// Replaces pages of this document with pages of a source document, pair by pair: the first
/// source page takes the first target's place, and so on. What pointed at a replaced page
/// (outline items, links, named destinations, `GoTo` actions, `/OpenAction`) now points at
/// the page that took its place, with the view `/Fit`, since coordinates on the old page mean
/// nothing on the new one. Targets left over are deleted (their references retargeted to the
/// nearest surviving page); source pages left over go after the last copy. The copies come
/// in as with [`InsertPagesFrom`]; the replaced pages are deleted as with
/// [`DeletePages`](crate::DeletePages), structure, widgets and earlier revisions included.
///
/// On the wire: `{ "op": "replacePages", "args": { "targets": ["p7.0"], "source": "<64 hex
/// digits>", "pages": ["p3.0"] } }`, with `carry` optional.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct ReplacePages {
    /// The pages to replace, in order.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub targets: Vec<PageId>,
    /// The source document.
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub source: SourceId,
    /// The source's pages that take their places, by their IDs in the source.
    #[serde(with = "wire::page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub pages: Vec<PageId>,
    /// What comes along with the pages.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub carry: Carry,
}

impl ReplacePages {
    /// The op that replaces `targets` with `pages` of `source`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(targets: Vec<PageId>, source: SourceId, pages: Vec<PageId>) -> Self {
        Self {
            targets,
            source,
            pages,
            carry: Carry::default(),
        }
    }

    /// With `carry` deciding what comes along.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_carry(mut self, carry: Carry) -> Self {
        self.carry = carry;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, sources: &Sources) -> Result<Effect> {
        let src = sources.get(&self.source).ok_or(Error::UnknownSource(self.source))?;
        if let Some(direct) = self.targets.iter().find(|p| p.as_lamina_id().obj_ref().is_none()) {
            return Err(Error::DirectPage(direct.clone()));
        }
        let before = pages_before(doc)?;
        let targets = checked(&before, &self.targets)?;
        if targets.is_empty() {
            return Ok(Effect::default());
        }
        let mut names = names_in_use(doc, &before)?;
        let imported = import(doc, src, &self.pages, self.carry, Signatures::Skip, &mut names)?;
        // Each copy goes before its target; extra copies follow the last one. Replacing every
        // page with nothing is refused by the deletion (`no-pages-left`).
        let copies = insert(doc, &imported, |i| match targets.get(i) {
            Some(target) => PagePosition::Before(target.clone()),
            None => PagePosition::End,
        })?;
        let with_copies = page_ids(doc)?;
        relabel(doc, &before, &with_copies)?;
        let mut references: Vec<ReferenceChange> = Vec::new();
        for (target, copy) in targets.iter().zip(&copies) {
            let held = doc.page_references(std::slice::from_ref(target))?.entries().to_vec();
            references.extend(destinations(
                doc,
                &held,
                &[target.clone(), copy.clone()],
                std::slice::from_ref(target),
                ReferencePolicy::Retarget,
            )?);
        }
        let deleted = DeletePages::new(targets).apply_to(doc)?;
        references.extend(deleted.references);
        let after = page_ids(doc)?;
        let mut pages = copies.clone();
        pages.extend(deleted.pages);
        Ok(Effect {
            pages,
            references,
            widgets: deleted.widgets,
            removed: true,
            ..effect(imported, copies, after)
        })
    }
}
