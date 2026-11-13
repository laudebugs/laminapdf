//! The annotation ops' shared core (ISO 32000-2 §12.5): [`UpdateAnnot`] changes the common and
//! markup entries of any annotation and moves it, and [`DeleteAnnot`] removes it with its Popup,
//! its replies and its group. Both are class `Annotation` and never need a full rewrite.
//!
//! Every write goes through the overlay: an annotation is changed in place (it keeps its object
//! and so its [`AnnotId`]), and a deleted one is unlinked from its page and freed. Keys the ops
//! don't own are kept untouched.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{Annot, AnnotFlags, AnnotId, AnnotSubtype, PageId, PdfDate};
use laminapdf_syntax::text::encode_text_string;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// A move in user space (ISO 32000-2 §8.3.2.3: y up), in user-space units.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct Offset {
    /// Along x.
    pub dx: f64,
    /// Along y (up is positive).
    pub dy: f64,
}

impl Offset {
    /// The move by `(dx, dy)`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }
}

impl Eq for Offset {}

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
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let found = Found::locate(doc, &self.id)?;
        let flags = found.annot.flags;
        let properties = self.author.is_some() || self.subject.is_some() || self.translate.is_some();
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
        if let Some(offset) = self.translate {
            translate(&mut dict, offset);
            if let Some(popup) = found.annot.popup.as_ref().and_then(|p| p.as_lamina_id().obj_ref()) {
                move_popup(doc, popup, offset)?;
            }
        }
        if dict == before {
            return Ok(Effect::default());
        }
        if let Some(at) = &self.modified {
            dict.insert("M", Object::String(PdfString::literal(at.to_pdf_string().into_bytes())));
        }
        doc.cos_mut().edit().set(found.object, Object::Dict(dict))?;
        Ok(Effect {
            pages: vec![found.page],
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
        let found = Found::locate(doc, &self.id)?;
        if found.annot.flags.has(AnnotFlags::LOCKED) {
            return Err(Error::AnnotationLocked(self.id.clone()));
        }
        let all = every_annotation(doc)?;
        let doomed = doomed(&found.annot, &all);
        let mut pages = Vec::new();
        for (page, annots) in &all {
            if annots.iter().any(|a| doomed.contains(&a.id)) {
                unlink(doc, page, &doomed)?;
                pages.push(page.clone());
            }
            for annot in annots.iter().filter(|a| !doomed.contains(&a.id)) {
                if annot.popup.as_ref().is_some_and(|p| doomed.contains(p)) {
                    drop_entry(doc, annot, "Popup")?;
                }
            }
        }
        for id in &doomed {
            if let Some(r) = id.as_lamina_id().obj_ref() {
                untag(doc, r)?;
                doc.cos_mut().edit().delete(r)?;
            }
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
struct Found {
    page: PageId,
    object: ObjRef,
    annot: Annot,
}

impl Found {
    fn locate(doc: &laminapdf_model::Document, id: &AnnotId) -> Result<Self> {
        for (page, annots) in every_annotation(doc)? {
            if let Some(annot) = annots.into_iter().find(|a| a.id == *id) {
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

    fn dict(&self, doc: &laminapdf_model::Document) -> Result<Dict> {
        object_dict(doc, self.object)?.ok_or_else(|| Error::UnknownAnnotation(self.annot.id.clone()))
    }
}

/// Every page's annotations, in page order.
fn every_annotation(doc: &laminapdf_model::Document) -> Result<Vec<(PageId, Vec<Annot>)>> {
    let pages = doc.pages()?;
    let mut all = Vec::with_capacity(pages.len());
    for page in pages.iter() {
        all.push((page.id().clone(), doc.annotations(&page)?));
    }
    Ok(all)
}

/// The annotation, the group it belongs to, and everything that replies to or pops up for
/// any of them, followed to a fixed point (so cycles end).
fn doomed(target: &Annot, all: &[(PageId, Vec<Annot>)]) -> BTreeSet<AnnotId> {
    let annots: Vec<&Annot> = all.iter().flat_map(|(_, a)| a).collect();
    let mut doomed = BTreeSet::from([target.id.clone()]);
    if target.reply_type == laminapdf_model::ReplyType::Group
        && let Some(primary) = &target.in_reply_to
    {
        doomed.insert(primary.clone());
    }
    loop {
        let before = doomed.len();
        for annot in &annots {
            let follows = annot.in_reply_to.as_ref().is_some_and(|p| doomed.contains(p))
                || annot.parent.as_ref().is_some_and(|p| doomed.contains(p));
            if follows {
                doomed.insert(annot.id.clone());
            }
            if doomed.contains(&annot.id)
                && let Some(popup) = &annot.popup
            {
                doomed.insert(popup.clone());
            }
        }
        if doomed.len() == before {
            return doomed;
        }
    }
}

/// Removes the doomed annotations from the page's `/Annots`, in the array object when it is
/// indirect, else in the page's own dictionary.
fn unlink(doc: &mut laminapdf_model::Document, page: &PageId, doomed: &BTreeSet<AnnotId>) -> Result<()> {
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
            .filter(|item| item.as_ref().is_none_or(|r| !doomed.contains(&AnnotId::object(r))))
            .cloned()
            .collect()
    };
    match value {
        Object::Ref(array) => {
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
fn drop_entry(doc: &mut laminapdf_model::Document, annot: &Annot, key: &str) -> Result<()> {
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

/// Removes the structure tree's object reference to the annotation `r` (§14.7.5.3): the `OBJR`
/// among the `/K` of the element its `/StructParent` names in the parent tree, and the element
/// itself from its parent when nothing else is left in it.
fn untag(doc: &mut laminapdf_model::Document, r: ObjRef) -> Result<()> {
    let Some(annot) = object_dict(doc, r)? else {
        return Ok(());
    };
    let Some(key) = annot.get("StructParent").and_then(Object::as_int) else {
        return Ok(());
    };
    let Some(tree) = doc.structure()? else { return Ok(()) };
    let Some(root) = tree.parent_tree else { return Ok(()) };
    let Some(elem) = doc.number_tree(root).get(key)?.and_then(|e| e.as_ref()) else {
        return Ok(());
    };
    let Some(mut dict) = object_dict(doc, elem)? else {
        return Ok(());
    };
    let Some(kids) = dict.get("K").cloned() else {
        return Ok(());
    };
    let is_this = |kid: &Object| match kid {
        Object::Dict(d) => d.get("Obj").and_then(Object::as_ref) == Some(r),
        _ => false,
    };
    let left: Vec<Object> = match kids {
        Object::Array(items) => items.into_iter().filter(|k| !is_this(k)).collect(),
        kid if is_this(&kid) => Vec::new(),
        kid => vec![kid],
    };
    if !left.is_empty() {
        dict.insert("K", Object::Array(left));
        return doc.cos_mut().edit().set(elem, Object::Dict(dict)).map_err(Error::from);
    }
    dict.remove("K");
    doc.cos_mut().edit().set(elem, Object::Dict(dict.clone()))?;
    let Some(parent) = dict.get("P").and_then(Object::as_ref) else {
        return Ok(());
    };
    let Some(mut parent_dict) = object_dict(doc, parent)? else {
        return Ok(());
    };
    let without = match parent_dict.get("K").cloned() {
        Some(Object::Array(items)) => Object::Array(items.into_iter().filter(|k| k.as_ref() != Some(elem)).collect()),
        Some(Object::Ref(only)) if only == elem => Object::Array(Vec::new()),
        _ => return Ok(()),
    };
    parent_dict.insert("K", without);
    doc.cos_mut()
        .edit()
        .set(parent, Object::Dict(parent_dict))
        .map_err(Error::from)
}

/// Moves a Popup's `/Rect` with its parent.
fn move_popup(doc: &mut laminapdf_model::Document, popup: ObjRef, offset: Offset) -> Result<()> {
    let Some(mut dict) = object_dict(doc, popup)? else {
        return Ok(());
    };
    translate(&mut dict, offset);
    doc.cos_mut().edit().set(popup, Object::Dict(dict)).map_err(Error::from)
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
            Some(Object::Real(value + if i % 2 == 0 { offset.dx } else { offset.dy }))
        })
        .collect()
}

/// Sets or removes a text-string entry: `None` leaves it, `Some(None)` removes it.
fn set_text(dict: &mut Dict, key: &str, value: Option<&Option<String>>) {
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
fn object_dict(doc: &laminapdf_model::Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(laminapdf_cos::tolerant(doc.cos().resolve(r))?
        .as_deref()
        .and_then(Object::as_dict)
        .cloned())
}
