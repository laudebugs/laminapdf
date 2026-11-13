//! Drawing an existing annotation's appearance again, when an [`UpdateAnnot`](super::UpdateAnnot)
//! changes what it draws: its colour, its opacity or its geometry. The annotation's spec is read
//! back from its keys, whichever app made it, so editing another app's highlight gives it Lamina
//! PDF's appearance (E4 plan §2.1).

use laminapdf_cos::{Dict, Object};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};

use super::add::{intent_space, write_spec};
use super::appearance::{AppearanceRequest, Appearances};
use super::geom::{self, AnnotColor, Opacity};
use super::spec::{AnnotSpec, AnnotSpecPatch, Caret, CaretSymbol, MarkupKind, Note, NoteIcon, TextMarkup};
use crate::error::{Error, Result};

/// What an update changes in the drawing.
pub(super) struct Redraw<'a> {
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
    pub(super) fn apply(
        &self,
        doc: &mut laminapdf_model::Document,
        dict: &mut Dict,
        appearances: Option<&dyn Appearances>,
    ) -> Result<()> {
        let current = read_spec(dict).ok_or(Error::InvalidOp(
            "only text markup, notes and carets have their appearance drawn again",
        ))?;
        let spec = match self.spec {
            Some(patch) => patch
                .applied_to(&current)
                .ok_or(Error::InvalidOp("the spec patch is for another kind of mark"))?,
            None => current,
        }
        .normalised();
        if !spec.is_valid() {
            return Err(Error::InvalidOp(
                "an annotation needs quads, and every coordinate finite",
            ));
        }
        let appearances = appearances.ok_or(Error::AppearanceUnavailable)?;
        let color = match self.color {
            Some(color) => color,
            None => dict
                .get("C")
                .and_then(components)
                .and_then(|c| AnnotColor::from_components(&c)),
        };
        let opacity = match self.opacity {
            Some(opacity) => opacity,
            None => current_opacity(doc, dict)?,
        };
        let request = AppearanceRequest {
            spec: &spec,
            color,
            opacity,
            intent: intent_space(doc)?,
        };
        let appearance = appearances.normal(&mut doc.cos_mut().edit(), &request)?;
        dict.insert("Rect", super::add::rect_object(&appearance.rect));
        dict.insert(
            "AP",
            Object::Dict([("N", Object::Ref(appearance.normal))].into_iter().collect()),
        );
        write_spec(dict, &spec);
        match color {
            Some(color) => {
                dict.insert("C", color.to_object());
            }
            None => {
                dict.remove("C");
            }
        }
        if opacity == Opacity::OPAQUE || doc.is_pdf2()? {
            dict.remove("CA");
        } else {
            dict.insert("CA", Object::Real(opacity.get()));
        }
        Ok(())
    }
}

/// The spec an annotation's keys describe, for the subtypes this round draws: quads read in
/// either order, a note at its `/Rect`'s upper-left corner, a caret filling its `/Rect`.
pub(super) fn read_spec(dict: &Dict) -> Option<AnnotSpec> {
    let subtype = dict.get_name("Subtype")?;
    let rect = dict.get("Rect").and_then(components).and_then(|r| match *r {
        [a, b, c, d] => Some(UserSpaceRect::from_corners(a, b, c, d)),
        _ => None,
    });
    if let Some(kind) = MarkupKind::from_subtype(subtype) {
        let values = dict.get("QuadPoints").and_then(components)?;
        let quads = values
            .chunks_exact(8)
            .map(|q| {
                let p = |i: usize| UserSpacePoint::new(q[2 * i], q[2 * i + 1]);
                geom::acrobat_order([p(0), p(1), p(2), p(3)])
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
        _ => None,
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

/// The ExtGStates of an annotation's normal appearance, in its resources' order; none when it
/// has no appearance or the appearance names none.
pub(super) fn appearance_states(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Vec<Dict>> {
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
    let Some(states) = resolve(resources.get("ExtGState"))? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for (_, state) in states.iter() {
        if let Some(state) = resolve(Some(state))? {
            out.push(state);
        }
    }
    Ok(out)
}

/// The numbers of an array.
fn components(value: &Object) -> Option<Vec<f64>> {
    value.as_array()?.iter().map(Object::as_f64).collect()
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
    fn a_square_has_no_spec_to_draw_again() {
        let d = dict(vec![
            ("Subtype", Object::name("Square")),
            ("Rect", numbers(&[0.0, 0.0, 1.0, 1.0])),
        ]);
        assert_eq!(read_spec(&d), None);
    }
}
