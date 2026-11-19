//! What the annotations the pending edits wrote break of the document's PDF/A or PDF/UA claim
//! (E4 plan §3). Only annotations in the overlay are checked, so a file's own faults are never
//! reported as the edit's; the warnings never block the edit or the save.

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::xmp::PacketState;
use laminapdf_model::{AnnotFlags, AnnotSubtype};

use super::redraw::{appearance_resources, appearance_states};
use super::{every_annotation, object_dict};
use crate::error::Result;
use crate::metadata::{ColorRules, ConformanceWarning, colors_fit, output_intent};

/// The flags PDF/A forbids on an annotation (ISO 19005-2 §6.3.2).
const PDFA_FORBIDDEN: u32 =
    AnnotFlags::INVISIBLE | AnnotFlags::HIDDEN | AnnotFlags::NO_VIEW | AnnotFlags::TOGGLE_NO_VIEW;

/// The warnings for the annotations the overlay holds, one per kind of fault, each naming its
/// annotations in page order.
pub(crate) fn warnings(doc: &laminapdf_model::Document) -> Result<Vec<ConformanceWarning>> {
    let PacketState::Parsed(packet) = doc.xmp()? else {
        return Ok(Vec::new());
    };
    let (pdfa_part, claims_ua) = (packet.pdfa_part(), packet.pdfua_part() == Some(1));
    if pdfa_part.is_none() && !claims_ua {
        return Ok(Vec::new());
    }
    let [
        mut untitled,
        mut untagged,
        mut flags,
        mut transparent,
        mut unembedded,
        mut off_intent,
    ] = [(); 6].map(|()| Vec::new());
    let rules = match pdfa_part {
        Some(part) => ColorRules::of(output_intent(doc)?, part),
        None => None,
    };
    for (_, annots) in every_annotation(doc)? {
        for annot in annots {
            // Popups belong to their parent, and links have checks of their own.
            if matches!(annot.subtype, AnnotSubtype::Popup | AnnotSubtype::Link) {
                continue;
            }
            let Some(r) = annot.id.as_lamina_id().obj_ref() else {
                continue;
            };
            if doc.overlay().get(r.num).is_none() {
                continue;
            }
            let Some(dict) = object_dict(doc, r)? else { continue };
            // An appearance an edit drew, such as an image stamp's picture; one the file brought is
            // the file's own.
            if let Some(rules) = rules
                && !new_appearance_fits(doc, &dict, rules)?
            {
                off_intent.push(annot.id.to_string());
            }
            // A widget a fill drew again in a standard 14 face, given no font to embed; its other
            // checks are the form's own.
            if annot.subtype == AnnotSubtype::Widget {
                let drawn_now = new_normal_appearance(doc, &dict)?.is_some();
                if pdfa_part.is_some() && drawn_now && !fonts_embedded(doc, &dict)? {
                    unembedded.push(annot.id.to_string());
                }
                continue;
            }
            let id = annot.id.to_string();
            if claims_ua {
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
            if pdfa_part.is_some() && (!annot.flags.has(AnnotFlags::PRINT) || annot.flags.0 & PDFA_FORBIDDEN != 0) {
                flags.push(id.clone());
            }
            // A text box or a dynamic stamp whose drawing uses a font that isn't embedded, as a
            // standard 14 font isn't; one given by its bytes is embedded as a subset, a standard
            // stamp's name is drawn as paths, and another app's mark keeps its own fonts until it
            // is drawn again.
            if pdfa_part.is_some()
                && matches!(annot.subtype, AnnotSubtype::FreeText | AnnotSubtype::Stamp)
                && !fonts_embedded(doc, &dict)?
            {
                unembedded.push(id.clone());
            }
            if pdfa_part == Some(1) && is_transparent(doc, &dict)? {
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
            pages: Vec::new(),
        });
    }
    if !unembedded.is_empty() {
        out.push(ConformanceWarning::PdfaFontNotEmbedded {
            annotations: unembedded,
            pages: Vec::new(),
        });
    }
    if !off_intent.is_empty() {
        out.push(ConformanceWarning::PdfaColorSpace {
            annotations: off_intent,
            pages: Vec::new(),
        });
    }
    Ok(out)
}

/// Whether the normal appearance an edit wrote paints only in the colour spaces `rules` allow;
/// `true` for an appearance the file brought, or none.
fn new_appearance_fits(doc: &laminapdf_model::Document, dict: &laminapdf_cos::Dict, rules: ColorRules) -> Result<bool> {
    match new_normal_appearance(doc, dict)? {
        Some(normal) => colors_fit(doc, &Object::Ref(normal), rules, None),
        None => Ok(true),
    }
}

/// The annotation's normal appearance stream when an edit wrote it (it is past the file's last
/// object), else `None`.
fn new_normal_appearance(doc: &laminapdf_model::Document, dict: &laminapdf_cos::Dict) -> Result<Option<ObjRef>> {
    let normal = match dict.get("AP") {
        Some(ap) => doc
            .deref(ap)?
            .as_dict()
            .and_then(|ap| ap.get("N"))
            .and_then(Object::as_ref),
        None => None,
    };
    Ok(normal.filter(|n| i64::from(n.num) >= doc.size_floor()))
}

/// Whether every font the annotation's normal appearance uses is embedded: a Type 3 font, or one
/// whose descriptor (a Type 0 font's descendant's) holds a font program.
fn fonts_embedded(doc: &laminapdf_model::Document, dict: &laminapdf_cos::Dict) -> Result<bool> {
    all_embedded(doc, appearance_resources(doc, dict, "Font")?)
}

/// Whether every font a Form XObject's own `/Resources` name is embedded, as
/// [`fonts_embedded`] judges them: the fonts a flattened appearance brings to its page.
pub(crate) fn form_fonts_embedded(doc: &laminapdf_model::Document, form: &laminapdf_cos::Stream) -> Result<bool> {
    let resolve = |value: Option<&Object>| -> Result<Option<laminapdf_cos::Dict>> {
        match value {
            Some(v) => Ok(doc.deref(v)?.as_dict().cloned()),
            None => Ok(None),
        }
    };
    let Some(fonts) = resolve(form.dict.get("Resources"))?
        .map(|r| resolve(r.get("Font")))
        .transpose()?
        .flatten()
    else {
        return Ok(true);
    };
    let mut entries = Vec::new();
    for (_, font) in fonts.iter() {
        entries.extend(resolve(Some(font))?);
    }
    all_embedded(doc, entries)
}

/// Whether each of `fonts` is a Type 3 font or has a font program in its descriptor (a Type 0
/// font's descendant's).
pub(crate) fn all_embedded(doc: &laminapdf_model::Document, fonts: Vec<laminapdf_cos::Dict>) -> Result<bool> {
    let resolve = |value: Option<&Object>| -> Result<Option<laminapdf_cos::Dict>> {
        match value {
            Some(v) => Ok(doc.deref(v)?.as_dict().cloned()),
            None => Ok(None),
        }
    };
    for font in fonts {
        let font = match font.get_name("Subtype") {
            Some(b"Type3") => continue,
            Some(b"Type0") => {
                let first = match font.get("DescendantFonts").map(|d| doc.deref(d)).transpose()? {
                    Some(d) => d.as_array().and_then(|a| a.first()).cloned(),
                    None => None,
                };
                match resolve(first.as_ref())? {
                    Some(descendant) => descendant,
                    None => return Ok(false),
                }
            }
            _ => font,
        };
        let Some(descriptor) = resolve(font.get("FontDescriptor"))? else {
            return Ok(false);
        };
        if !["FontFile", "FontFile2", "FontFile3"]
            .iter()
            .any(|k| descriptor.get(k).is_some())
        {
            return Ok(false);
        }
    }
    Ok(true)
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
