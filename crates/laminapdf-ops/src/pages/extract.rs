//! New documents made from pages of this one (E2 plan §2.1, §3, §6): `extract_pages` and
//! `export_ranges`. Each is built in the overlay of an empty base and carries only what its
//! pages reach, so saving it writes a whole file with nothing of the rest.

use laminapdf_cos::{Dict, Object, OpenOptions};
use laminapdf_model::{FieldNames, OutlineTree, PageId, PagePosition, SiblingPosition};

use super::import::{Carry, Signatures, import_with};
use super::import_tags::Tags;
use super::merge_parts::{CopiedItem, copy_outline, merge_layers, source_layers};
use super::page_ids;
use crate::document::Document;
use crate::error::Result;

impl Document {
    /// A new document of `pages` of this one (each once, in the order given), as this document
    /// stands now, edits included. The pages are deep copies with their annotations, links
    /// between them, the named destinations those links use, form fields (signature fields
    /// unsigned: a signature covers the file it was made in) and the page labels they show
    /// here, with the bookmarks that go to them (a bookmark whose page was left behind is left
    /// out unless one of its children came) and the layers they use. Links to pages left behind
    /// are removed, and the pages come without tags. The new
    /// document has the same PDF version and limits, no password, and a base that is empty, so
    /// saving it writes a whole file holding only what its pages reach.
    ///
    /// ```
    /// use laminapdf_ops::Document;
    /// # use laminapdf_cos::{Dict, Object};
    /// # use laminapdf_save::test_support::Writer;
    /// # let mut w = Writer::new(1, 7);
    /// # let node = w.reserve();
    /// # let page = |w: &mut Writer| w.add([("Type", Object::name("Page")), ("Parent", node.into())].into_iter().collect::<Dict>());
    /// # let (a, b) = (page(&mut w), page(&mut w));
    /// # w.set(node, [("Type", Object::name("Pages")), ("Kids", vec![a.into(), b.into()].into()), ("Count", 2.into()), ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into())].into_iter().collect::<Dict>());
    /// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", node.into())].into_iter().collect::<Dict>());
    /// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
    /// let doc = Document::open(bytes, Default::default())?;
    /// let second = doc.pages()?.iter().nth(1).map(|p| p.id().clone()).unwrap();
    /// let extracted = doc.extract_pages(&[second])?;
    /// assert_eq!(extracted.page_count()?, 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// [`Error::NoPagesLeft`](crate::Error::NoPagesLeft) for no pages, since a document needs
    /// one; [`Error::UnknownPage`](crate::Error::UnknownPage) for a page this document lacks;
    /// errors reading this document; [`Limits`](laminapdf_cos::Limits) passed by the copy.
    ///
    /// Added in 0.3.0.
    pub fn extract_pages(&self, pages: &[PageId]) -> Result<Self> {
        if pages.is_empty() {
            return Err(crate::Error::NoPagesLeft);
        }
        let source: &laminapdf_model::Document = self;
        let mut out = new_document(source)?;
        let mut names = FieldNames::default();
        let mut outline = Vec::new();
        let mut layers = None;
        let tags = if source.is_tagged()? { Tags::Keep } else { Tags::Drop };
        let imported = import_with(
            &mut out,
            source,
            pages,
            (Carry::default(), Signatures::Unsign, &tags),
            &mut names,
            |doc, copier, dests| {
                outline = copy_outline(doc, source, dests)?.0;
                layers = source_layers(source, copier)?;
                Ok(CopiedItem::objects(&outline))
            },
        )?;
        if let Some(layers) = layers {
            merge_layers(&mut out, &layers)?;
        }
        {
            let mut editor = out.edit_page_tree()?;
            for &copy in &imported.copies {
                editor.insert(copy, &PagePosition::End)?;
            }
            editor.finish()?;
        }
        let labels = source.page_labels()?;
        if !labels.is_empty() {
            let mut chosen: Vec<PageId> = Vec::with_capacity(pages.len());
            for page in pages {
                if !chosen.contains(page) {
                    chosen.push(page.clone());
                }
            }
            let all = page_ids(source)?;
            labels.remap(&all, &chosen).write(&mut out.cos_mut().edit())?;
        }
        if !outline.is_empty() {
            let mut tree = OutlineTree::read(&out)?;
            add_items(&mut tree, None, outline)?;
            tree.write(&mut out.cos_mut().edit())?;
        }
        Ok(Self::from(out))
    }

    /// One new document per range of `ranges` (each a list of pages, as
    /// [`Document::extract_pages`] takes), in order.
    ///
    /// # Errors
    /// As [`Document::extract_pages`], for the first range that fails.
    ///
    /// Added in 0.3.0.
    pub fn export_ranges(&self, ranges: &[Vec<PageId>]) -> Result<Vec<Self>> {
        ranges.iter().map(|pages| self.extract_pages(pages)).collect()
    }
}

/// An empty document of `like`'s version and limits: a catalog and an empty page tree,
/// written in the overlay of an empty base.
fn new_document(like: &laminapdf_model::Document) -> Result<laminapdf_model::Document> {
    empty_document(like.version()?.as_deref(), *like.limits())
}

/// An empty document of this `version` (the writer's default when `None`) and `limits`: a
/// catalog and an empty page tree, written in the overlay of an empty base.
pub(super) fn empty_document(
    version: Option<&str>,
    limits: laminapdf_cos::Limits,
) -> Result<laminapdf_model::Document> {
    let options = OpenOptions::default().with_limits(limits);
    let mut cos = laminapdf_cos::Document::empty(version, &options);
    let catalog = cos.reserve_number()?;
    let pages = cos.reserve_number()?;
    {
        let mut edit = cos.edit();
        let tree: Dict = [
            ("Type", Object::name("Pages")),
            ("Kids", Object::Array(Vec::new())),
            ("Count", 0.into()),
        ]
        .into_iter()
        .collect();
        edit.set(pages, Object::Dict(tree))?;
        let root: Dict = [("Type", Object::name("Catalog")), ("Pages", Object::Ref(pages))]
            .into_iter()
            .collect();
        edit.set(catalog, Object::Dict(root))?;
        edit.set_trailer([("Root", Object::Ref(catalog))].into_iter().collect());
    }
    Ok(laminapdf_model::Document::from(cos))
}

/// Inserts copied `items` under `parent`, last, with their children and open state.
pub(super) fn add_items(
    tree: &mut OutlineTree,
    parent: Option<&laminapdf_model::OutlineItemId>,
    items: Vec<CopiedItem>,
) -> Result<()> {
    for item in items {
        let id = tree
            .insert(item.item, parent, &SiblingPosition::Last)
            .map_err(crate::outline::tree_error)?;
        add_items(tree, Some(&id), item.children)?;
        if item.open {
            tree.set_open(&id, true).map_err(crate::outline::tree_error)?;
        }
    }
    Ok(())
}
