//! Content added to pages (E6 plan §2.1): [`AddContentObject`] puts an image, a shape or a page
//! of another PDF on a page as page content, not as an annotation; [`UpdateContentObject`]
//! moves, resizes, turns, fades, crops, recolours, restacks or replaces it, and
//! [`DeleteContentObject`] takes it off, before or after a save. [`added_objects`] lists what a
//! page holds.
//!
//! Each object is its own Form XObject, drawn by one small stream of its own in the page's
//! `/Contents` and named by the form's page-piece dictionary (`place`), so a change rewrites that
//! stream and never parses the page's own operators.

mod place;
mod spec;
mod tags;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use laminapdf_content::{PageContentWriter, ResourceName};
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{Matrix, PageFrame, PageId, PdfDate, Pt, RectPt, UserSpaceRect};
use serde::{Deserialize, Serialize};

use place::{Mark, Placement, Private, drawing, fade, mark_form, place_form, scan};
pub use spec::{Alt, ContentId, ContentLayer, ContentObjectSpec, PathSpec, Stroke, StrokeDash};

use crate::annot::{AnnotColor, AppearanceRefusal, Appearances, IntentSpace, Opacity};
use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// What `invalid-op` says of an object whose geometry is missing or out of range.
const INVALID: &str = "an added object's matrix, path, stroke or crop is not finite or out of range (a polygon needs three points, a shape a fill or a stroke)";

/// What [`Appearances::content`] draws: an object for [`AddContentObject`], as a Form XObject in
/// the object's own space ([`ContentObjectSpec`]).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct ContentRequest<'a> {
    /// The object.
    pub spec: &'a ContentObjectSpec,
    /// The session's bytes, which an image names.
    pub blobs: &'a BlobStore,
    /// The PDF a [`ContentObjectSpec::Form`] names, opened.
    pub source: Option<&'a laminapdf_model::Document>,
    /// The device space of a PDF/A file's output intent, which a shape paints in.
    pub intent: Option<IntentSpace>,
}

/// Adds an image, a shape or a page of another PDF to a page as page content (ISO 32000-2
/// §8.10): a Form XObject of its own, drawn over the page's content, or under it for
/// [`ContentLayer::Behind`], over the objects added there before.
///
/// `matrix` places the object's space ([`ContentObjectSpec`]) in page space ([ADR 0027]):
/// `[144, 0, 0, 44, 72, 72]` draws an image 144 by 44 points with its top-left corner 72 points
/// from the page's top-left corner as shown. An `opacity` below 1 is set by an ExtGState around
/// the drawing.
///
/// The form keeps `id` in its page-piece dictionary (§14.5), dated `modified`, so later ops find
/// the object after a save. An ID another added object on the page has is `content-id-in-use`.
/// In a tagged file the object is a `Figure` with `alt`'s text, else an artifact; on a file that
/// claims PDF/UA-1, an image or page with no `alt` is `content-needs-alt-text`. Bytes that aren't
/// a JPEG or PNG image, or a PDF with the page named, are `content-object-invalid`; a document
/// without a generator is `appearance-unavailable`.
///
/// On the wire: `{ "op": "addContentObject", "args": { "id": "logo", "page": "p3.0", "object":
/// { "image": { "blob": "…" } }, "matrix": [144, 0, 0, 44, 72, 72], "modified":
/// "D:20261118120000Z" } }`.
///
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddContentObject {
    /// The object's ID, unique on the page.
    pub id: ContentId,
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The object.
    pub object: ContentObjectSpec,
    /// The object's space to page space, `[a, b, c, d, e, f]`.
    #[serde(with = "wire::matrix")]
    #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number, number, number]"))]
    pub matrix: Matrix,
    /// The opacity; opaque when absent.
    #[serde(default, skip_serializing_if = "Opacity::is_opaque")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub opacity: Opacity,
    /// In front of the page's content, or behind it.
    #[serde(default, skip_serializing_if = "ContentLayer::is_front")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub layer: ContentLayer,
    /// The alternate text, or decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub alt: Option<Alt>,
    /// The form's `/LastModified`.
    #[serde(with = "wire::date")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub modified: PdfDate,
}

// Sound: as for `ContentObjectSpec`; a matrix that isn't finite is refused before it writes.
impl Eq for AddContentObject {}

impl AddContentObject {
    /// The op that adds `object` to `page` as `id`, placed by `matrix`, opaque, in front, with
    /// no alternate text.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(id: ContentId, page: PageId, object: ContentObjectSpec, matrix: Matrix, modified: PdfDate) -> Self {
        Self {
            id,
            page,
            object,
            matrix,
            opacity: Opacity::OPAQUE,
            layer: ContentLayer::Front,
            alt: None,
            modified,
        }
    }

    /// The same op with `opacity`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = opacity;
        self
    }

    /// The same op, behind the page's content or in front of it.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_layer(mut self, layer: ContentLayer) -> Self {
        self.layer = layer;
        self
    }

    /// The same op with alternate text, or as decoration.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_alt(mut self, alt: Alt) -> Self {
        self.alt = Some(alt);
        self
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        if !self.object.is_valid() || !placeable(&self.matrix) {
            return Err(Error::InvalidOp(INVALID));
        }
        let (page, frame) = page_parts(doc, &self.page)?;
        let page_ref = page_ref(&page, &self.page)?;
        let placed = scan(doc, &page)?;
        if placed.iter().any(|p| p.private.id == self.id) {
            return Err(Error::ContentIdInUse(self.id.clone()));
        }
        let picture = !matches!(self.object, ContentObjectSpec::Shape { .. });
        if picture && self.alt.is_none() && crate::pages::claims_pdfua(doc)? {
            return Err(Error::ContentNeedsAltText);
        }
        let form = draw(doc, appearances, blobs, &self.object)?;
        let private = Private {
            id: self.id.clone(),
            kind: self.object.kind().to_owned(),
            spec: Some(self.object.clone()),
            alt: self.alt.clone(),
            layer: self.layer,
        };
        mark_form(doc, form, &private, &self.modified)?;
        let mark = tags::tag(doc, page_ref, self.alt.as_ref())?;
        let behind: Vec<ObjRef> = placed
            .iter()
            .filter(|p| p.private.layer == ContentLayer::Behind)
            .map(|p| p.stream)
            .collect();
        let placement = Placement {
            cm: self.matrix.then(&frame.to_user_space_matrix()),
            opacity: self.opacity.get(),
            mark,
            layer: self.layer,
        };
        place_form(doc, &page, form, placement, &behind)?;
        Ok(Effect {
            pages: vec![self.page.clone()],
            ..Effect::default()
        })
    }
}

/// Changes an object Lamina PDF added to a page ([`AddContentObject`]), before or after a save.
/// Each field left out keeps its value.
///
/// - `matrix` and `opacity` move, resize, turn and fade it.
/// - `crop` (an image's; `null` shows it whole), `fill` and `stroke` (a shape's; `null` removes
///   one) and `blob` (an image's or a page's new bytes) draw its form again. A field for another
///   kind is `invalid-op`. An image's crop changes its form's clip only, so it works on an image
///   added before the file was saved, whose bytes the session no longer holds; the other three
///   need the bytes the object names to be in the session (`unknown-blob`).
/// - `alt` sets its alternate text, or makes it decoration.
/// - `order` moves it among the objects added on its layer: its index from the back, 0 the
///   backmost, or counted from the front when negative, -1 the frontmost.
///
/// The form's `/LastModified` becomes `modified`. An ID the page draws no added object with is
/// `unknown-content-object`.
///
/// On the wire: `{ "op": "updateContentObject", "args": { "id": "logo", "page": "p3.0",
/// "modified": "D:20261118120000Z", "opacity": 0.5, "order": -1 } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct UpdateContentObject {
    /// The object.
    pub id: ContentId,
    /// Its page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The form's new `/LastModified`.
    #[serde(with = "wire::date")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub modified: PdfDate,
    /// The new matrix.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::matrix::opt")]
    #[cfg_attr(
        feature = "tsify",
        tsify(optional, type = "[number, number, number, number, number, number]")
    )]
    pub matrix: Option<Matrix>,
    /// The new opacity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub opacity: Option<Opacity>,
    /// An image's new crop; `null` shows it whole.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::rect_pt::double")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "[number, number, number, number] | null"))]
    pub crop: Option<Option<RectPt>>,
    /// A shape's new fill; `null` removes it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "AnnotColor | null"))]
    pub fill: Option<Option<AnnotColor>>,
    /// A shape's new stroke; `null` removes it.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "Stroke | null"))]
    pub stroke: Option<Option<Stroke>>,
    /// An image's or a page's new bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub blob: Option<BlobId>,
    /// The new alternate text, or decoration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub alt: Option<Alt>,
    /// The new place among the objects added on its layer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub order: Option<i32>,
}

// Sound: as for `AddContentObject`.
impl Eq for UpdateContentObject {}

impl UpdateContentObject {
    /// The op that changes nothing yet of `id` on `page` but its `/LastModified`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, page: PageId, modified: PdfDate) -> Self {
        Self {
            id,
            page,
            modified,
            matrix: None,
            opacity: None,
            crop: None,
            fill: None,
            stroke: None,
            blob: None,
            alt: None,
            order: None,
        }
    }

    /// The same op, moving the object to `matrix`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_matrix(mut self, matrix: Matrix) -> Self {
        self.matrix = Some(matrix);
        self
    }

    /// The same op, setting the opacity.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_opacity(mut self, opacity: Opacity) -> Self {
        self.opacity = Some(opacity);
        self
    }

    /// The same op, cropping an image to `crop`, or showing it whole for `None`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_crop(mut self, crop: Option<RectPt>) -> Self {
        self.crop = Some(crop);
        self
    }

    /// The same op, filling a shape with `fill`, or with nothing for `None`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_fill(mut self, fill: Option<AnnotColor>) -> Self {
        self.fill = Some(fill);
        self
    }

    /// The same op, stroking a shape with `stroke`, or with nothing for `None`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_stroke(mut self, stroke: Option<Stroke>) -> Self {
        self.stroke = Some(stroke);
        self
    }

    /// The same op, replacing an image's or a page's bytes with `blob`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_blob(mut self, blob: BlobId) -> Self {
        self.blob = Some(blob);
        self
    }

    /// The same op, setting the alternate text or making the object decoration.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn with_alt(mut self, alt: Alt) -> Self {
        self.alt = Some(alt);
        self
    }

    /// The same op, moving the object to `order` among those on its layer.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn with_order(mut self, order: i32) -> Self {
        self.order = Some(order);
        self
    }

    /// Whether the patch changes what the form draws: an image's crop, a shape's fill or
    /// stroke, or new bytes.
    const fn redraws(&self) -> bool {
        self.crop.is_some() || self.fill.is_some() || self.stroke.is_some() || self.blob.is_some()
    }

    /// The spec with the patch's crop, fill, stroke and bytes; `None` when it changes none of
    /// them. A field for another kind of object is `invalid-op`.
    fn patched(&self, spec: &ContentObjectSpec) -> Result<Option<ContentObjectSpec>> {
        const WRONG_KIND: &str = "the patch changes what another kind of added object has (a crop is an image's, a fill and stroke a shape's, new bytes an image's or a page's)";
        if !self.redraws() {
            return Ok(None);
        }
        let mut spec = spec.clone();
        match &mut spec {
            ContentObjectSpec::Image { blob, crop } => {
                if self.fill.is_some() || self.stroke.is_some() {
                    return Err(Error::InvalidOp(WRONG_KIND));
                }
                if let Some(new) = self.crop {
                    *crop = new;
                }
                if let Some(new) = self.blob {
                    *blob = new;
                }
            }
            ContentObjectSpec::Shape { fill, stroke, .. } => {
                if self.crop.is_some() || self.blob.is_some() {
                    return Err(Error::InvalidOp(WRONG_KIND));
                }
                if let Some(new) = self.fill {
                    *fill = new;
                }
                if let Some(new) = &self.stroke {
                    stroke.clone_from(new);
                }
            }
            ContentObjectSpec::Form { blob, .. } => {
                if self.crop.is_some() || self.fill.is_some() || self.stroke.is_some() {
                    return Err(Error::InvalidOp(WRONG_KIND));
                }
                if let Some(new) = self.blob {
                    *blob = new;
                }
            }
        }
        if !spec.is_valid() {
            return Err(Error::InvalidOp(INVALID));
        }
        Ok(Some(spec))
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        if self.matrix.as_ref().is_some_and(|m| !placeable(m)) {
            return Err(Error::InvalidOp(INVALID));
        }
        let (page, frame) = page_parts(doc, &self.page)?;
        let page_ref = page_ref(&page, &self.page)?;
        let placed = scan(doc, &page)?;
        let old = placed
            .iter()
            .find(|p| p.private.id == self.id)
            .ok_or_else(|| Error::UnknownContentObject(self.id.clone()))?
            .clone();
        let to_user = frame.to_user_space_matrix();
        let to_page = to_user
            .invert()
            .ok_or(Error::InvalidOp("the page's frame can't be inverted"))?;
        let mut private = old.private.clone();
        // The form drawn from now on: the old one, its clip changed for a crop alone, or a new one.
        let mut form = old.form;
        let patched = match &old.private.spec {
            Some(spec) => self.patched(spec)?,
            None if !self.redraws() => None,
            None => {
                return Err(Error::InvalidOp(
                    "the added object's form doesn't say what it holds, so it can't be drawn again",
                ));
            }
        };
        if let Some(spec) = patched {
            // A crop alone changes the clip, which needs none of the image's bytes.
            let crop_only = self.fill.is_none() && self.stroke.is_none() && self.blob.is_none();
            if let (true, ContentObjectSpec::Image { crop, .. }) = (crop_only, &spec) {
                set_bbox(doc, form, crop.unwrap_or(ContentObjectSpec::UNIT))?;
            } else {
                form = draw(doc, appearances, blobs, &spec)?;
            }
            private.spec = Some(spec);
        }
        if let Some(alt) = &self.alt {
            private.alt = Some(alt.clone());
        }
        mark_form(doc, form, &private, &self.modified)?;
        // The alternate text, kept by the same `Figure` while there is text, else the object
        // becomes an artifact (or a figure, from one).
        let mut mark = old.drawing.mark;
        if let Some(alt) = &self.alt {
            mark = match (old.drawing.mark, alt) {
                (Mark::Figure(mcid), Alt::Text(text)) => {
                    tags::set_alt(doc, page_ref, mcid, text)?;
                    Mark::Figure(mcid)
                }
                (Mark::Figure(mcid), Alt::Decorative) => {
                    tags::untag(doc, page_ref, mcid)?;
                    Mark::Artifact
                }
                (Mark::Artifact, Alt::Text(_)) => tags::tag(doc, page_ref, Some(alt))?,
                (other, _) => other,
            };
        }
        let matrix = match self.matrix {
            Some(m) => m,
            None => old.drawing.cm.then(&to_page),
        };
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        // A new ExtGState only for a new opacity: the old one stays the drawing's otherwise.
        let gs = match self.opacity {
            Some(opacity) => fade(&mut writer, opacity.get())?,
            None => old.drawing.gs.clone(),
        };
        let new = drawing(&mut writer, form, matrix.then(&to_user), gs, mark)?;
        let index = writer
            .contents()
            .iter()
            .position(|o| o.as_ref() == Some(old.stream))
            .unwrap_or(old.index);
        writer.remove(old.stream)?;
        let stream = writer.insert(index, &new.to_bytes())?;
        // The old form and ExtGState, when the drawing no longer names them: both are the
        // object's own (`drawing` registers its ExtGState as an object of its own).
        let mut gone = BTreeSet::new();
        if new.form != old.drawing.form {
            gone.insert(ResourceName::new("XObject", old.drawing.form.clone()));
        }
        if let Some(gs) = old.drawing.gs.as_ref().filter(|gs| new.gs.as_ref() != Some(*gs)) {
            gone.insert(ResourceName::new("ExtGState", gs.clone()));
        }
        writer.remove_resources(&gone)?;
        if let Some(order) = self.order {
            let layer: Vec<ObjRef> = placed
                .iter()
                .filter(|p| p.private.layer == old.private.layer)
                .map(|p| if p.stream == old.stream { stream } else { p.stream })
                .collect();
            restack(&mut writer, &layer, stream, order)?;
        }
        Ok(Effect {
            pages: vec![self.page.clone()],
            ..Effect::default()
        })
    }
}

/// Takes an object Lamina PDF added off its page, with its `Figure` in a tagged file. The
/// page's earlier revisions still draw it after an incremental save. An ID the page draws no
/// added object with is `unknown-content-object`.
///
/// On the wire: `{ "op": "deleteContentObject", "args": { "id": "logo", "page": "p3.0" } }`.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeleteContentObject {
    /// The object.
    pub id: ContentId,
    /// Its page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
}

impl DeleteContentObject {
    /// The op that deletes `id` from `page`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub const fn new(id: ContentId, page: PageId) -> Self {
        Self { id, page }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let (page, _) = page_parts(doc, &self.page)?;
        let page_ref = page_ref(&page, &self.page)?;
        let old = scan(doc, &page)?
            .into_iter()
            .find(|p| p.private.id == self.id)
            .ok_or_else(|| Error::UnknownContentObject(self.id.clone()))?;
        if let Mark::Figure(mcid) = old.drawing.mark {
            tags::untag(doc, page_ref, mcid)?;
        }
        let mut writer = PageContentWriter::new(doc.cos_mut().edit(), &page)?;
        writer.remove(old.stream)?;
        let mut gone: BTreeSet<ResourceName> = [ResourceName::new("XObject", old.drawing.form)].into();
        gone.extend(old.drawing.gs.map(|gs| ResourceName::new("ExtGState", gs)));
        writer.remove_resources(&gone)?;
        Ok(Effect {
            pages: vec![self.page.clone()],
            removed: true,
            ..Effect::default()
        })
    }
}

/// An object Lamina PDF added to a page, as [`added_objects`] lists it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddedObject {
    /// Its ID.
    pub id: ContentId,
    /// The object as it was added or last changed; `None` when its form was written by a version
    /// that stored it differently.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub object: Option<ContentObjectSpec>,
    /// Its space to page space.
    #[serde(with = "wire::matrix")]
    #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number, number, number]"))]
    pub matrix: Matrix,
    /// Its opacity.
    pub opacity: Opacity,
    /// In front of the page's content, or behind it.
    pub layer: ContentLayer,
    /// Its alternate text, or decoration.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub alt: Option<Alt>,
    /// The page-space box its form covers, its `/BBox` placed by `matrix`.
    #[serde(with = "wire::rect_pt")]
    #[cfg_attr(feature = "tsify", tsify(type = "[number, number, number, number]"))]
    pub bounds: RectPt,
}

/// The objects Lamina PDF added to `page` ([`AddContentObject`]), in the order they paint: those
/// behind the page's content first. An object whose drawing or form another app changed is the
/// page's own content and isn't listed.
///
/// # Errors
/// `unknown-page`; reading errors.
///
/// Added in 0.7.0.
pub fn added_objects(doc: &laminapdf_model::Document, page: &PageId) -> Result<Vec<AddedObject>> {
    let (cos_page, frame) = page_parts(doc, page)?;
    let to_page = frame
        .to_user_space_matrix()
        .invert()
        .ok_or(Error::InvalidOp("the page's frame can't be inverted"))?;
    Ok(scan(doc, &cos_page)?
        .into_iter()
        .map(|p| {
            let matrix = p.drawing.cm.then(&to_page);
            let bbox = p
                .form_dict
                .get("BBox")
                .and_then(crate::annot::geom::rect_of)
                .map(|r| RectPt::new(Pt::new(r.x0), Pt::new(r.y0), Pt::new(r.x1), Pt::new(r.y1)));
            AddedObject {
                id: p.private.id,
                object: p.private.spec,
                matrix,
                opacity: Opacity::new(p.opacity).unwrap_or_default(),
                layer: p.private.layer,
                alt: p.private.alt,
                bounds: bounds(&matrix, bbox.unwrap_or(ContentObjectSpec::UNIT)),
            }
        })
        .collect())
}

/// Whether `m` is finite and invertible, so it places something that can be seen and moved.
fn placeable(m: &Matrix) -> bool {
    // A determinant this small squashes the object to a line no reader draws, and its inverse,
    // which a later move needs, loses every digit.
    const FLAT: f64 = 1e-12;
    m.to_array().iter().all(|v| v.is_finite()) && (m.a * m.d - m.b * m.c).abs() > FLAT
}

/// The page-space box `rect` of an object's space covers once placed by `m`.
fn bounds(m: &Matrix, rect: RectPt) -> RectPt {
    let space = UserSpaceRect::from_corners(rect.x0.get(), rect.y0.get(), rect.x1.get(), rect.y1.get());
    let placed = space.transformed(m);
    RectPt::new(
        Pt::new(placed.x0),
        Pt::new(placed.y0),
        Pt::new(placed.x1),
        Pt::new(placed.y1),
    )
}

/// The page's COS page and its frame.
fn page_parts(doc: &laminapdf_model::Document, id: &PageId) -> Result<(laminapdf_cos::Page, PageFrame)> {
    let pages = doc.pages()?;
    let page = pages.by_id(id).ok_or_else(|| Error::UnknownPage(id.clone()))?;
    Ok((page.cos().clone(), page.frame()?))
}

fn page_ref(page: &laminapdf_cos::Page, id: &PageId) -> Result<ObjRef> {
    page.obj_ref.ok_or_else(|| Error::DirectPage(id.clone()))
}

/// Draws `spec`'s form through the document's generator.
fn draw(
    doc: &mut laminapdf_model::Document,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
    spec: &ContentObjectSpec,
) -> Result<ObjRef> {
    let generator = appearances.ok_or(Error::AppearanceUnavailable)?;
    let source = match spec {
        ContentObjectSpec::Form { blob, page } => {
            let bytes = blobs.get(blob).ok_or(Error::UnknownBlob(*blob))?;
            let source = laminapdf_model::Document::open(bytes.clone(), laminapdf_model::OpenOptions::default())
                .map_err(|_| Error::ContentObjectInvalid)?;
            let count = source.pages().map_err(|_| Error::ContentObjectInvalid)?.len();
            if usize::try_from(*page).map_or(true, |p| p >= count) {
                return Err(Error::ContentObjectInvalid);
            }
            Some(source)
        }
        ContentObjectSpec::Image { blob, .. } => {
            blobs.get(blob).ok_or(Error::UnknownBlob(*blob))?;
            None
        }
        ContentObjectSpec::Shape { .. } => None,
    };
    let request = ContentRequest {
        spec,
        blobs,
        source: source.as_ref(),
        intent: crate::annot::add::intent_space(doc)?,
    };
    generator.check_content(&request).map_err(|refusal| match refusal {
        AppearanceRefusal::ImageInvalid => Error::ContentObjectInvalid,
        other => Error::from(other),
    })?;
    Ok(generator.content(&mut doc.cos_mut().edit(), &request)?)
}

/// Gives the form `form` the clip `bbox`, keeping everything else.
fn set_bbox(doc: &mut laminapdf_model::Document, form: ObjRef, bbox: RectPt) -> Result<()> {
    let mut cos = doc.cos_mut().edit();
    let Some(current) = cos.get(form)? else {
        return Ok(());
    };
    if let Object::Stream(mut stream) = (*current).clone() {
        let corners = [bbox.x0, bbox.y0, bbox.x1, bbox.y1].map(|v| Object::number(v.get()));
        stream.dict.insert("BBox", Object::Array(corners.into()));
        cos.set(form, Object::Stream(stream))?;
    }
    Ok(())
}

/// Moves `stream` to `order` among `layer`'s streams (in painting order, `stream` among them):
/// its index from the back, or from the front when negative.
fn restack(writer: &mut PageContentWriter<'_>, layer: &[ObjRef], stream: ObjRef, order: i32) -> Result<()> {
    let others: Vec<ObjRef> = layer.iter().copied().filter(|r| *r != stream).collect();
    let count = i64::try_from(others.len()).unwrap_or(i64::MAX);
    let rank = i64::from(order);
    let rank = if rank < 0 { count + 1 + rank } else { rank }.clamp(0, count);
    let rank = usize::try_from(rank).unwrap_or(0);
    let contents = writer.contents();
    // The stream goes right before the object now at `rank`, or right after the last one.
    let at = match others.get(rank) {
        Some(next) => contents.iter().position(|o| o.as_ref() == Some(*next)),
        None => others
            .last()
            .and_then(|last| contents.iter().position(|o| o.as_ref() == Some(*last)))
            .map(|i| i + 1),
    };
    let Some(mut at) = at else {
        return Ok(());
    };
    // Counted once the stream is taken out.
    if contents
        .iter()
        .position(|o| o.as_ref() == Some(stream))
        .is_some_and(|i| i < at)
    {
        at -= 1;
    }
    writer.move_to(stream, at)?;
    Ok(())
}

/// The pages among `pages` whose added objects break a PDF/A claim, each list in page order.
#[derive(Debug, Default)]
pub(crate) struct PdfaPages {
    /// Those with a see-through object, on PDF/A-1.
    pub(crate) transparent: Vec<String>,
    /// Those with a form that uses a font that isn't embedded.
    pub(crate) unembedded: Vec<String>,
    /// Those with an image or a form that paints in a device space the output intent doesn't
    /// allow.
    pub(crate) off_intent: Vec<String>,
}

/// What the added objects on `pages` break of the file's PDF/A-`part` claim.
pub(crate) fn pdfa_pages(doc: &laminapdf_model::Document, pages: &BTreeSet<PageId>, part: u8) -> Result<PdfaPages> {
    let rules = crate::metadata::ColorRules::of(crate::metadata::output_intent(doc)?, part);
    let mut out = PdfaPages::default();
    for page in doc.pages()?.iter() {
        if !pages.contains(page.id()) {
            continue;
        }
        let placed = scan(doc, page.cos())?;
        if part == 1 && see_through(doc, &placed)? {
            out.transparent.push(page.id().to_string());
        }
        let (mut embedded, mut fits) = (true, true);
        for p in &placed {
            let form = Object::Ref(p.form);
            embedded &= fonts_embedded(doc, &form, 0)?;
            if let Some(rules) = rules {
                fits &= crate::metadata::colors_fit(doc, &form, rules, page.cos().resources.clone())?;
            }
        }
        if !embedded {
            out.unembedded.push(page.id().to_string());
        }
        if !fits {
            out.off_intent.push(page.id().to_string());
        }
    }
    Ok(out)
}

/// How deep the PDF/A checks of an added form, [`fonts_embedded`] and the colour spaces' check,
/// look into forms inside forms: an added page is one form inside the object's, and its own forms
/// one more.
pub(crate) const MAX_FORM_DEPTH: u8 = 4;

/// Whether every font the form `form` uses is embedded, its own and those of the forms it draws.
fn fonts_embedded(doc: &laminapdf_model::Document, form: &Object, depth: u8) -> Result<bool> {
    let Object::Stream(s) = &*doc.deref(form)? else {
        return Ok(true);
    };
    if !crate::annot::form_fonts_embedded(doc, s)? {
        return Ok(false);
    }
    if depth >= MAX_FORM_DEPTH {
        return Ok(true);
    }
    let xobjects = match s.dict.get("Resources") {
        Some(r) => match doc.deref(r)?.as_dict().and_then(|d| d.get("XObject")) {
            Some(x) => doc.deref(x)?.as_dict().cloned().unwrap_or_default(),
            None => return Ok(true),
        },
        None => return Ok(true),
    };
    for (_, x) in xobjects.iter() {
        if !fonts_embedded(doc, x, depth + 1)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether any of `placed` is see-through: faded, or its form draws transparency of its own.
fn see_through(doc: &laminapdf_model::Document, placed: &[place::Placed]) -> Result<bool> {
    for p in placed {
        if p.opacity < 1.0 || form_is_transparent(doc, &p.form_dict)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether an added form draws transparency of its own: an image under it with a soft mask, or
/// a transparency group.
fn form_is_transparent(doc: &laminapdf_model::Document, form: &Dict) -> Result<bool> {
    if form.get("Group").is_some() {
        return Ok(true);
    }
    let resources = match form.get("Resources") {
        Some(r) => doc.deref(r)?.as_dict().cloned().unwrap_or_default(),
        None => return Ok(false),
    };
    let xobjects = match resources.get("XObject") {
        Some(x) => doc.deref(x)?.as_dict().cloned().unwrap_or_default(),
        None => return Ok(false),
    };
    for (_, x) in xobjects.iter() {
        if let Object::Stream(s) = &*doc.deref(x)?
            && (s.dict.get("SMask").is_some() || s.dict.get("Group").is_some())
        {
            return Ok(true);
        }
    }
    Ok(false)
}
