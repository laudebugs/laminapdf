//! Drawing an existing annotation's appearance again, when an [`UpdateAnnot`](super::UpdateAnnot)
//! changes what it draws: its colour, its opacity or its geometry. The annotation's spec is read
//! back from its keys, whichever app made it, so editing another app's highlight gives it Lamina
//! PDF's appearance (E4 plan §2.1).

use laminapdf_cos::{Dict, Object};
use laminapdf_model::UserSpacePoint;

use super::add::{intent_space, write_spec};
use super::appearance::{AppearanceRequest, Appearances};
use super::geom::{self, AnnotColor, Opacity, numbers_of, rect_of};
use super::spec::{AnnotSpec, AnnotSpecPatch, Caret, CaretSymbol, MarkupKind, Note, NoteIcon, TextMarkup};
use crate::blob::BlobStore;
use crate::error::{Error, Result};

/// What an update changes in the drawing.
pub(super) struct Redraw<'a> {
    /// Absent to keep `/C`, `null` to remove it, or a colour: three cases.
    #[expect(clippy::option_option, reason = "absent, null and a colour are three cases")]
    pub(super) color: Option<Option<AnnotColor>>,
    pub(super) opacity: Option<Opacity>,
    pub(super) spec: Option<&'a AnnotSpecPatch>,
}

impl Redraw<'_> {
    /// Whether anything is drawn again.
    pub(super) const fn any(&self) -> bool {
        self.color.is_some() || self.opacity.is_some() || self.spec.is_some()
    }

    /// Draws the annotation `dict` again with the changes, and writes its `/AP`, `/Rect`, `/C`,
    /// `/CA` and the spec's own keys. Other appearance states (`/D`, `/R`) go: they showed the
    /// old drawing.
    ///
    /// Returns whether a text box's rich text lost styles outside the subset (`/RC` it read but
    /// can't keep, E4 plan §11).
    pub(super) fn apply(
        &self,
        doc: &mut laminapdf_model::Document,
        dict: &mut Dict,
        appearances: Option<&dyn Appearances>,
        blobs: &BlobStore,
    ) -> Result<bool> {
        let (current, current_color, simplified) = current(doc, dict)?.ok_or(Error::InvalidOp(
            "only text markup, notes, carets, ink, shapes, lines, polygons, text boxes, standard and dynamic stamps and file attachments have their appearance drawn again",
        ))?;
        let spec = match self.spec {
            Some(patch) => patch.applied_to(&current).ok_or(Error::InvalidOp(match (patch, &current) {
                (AnnotSpecPatch::Ink(_), AnnotSpec::Ink(_)) => {
                    "an ink patch's remove names a stroke the mark doesn't have, or its strokes come with remove or add"
                }
                _ => "the spec patch is for another kind of mark",
            }))?,
            None => current,
        };
        // Checked as given, as AddAnnot checks it: thinning would hide widths that don't match
        // their points.
        if !spec.is_valid() {
            return Err(Error::InvalidOp(super::add::INVALID_GEOMETRY));
        }
        let spec = spec.normalised();
        // A file attachment's file is already written; only its icon is drawn again.
        if !matches!(spec, AnnotSpec::FileAttachment(_)) {
            super::add::check_blobs(&spec, blobs)?;
        }
        let appearances = appearances.ok_or(Error::AppearanceUnavailable)?;
        let color = self.color.unwrap_or(current_color);
        let opacity = match self.opacity {
            Some(opacity) => opacity,
            None => current_opacity(doc, dict)?,
        };
        let request = AppearanceRequest {
            spec: &spec,
            color,
            opacity,
            intent: intent_space(doc)?,
            blobs: Some(blobs),
        };
        appearances.check(&request)?;
        let appearance = appearances.normal(&mut doc.cos_mut().edit(), &request)?;
        dict.insert("Rect", super::add::rect_object(&appearance.rect));
        dict.insert(
            "AP",
            Object::Dict([("N", Object::Ref(appearance.normal))].into_iter().collect()),
        );
        write_spec(dict, &spec, color);
        // A text box keeps its colour in `/DA` and its fill in `/C`, which `write_spec` wrote.
        if !matches!(spec, AnnotSpec::FreeText(_)) {
            match color {
                Some(color) => {
                    dict.insert("C", color.to_object());
                }
                None => {
                    dict.remove("C");
                }
            }
        }
        if opacity == Opacity::OPAQUE || doc.is_pdf2()? {
            dict.remove("CA");
        } else {
            dict.insert("CA", Object::Real(opacity.get()));
        }
        // Whatever the change, the box's keys are written again from what the subset kept.
        Ok(simplified)
    }
}

/// Whether Lamina PDF draws appearances for the annotation's kind and can read its spec from its
/// keys: what [`Redraw::apply`] needs before it draws.
pub(super) fn drawable(doc: &laminapdf_model::Document, dict: &Dict) -> Result<bool> {
    Ok(current(doc, dict)?.is_some())
}

/// The annotation's spec as its keys say, its colour, and whether reading a text box's rich text
/// dropped styles outside the subset.
fn current(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Option<(AnnotSpec, Option<AnnotColor>, bool)>> {
    if dict.get_name("Subtype") == Some(b"FreeText".as_slice()) {
        let (inline, unreadable) = text_keys_inline(doc, dict)?;
        return Ok(super::freetext::read(&inline).map(|read| {
            (
                AnnotSpec::FreeText(read.text),
                read.color,
                read.simplified || unreadable,
            )
        }));
    }
    let color = dict
        .get("C")
        .and_then(numbers_of)
        .and_then(|c| AnnotColor::from_components(&c));
    Ok(read_spec(dict).map(|spec| (spec, color, false)))
}

/// A text box's keys with its text values as strings: an indirect one resolved, and `/RC` as a
/// text stream (ISO 32000-2 §12.5.6.6) decoded. True when `/RC` couldn't be decoded, so the
/// rich text it held is lost.
fn text_keys_inline(doc: &laminapdf_model::Document, dict: &Dict) -> Result<(Dict, bool)> {
    let mut inline = dict.clone();
    let mut unreadable = false;
    for key in ["RC", "DA", "DS", "Contents"] {
        let Some(value) = dict.get(key) else { continue };
        if !matches!(value, Object::Ref(_) | Object::Stream(_)) {
            continue;
        }
        let resolved = doc.deref(value)?.into_owned();
        let text = match resolved {
            Object::Stream(stream) => match doc.cos().decode_stream(&stream) {
                // A stream's XHTML is UTF-8 as a rule, with no byte order mark to say so.
                Ok(bytes) if std::str::from_utf8(&bytes).is_ok() && !bytes.starts_with(&[0xef, 0xbb, 0xbf]) => {
                    let marked = [&[0xef, 0xbb, 0xbf][..], &bytes].concat();
                    Some(Object::String(laminapdf_cos::PdfString::literal(marked)))
                }
                Ok(bytes) => Some(Object::String(laminapdf_cos::PdfString::literal(bytes))),
                Err(_) => {
                    unreadable |= key == "RC";
                    None
                }
            },
            other @ Object::String(_) => Some(other),
            _ => None,
        };
        match text {
            Some(text) => {
                inline.insert(key, text);
            }
            None => {
                inline.remove(key);
            }
        }
    }
    Ok((inline, unreadable))
}

/// The spec an annotation's keys describe, for the subtypes this round draws: quads read in
/// either order, a note at its `/Rect`'s upper-left corner, a caret filling its `/Rect`.
pub(super) fn read_spec(dict: &Dict) -> Option<AnnotSpec> {
    let subtype = dict.get_name("Subtype")?;
    let rect = dict.get("Rect").and_then(rect_of);
    if let Some(kind) = MarkupKind::from_subtype(subtype) {
        let values = dict.get("QuadPoints").and_then(numbers_of)?;
        let (quads, _) = values.as_chunks::<8>();
        let quads = quads
            .iter()
            .map(|[ax, ay, bx, by, cx, cy, dx, dy]| {
                let p = UserSpacePoint::new;
                geom::acrobat_order([p(*ax, *ay), p(*bx, *by), p(*cx, *cy), p(*dx, *dy)])
            })
            .collect();
        return Some(AnnotSpec::TextMarkup(TextMarkup::new(kind, quads)));
    }
    match subtype {
        b"Text" => {
            let rect = rect?;
            let icon = dict.get_name("Name").map_or(NoteIcon::Note, NoteIcon::from_name);
            let open = matches!(dict.get("Open"), Some(Object::Bool(true)));
            Some(AnnotSpec::Note(
                Note::new(UserSpacePoint::new(rect.x0, rect.y1), icon).with_open(open),
            ))
        }
        b"Caret" => {
            let mut caret = Caret::new(rect?);
            if dict.get_name("Sy") == Some(b"P".as_slice()) {
                caret.symbol = CaretSymbol::Paragraph;
            }
            Some(AnnotSpec::Caret(caret))
        }
        b"FreeText" => super::freetext::read(dict).map(|r| AnnotSpec::FreeText(r.text)),
        b"Stamp" => super::Stamp::read(dict, rect).map(AnnotSpec::Stamp),
        b"FileAttachment" => super::FileAttachment::read(dict, rect).map(AnnotSpec::FileAttachment),
        other => super::drawing::read(other, dict, rect),
    }
}

/// An annotation's opacity: `/CA`, else the `/ca` of its appearance's first ExtGState (where a
/// PDF 2.0 file keeps it), else opaque.
fn current_opacity(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Opacity> {
    if let Some(ca) = dict.get("CA").and_then(Object::as_f64) {
        return Ok(Opacity::new(ca).unwrap_or_default());
    }
    Ok(appearance_states(doc, dict)?
        .first()
        .and_then(|gs| gs.get("ca").and_then(Object::as_f64))
        .and_then(Opacity::new)
        .unwrap_or_default())
}

/// The `ExtGState`s of an annotation's normal appearance, in its resources' order; none when it
/// has no appearance or the appearance names none.
pub(super) fn appearance_states(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Vec<Dict>> {
    appearance_resources(doc, dict, "ExtGState")
}

/// The entries of the `kind` resources (`ExtGState`, `Font`) of the annotation's normal
/// appearance, resolved.
pub(super) fn appearance_resources(doc: &laminapdf_model::Document, dict: &Dict, kind: &str) -> Result<Vec<Dict>> {
    let resolve = |value: Option<&Object>| -> Result<Option<Dict>> {
        match value {
            Some(v) => Ok(doc.deref(v)?.as_dict().cloned()),
            None => Ok(None),
        }
    };
    let Some(ap) = resolve(dict.get("AP"))? else {
        return Ok(Vec::new());
    };
    // `as_dict` gives a stream's dictionary too, which a Form XObject is.
    let Some(normal) = resolve(ap.get("N"))? else {
        return Ok(Vec::new());
    };
    let Some(resources) = resolve(normal.get("Resources"))? else {
        return Ok(Vec::new());
    };
    let Some(entries) = resolve(resources.get(kind))? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (_, entry) in entries.iter() {
        if let Some(entry) = resolve(Some(entry))? {
            out.push(entry);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dict(entries: Vec<(&str, Object)>) -> Dict {
        entries.into_iter().collect()
    }

    fn numbers(values: &[f64]) -> Object {
        Object::Array(values.iter().copied().map(Object::Real).collect())
    }

    #[test]
    fn a_highlight_stored_in_the_standards_order_reads_in_acrobats() {
        let d = dict(vec![
            ("Subtype", Object::name("Highlight")),
            ("QuadPoints", numbers(&[0.0, 0.0, 50.0, 0.0, 50.0, 10.0, 0.0, 10.0])),
        ]);
        let p = UserSpacePoint::new;
        assert_eq!(
            read_spec(&d),
            Some(AnnotSpec::TextMarkup(TextMarkup::new(
                MarkupKind::Highlight,
                vec![[p(0.0, 10.0), p(50.0, 10.0), p(0.0, 0.0), p(50.0, 0.0)]]
            )))
        );
    }

    #[test]
    fn a_note_reads_from_its_rect_and_name() {
        let d = dict(vec![
            ("Subtype", Object::name("Text")),
            ("Rect", numbers(&[100.0, 680.0, 120.0, 700.0])),
            ("Name", Object::name("Key")),
        ]);
        assert_eq!(
            read_spec(&d),
            Some(AnnotSpec::Note(Note::new(
                UserSpacePoint::new(100.0, 700.0),
                NoteIcon::Key
            )))
        );
    }

    #[test]
    fn an_image_stamp_has_no_spec_to_draw_again() {
        let d = dict(vec![
            ("Subtype", Object::name("Stamp")),
            ("Name", Object::name("LMNA_Image")),
            ("Rect", numbers(&[0.0, 0.0, 1.0, 1.0])),
        ]);
        assert_eq!(read_spec(&d), None);
    }
}
