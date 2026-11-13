//! `MergeDocuments`: whole documents, or chosen pages of them, merged into this one as one
//! undo step (E2 plan §2.1, §2.2). Each source's pages come in as `InsertPagesFrom` brings them;
//! the merge adds what belongs to the documents rather than to their pages: their outlines,
//! their optional content (layers) and their page labels.

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{OutlineTree, PageId, PagePosition, SiblingPosition};

use super::duplicate::names_in_use;
use super::extract::add_items;
use super::from::insert;
use super::import::{Carry, FieldClash, Imported, Signatures, import_with};
use super::merge_parts::{CopiedItem, Label, SourceLabels, copy_outline, merge_labels, merge_layers, source_layers};
use super::{page_ids, pages_before};
use crate::SourceId;
use crate::document::{Document, Sources};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::tx::Origin;
use crate::wire;

/// What becomes of the sources' outlines (bookmarks) in a merge.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum OutlineMerge {
    /// Each source's bookmarks go under a new top-level bookmark named after it
    /// ([`MergeSource::title`], else the source's own title), which opens its first page; a
    /// source with neither name adds its bookmarks at the top level.
    #[default]
    PerFile,
    /// Each source's bookmarks are added at the top level, after the document's own.
    Keep,
    /// The sources' bookmarks are left out.
    None,
}

/// One document a merge brings in.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MergeSource {
    /// The source document, added with [`Document::add_source`](crate::Document::add_source).
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub source: SourceId,
    /// The source's pages to bring in, by their IDs in the source, in order; all of them when
    /// left out.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_page_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]", optional))]
    pub pages: Option<Vec<PageId>>,
    /// The name of the bookmark [`OutlineMerge::PerFile`] gives the source's bookmarks (a file
    /// name, say); the source's own title when left out.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub title: Option<String>,
}

impl MergeSource {
    /// Every page of `source`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn new(source: SourceId) -> Self {
        Self {
            source,
            pages: None,
            title: None,
        }
    }

    /// Only these `pages` of the source, in this order.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn with_pages(mut self, pages: Vec<PageId>) -> Self {
        self.pages = Some(pages);
        self
    }

    /// With `title` naming its bookmark under [`OutlineMerge::PerFile`].
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// How a merge brings in what the sources hold besides their pages.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MergeOptions {
    /// What becomes of the sources' bookmarks; `perFile` when left out.
    #[serde(default)]
    pub outline: OutlineMerge,
    /// What happens to a form field whose name is already used; `rename` when left out.
    #[serde(default)]
    pub fields: FieldClash,
}

impl MergeOptions {
    /// With `outline` deciding what becomes of the sources' bookmarks.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn with_outline(mut self, outline: OutlineMerge) -> Self {
        self.outline = outline;
        self
    }

    /// With `fields` deciding what happens to a form field whose name is already used.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn with_fields(mut self, fields: FieldClash) -> Self {
        self.fields = fields;
        self
    }
}

/// A named destination a merge or a page import brought in under another name, because the
/// document already used its own (E2 plan §2.2).
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct RenamedDestination {
    /// Its name in the source, decoded as UTF-8 (bytes that aren't are replaced).
    pub from: String,
    /// Its name in this document.
    pub to: String,
}

impl RenamedDestination {
    pub(crate) fn of(renames: &[(Vec<u8>, Vec<u8>)]) -> Vec<Self> {
        renames
            .iter()
            .map(|(from, to)| Self {
                from: String::from_utf8_lossy(from).into_owned(),
                to: String::from_utf8_lossy(to).into_owned(),
            })
            .collect()
    }
}

/// Merges source documents into this one at `at`, one after another, as one edit. Each
/// source's pages come in as [`InsertPagesFrom`](crate::InsertPagesFrom) brings them (their
/// annotations, links and the named destinations those use, form fields under the clash rule
/// of `options.fields`, signature fields left off, tags dropped), and with them what belongs to
/// the whole source:
///
/// - its bookmarks, per `options.outline`, going to the copies of their pages; a bookmark whose
///   page didn't come along is left out unless one of its children came
///   ([`Applied::bookmarks_dropped`](crate::Applied::bookmarks_dropped));
/// - its layers (optional content groups the copied pages use), added to this document's list
///   in the source's order and visible or hidden as the source showed them;
/// - its page labels: each copy keeps the label it had in its source, and this document's
///   pages keep theirs (a document without labels numbers its pages from 1);
/// - its form's default appearance and `/NeedAppearances`, and its PDF version when newer.
///
/// Named destinations that clash are renamed `<name>#<n>`
/// ([`Applied::renamed_destinations`](crate::Applied::renamed_destinations)).
///
/// On the wire: `{ "op": "mergeDocuments", "args": { "sources": [{ "source": "<64 hex
/// digits>" }], "at": "end" } }`, with each source's `pages` and `title`, and `options`,
/// optional.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct MergeDocuments {
    /// The documents to bring in, in order.
    pub sources: Vec<MergeSource>,
    /// Where their pages go.
    #[serde(with = "wire::position")]
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "\"start\" | \"end\" | { before: string } | { after: string }")
    )]
    pub at: PagePosition,
    /// What comes along besides the pages.
    #[serde(default)]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub options: MergeOptions,
}

impl MergeDocuments {
    /// The op that merges `sources` at `at`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(sources: Vec<MergeSource>, at: PagePosition) -> Self {
        Self {
            sources,
            at,
            options: MergeOptions::default(),
        }
    }

    /// With `options` deciding what comes along besides the pages.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub const fn with_options(mut self, options: MergeOptions) -> Self {
        self.options = options;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, sources: &Sources) -> Result<Effect> {
        let before = pages_before(doc)?;
        let labels_before = doc.page_labels()?;
        let mut names = names_in_use(doc, &before)?;
        let mut all = Imported::default();
        let mut merged: Vec<Merged> = Vec::with_capacity(self.sources.len());
        let mut bookmarks_dropped = 0;
        for part in &self.sources {
            let src = sources.get(&part.source).ok_or(Error::UnknownSource(part.source))?;
            let pages = match &part.pages {
                Some(pages) => once_each(pages),
                None => page_ids(src)?,
            };
            let keep_outline = self.options.outline != OutlineMerge::None;
            let mut outline = Vec::new();
            let mut dropped = 0;
            let mut layers = None;
            let imported = import_with(
                doc,
                src,
                &pages,
                (Carry::default().with_fields(self.options.fields), Signatures::Skip),
                &mut names,
                |doc, copier, dests| {
                    if keep_outline {
                        (outline, dropped) = copy_outline(doc, src, dests)?;
                    }
                    layers = source_layers(src, copier)?;
                    Ok(CopiedItem::objects(&outline))
                },
            )?;
            if let Some(layers) = layers {
                merge_layers(doc, &layers)?;
            }
            let title = match &part.title {
                Some(title) => Some(title.clone()),
                None => match src.info()? {
                    Some(info) => info.title()?.filter(|t| !t.trim().is_empty()),
                    None => None,
                },
            };
            bookmarks_dropped += dropped;
            merged.push(Merged {
                copies: imported.copies.clone(),
                labels: SourceLabels::of(src, &pages)?,
                title,
                outline,
            });
            all.append(imported);
        }
        if all.copies.is_empty() {
            return Ok(Effect::default());
        }
        let copies = insert(doc, &all, |_| self.at.clone())?;
        let after = page_ids(doc)?;
        merge_labels(doc, &labels_before, &before, &after, &merged_labels(&merged, &copies))?;
        let created = self.merge_outlines(doc, &mut merged, &copies)?;
        Ok(Effect {
            created_outline_items: created,
            bookmarks_dropped,
            ..super::from::effect(all, copies, after)
        })
    }

    /// Adds each source's copied bookmarks to the outline as `options.outline` says; returns the
    /// per-file bookmarks it created.
    fn merge_outlines(
        &self,
        doc: &mut laminapdf_model::Document,
        merged: &mut [Merged],
        copies: &[PageId],
    ) -> Result<Vec<laminapdf_model::OutlineItemId>> {
        if merged.iter().all(|m| m.outline.is_empty()) && self.options.outline != OutlineMerge::PerFile {
            return Ok(Vec::new());
        }
        let mut tree = OutlineTree::read(doc)?;
        let mut created = Vec::new();
        let mut index = 0;
        for part in merged.iter_mut() {
            if part.copies.is_empty() {
                // No pages came from this source, so neither did any of its bookmarks.
                continue;
            }
            let first = copies.get(index).and_then(|p| p.as_lamina_id().obj_ref());
            index += part.copies.len();
            let items = std::mem::take(&mut part.outline);
            let parent = match (self.options.outline, &part.title, first) {
                (OutlineMerge::PerFile, Some(title), Some(first)) => {
                    let dict: Dict = [
                        ("Title", crate::outline::title(title)),
                        ("Dest", Object::Array(vec![Object::Ref(first), Object::name("Fit")])),
                    ]
                    .into_iter()
                    .collect();
                    let item = doc.cos_mut().edit().create(Object::Dict(dict))?;
                    let id = tree
                        .insert(item, None, &SiblingPosition::Last)
                        .map_err(crate::outline::tree_error)?;
                    created.push(id.clone());
                    Some(id)
                }
                (OutlineMerge::None, ..) => continue,
                _ => None,
            };
            add_items(&mut tree, parent.as_ref(), items)?;
        }
        tree.write(&mut doc.cos_mut().edit())?;
        Ok(created)
    }
}

/// One file [`Document::merge`] brings in.
///
/// Added in 0.3.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct MergeInput {
    /// The file's bytes.
    pub bytes: std::sync::Arc<[u8]>,
    /// Its password, when it has one.
    pub password: Option<String>,
    /// The name of its bookmark under [`OutlineMerge::PerFile`] (its file name, say).
    pub title: Option<String>,
}

impl MergeInput {
    /// The file `bytes`, with no password or title.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(bytes: impl Into<std::sync::Arc<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
            password: None,
            title: None,
        }
    }

    /// With the file's `password`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// With `title` naming its bookmark under [`OutlineMerge::PerFile`].
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

impl Document {
    /// An empty document: a catalog and an empty page tree, in the overlay of an empty base,
    /// with default limits. A document needs a page to show, so this one is a target to merge
    /// files into ([`Document::merge`] starts from one) or to build pages in.
    ///
    /// # Errors
    /// Errors of the overlay.
    ///
    /// Added in 0.3.0.
    pub fn empty() -> Result<Self> {
        Ok(Self::from(super::extract::empty_document(
            None,
            laminapdf_cos::Limits::default(),
        )?))
    }

    /// A new document of `inputs`' pages, in order: an empty document with one
    /// [`MergeDocuments`] applied, so it holds what that op brings in (bookmarks per
    /// `options.outline`, fields under `options.fields`' clash rule, layers, page labels) and
    /// the newest input's PDF version. Saving it writes a whole file holding only what its pages
    /// reach; signature fields are left off, since a signature covers the file it was made in.
    ///
    /// ```
    /// use laminapdf_ops::{Document, MergeInput, MergeOptions};
    /// # use laminapdf_cos::{Dict, Object};
    /// # use laminapdf_save::test_support::Writer;
    /// # let file = |pages: usize| -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    /// #     let mut w = Writer::new(1, 7);
    /// #     let node = w.reserve();
    /// #     let kids: Vec<Object> = (0..pages).map(|_| w.add([("Type", Object::name("Page")), ("Parent", node.into())].into_iter().collect::<Dict>()).into()).collect();
    /// #     w.set(node, [("Type", Object::name("Pages")), ("Kids", kids.into()), ("Count", (pages as i64).into()), ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into())].into_iter().collect::<Dict>());
    /// #     let root = w.add([("Type", Object::name("Catalog")), ("Pages", node.into())].into_iter().collect::<Dict>());
    /// #     Ok(w.finish(&[("Root", root)].into_iter().collect())?)
    /// # };
    /// let merged = Document::merge(
    ///     vec![MergeInput::new(file(2)?), MergeInput::new(file(3)?)],
    ///     MergeOptions::default(),
    /// )?;
    /// assert_eq!(merged.page_count()?, 5);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// [`Error::NoPagesLeft`] for no inputs; errors opening an input (a wrong or missing
    /// password among them) or applying the merge.
    ///
    /// Added in 0.3.0.
    pub fn merge(inputs: Vec<MergeInput>, options: MergeOptions) -> Result<Self> {
        if inputs.is_empty() {
            return Err(Error::NoPagesLeft);
        }
        let mut doc = Self::empty()?;
        let mut sources = Vec::with_capacity(inputs.len());
        for input in inputs {
            let id = doc.add_source(input.bytes, input.password.as_deref())?;
            sources.push(MergeSource {
                source: id,
                pages: None,
                title: input.title,
            });
        }
        let op = MergeDocuments::new(sources, PagePosition::End).with_options(options);
        doc.apply_all(Origin::Local, [op.into()])?;
        Ok(doc)
    }
}

/// What one source brought in, kept until every source's pages are in the page tree.
pub(super) struct Merged {
    /// The new page objects, in order.
    copies: Vec<ObjRef>,
    /// The labels the copied pages showed in the source.
    labels: SourceLabels,
    /// The name of its per-file bookmark.
    title: Option<String>,
    /// Its copied bookmarks, not yet in the outline.
    outline: Vec<CopiedItem>,
}

/// Each copy's label in its source, by the copy's page ID.
fn merged_labels(merged: &[Merged], copies: &[PageId]) -> Vec<(PageId, Option<(Label, bool)>)> {
    let mut out = Vec::with_capacity(copies.len());
    let mut ids = copies.iter();
    for part in merged {
        for i in 0..part.copies.len() {
            if let Some(id) = ids.next() {
                out.push((id.clone(), part.labels.get(i)));
            }
        }
    }
    out
}

/// `pages` without repeats, in order.
fn once_each(pages: &[PageId]) -> Vec<PageId> {
    let mut seen = std::collections::BTreeSet::new();
    pages.iter().filter(|p| seen.insert(*p)).cloned().collect()
}

impl Imported {
    /// Adds `other`'s copies and reports to these.
    pub(super) fn append(&mut self, other: Self) {
        self.copies.extend(other.copies);
        self.fields.extend(other.fields);
        self.links_removed.extend(other.links_removed);
        self.tags_dropped.extend(other.tags_dropped);
        self.widgets.extend(other.widgets);
        self.destinations_renamed.extend(other.destinations_renamed);
        self.pdfa_unchecked.extend(other.pdfa_unchecked);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_each_keeps_the_first_of_each_page() {
        let ids: Vec<PageId> = [1, 2, 1, 3]
            .iter()
            .map(|&n| PageId::object(ObjRef::new(n, 0)))
            .collect();
        let got: Vec<u32> = once_each(&ids)
            .iter()
            .filter_map(|p| p.as_lamina_id().obj_ref().map(|r| r.num))
            .collect();
        assert_eq!(got, [1, 2, 3]);
    }
}
