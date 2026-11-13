//! The annotation ops' shared core (ISO 32000-2 §12.5): [`UpdateAnnot`] changes the common and
//! markup entries of any annotation and moves it, and [`DeleteAnnot`] removes it with its Popup,
//! its replies and its group. Both are class `Annotation` and never need a full rewrite.
//!
//! Every write goes through the overlay: an annotation is changed in place (it keeps its object
//! and so its [`AnnotId`]), and a deleted one is unlinked from its page and freed. Keys the ops
//! don't own are kept untouched.

mod add;
mod appearance;
mod geom;
mod redraw;
mod spec;
mod tags;

pub use add::{AddAnnot, AnnotCommon};
pub(crate) use appearance::Generator;
pub use appearance::{Appearance, AppearanceRequest, Appearances, IntentSpace};
pub use geom::{AnnotColor, Opacity};
pub use spec::{
    AnnotSpec, AnnotSpecPatch, Caret, CaretPatch, CaretSymbol, MarkupKind, Note, NoteIcon, NotePatch, TextMarkup,
    TextMarkupPatch,
};

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{AnnotFlags, AnnotId, AnnotLink, AnnotSubtype, PageId, PdfDate};
use laminapdf_syntax::text::encode_text_string;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// A move in user space (ISO 32000-2 §8.3.2.3: y up), in user-space units. Both distances are
/// finite: [`Offset::new`] and the wire form refuse anything else, so a move never writes a
/// coordinate a PDF can't hold.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub struct Offset {
    dx: f64,
    dy: f64,
}

impl Offset {
    /// The move by `(dx, dy)`; `None` unless both are finite.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(dx: f64, dy: f64) -> Option<Self> {
        if dx.is_finite() && dy.is_finite() {
            Some(Self { dx, dy })
        } else {
            None
        }
    }

    /// Along x.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn dx(self) -> f64 {
        self.dx
    }

    /// Along y (up is positive).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn dy(self) -> f64 {
        self.dy
    }
}

// Sound: both fields are finite (`Offset::new` and the deserialiser check), so no NaN breaks
// reflexivity.
impl Eq for Offset {}

impl<'de> Deserialize<'de> for Offset {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> core::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            dx: f64,
            dy: f64,
        }
        let wire = Wire::deserialize(d)?;
        Self::new(wire.dx, wire.dy).ok_or_else(|| serde::de::Error::custom("a move must be finite"))
    }
}

/// Changes an annotation's common and markup entries and moves it.
///
/// An absent field leaves the entry alone and `null` removes it. `translate` moves `/Rect` and
/// the subtype's geometry (`/QuadPoints`, `/InkList`, `/Vertices`, `/L`, `/CL`) and the
/// annotation's Popup with it, and keeps the appearance, which §12.5.5 maps onto the new
/// `/Rect`. A `Locked` annotation refuses everything but a change of its flags and its
/// contents; a `LockedContents` one refuses a change of its contents (`annotation-locked`).
/// Widgets and annotations stored as direct dictionaries are refused (`annotation-not-editable`).
///
/// On the wire: `{ "op": "updateAnnot", "args": { "id": "a12.0", "contents": "Checked",
/// "translate": { "dx": 0, "dy": -10 } } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct UpdateAnnot {
    /// The annotation.
    #[serde(with = "wire::annot_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub id: AnnotId,
    /// `/Contents`: the comment, or the alternative description.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub contents: Option<Option<String>>,
    /// `/T`: the author.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub author: Option<Option<String>>,
    /// `/Subj`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub subject: Option<Option<String>>,
    /// `/M`, the modification date the edit writes.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub modified: Option<PdfDate>,
    /// `/F`, all the flags at once (Table 167).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub flags: Option<u32>,
    /// A move in user space.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub translate: Option<Offset>,
    /// `/C`; `null` removes it, and the mark is drawn in its usual colour. The appearance is
    /// drawn again.
    ///
    /// Added in 0.3.0.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "wire::double_option::deserialize"
    )]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "[number] | [number, number, number] | [number, number, number, number] | null"
        )
    )]
    pub color: Option<Option<AnnotColor>>,
    /// The opacity. The appearance is drawn again.
    ///
    /// Added in 0.3.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub opacity: Option<Opacity>,
    /// A change to the mark's geometry or icon. The appearance is drawn again.
    ///
    /// Added in 0.3.0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub spec: Option<AnnotSpecPatch>,
}

impl UpdateAnnot {
    /// The op that changes nothing yet on `id`; set the fields to change.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(id: AnnotId) -> Self {
        Self {
            id,
            contents: None,
            author: None,
            subject: None,
            modified: None,
            flags: None,
            translate: None,
            color: None,
            opacity: None,
            spec: None,
        }
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
    ) -> Result<Effect> {
        let all = every_annotation(doc)?;
        let found = Found::locate(&all, &self.id)?;
        let flags = found.annot.flags;
        let redraw = redraw::Redraw {
            color: self.color,
            opacity: self.opacity,
            spec: self.spec.as_ref(),
        };
        let properties = self.author.is_some() || self.subject.is_some() || self.translate.is_some() || redraw.any();
        if (properties && flags.has(AnnotFlags::LOCKED))
            || (self.contents.is_some() && flags.has(AnnotFlags::LOCKED_CONTENTS))
        {
            return Err(Error::AnnotationLocked(self.id.clone()));
        }
        let mut dict = found.dict(doc)?;
        let before = dict.clone();
        set_text(&mut dict, "Contents", self.contents.as_ref());
        set_text(&mut dict, "T", self.author.as_ref());
        set_text(&mut dict, "Subj", self.subject.as_ref());
        if let Some(flags) = self.flags {
            dict.insert("F", Object::Integer(i64::from(flags)));
        }
        if redraw.any() {
            redraw.apply(doc, &mut dict, appearances)?;
        }
        // The Popup moves with its parent, unless it is locked, which refuses the move.
        let popup = match (self.translate, &found.annot.popup) {
            (Some(offset), Some(popup)) => match Found::locate(&all, popup) {
                Ok(popup) => {
                    let mut moved = popup.dict(doc)?;
                    let was = moved.clone();
                    translate(&mut moved, offset);
                    if moved != was && popup.annot.flags.has(AnnotFlags::LOCKED) {
                        return Err(Error::AnnotationLocked(popup.annot.id));
                    }
                    (moved != was).then_some((popup, moved))
                }
                Err(_) => None,
            },
            _ => None,
        };
        if let Some(offset) = self.translate {
            translate(&mut dict, offset);
        }
        if dict == before && popup.is_none() {
            return Ok(Effect::default());
        }
        let mut pages = vec![found.page.clone()];
        if dict != before {
            if let Some(at) = &self.modified {
                dict.insert("M", Object::String(PdfString::literal(at.to_pdf_string().into_bytes())));
            }
            doc.cos_mut().edit().set(found.object, Object::Dict(dict))?;
        }
        if let Some((popup, moved)) = popup {
            doc.cos_mut().edit().set(popup.object, Object::Dict(moved))?;
            if popup.page != found.page {
                pages.push(popup.page);
            }
        }
        Ok(Effect {
            pages,
            ..Effect::default()
        })
    }
}

/// Deletes an annotation with what belongs to it: its Popup, every reply whose `/IRT` chain
/// reaches it, and, for a member of a group (`/RT /Group`, §12.5.6.2), the whole group, which
/// is one unit. Chains are followed through cycles once. In a tagged document, the structure
/// element's object reference to each deleted annotation is removed, and so is an element left
/// with nothing in it. Each deleted object is freed.
///
/// A `Locked` annotation is refused (`annotation-locked`), and so are widgets and direct
/// dictionaries (`annotation-not-editable`). Deleting a Popup alone removes the `/Popup` entry
/// that names it.
///
/// On the wire: `{ "op": "deleteAnnot", "args": { "id": "a12.0" } }`.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeleteAnnot {
    /// The annotation.
    #[serde(with = "wire::annot_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub id: AnnotId,
}

impl DeleteAnnot {
    /// The op that deletes `id`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(id: AnnotId) -> Self {
        Self { id }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let all = every_annotation(doc)?;
        let found = Found::locate(&all, &self.id)?;
        let doomed = doomed(&found.annot, &all);
        if let Some(locked) = all
            .iter()
            .flat_map(|(_, annots)| annots)
            .find(|a| doomed.contains(&a.id) && a.flags.has(AnnotFlags::LOCKED))
        {
            return Err(Error::AnnotationLocked(locked.id.clone()));
        }
        let parent_tree = parent_tree(doc)?;
        let mut pages = Vec::new();
        let mut arrays = Vec::new();
        for (page, annots) in &all {
            let gone: Vec<usize> = annots
                .iter()
                .filter(|a| doomed.contains(&a.id))
                .map(|a| a.index)
                .collect();
            if !gone.is_empty() {
                unlink(doc, page, &gone, &mut arrays)?;
                pages.push(page.clone());
            }
            for annot in annots.iter().filter(|a| !doomed.contains(&a.id)) {
                if annot.popup.as_ref().is_some_and(|p| doomed.contains(p)) {
                    drop_entry(doc, annot, "Popup")?;
                }
            }
        }
        let mut freed_keys = Vec::new();
        for id in &doomed {
            if let Some(r) = id.as_lamina_id().obj_ref() {
                if let Some(key) = untag(doc, parent_tree.as_ref(), r)? {
                    freed_keys.push(key);
                }
                doc.cos_mut().edit().delete(r)?;
            }
        }
        if let Some((root, tree)) = &parent_tree {
            drop_parent_tree_keys(doc, *root, tree, &freed_keys)?;
        }
        Ok(Effect {
            pages,
            removed: true,
            also_deleted: doomed.into_iter().filter(|id| *id != self.id).collect(),
            ..Effect::default()
        })
    }
}

/// The annotation an op names, with its page and object.
pub(crate) struct Found {
    pub(crate) page: PageId,
    pub(crate) object: ObjRef,
    pub(crate) annot: AnnotLink,
}

impl Found {
    pub(crate) fn locate(all: &[(PageId, Vec<AnnotLink>)], id: &AnnotId) -> Result<Self> {
        for (page, annots) in all {
            if let Some(annot) = annots.iter().find(|a| a.id == *id) {
                let (page, annot) = (page.clone(), annot.clone());
                let object = id
                    .as_lamina_id()
                    .obj_ref()
                    .filter(|_| annot.subtype != AnnotSubtype::Widget)
                    .ok_or_else(|| Error::AnnotationNotEditable(id.clone()))?;
                return Ok(Self { page, object, annot });
            }
        }
        Err(Error::UnknownAnnotation(id.clone()))
    }

    pub(crate) fn dict(&self, doc: &laminapdf_model::Document) -> Result<Dict> {
        object_dict(doc, self.object)?.ok_or_else(|| Error::UnknownAnnotation(self.annot.id.clone()))
    }
}

/// Every page's annotations, in page order.
pub(crate) fn every_annotation(doc: &laminapdf_model::Document) -> Result<Vec<(PageId, Vec<AnnotLink>)>> {
    let pages = doc.pages()?;
    let mut all = Vec::with_capacity(pages.len());
    for page in pages.iter() {
        all.push((page.id().clone(), doc.annotation_links(&page)?));
    }
    Ok(all)
}

/// The annotation, the group it belongs to, and everything that replies to or pops up for
/// any of them, followed to a fixed point (so cycles end).
fn doomed(target: &AnnotLink, all: &[(PageId, Vec<AnnotLink>)]) -> Vec<AnnotId> {
    let annots: Vec<&AnnotLink> = all.iter().flat_map(|(_, a)| a).collect();
    // Only annotations some page lists are deleted: an `/IRT` or `/Popup` that names any other
    // object (a page, a font, in a malformed file) is ignored, never freed.
    let listed = |id: &AnnotId| annots.iter().any(|a| a.id == *id);
    let mut doomed = vec![target.id.clone()];
    if target.reply_type == laminapdf_model::ReplyType::Group
        && let Some(primary) = target.in_reply_to.as_ref().filter(|p| listed(p))
        && *primary != target.id
    {
        doomed.push(primary.clone());
    }
    loop {
        let before = doomed.len();
        for annot in &annots {
            let follows = annot.in_reply_to.as_ref().is_some_and(|p| doomed.contains(p))
                || annot.parent.as_ref().is_some_and(|p| doomed.contains(p));
            if follows && !doomed.contains(&annot.id) {
                doomed.push(annot.id.clone());
            }
            if doomed.contains(&annot.id)
                && let Some(popup) = annot.popup.as_ref().filter(|p| listed(p))
                && !doomed.contains(popup)
            {
                doomed.push(popup.clone());
            }
        }
        if doomed.len() == before {
            return doomed;
        }
    }
}

/// Removes the entries at the positions `gone` from the page's `/Annots`, in the array object
/// when it is indirect, else in the page's own dictionary. Positions, not references, so a
/// direct dictionary goes too. An indirect array that several pages share is edited once:
/// `arrays` holds the ones already edited, since each page lists the same entries.
fn unlink(doc: &mut laminapdf_model::Document, page: &PageId, gone: &[usize], arrays: &mut Vec<ObjRef>) -> Result<()> {
    let page_ref = page
        .as_lamina_id()
        .obj_ref()
        .ok_or_else(|| Error::DirectPage(page.clone()))?;
    let mut page_dict = object_dict(doc, page_ref)?.ok_or_else(|| Error::UnknownPage(page.clone()))?;
    let Some(value) = page_dict.get("Annots").cloned() else {
        return Ok(());
    };
    let kept = |items: &[Object]| -> Vec<Object> {
        items
            .iter()
            .enumerate()
            .filter(|(i, _)| !gone.contains(i))
            .map(|(_, item)| item.clone())
            .collect()
    };
    match value {
        Object::Ref(array) => {
            if arrays.contains(&array) {
                return Ok(());
            }
            arrays.push(array);
            let current = laminapdf_cos::tolerant(doc.cos().resolve(array))?;
            let items = current.as_deref().and_then(Object::as_array).unwrap_or_default();
            let items = kept(items);
            doc.cos_mut().edit().set(array, Object::Array(items))?;
        }
        Object::Array(items) => {
            page_dict.insert("Annots", Object::Array(kept(&items)));
            doc.cos_mut().edit().set(page_ref, Object::Dict(page_dict))?;
        }
        _ => {}
    }
    Ok(())
}

/// Removes `key` from a surviving annotation's dictionary.
fn drop_entry(doc: &mut laminapdf_model::Document, annot: &AnnotLink, key: &str) -> Result<()> {
    let Some(r) = annot.id.as_lamina_id().obj_ref() else {
        return Ok(());
    };
    if let Some(mut dict) = object_dict(doc, r)?
        && dict.remove(key).is_some()
    {
        doc.cos_mut().edit().set(r, Object::Dict(dict))?;
    }
    Ok(())
}

/// The structure tree's parent tree, read once per op: the `/ParentTree` reference of the
/// catalog's `/StructTreeRoot`, and its entries.
type ParentTree = (ObjRef, Vec<(i64, Object)>);

fn parent_tree(doc: &laminapdf_model::Document) -> Result<Option<ParentTree>> {
    let catalog = doc.catalog()?;
    let Some(root) = catalog.get("StructTreeRoot").and_then(Object::as_ref) else {
        return Ok(None);
    };
    let Some(root) = object_dict(doc, root)? else {
        return Ok(None);
    };
    let Some(tree) = root.get("ParentTree").and_then(Object::as_ref) else {
        return Ok(None);
    };
    let entries = doc.number_tree(Object::Ref(tree)).entries()?;
    Ok(Some((tree, entries)))
}

/// Removes the structure tree's object reference to the annotation `r` (§14.7.5.3): the `OBJR`,
/// direct or indirect, among the `/K` of the element its `/StructParent` names in the parent
/// tree, and the element itself from its parent when nothing else is left in it. Returns the
/// `/StructParent` key, for the parent tree to drop.
fn untag(doc: &mut laminapdf_model::Document, tree: Option<&ParentTree>, r: ObjRef) -> Result<Option<i64>> {
    let Some((_, entries)) = tree else { return Ok(None) };
    let Some(annot) = object_dict(doc, r)? else {
        return Ok(None);
    };
    let Some(key) = annot.get("StructParent").and_then(Object::as_int) else {
        return Ok(None);
    };
    let Some(elem) = entries.iter().find(|(k, _)| *k == key).and_then(|(_, e)| e.as_ref()) else {
        return Ok(Some(key));
    };
    let Some(mut dict) = object_dict(doc, elem)? else {
        return Ok(Some(key));
    };
    let Some(kids) = dict.get("K").cloned() else {
        return Ok(Some(key));
    };
    let is_this = |kid: &Object| -> Result<bool> {
        let objr = match kid {
            Object::Dict(d) => Some(d.clone()),
            Object::Ref(k) => object_dict(doc, *k)?,
            _ => None,
        };
        Ok(objr.is_some_and(|d| d.get("Obj").and_then(Object::as_ref) == Some(r)))
    };
    let items = match kids {
        Object::Array(items) => items,
        kid => vec![kid],
    };
    let mut left = Vec::with_capacity(items.len());
    for kid in items {
        if !is_this(&kid)? {
            left.push(kid);
        }
    }
    if !left.is_empty() {
        dict.insert("K", Object::Array(left));
        doc.cos_mut().edit().set(elem, Object::Dict(dict))?;
        return Ok(Some(key));
    }
    dict.remove("K");
    doc.cos_mut().edit().set(elem, Object::Dict(dict.clone()))?;
    if let Some(parent) = dict.get("P").and_then(Object::as_ref)
        && let Some(mut parent_dict) = object_dict(doc, parent)?
    {
        let without = match parent_dict.get("K").cloned() {
            Some(Object::Array(items)) => Some(Object::Array(
                items.into_iter().filter(|k| k.as_ref() != Some(elem)).collect(),
            )),
            Some(Object::Ref(only)) if only == elem => Some(Object::Array(Vec::new())),
            _ => None,
        };
        if let Some(without) = without {
            parent_dict.insert("K", without);
            doc.cos_mut().edit().set(parent, Object::Dict(parent_dict))?;
        }
    }
    Ok(Some(key))
}

/// Removes the keys of deleted annotations from the parent tree's leaves, in place (§7.9.7):
/// each leaf's `/Nums` loses those pairs, and nothing else changes, so a `/Limits` stays true.
/// Nodes are followed through `/Kids` once each, up to `max_nesting_depth`.
fn drop_parent_tree_keys(
    doc: &mut laminapdf_model::Document,
    root: ObjRef,
    entries: &[(i64, Object)],
    keys: &[i64],
) -> Result<()> {
    if !entries.iter().any(|(k, _)| keys.contains(k)) {
        return Ok(());
    }
    let depth = doc.cos().limits().max_nesting_depth();
    let mut seen: Vec<ObjRef> = Vec::new();
    let mut next = vec![(root, 0_usize)];
    while let Some((node, level)) = next.pop() {
        if level > depth || seen.contains(&node) {
            continue;
        }
        seen.push(node);
        let Some(mut dict) = object_dict(doc, node)? else {
            continue;
        };
        if let Some(Object::Array(kids)) = dict.get("Kids") {
            next.extend(kids.iter().filter_map(Object::as_ref).map(|k| (k, level + 1)));
        }
        let Some(Object::Array(nums)) = dict.get("Nums").cloned() else {
            continue;
        };
        let (pairs, _) = nums.as_chunks::<2>();
        let kept: Vec<Object> = pairs
            .iter()
            .filter(|[k, _]| k.as_int().is_none_or(|k| !keys.contains(&k)))
            .flat_map(|pair| pair.iter().cloned())
            .collect();
        if kept.len() != nums.len() {
            dict.insert("Nums", Object::Array(kept));
            doc.cos_mut().edit().set(node, Object::Dict(dict))?;
        }
    }
    Ok(())
}

/// Shifts every coordinate pair of the geometry keys by `offset`. A value that isn't an array
/// of numbers is left as it is (a reference included: the op moves only what it can read here).
fn translate(dict: &mut Dict, offset: Offset) {
    for key in ["Rect", "QuadPoints", "Vertices", "L", "CL"] {
        if let Some(Object::Array(items)) = dict.get(key).cloned()
            && let Some(moved) = shift_pairs(&items, offset)
        {
            dict.insert(key, Object::Array(moved));
        }
    }
    if let Some(Object::Array(strokes)) = dict.get("InkList").cloned() {
        let moved: Option<Vec<Object>> = strokes
            .iter()
            .map(|stroke| match stroke {
                Object::Array(points) => shift_pairs(points, offset).map(Object::Array),
                _ => None,
            })
            .collect();
        if let Some(moved) = moved {
            dict.insert("InkList", Object::Array(moved));
        }
    }
}

/// `items` as x, y pairs moved by `offset`; `None` unless every item is a number and they pair up.
fn shift_pairs(items: &[Object], offset: Offset) -> Option<Vec<Object>> {
    if !items.len().is_multiple_of(2) {
        return None;
    }
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let value = item.as_f64()?;
            Some(Object::number(value + if i % 2 == 0 { offset.dx } else { offset.dy }))
        })
        .collect()
}

/// Sets or removes a text-string entry: `None` leaves it, `Some(None)` removes it.
pub(crate) fn set_text(dict: &mut Dict, key: &str, value: Option<&Option<String>>) {
    match value {
        Some(Some(text)) => {
            dict.insert(key, Object::String(PdfString::literal(encode_text_string(text))));
        }
        Some(None) => {
            dict.remove(key);
        }
        None => {}
    }
}

/// The dictionary of the object `r` as the overlay holds it now.
pub(crate) fn object_dict(doc: &laminapdf_model::Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(laminapdf_cos::tolerant(doc.cos().resolve(r))?
        .as_deref()
        .and_then(Object::as_dict)
        .cloned())
}
