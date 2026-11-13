//! What the annotations the pending edits wrote break of the document's PDF/A or PDF/UA claim
//! (E4 plan §3). Only annotations in the overlay are checked, so a file's own faults are never
//! reported as the edit's; the warnings never block the edit or the save.

use laminapdf_cos::Object;
use laminapdf_model::xmp::PacketState;
use laminapdf_model::{AnnotFlags, AnnotSubtype};

use super::redraw::appearance_states;
use super::{every_annotation, object_dict};
use crate::error::Result;
use crate::metadata::ConformanceWarning;

/// The flags PDF/A forbids on an annotation (ISO 19005-2 §6.3.2).
const PDFA_FORBIDDEN: u32 =
    AnnotFlags::INVISIBLE | AnnotFlags::HIDDEN | AnnotFlags::NO_VIEW | AnnotFlags::TOGGLE_NO_VIEW;

/// The warnings for the annotations the overlay holds, one per kind of fault, each naming its
/// annotations in page order.
pub(crate) fn warnings(doc: &laminapdf_model::Document) -> Result<Vec<ConformanceWarning>> {
    let PacketState::Parsed(packet) = doc.xmp()? else {
        return Ok(Vec::new());
    };
    let (pdfa, pdfua) = (packet.pdfa_part(), packet.pdfua_part() == Some(1));
    if pdfa.is_none() && !pdfua {
        return Ok(Vec::new());
    }
    let [mut untitled, mut untagged, mut flags, mut transparent] = [(); 4].map(|()| Vec::new());
    for (_, annots) in every_annotation(doc)? {
        for annot in annots {
            // Popups belong to their parent, and widgets and links have checks of their own.
            if matches!(
                annot.subtype,
                AnnotSubtype::Popup | AnnotSubtype::Widget | AnnotSubtype::Link
            ) {
                continue;
            }
            let Some(r) = annot.id.as_lamina_id().obj_ref() else {
                continue;
            };
            if doc.overlay().get(r.num).is_none() {
                continue;
            }
            let Some(dict) = object_dict(doc, r)? else { continue };
            let id = annot.id.to_string();
            if pdfua {
                let empty = match dict.get("Contents") {
                    Some(Object::String(s)) => s.bytes.is_empty(),
                    _ => true,
                };
                if empty {
                    untitled.push(id.clone());
                }
                // Only a new annotation is the edit's to place: one the file already had and an
                // edit changed is as tagged as the file left it.
                let new = i64::from(r.num) >= doc.size_floor();
                if new && dict.get("StructParent").is_none() {
                    untagged.push(id.clone());
                }
            }
            if pdfa.is_some() && (!annot.flags.has(AnnotFlags::PRINT) || annot.flags.0 & PDFA_FORBIDDEN != 0) {
                flags.push(id.clone());
            }
            if pdfa == Some(1) && is_transparent(doc, &dict)? {
                transparent.push(id);
            }
        }
    }
    let mut out = Vec::new();
    if !untitled.is_empty() {
        out.push(ConformanceWarning::AnnotWithoutContents { annotations: untitled });
    }
    if !untagged.is_empty() {
        out.push(ConformanceWarning::AnnotNotTagged { annotations: untagged });
    }
    if !flags.is_empty() {
        out.push(ConformanceWarning::PdfaAnnotFlags { annotations: flags });
    }
    if !transparent.is_empty() {
        out.push(ConformanceWarning::PdfaTransparency {
            annotations: transparent,
        });
    }
    Ok(out)
}

/// Whether the annotation or its appearance uses transparency, which PDF/A-1 forbids
/// (ISO 19005-1 §6.4): a `/CA` below 1, or an ExtGState with an opacity below 1 or a blend mode
/// other than Normal.
fn is_transparent(doc: &laminapdf_model::Document, dict: &laminapdf_cos::Dict) -> Result<bool> {
    let below_one = |v: Option<&Object>| v.and_then(Object::as_f64).is_some_and(|v| v < 1.0);
    if below_one(dict.get("CA")) {
        return Ok(true);
    }
    Ok(appearance_states(doc, dict)?.iter().any(|gs| {
        below_one(gs.get("ca"))
            || below_one(gs.get("CA"))
            || gs
                .get_name("BM")
                .is_some_and(|bm| bm != b"Normal".as_slice() && bm != b"Compatible".as_slice())
    }))
}
