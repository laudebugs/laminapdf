//! The covers of a page's annotations: viewers draw an annotation's normal appearance over the
//! page (ISO 32000-2 §12.5.5), so its opaque areas cover every glyph under them. Annotations
//! flagged `Hidden` or `NoView` (§12.5.3), popups, those with `/CA` below 1, a blend mode other
//! than `Normal` (`/BM`, PDF 2.0) or optional content (`/OC`, on the annotation or its
//! appearance), and those without an appearance stream are left out. The covers only take text
//! out, so an annotation that can't be read for any reason, a hard limit included, is skipped
//! (ADR 0003's tolerant reading): it covers nothing, and the page's text stays.

use laminapdf_content::{Visitor, interpret_content};
use laminapdf_cos::{Result, absent_if_malformed, tolerant};
use laminapdf_model::BlendMode;
use laminapdf_model::{Document, Page, PageFrame};
use laminapdf_syntax::CancelToken;
use laminapdf_syntax::{Dict, Object, Stream};

use super::{Collector, Cover};

/// `/F` bit 2: `Hidden`.
const HIDDEN: i64 = 1 << 1;
/// `/F` bit 6: `NoView`.
const NO_VIEW: i64 = 1 << 5;

/// The covers of `page`'s annotations, each drawn after the whole page. Stops early, with the
/// covers so far, once `cancel` is set; the caller then fails the job.
///
/// # Errors
/// What [`tolerant`] and [`absent_if_malformed`] pass on: strict mode, pending data or a
/// source error, and for the `/Annots` array itself a reference cycle or a limit.
pub(crate) fn covers(doc: &Document, page: &Page<'_>, frame: &PageFrame, cancel: &CancelToken) -> Result<Vec<Cover>> {
    let mut out = Vec::new();
    let Some(annots) = absent_if_malformed(page.annots())? else {
        return Ok(out);
    };
    for annot in annots {
        if cancel.is_cancelled() {
            break;
        }
        if let Some(covers) = tolerant(annotation(doc, annot, frame))? {
            out.extend(covers);
        }
    }
    Ok(out)
}

/// The normal appearance stream ([`Document::normal_appearance`]), when the annotation is shown
/// on screen at full opacity.
fn appearance(doc: &Document, dict: &Dict) -> Result<Option<Stream>> {
    if matches!(dict.get("Subtype"), Some(Object::Name(n)) if n.as_bytes() == b"Popup") {
        return Ok(None);
    }
    let flags = match dict.get("F") {
        Some(f) => doc.deref(f)?.as_int().unwrap_or(0),
        None => 0,
    };
    if flags & (HIDDEN | NO_VIEW) != 0 {
        return Ok(None);
    }
    if let Some(ca) = dict.get("CA")
        && doc.deref(ca)?.as_f64().is_none_or(|ca| ca < 1.0)
    {
        return Ok(None);
    }
    if dict.get("OC").is_some() {
        return Ok(None);
    }
    let normal = |bm: &Object| matches!(bm, Object::Name(n) if BlendMode::from_pdf_name(n.as_bytes()) == Some(BlendMode::Normal));
    if dict.get("BM").is_some_and(|bm| !normal(bm)) {
        return Ok(None);
    }
    Ok(doc.normal_appearance(dict)?.map(|a| a.stream))
}

fn annotation(doc: &Document, annot: laminapdf_syntax::ObjRef, frame: &PageFrame) -> Result<Vec<Cover>> {
    let object = doc.resolve(annot)?;
    let Some(dict) = object.as_dict() else {
        return Ok(Vec::new());
    };
    let Some(stream) = appearance(doc, dict)?.filter(|s| s.dict.get("OC").is_none()) else {
        return Ok(Vec::new());
    };
    let Some(target) = doc.annotation_rect(dict)? else {
        return Ok(Vec::new());
    };
    let Some(outer) = doc.appearance_placement(dict, &stream)?.map(|p| p.to_user_space()) else {
        return Ok(Vec::new());
    };
    let content = doc.decode_stream(&stream)?;
    let resources = match stream.dict.get("Resources") {
        Some(r) => Some(doc.deref(r)?.into_arc()),
        None => None,
    };
    let mut collector = Collector::appearance(frame, outer, frame.rect_to_page(&target));
    // A trait object, as `laminapdf-text` passes one: one interpreter in the WebAssembly modules.
    interpret_content(doc, &content, resources, &mut collector as &mut dyn Visitor)?;
    Ok(collector.finish().1)
}
