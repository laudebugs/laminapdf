//! `EditContent`: change what a page already draws (E7 plan §2.1, §2.3; ADR 0022): move,
//! resize and rotate objects, delete them, replace images, crop them, and change the stacking
//! order. Every edit goes through `laminapdf-content`'s IR, so only the objects it names are
//! written again; the rest of the content keeps its bytes.

mod structure;
mod text;
mod wire;

use std::collections::BTreeSet;

use laminapdf_content::{
    ContentObjectId, ContentObjectKind, ContentObjectRef, ContentOps, ContentOwner, PageContentWriter, Placement,
    ResourceName, interpret_page,
};
use laminapdf_model::{Matrix, PageId, RectPt};
use serde::{Deserialize, Serialize};

use crate::images::ImageXObjects;
pub(crate) use structure::tags_broken;
pub use text::{ContentTextRun, EditText, TextRunRef, text_runs};

use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};
use crate::op::Effect;

/// Changes objects a page already draws, in order ([E7 plan] §2.1).
///
/// Each edit names its object by an [`ObjectRef`]: its ordinal among the page's content
/// objects and the hash of its bytes, as `laminapdf::Document::content_objects` lists them.
/// Every name is checked against the page as it is before the op, so the edits of one op name
/// objects as one list shows them, whatever the earlier edits do; a name whose hash no longer
/// matches fails with `content-object-stale`. Geometry is in page space ([ADR 0027]).
///
/// The page's content streams aren't changed: each one an edit touches is written again as a new
/// stream, copying every operator no edit changed byte for byte, so a stream another page also
/// draws stays as it was there. A resource the deleted or replaced objects named that nothing
/// names any more leaves the page's own copy of its resources. Saving is an incremental update;
/// what an edit deletes stays in the file's earlier revisions.
///
/// On the wire:
/// `{ "op": "editContent", "args": { "page": "p12.0", "edits": [{ "transform": { "object": { "ordinal": 3, "hash": "a3f1c2…" }, "matrix": [1, 0, 0, 1, 10, 0] } }] } }`.
///
/// [E7 plan]: https://github.com/laudebugs/laminapdf/blob/main/docs/plans/editor/e7-editing-existing-content.md
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct EditContent {
    /// The page.
    #[serde(with = "crate::wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The edits, applied in order.
    pub edits: Vec<ContentEdit>,
}

impl EditContent {
    /// The op that makes `edits` on `page`.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(page: PageId, edits: Vec<ContentEdit>) -> Self {
        Self { page, edits }
    }
}

/// An object a page draws, named so the name lasts (ADR 0022 §2): its 0-based ordinal among the
/// page's content objects, in the order they paint, and the 64-bit hash of its bytes, as 16
/// lowercase hex digits on the wire: `{ "ordinal": 3, "hash": "a3f1c2d4e5f60718" }`.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct ObjectRef {
    /// The ordinal.
    pub ordinal: u32,
    /// The hash.
    #[serde(with = "wire::hash")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub hash: u64,
}

impl ObjectRef {
    /// The object with this ordinal and hash.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn new(ordinal: u32, hash: u64) -> Self {
        Self { ordinal, hash }
    }

    /// The name `laminapdf-content` gives the object on `page`.
    fn on(self, page: &PageId) -> ContentObjectRef {
        ContentObjectRef {
            owner: ContentOwner::Page(page.clone()),
            ordinal: self.ordinal,
            hash: self.hash,
        }
    }
}

impl From<&ContentObjectRef> for ObjectRef {
    fn from(r: &ContentObjectRef) -> Self {
        Self::new(r.ordinal, r.hash)
    }
}

/// One change to an object ([`EditContent`]).
///
/// Added in 0.5.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum ContentEdit {
    /// Moves, resizes or rotates the object: where it drew a point `p` of the page, it now draws
    /// `matrix(p)`, `[a, b, c, d, e, f]` in page space (ISO 32000-2 §8.3.4's form).
    Transform {
        /// The object.
        object: ObjectRef,
        /// The matrix.
        #[serde(with = "crate::wire::matrix")]
        #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number, number, number]"))]
        matrix: Matrix,
    },
    /// Deletes the object.
    Delete {
        /// The object.
        object: ObjectRef,
    },
    /// Replaces an image with the JPEG or PNG in the blob store under `blob`
    /// (`laminapdf::Document::add_blob`), filling the same box.
    ReplaceImage {
        /// The image.
        object: ObjectRef,
        /// The new image's bytes.
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        blob: BlobId,
    },
    /// Clips an image, a form or a shading to `rect`, `[x0, y0, x1, y1]` in page space.
    Crop {
        /// The object.
        object: ObjectRef,
        /// The rectangle.
        #[serde(with = "crate::wire::rect_pt")]
        #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number]"))]
        rect: RectPt,
    },
    /// Moves the object in the stacking order.
    Reorder {
        /// The object.
        object: ObjectRef,
        /// Where it goes.
        to: StackPosition,
    },
}

// Ops compare by value (`Op: Eq`); NaN never reaches an edit from JSON, and is refused as
// `invalid-op` when a caller builds one.
impl Eq for ContentEdit {}

impl ContentEdit {
    /// The object the edit changes.
    ///
    /// Added in 0.5.0.
    #[must_use]
    pub const fn object(&self) -> ObjectRef {
        match self {
            Self::Transform { object, .. }
            | Self::Delete { object }
            | Self::ReplaceImage { object, .. }
            | Self::Crop { object, .. }
            | Self::Reorder { object, .. } => *object,
        }
    }
}

/// Where [`ContentEdit::Reorder`] puts an object: `"front"`, `"back"`, `{ "before": … }` or
/// `{ "after": … }` with another object of the page.
///
/// Added in 0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum StackPosition {
    /// Over every other object.
    Front,
    /// Under every other object.
    Back,
    /// Just under this object.
    Before(ObjectRef),
    /// Just over this object.
    After(ObjectRef),
}

impl EditContent {
    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        blobs: &BlobStore,
        images: Option<&dyn ImageXObjects>,
    ) -> Result<Effect> {
        self.check()?;
        let (page, frame) = {
            let pages = doc.pages()?;
            let page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            (page.cos().clone(), page.frame()?)
        };
        let mut ir = ContentOps::read_page(doc, &page, ContentOwner::Page(self.page.clone()))?;
        {
            let pages = doc.pages()?;
            let model_page = pages
                .by_id(&self.page)
                .ok_or_else(|| Error::UnknownPage(self.page.clone()))?;
            let mut recorder = ir.recorder(frame);
            interpret_page(doc, &model_page, &mut recorder)?;
        }
        let targets = self.resolve(&ir)?;
        let tagged_before = ir.drawn_mcids();
        let mut figures: BTreeSet<i64> = BTreeSet::new();
        // Tagged XObjects whose drawing a delete or a replacement takes away: their elements
        // can't be kept up to date here, so the save checks the page's tags.
        let mut taken: BTreeSet<ResourceName> = BTreeSet::new();
        for (edit, &(id, _)) in self.edits.iter().zip(&targets) {
            if matches!(edit, ContentEdit::Delete { .. } | ContentEdit::ReplaceImage { .. }) {
                taken.extend(ir.object_resource_names(id));
            }
        }
        let tagged_xobjects = structure::tagged_xobjects(doc, &page, &taken)?;
        // A form or Type 3 font without resources of its own draws with the page's (§7.8.3), so
        // a name the page's operators no longer use may still be needed: then nothing is pruned.
        let prune = !borrows_page_resources(doc, &page);
        let mut dropped: BTreeSet<ResourceName> = BTreeSet::new();
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        let mut removed = false;
        for (edit, &(id, placement)) in self.edits.iter().zip(&targets) {
            let refused = |reason| Error::ContentNotEditable { reason };
            let kind = ir.object(id).map(laminapdf_content::ContentObject::kind);
            check_kind(edit, kind)?;
            match edit {
                ContentEdit::Transform { matrix, .. } => ir.transform(id, matrix, &frame).map_err(refused)?,
                ContentEdit::Crop { rect, .. } => ir.crop(id, rect, &frame).map_err(refused)?,
                ContentEdit::Delete { .. } => {
                    dropped.extend(ir.object_resource_names(id));
                    ir.delete(id).map_err(refused)?;
                    removed = true;
                }
                ContentEdit::ReplaceImage { blob, .. } => {
                    let bytes = blobs.get(blob).ok_or(Error::UnknownBlob(*blob))?;
                    let images = images.ok_or(Error::ImagesUnavailable)?;
                    let written = images.write(writer.cos(), bytes)?.ok_or(Error::InvalidOp(
                        "the replacement image isn't a JPEG or PNG Lamina PDF reads",
                    ))?;
                    let name = writer.add_image(written.object)?;
                    figures.extend(ir.object_mcids(id));
                    dropped.extend(ir.object_resource_names(id));
                    ir.replace_image(id, name, &written.unit).map_err(refused)?;
                    removed = true;
                }
                ContentEdit::Reorder { .. } => {
                    ir.reorder(id, placement.unwrap_or(Placement::Front)).map_err(refused)?;
                }
            }
        }
        writer.write_parts(&ir, &ir.encode())?;
        if prune {
            let still_named = ir.resource_names();
            dropped.retain(|name| !still_named.contains(name));
            writer.remove_resources(&dropped)?;
        }
        drop(writer);
        let tagged_after = ir.drawn_mcids();
        let gone = structure::gone(&tagged_before, &tagged_after);
        let mut alt_cleared = Vec::new();
        if let Some(tags) = structure::PageTags::read(doc, &self.page)? {
            let owners: Vec<_> = figures
                .iter()
                .filter(|m| !gone.contains(m))
                .filter_map(|&m| tags.owner(m))
                .collect();
            tags.untag(doc, &gone)?;
            if !owners.is_empty()
                && let Some(tree) = doc.structure()?
            {
                for elem in owners {
                    if structure::clear_figure_alt(doc, &tree, elem)? {
                        alt_cleared.push(elem);
                    }
                }
            }
        }
        Ok(Effect {
            pages: vec![self.page.clone()],
            removed,
            dropped_xobjects: tagged_xobjects.into_iter().map(|r| (self.page.clone(), r)).collect(),
            alt_cleared,
            ..Effect::default()
        })
    }

    /// Refuses an op with no edits, or with a number that isn't finite.
    fn check(&self) -> Result<()> {
        if self.edits.is_empty() {
            return Err(Error::InvalidOp("editContent needs at least one edit"));
        }
        let finite = self.edits.iter().all(|edit| match edit {
            ContentEdit::Transform { matrix: m, .. } => [m.a, m.b, m.c, m.d, m.e, m.f].iter().all(|v| v.is_finite()),
            ContentEdit::Crop { rect: r, .. } => [r.x0, r.y0, r.x1, r.y1].iter().all(|v| v.get().is_finite()),
            _ => true,
        });
        if finite {
            Ok(())
        } else {
            Err(Error::InvalidOp("editContent's numbers must be finite"))
        }
    }

    /// Every edit's object, and a reorder's placement, found in the page as it is before the op.
    fn resolve(&self, ir: &ContentOps) -> Result<Vec<(ContentObjectId, Option<Placement>)>> {
        let find = |object| find(ir, &self.page, object);
        self.edits
            .iter()
            .map(|edit| {
                let placement = match edit {
                    ContentEdit::Reorder { to, .. } => Some(match to {
                        StackPosition::Front => Placement::Front,
                        StackPosition::Back => Placement::Back,
                        StackPosition::Before(r) => Placement::Before(find(*r)?),
                        StackPosition::After(r) => Placement::After(find(*r)?),
                    }),
                    _ => None,
                };
                Ok((find(edit.object())?, placement))
            })
            .collect()
    }
}

/// Refuses an edit its object's kind can't take: only an image can be replaced, and only an
/// image, a form or a shading cropped (a path's or text's clip is edited with it).
fn check_kind(edit: &ContentEdit, kind: Option<ContentObjectKind>) -> Result<()> {
    use ContentObjectKind as K;
    match (edit, kind) {
        (ContentEdit::ReplaceImage { .. }, Some(k)) if k != K::Image => {
            Err(Error::InvalidOp("replaceImage names an object that isn't an image"))
        }
        (ContentEdit::Crop { .. }, Some(K::Path | K::Text)) => Err(Error::InvalidOp(
            "crop names an image, a form or a shading, not a path or text",
        )),
        _ => Ok(()),
    }
}

/// The object `object` names on `page`, or `content-object-stale`.
fn find(ir: &ContentOps, page: &PageId, object: ObjectRef) -> Result<ContentObjectId> {
    ir.find(&object.on(page)).ok_or(Error::ContentObjectStale {
        ordinal: object.ordinal,
    })
}

#[cfg(test)]
mod tests;

/// Whether a Form XObject or Type 3 font in the page's resources has no `/Resources` of its own,
/// and so reads the page's (§7.8.3: a PDF 1.1 form or font may omit them).
fn borrows_page_resources(doc: &laminapdf_model::Document, page: &laminapdf_cos::Page) -> bool {
    let Some(resources) = page.resources_dict() else {
        return false;
    };
    let entries = |category: &str| -> Vec<laminapdf_cos::Object> {
        match resources.get(category).and_then(|o| doc.deref(o).ok()).as_deref() {
            Some(laminapdf_cos::Object::Dict(dict)) => dict.iter().map(|(_, o)| o.clone()).collect(),
            _ => Vec::new(),
        }
    };
    let borrows = |object: &laminapdf_cos::Object, subtype: &[u8]| -> bool {
        let Ok(object) = doc.deref(object) else {
            return false;
        };
        let dict = match &*object {
            laminapdf_cos::Object::Dict(dict) => dict,
            laminapdf_cos::Object::Stream(stream) => &stream.dict,
            _ => return false,
        };
        matches!(dict.get("Subtype"), Some(laminapdf_cos::Object::Name(name)) if name.as_bytes() == subtype)
            && dict.get("Resources").is_none()
    };
    entries("XObject").iter().any(|o| borrows(o, b"Form")) || entries("Font").iter().any(|o| borrows(o, b"Type3"))
}
