//! What content edits break of the document's PDF/A or PDF/UA claim (E7 plan §3): a font an edit
//! added to an edited page that isn't embedded, a figure whose image was replaced and has no
//! alternative text yet, and tags the edits couldn't keep whole. Each is checked against the
//! document as it is now, so an undone edit, or a fix made since, warns of nothing.

use std::collections::BTreeSet;

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::PageId;
use laminapdf_model::xmp::PacketState;

use super::ConformanceWarning;
use crate::annot::{all_embedded, object_dict};
use crate::error::Result;

/// Adds to `out` the warnings `pending`'s content edits raise on the file's claims.
pub(super) fn warnings(
    doc: &laminapdf_model::Document,
    pending: &crate::PendingChanges,
    out: &mut Vec<ConformanceWarning>,
) -> Result<()> {
    if pending.content_pages().is_empty() && pending.alt_cleared().is_empty() && pending.tags_broken().is_empty() {
        return Ok(());
    }
    let PacketState::Parsed(packet) = doc.xmp()? else {
        return Ok(());
    };
    // Each live page by its object, so a page an edit touched and then deleted counts no more.
    let live: Vec<(ObjRef, PageId)> = doc
        .pages()?
        .iter()
        .filter_map(|p| p.id().as_lamina_id().obj_ref().map(|r| (r, p.id().clone())))
        .collect();
    if packet.pdfua_part() == Some(1) {
        let figures = figures_without_alt(doc, pending.alt_cleared(), &live)?;
        if !figures.is_empty() {
            out.push(ConformanceWarning::FigureWithoutAlt { pages: figures });
        }
        let broken = broken_tags(doc, pending.tags_broken(), &live)?;
        if !broken.is_empty() {
            out.push(ConformanceWarning::ContentTagsBroken { pages: broken });
        }
    }
    if packet.pdfa_part().is_some() {
        let mut pages = Vec::new();
        for (r, page) in &live {
            if pending.content_pages().contains(page) && !added_fonts_embedded(doc, *r)? {
                pages.push(page.to_string());
            }
        }
        if !pages.is_empty() {
            out.push(ConformanceWarning::PdfaFontNotEmbedded {
                annotations: Vec::new(),
                pages,
            });
        }
    }
    Ok(())
}

/// The pages of the figures in `cleared` that still have neither `/Alt` nor `/ActualText` and
/// are still in the tree, each page once, in page order.
fn figures_without_alt(
    doc: &laminapdf_model::Document,
    cleared: &BTreeSet<ObjRef>,
    live: &[(ObjRef, PageId)],
) -> Result<Vec<String>> {
    let mut pages: BTreeSet<ObjRef> = BTreeSet::new();
    for &elem in cleared {
        let Some(dict) = object_dict(doc, elem)? else { continue };
        let described = dict.get("Alt").is_some() || dict.get("ActualText").is_some();
        let Some(page) = dict.get("Pg").and_then(Object::as_ref) else {
            continue;
        };
        if !described && dict.get("K").is_some() && live.iter().any(|(r, _)| *r == page) {
            pages.insert(page);
        }
    }
    Ok(live
        .iter()
        .filter(|(r, _)| pages.contains(r))
        .map(|(_, p)| p.to_string())
        .collect())
}

/// The pages of `flagged` whose tags are still broken, in page order.
fn broken_tags(
    doc: &laminapdf_model::Document,
    flagged: &BTreeSet<PageId>,
    live: &[(ObjRef, PageId)],
) -> Result<Vec<String>> {
    let mut out = Vec::new();
    for (_, page) in live.iter().filter(|(_, p)| flagged.contains(p)) {
        if crate::content::tags_broken(doc, page)? {
            out.push(page.to_string());
        }
    }
    Ok(out)
}

/// Whether every font the page's own resources name that the edits added (an object new since
/// the file was opened) is embedded.
fn added_fonts_embedded(doc: &laminapdf_model::Document, page: ObjRef) -> Result<bool> {
    let resolve = |value: Option<&Object>| -> Result<Option<laminapdf_cos::Dict>> {
        match value {
            Some(v) => Ok(doc.deref(v)?.as_dict().cloned()),
            None => Ok(None),
        }
    };
    let Some(page_dict) = object_dict(doc, page)? else {
        return Ok(true);
    };
    let Some(fonts) = resolve(page_dict.get("Resources"))?
        .map(|r| resolve(r.get("Font")))
        .transpose()?
        .flatten()
    else {
        return Ok(true);
    };
    let mut added = Vec::new();
    for (_, font) in fonts.iter() {
        if let Object::Ref(r) = font
            && i64::from(r.num) >= doc.size_floor()
        {
            added.extend(resolve(Some(font))?);
        }
    }
    all_embedded(doc, added)
}
