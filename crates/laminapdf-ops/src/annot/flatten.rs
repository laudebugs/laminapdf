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
use super::{Appearances, DeleteAnnot, doomed, every_annotation, object_dict};
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
/// - one that has no appearance and is of a kind Lamina PDF can't draw, such as a file
///   attachment. Notes, carets, text markup, ink, shapes, lines, polygons and text boxes get an
///   appearance first, through the document's [`Appearances`]; the others are listed in
///   [`Applied::not_flattened`](crate::Applied::not_flattened) (`flatten-no-appearance`).
///
/// An annotation in optional content that is on keeps its membership: its drawing is marked
/// content `/OC /OcN BDC … EMC`, so it still follows its layer. `/CA`, `/ca` and `/BM` apply
/// through an ExtGState, and a `NoRotate` annotation on a turned page is drawn turned back about
/// its upper-left corner, as a viewer shows it (§12.5.3).
///
/// A widget is refused (`flatten-widget`): flattening form fields needs an edit that knows about
/// their values, which Lamina PDF doesn't have yet, and a signature field is never flattened. A
/// `Locked` annotation, or one whose Popup, reply or group member is `Locked`, is refused
/// (`annotation-locked`) before anything is drawn, since flattening removes it.
///
/// In a tagged document the drawing is marked as an artifact and each removed annotation's
/// `Annot` element leaves the structure tree; on a file that claims PDF/UA-1 the save planner
/// then reports `flatten-untagged`, and on one that claims PDF/A, `pdfa-font-not-embedded` for an
/// appearance whose fonts aren't embedded. The op is class `Content`: a certification refuses it at
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
        let claims_pdfa = crate::metadata::read_metadata(doc)?.pdfa.is_some();
        // What the removals so far take: an ID among them is flattened already.
        let mut removed: Vec<AnnotId> = Vec::new();
        let mut flattened: Vec<AnnotId> = Vec::new();
        let mut not_flattened = Vec::new();
        let mut unembedded = Vec::new();
        let mut draws: Vec<Draw> = Vec::new();
        for id in &self.annots {
            if removed.contains(id) {
                continue;
            }
            let target = Placed::find(&all, id).ok_or_else(|| Error::UnknownAnnotation(id.clone()))?;
            if target.link.subtype == AnnotSubtype::Widget {
                return Err(Error::FlattenWidget(id.clone()));
            }
            let object = id
                .as_lamina_id()
                .obj_ref()
                .ok_or_else(|| Error::AnnotationNotEditable(id.clone()))?;
            let mut dict = object_dict(doc, object)?.ok_or_else(|| Error::UnknownAnnotation(id.clone()))?;
            if !shown(doc, &dict, target.link.flags, self.intent)? {
                continue;
            }
            // The annotation, its Popup, its replies and its group: what flattening removes.
            let cascade = doomed(target.link, &all);
            // Removing refuses a locked annotation, so refuse it before anything is drawn.
            if let Some(locked) = cascade
                .iter()
                .filter_map(|m| Placed::find(&all, m))
                .find(|m| m.link.flags.has(AnnotFlags::LOCKED))
            {
                return Err(Error::AnnotationLocked(locked.link.id.clone()));
            }
            if doc.normal_appearance(&dict)?.is_none() && !draw_appearance(doc, &mut dict, appearances, blobs)? {
                not_flattened.push(id.clone());
                continue;
            }
            for member in cascade.iter().filter_map(|m| Placed::find(&all, m)) {
                let member_dict = if member.link.id == *id {
                    Some(dict.clone())
                } else {
                    match member_dict(doc, member.link)? {
                        Some(d) if shown(doc, &d, member.link.flags, self.intent)? => Some(d),
                        _ => None,
                    }
                };
                let Some(member_dict) = member_dict else { continue };
                let rotation = page_rotation(doc, member.page)?;
                if let Some(prepared) = prepare(doc, &member_dict, member.link.flags, rotation)? {
                    if claims_pdfa && !prepared.fonts_embedded && !unembedded.contains(id) {
                        unembedded.push(id.clone());
                    }
                    draws.push(Draw {
                        page: member.page.clone(),
                        order: member.order,
                        prepared,
                    });
                }
            }
            removed.extend(cascade);
            flattened.push(id.clone());
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
            let on_page: Vec<&Prepared> = draws.iter().filter(|d| d.page == *page).map(|d| &d.prepared).collect();
            write_page(doc, page, &on_page, tagged)?;
        }
        let mut also_deleted = Vec::new();
        for id in &flattened {
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
            removed: !flattened.is_empty(),
            also_deleted,
            flattened,
            not_flattened,
            flattened_unembedded: unembedded,
            ..Effect::default()
        })
    }
}

/// The annotations to flatten on `pages` (every page when `None`), in page order: each comment
/// and mark but form fields' widgets, links, Popups and replies (a reply goes with the comment it
/// answers), and but those whose removal would take a `Locked` annotation (itself, a reply, a
/// member of its group), which [`FlattenAnnots`] refuses. The façade's scopes are built on it.
///
/// # Errors
/// Reading errors.
///
/// Added in 0.6.0.
pub fn flatten_candidates(doc: &laminapdf_model::Document, pages: Option<&[PageId]>) -> Result<Vec<AnnotId>> {
    let all = every_annotation(doc)?;
    let mut out = Vec::new();
    for (page, annots) in &all {
        if pages.is_some_and(|wanted| !wanted.contains(page)) {
            continue;
        }
        for link in annots {
            if link.in_reply_to.is_some()
                || matches!(
                    link.subtype,
                    AnnotSubtype::Widget | AnnotSubtype::Link | AnnotSubtype::Popup
                )
            {
                continue;
            }
            let locked = doomed(link, &all)
                .iter()
                .filter_map(|m| Placed::find(&all, m))
                .any(|m| m.link.flags.has(AnnotFlags::LOCKED));
            if !locked {
                out.push(link.id.clone());
            }
        }
    }
    Ok(out)
}

/// Where viewers paint an annotation: its page's place in the document, then its place in the
/// page's `/Annots`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PaintOrder {
    page: usize,
    annot: usize,
}

/// An annotation some page lists, with that page and its painting order.
struct Placed<'a> {
    page: &'a PageId,
    order: PaintOrder,
    link: &'a AnnotLink,
}

impl<'a> Placed<'a> {
    fn find(all: &'a [(PageId, Vec<AnnotLink>)], id: &AnnotId) -> Option<Self> {
        all.iter().enumerate().find_map(|(page_index, (page, annots))| {
            annots.iter().find(|a| a.id == *id).map(|link| Self {
                page,
                order: PaintOrder {
                    page: page_index,
                    annot: link.index,
                },
                link,
            })
        })
    }
}

/// One appearance to draw, on its page.
struct Draw {
    page: PageId,
    order: PaintOrder,
    prepared: Prepared,
}

/// How to draw one appearance.
struct Prepared {
    form: ObjRef,
    /// What the page sets with `cm` before `Do`: the fit onto `/Rect`, then the turn that keeps a
    /// `NoRotate` annotation upright on a turned page.
    matrix: Matrix,
    /// `/CA` and `/ca` (§12.5.2), when either is below 1: they apply to the whole appearance.
    opacity: Option<(f64, f64)>,
    /// `/BM` (PDF 2.0, §12.5.2), when not `Normal`.
    blend: Option<Object>,
    /// The annotation's `/OC`, which the drawing keeps as marked content so it follows its layer.
    optional_content: Option<Object>,
    /// Whether every font the appearance's resources name is embedded.
    fonts_embedded: bool,
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

/// The page's `/Rotate` in degrees, clockwise.
fn page_rotation(doc: &laminapdf_model::Document, page: &PageId) -> Result<u16> {
    let pages = doc.pages()?;
    let found = pages.by_id(page).ok_or_else(|| Error::UnknownPage(page.clone()))?;
    Ok(found.rotate()?.degrees())
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
        && doc.optional_content_off(oc)?
    {
        return Ok(false);
    }
    Ok(true)
}

/// Gives an annotation without an appearance the one Lamina PDF draws for its kind, in `dict`;
/// `false` when its kind is one Lamina PDF doesn't draw, or the generator can't draw this one
/// (there is none, or a text box needs text layout or fonts it lacks).
fn draw_appearance(
    doc: &mut laminapdf_model::Document,
    dict: &mut Dict,
    appearances: Option<&dyn Appearances>,
    blobs: &BlobStore,
) -> Result<bool> {
    if !super::redraw::drawable(doc, dict)? {
        return Ok(false);
    }
    let redraw = Redraw {
        color: None,
        opacity: None,
        spec: None,
    };
    match redraw.apply(doc, dict, appearances, blobs) {
        Ok(_) => Ok(true),
        Err(
            Error::AppearanceUnavailable
            | Error::AppearanceNeedsText
            | Error::AnnotationGlyphMissing
            | Error::AnnotationFontInvalid,
        ) => Ok(false),
        Err(e) => Err(e),
    }
}

/// How to draw `dict`'s normal appearance on a page turned by `rotation`; `None` when it has
/// none, the appearance's optional content is off, or its box maps onto nothing.
fn prepare(
    doc: &mut laminapdf_model::Document,
    dict: &Dict,
    flags: AnnotFlags,
    rotation: u16,
) -> Result<Option<Prepared>> {
    let Some(appearance) = doc.normal_appearance(dict)? else {
        return Ok(None);
    };
    if let Some(oc) = appearance.stream.dict.get("OC")
        && doc.optional_content_off(oc)?
    {
        return Ok(None);
    }
    // `Do` applies the form's own `/Matrix`: the page sets only the fit.
    let Some(mut matrix) = doc.appearance_placement(dict, &appearance.stream)?.map(|p| p.fit) else {
        return Ok(None);
    };
    if flags.has(AnnotFlags::NO_ROTATE)
        && rotation != 0
        && let Some(rect) = doc.annotation_rect(dict)?
    {
        matrix = matrix.then(&upright(rect.x0, rect.y1, rotation));
    }
    let fonts_embedded = super::conformance::form_fonts_embedded(doc, &appearance.stream)?;
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
    let number = |key: &str| -> Result<Option<f64>> {
        Ok(match dict.get(key) {
            Some(v) => doc.deref(v)?.as_f64().map(|v| v.clamp(0.0, 1.0)),
            None => None,
        })
    };
    // `/ca` (PDF 2.0) is the non-stroking opacity, `/CA` the stroking one and, without `/ca`,
    // both (Table 172).
    let stroke = number("CA")?.unwrap_or(1.0);
    let fill = number("ca")?.unwrap_or(stroke);
    let opacity = (stroke < 1.0 || fill < 1.0).then_some((stroke, fill));
    let blend = dict
        .get("BM")
        .filter(|bm| !matches!(bm, Object::Name(n) if n.as_bytes() == b"Normal"))
        .cloned();
    Ok(Some(Prepared {
        form,
        matrix,
        opacity,
        blend,
        optional_content: dict.get("OC").cloned(),
        fonts_embedded,
    }))
}

/// The turn that keeps a `NoRotate` annotation upright on a page shown turned `rotation` degrees
/// clockwise: the same angle the other way, about the `/Rect`'s upper-left corner `(x, y)`,
/// which stays where it is (ISO 32000-2 §12.5.3, Figure 85).
fn upright(x: f64, y: f64, rotation: u16) -> Matrix {
    Matrix::translate(-x, -y)
        .then(&Matrix::rotate(f64::from(rotation)))
        .then(&Matrix::translate(x, y))
}

/// Appends the page's drawings after its content, wrapped once.
fn write_page(doc: &mut laminapdf_model::Document, page: &PageId, draws: &[&Prepared], tagged: bool) -> Result<()> {
    let cos_page = {
        let pages = doc.pages()?;
        let found = pages.by_id(page).ok_or_else(|| Error::UnknownPage(page.clone()))?;
        found.cos().clone()
    };
    let mut writer = laminapdf_content::PageContentWriter::new(doc.cos_mut().edit(), &cos_page)?;
    writer.wrap_existing()?;
    let mut content = String::new();
    let name = |n: &laminapdf_syntax::Name| String::from_utf8_lossy(n.as_bytes()).into_owned();
    for draw in draws {
        let form = writer.add_xobject(draw.form)?;
        if tagged {
            content.push_str("/Artifact BMC\n");
        }
        if let Some(oc) = &draw.optional_content {
            let properties = writer.add_properties(oc.clone())?;
            let _ = writeln!(content, "/OC /{} BDC", name(&properties));
        }
        content.push_str("q\n");
        if draw.opacity.is_some() || draw.blend.is_some() {
            let mut gs = Dict::new();
            if let Some((stroke, fill)) = draw.opacity {
                gs.insert("CA", Object::Real(stroke));
                gs.insert("ca", Object::Real(fill));
            }
            if let Some(blend) = &draw.blend {
                gs.insert("BM", blend.clone());
            }
            let gs = writer.add_extgstate(Object::Dict(gs))?;
            let _ = writeln!(content, "/{} gs", name(&gs));
        }
        let matrix = draw.matrix.to_array().map(format_real).join(" ");
        let _ = write!(content, "{matrix} cm\n/{} Do\nQ\n", name(&form));
        if draw.optional_content.is_some() {
            content.push_str("EMC\n");
        }
        if tagged {
            content.push_str("EMC\n");
        }
    }
    writer.append(content.as_bytes())?;
    Ok(())
}
