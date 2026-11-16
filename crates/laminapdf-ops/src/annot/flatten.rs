//! Flattening (ISO 32000-2 §12.5.5; E5 plan §2.1): [`FlattenAnnots`] draws each chosen
//! annotation's normal appearance into its page through the content append API, then removes
//! the annotation with its Popup and its replies, as [`DeleteAnnot`] does.
//!
//! The page's content is wrapped once in `q … Q`, so the state it leaves behind can't change the
//! drawing; each appearance is registered as a Form XObject and drawn as `q <A> cm /FmN Do Q`,
//! `A` being the matrix that maps the appearance's box onto `/Rect`. Annotations are drawn in
//! the order their pages list them, which is the order viewers paint them.

use std::fmt::Write as _;

use laminapdf_cos::{Dict, ObjRef, Object, Stream};
use laminapdf_model::{AnnotFlags, AnnotId, AnnotLink, AnnotSubtype, Matrix, PageId};
use laminapdf_syntax::format_real;
use serde::{Deserialize, Serialize};

use super::redraw::Redraw;
use super::{Appearances, DeleteAnnot, Found, doomed, every_annotation, object_dict};
use crate::blob::BlobStore;
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::wire;

/// Which appearance of the page flattening keeps: what a viewer shows on screen, or what it
/// prints (§12.5.3, Table 167).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum FlattenIntent {
    /// On screen: annotations flagged `Hidden` or `NoView` are left as they are.
    View,
    /// Printed: annotations flagged `Hidden`, or without the `Print` flag, are left as they are.
    Print,
}

/// Draws annotations into their pages' content and removes them (ISO 32000-2 §12.5.5).
///
/// For each annotation, in the order given, the normal appearance (the entry `/AS` names, for a
/// dictionary of states) is drawn where the annotation shows it, then the annotation is removed
/// with its Popup, its replies and the rest of its group, as [`DeleteAnnot`] removes them. A
/// reply or group member that has an appearance and is shown for the intent is drawn too, so the
/// page looks as it did. An ID that an earlier one's removal already took is skipped.
///
/// What is left in place:
///
/// - an annotation hidden for `intent` (`Hidden`, `NoView` on screen, no `Print` flag when
///   printed), or in optional content that is off by default;
/// - one without an appearance that Lamina PDF can't draw one for: notes, carets, text markup,
///   ink, shapes, lines, polygons and text boxes get one first, through the document's
///   [`Appearances`]; any other is listed in
///   [`Applied::not_flattened`](crate::Applied::not_flattened) (`flatten-no-appearance`).
///
/// A widget is refused (`flatten-widget`): form fields have their own flattening, and a
/// signature field is never flattened. A `Locked` annotation is refused (`annotation-locked`),
/// since flattening removes it.
///
/// In a tagged document the drawing is marked as an artifact and each removed annotation's
/// `Annot` element leaves the structure tree; on a file that claims PDF/UA-1 the save planner
/// then reports `flatten-untagged`. The op is class `Content`: a certification refuses it at
/// every level. The annotations stay in the file's earlier revisions until a full rewrite
/// ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)).
///
/// On the wire: `{ "op": "flattenAnnots", "args": { "annots": ["a12.0", "a14.0"], "intent":
/// "print" } }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct FlattenAnnots {
    /// The annotations, in order.
    #[serde(with = "wire::annot_ids")]
    #[cfg_attr(feature = "tsify", tsify(type = "string[]"))]
    pub annots: Vec<AnnotId>,
    /// Which appearance of the page to keep.
    pub intent: FlattenIntent,
}

impl FlattenAnnots {
    /// The op that flattens `annots` for `intent`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(annots: Vec<AnnotId>, intent: FlattenIntent) -> Self {
        Self { annots, intent }
    }

    pub(crate) fn apply_to(
        &self,
        doc: &mut laminapdf_model::Document,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<Effect> {
        let all = every_annotation(doc)?;
        let mut taken: Vec<AnnotId> = Vec::new();
        let mut chosen: Vec<AnnotId> = Vec::new();
        let mut not_flattened = Vec::new();
        let mut draws: Vec<Draw> = Vec::new();
        for id in &self.annots {
            if taken.contains(id) {
                continue;
            }
            let link = all
                .iter()
                .flat_map(|(_, annots)| annots)
                .find(|a| a.id == *id)
                .ok_or_else(|| Error::UnknownAnnotation(id.clone()))?;
            if link.subtype == AnnotSubtype::Widget {
                return Err(Error::FlattenWidget(id.clone()));
            }
            let found = Found::locate(&all, id)?;
            let mut dict = found.dict(doc)?;
            if !shown(doc, &dict, link.flags, self.intent)? {
                continue;
            }
            if doc.normal_appearance(&dict)?.is_none() && !draw_appearance(doc, &mut dict, appearances, blobs)? {
                not_flattened.push(id.clone());
                continue;
            }
            let removed = doomed(&found.annot, &all);
            for member in &removed {
                let Some((page, order, member_link)) = place(&all, member) else {
                    continue;
                };
                let draw = if member == id {
                    prepare(doc, &dict)?
                } else {
                    match member_dict(doc, member_link)? {
                        Some(d) if shown(doc, &d, member_link.flags, self.intent)? => prepare(doc, &d)?,
                        _ => None,
                    }
                };
                if let Some(prepared) = draw {
                    draws.push(Draw {
                        page: page.clone(),
                        order,
                        prepared,
                    });
                }
            }
            taken.extend(removed);
            chosen.push(id.clone());
        }
        draws.sort_by_key(|d| d.order);
        let tagged = doc.is_tagged()? || doc.catalog()?.get("StructTreeRoot").is_some();
        let mut pages: Vec<PageId> = Vec::new();
        for draw in &draws {
            if !pages.contains(&draw.page) {
                pages.push(draw.page.clone());
            }
        }
        for page in &pages {
            let on_page: Vec<&Draw> = draws.iter().filter(|d| d.page == *page).collect();
            write_page(doc, page, &on_page, tagged)?;
        }
        let mut also_deleted = Vec::new();
        for id in &chosen {
            let effect = DeleteAnnot::new(id.clone()).apply_to(doc)?;
            for page in effect.pages {
                if !pages.contains(&page) {
                    pages.push(page);
                }
            }
            also_deleted.extend(effect.also_deleted);
        }
        Ok(Effect {
            pages,
            removed: !chosen.is_empty(),
            also_deleted,
            flattened: chosen,
            not_flattened,
            ..Effect::default()
        })
    }
}

/// One appearance to draw, on its page.
struct Draw {
    page: PageId,
    /// The page's place in the document and the annotation's in `/Annots`: the painting order.
    order: (usize, usize),
    prepared: Prepared,
}

/// How to draw one appearance.
struct Prepared {
    form: ObjRef,
    matrix: Matrix,
    /// `/CA` below 1 (§12.5.2), which applies to the whole appearance.
    opacity: Option<f64>,
    /// `/BM` (PDF 2.0, §12.5.2), when not `Normal`.
    blend: Option<Object>,
}

/// The annotation's page, its painting order and its link.
fn place<'a>(all: &'a [(PageId, Vec<AnnotLink>)], id: &AnnotId) -> Option<(&'a PageId, (usize, usize), &'a AnnotLink)> {
    all.iter()
        .enumerate()
        .find_map(|(p, (page, annots))| annots.iter().find(|a| a.id == *id).map(|a| (page, (p, a.index), a)))
}

fn member_dict(doc: &laminapdf_model::Document, link: &AnnotLink) -> Result<Option<Dict>> {
    if link.subtype == AnnotSubtype::Popup {
        return Ok(None);
    }
    match link.id.as_lamina_id().obj_ref() {
        Some(r) => object_dict(doc, r),
        None => Ok(None),
    }
}

/// Whether the annotation shows for `intent` (Table 167) and its optional content is on.
fn shown(doc: &laminapdf_model::Document, dict: &Dict, flags: AnnotFlags, intent: FlattenIntent) -> Result<bool> {
    let hidden = flags.has(AnnotFlags::HIDDEN)
        || match intent {
            FlattenIntent::View => flags.has(AnnotFlags::NO_VIEW),
            FlattenIntent::Print => !flags.has(AnnotFlags::PRINT),
        };
    if hidden {
        return Ok(false);
    }
    if let Some(oc) = dict.get("OC")
        && super::flatten_oc::is_off(doc, oc)?
    {
        return Ok(false);
    }
    Ok(true)
}

/// Gives an annotation without an appearance the one Lamina PDF draws for its kind, in `dict`;
/// `false` when it can't draw one (another kind, no generator, a text box it can't lay out).
fn draw_appearance(
    doc: &mut laminapdf_model::Document,
    dict: &mut Dict,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
) -> Result<bool> {
    let redraw = Redraw {
        color: None,
        opacity: None,
        spec: None,
    };
    match redraw.apply(doc, dict, appearances, blobs) {
        Ok(_) => Ok(true),
        Err(
            Error::InvalidOp(_)
            | Error::AppearanceUnavailable
            | Error::AppearanceNeedsText
            | Error::AnnotationGlyphMissing
            | Error::AnnotationFontInvalid
            | Error::UnknownBlob(_),
        ) => Ok(false),
        Err(e) => Err(e),
    }
}

/// How to draw `dict`'s normal appearance; `None` when it has none, its optional content is off,
/// or its box maps onto nothing.
fn prepare(doc: &mut laminapdf_model::Document, dict: &Dict) -> Result<Option<Prepared>> {
    let Some(appearance) = doc.normal_appearance(dict)? else {
        return Ok(None);
    };
    if let Some(oc) = appearance.stream.dict.get("OC")
        && super::flatten_oc::is_off(doc, oc)?
    {
        return Ok(None);
    }
    // `Do` applies the form's own `/Matrix`: the page sets only the fit.
    let Some(matrix) = doc.appearance_placement(dict, &appearance.stream)?.map(|p| p.fit) else {
        return Ok(None);
    };
    let form = match appearance.object {
        Some(r) if appearance.stream.dict.get_name("Subtype") == Some(b"Form".as_slice()) => r,
        // A stream written in place, or one without `/Subtype /Form`: drawn from a copy that is
        // a Form XObject, so `Do` draws it in every viewer.
        _ => {
            let mut form: Stream = appearance.stream;
            form.dict.insert("Type", Object::name("XObject"));
            form.dict.insert("Subtype", Object::name("Form"));
            doc.cos_mut().edit().create(Object::Stream(form))?
        }
    };
    let opacity = match dict.get("CA") {
        Some(ca) => doc.deref(ca)?.as_f64().filter(|ca| *ca < 1.0).map(|ca| ca.max(0.0)),
        None => None,
    };
    let blend = dict
        .get("BM")
        .filter(|bm| !matches!(bm, Object::Name(n) if n.as_bytes() == b"Normal"))
        .cloned();
    Ok(Some(Prepared {
        form,
        matrix,
        opacity,
        blend,
    }))
}

/// Appends the page's drawings after its content, wrapped once.
fn write_page(doc: &mut laminapdf_model::Document, page: &PageId, draws: &[&Draw], tagged: bool) -> Result<()> {
    let cos_page = {
        let pages = doc.pages()?;
        let found = pages.by_id(page).ok_or_else(|| Error::UnknownPage(page.clone()))?;
        found.cos().clone()
    };
    let mut writer = laminapdf_content::PageContentWriter::new(doc.cos_mut().edit(), &cos_page)?;
    writer.wrap_existing()?;
    let mut content = String::new();
    for draw in draws {
        let draw = &draw.prepared;
        let form = writer.add_xobject(draw.form)?;
        if tagged {
            content.push_str("/Artifact BMC\n");
        }
        content.push_str("q\n");
        if draw.opacity.is_some() || draw.blend.is_some() {
            let mut gs = Dict::new();
            if let Some(opacity) = draw.opacity {
                gs.insert("CA", Object::Real(opacity));
                gs.insert("ca", Object::Real(opacity));
            }
            if let Some(blend) = &draw.blend {
                gs.insert("BM", blend.clone());
            }
            let name = writer.add_extgstate(Object::Dict(gs))?;
            let _ = writeln!(content, "/{} gs", String::from_utf8_lossy(name.as_bytes()));
        }
        let matrix = draw.matrix.to_array().map(format_real).join(" ");
        let form = String::from_utf8_lossy(form.as_bytes());
        let _ = write!(content, "{matrix} cm\n/{form} Do\nQ\n");
        if tagged {
            content.push_str("EMC\n");
        }
    }
    writer.append(content.as_bytes())?;
    Ok(())
}
