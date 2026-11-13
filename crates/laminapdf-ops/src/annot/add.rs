//! [`AddAnnot`]: a new annotation with its appearance, its Popup for a note, its group for a text
//! correction, and its place in the structure tree in a tagged file.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{AnnotFlags, AnnotId, PageFrame, PageId, PdfDate, PointPt, UserSpaceRect};
use serde::{Deserialize, Serialize};

use super::appearance::{AppearanceRequest, Appearances, IntentSpace};
use super::geom::quad_points;
use super::geom::{AnnotColor, Opacity};
use super::spec::{AnnotSpec, CaretSymbol, Note};
use super::{Found, every_annotation, set_text, tags};
use crate::error::{Error, Result};
use crate::link::append_annot;
use crate::op::Effect;
use crate::outline::target;
use crate::pages::edit_dict;
use crate::wire;

/// The entries every annotation has, whatever its kind (ISO 32000-2 Tables 166 and 170). Every
/// value is in the payload, never from a clock or a setting, so the op gives the same file
/// wherever it is applied.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AnnotCommon {
    /// `/Contents`: the comment, or for a mark the text a screen reader reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub contents: Option<String>,
    /// `/T`: the author.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub author: Option<String>,
    /// `/Subj`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub subject: Option<String>,
    /// `/CreationDate`.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub created: Option<PdfDate>,
    /// `/M`, the modification date.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_date")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub modified: Option<PdfDate>,
    /// `/C`; the mark's usual colour when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub color: Option<AnnotColor>,
    /// The opacity, opaque when absent.
    #[serde(default, skip_serializing_if = "is_opaque")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub opacity: Opacity,
    /// `/F`, all the flags at once (Table 167); Print when absent, and for a note NoZoom and
    /// NoRotate too, as Acrobat writes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub flags: Option<u32>,
    /// An annotation on the same page to group with (`/IRT` with `/RT /Group`, §12.5.6.2): the
    /// group shows, moves and is deleted as one, as a text correction's strike-out and caret
    /// are.
    #[serde(default, skip_serializing_if = "Option::is_none", with = "wire::opt_annot_id")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub group_with: Option<AnnotId>,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde's skip_serializing_if passes a reference"
)]
fn is_opaque(o: &Opacity) -> bool {
    *o == Opacity::OPAQUE
}

/// Adds an annotation to `page`: a text markup, a note or a caret ([`AnnotSpec`]), with an
/// appearance stream drawn by the document's [`Appearances`] and the common entries of
/// [`AnnotCommon`]. A note also gets its Popup, listed on the page after it.
///
/// The new annotation's ID is in [`Applied::created_annotations`](crate::Applied::created_annotations).
/// `nm` is its `/NM`, a name for matching it in exchanged comments; its identity is its ID.
///
/// In a tagged document the annotation becomes an `Annot` element holding an object reference
/// to it, placed after the element that owns the first of `marked` (the MCIDs of the content
/// under it), else last in the document element; the page gets `/Tabs /S`.
///
/// A spec with no quads or a coordinate that isn't finite is `invalid-op`; a `group_with` that
/// names no annotation on the page is `unknown-annotation`; a document without a generator is
/// `appearance-unavailable`.
///
/// On the wire: `{ "op": "addAnnot", "args": { "page": "p3.0", "spec": { "type": "textMarkup",
/// "kind": "highlight", "quads": [[{ "x": 72, "y": 714 }, { "x": 180, "y": 714 }, { "x": 72,
/// "y": 700 }, { "x": 180, "y": 700 }]] } } }`.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AddAnnot {
    /// The page.
    #[serde(with = "wire::page_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub page: PageId,
    /// The mark and its geometry.
    pub spec: AnnotSpec,
    /// The common entries.
    #[serde(default, skip_serializing_if = "is_default")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub common: AnnotCommon,
    /// Its `/NM`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub nm: Option<String>,
    /// The MCIDs of the page's content under it, in reading order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub marked: Vec<u32>,
}

fn is_default(common: &AnnotCommon) -> bool {
    *common == AnnotCommon::default()
}

impl AddAnnot {
    /// The op that adds `spec` to `page`, with no common entries, `/NM` or marked content.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn new(page: PageId, spec: AnnotSpec) -> Self {
        Self {
            page,
            spec,
            common: AnnotCommon::default(),
            nm: None,
            marked: Vec::new(),
        }
    }

    /// The same op with `common`.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn with_common(mut self, common: AnnotCommon) -> Self {
        self.common = common;
        self
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
    ) -> Result<Effect> {
        if !self.spec.is_valid() {
            return Err(Error::InvalidOp(
                "an annotation needs quads, and every coordinate finite",
            ));
        }
        let appearances = appearances.ok_or(Error::AppearanceUnavailable)?;
        let page = target::page_ref(doc, &self.page)?;
        let group = match &self.common.group_with {
            Some(id) => Some(group_target(doc, &self.page, id)?),
            None => None,
        };
        let spec = self.spec.normalised();
        let request = AppearanceRequest {
            spec: &spec,
            color: self.common.color,
            opacity: self.common.opacity,
            intent: intent_space(doc)?,
        };
        let appearance = appearances.normal(&mut doc.cos_mut().edit(), &request)?;
        let mut dict = common_dict(spec.subtype(), page, &appearance.rect, appearance.normal);
        self.write_common(&mut dict, doc.is_pdf2()?);
        write_spec(&mut dict, &spec);
        if let Some(group) = group {
            dict.insert("IRT", Object::Ref(group));
            dict.insert("RT", Object::name("Group"));
        }
        let annot = doc.cos_mut().edit().create(Object::Dict(dict))?;
        append_annot(doc, page, annot)?;
        if let AnnotSpec::Note(note) = &spec {
            let frame = page_frame(doc, &self.page)?;
            let popup = popup(doc, note, &popup_rect(&frame, note), page, annot)?;
            edit_dict(doc, annot, |d| {
                d.insert("Popup", Object::Ref(popup));
            })?;
            append_annot(doc, page, popup)?;
        }
        tags::tag(doc, page, annot, &self.marked)?;
        Ok(Effect {
            pages: vec![self.page.clone()],
            created_annotations: vec![AnnotId::object(annot)],
            ..Effect::default()
        })
    }

    /// The entries of [`AnnotCommon`] and `/NM`. `/CA` repeats the appearance's opacity for
    /// readers that draw from the keys, except in PDF 2.0, which tells readers to ignore it when
    /// they draw `/AP` (§12.5.2): the appearance alone carries it there.
    fn write_common(&self, dict: &mut Dict, pdf2: bool) {
        let c = &self.common;
        set_text(dict, "Contents", Some(&c.contents));
        set_text(dict, "T", Some(&c.author));
        set_text(dict, "Subj", Some(&c.subject));
        set_text(dict, "NM", Some(&self.nm));
        for (key, date) in [("CreationDate", &c.created), ("M", &c.modified)] {
            if let Some(date) = date {
                dict.insert(
                    key,
                    Object::String(PdfString::literal(date.to_pdf_string().into_bytes())),
                );
            }
        }
        if let Some(color) = c.color {
            dict.insert("C", color.to_object());
        }
        if c.opacity != Opacity::OPAQUE && !pdf2 {
            dict.insert("CA", Object::Real(c.opacity.get()));
        }
        let flags = c.flags.unwrap_or(match self.spec {
            AnnotSpec::Note(_) => AnnotFlags::PRINT | AnnotFlags::NO_ZOOM | AnnotFlags::NO_ROTATE,
            _ => AnnotFlags::PRINT,
        });
        dict.insert("F", Object::Integer(i64::from(flags)));
    }
}

/// The entries every new annotation starts with.
fn common_dict(subtype: &str, page: ObjRef, rect: &UserSpaceRect, normal: ObjRef) -> Dict {
    [
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name(subtype)),
        ("Rect", rect_object(rect)),
        ("P", Object::Ref(page)),
        ("AP", Object::Dict([("N", Object::Ref(normal))].into_iter().collect())),
    ]
    .into_iter()
    .collect()
}

/// `/Rect`'s array.
pub(crate) fn rect_object(r: &UserSpaceRect) -> Object {
    Object::Array(r.to_array().into_iter().map(Object::Real).collect())
}

/// The entries only this kind of mark has.
pub(crate) fn write_spec(dict: &mut Dict, spec: &AnnotSpec) {
    match spec {
        AnnotSpec::TextMarkup(m) => {
            dict.insert("QuadPoints", quad_points(&m.quads));
        }
        AnnotSpec::Note(n) => {
            dict.insert("Name", Object::name(n.icon.name()));
            dict.insert("Open", Object::Bool(n.open));
        }
        AnnotSpec::Caret(c) => {
            let symbol = match c.symbol {
                CaretSymbol::None => "None",
                CaretSymbol::Paragraph => "P",
            };
            dict.insert("Sy", Object::name(symbol));
        }
    }
}

/// A note's Popup over `rect`, open as the note says, printed with it.
fn popup(
    doc: &mut laminapdf_model::Document,
    note: &Note,
    rect: &UserSpaceRect,
    page: ObjRef,
    parent: ObjRef,
) -> Result<ObjRef> {
    let dict: Dict = [
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Popup")),
        ("Rect", rect_object(rect)),
        ("P", Object::Ref(page)),
        ("Parent", Object::Ref(parent)),
        ("Open", Object::Bool(note.open)),
        (
            "F",
            Object::Integer(i64::from(
                AnnotFlags::PRINT | AnnotFlags::NO_ZOOM | AnnotFlags::NO_ROTATE,
            )),
        ),
    ]
    .into_iter()
    .collect();
    Ok(doc.cos_mut().edit().create(Object::Dict(dict))?)
}

/// Where a note's Popup goes, as the page is shown: right of the icon, its top level with the
/// icon's, or left of it when the page has no room on the right, and never past the page's top
/// or bottom. The icon keeps its upper-left corner at `at` however the page turns (NoRotate,
/// ISO 32000-2 §12.5.3), so the Popup is placed from there in page space.
fn popup_rect(frame: &PageFrame, note: &Note) -> UserSpaceRect {
    let at = frame.to_page(note.at);
    let (w, h) = Note::POPUP;
    let (page_w, page_h) = (frame.width().get(), frame.height().get());
    let right = at.x.get() + Note::ICON;
    let x = if right + w <= page_w {
        right
    } else {
        (at.x.get() - w).max(0.0)
    };
    let y = at.y.get().clamp(0.0, (page_h - h).max(0.0));
    let a = frame.to_user_space(PointPt::from_points(x, y));
    let b = frame.to_user_space(PointPt::from_points(x + w, y + h));
    UserSpaceRect::from_corners(a.x, a.y, b.x, b.y)
}

/// The frame of the page `id`.
fn page_frame(doc: &laminapdf_model::Document, id: &PageId) -> Result<PageFrame> {
    let pages = doc.pages()?;
    let page = pages
        .iter()
        .find(|p| p.id() == id)
        .ok_or_else(|| Error::UnknownPage(id.clone()))?;
    Ok(page.frame()?)
}

/// The annotation a new one groups with: an editable annotation on the same page.
fn group_target(doc: &laminapdf_model::Document, page: &PageId, id: &AnnotId) -> Result<ObjRef> {
    let all = every_annotation(doc)?;
    let found = Found::locate(&all, id)?;
    if found.page != *page {
        return Err(Error::InvalidOp("a group's annotations are on one page"));
    }
    Ok(found.object)
}

/// The device space of the output intent of a file that claims PDF/A (ISO 19005-2 §6.2.3): the
/// first `GTS_PDFA1` intent's profile's `/N`. `None` for any other file, or a profile that is
/// neither RGB nor CMYK, which leaves colours as given.
pub(crate) fn intent_space(doc: &laminapdf_model::Document) -> Result<Option<IntentSpace>> {
    if crate::metadata::read_metadata(doc)?.pdfa.is_none() {
        return Ok(None);
    }
    let catalog = doc.catalog()?;
    let Some(intents) = catalog.get("OutputIntents") else {
        return Ok(None);
    };
    let intents = doc
        .deref(intents)?
        .as_array()
        .map(<[Object]>::to_vec)
        .unwrap_or_default();
    for intent in intents {
        let intent = doc.deref(&intent)?;
        let Some(intent) = intent.as_dict() else { continue };
        if intent.get_name("S") != Some(b"GTS_PDFA1".as_slice()) {
            continue;
        }
        let Some(profile) = intent.get("DestOutputProfile").and_then(Object::as_ref) else {
            return Ok(None);
        };
        let components =
            laminapdf_cos::tolerant(doc.cos().resolve(profile))?.and_then(|o| o.as_dict().and_then(|d| d.get_int("N")));
        return Ok(match components {
            Some(3) => Some(IntentSpace::Rgb),
            Some(4) => Some(IntentSpace::Cmyk),
            _ => None,
        });
    }
    Ok(None)
}
