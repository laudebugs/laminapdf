//! What the added objects on a page break of a PDF/A claim: see-through objects (PDF/A-1), fonts
//! that aren't embedded and colours outside the output intent.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, Object};
use laminapdf_model::PageId;

use super::place::{self, scan};
use crate::error::Result;

/// The pages among `pages` whose added objects break a PDF/A claim, each list in page order.
#[derive(Debug, Default)]
pub(crate) struct PdfaPages {
    /// Those with a see-through object, on PDF/A-1.
    pub(crate) transparent: Vec<String>,
    /// Those with a form that uses a font that isn't embedded.
    pub(crate) unembedded: Vec<String>,
    /// Those with an image or a form that paints in a device space the output intent doesn't
    /// allow.
    pub(crate) off_intent: Vec<String>,
}

/// What the added objects on `pages` break of the file's PDF/A-`part` claim.
pub(crate) fn pdfa_pages(doc: &laminapdf_model::Document, pages: &BTreeSet<PageId>, part: u8) -> Result<PdfaPages> {
    let rules = crate::metadata::ColorRules::of(crate::metadata::output_intent(doc)?, part);
    let mut out = PdfaPages::default();
    for page in doc.pages()?.iter() {
        if !pages.contains(page.id()) {
            continue;
        }
        let placed = scan(doc, page.cos())?;
        if part == 1 && see_through(doc, &placed)? {
            out.transparent.push(page.id().to_string());
        }
        let (mut embedded, mut fits) = (true, true);
        for p in &placed {
            let form = Object::Ref(p.form);
            embedded &= fonts_embedded(doc, &form, 0)?;
            if let Some(rules) = rules {
                fits &= crate::metadata::colors_fit(doc, &form, rules, page.cos().resources.clone())?;
            }
        }
        if !embedded {
            out.unembedded.push(page.id().to_string());
        }
        if !fits {
            out.off_intent.push(page.id().to_string());
        }
    }
    Ok(out)
}

/// How deep the PDF/A checks of an added form, [`fonts_embedded`] and the colour spaces' check,
/// look into forms inside forms: an added page is one form inside the object's, and its own forms
/// one more.
pub(crate) const MAX_FORM_DEPTH: u8 = 4;

/// Whether every font the form `form` uses is embedded, its own and those of the forms it draws.
fn fonts_embedded(doc: &laminapdf_model::Document, form: &Object, depth: u8) -> Result<bool> {
    let Object::Stream(s) = &*doc.deref(form)? else {
        return Ok(true);
    };
    if !crate::annot::form_fonts_embedded(doc, s)? {
        return Ok(false);
    }
    if depth >= MAX_FORM_DEPTH {
        return Ok(true);
    }
    let xobjects = match s.dict.get("Resources") {
        Some(r) => match doc.deref(r)?.as_dict().and_then(|d| d.get("XObject")) {
            Some(x) => doc.deref(x)?.as_dict().cloned().unwrap_or_default(),
            None => return Ok(true),
        },
        None => return Ok(true),
    };
    for (_, x) in xobjects.iter() {
        if !fonts_embedded(doc, x, depth + 1)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether any of `placed` is see-through: faded, or its form draws transparency of its own.
fn see_through(doc: &laminapdf_model::Document, placed: &[place::Placed]) -> Result<bool> {
    for p in placed {
        if p.opacity < 1.0 || form_is_transparent(doc, &p.form_dict)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether an added form draws transparency of its own: an image under it with a soft mask, or
/// a transparency group.
fn form_is_transparent(doc: &laminapdf_model::Document, form: &Dict) -> Result<bool> {
    if form.get("Group").is_some() {
        return Ok(true);
    }
    let resources = match form.get("Resources") {
        Some(r) => doc.deref(r)?.as_dict().cloned().unwrap_or_default(),
        None => return Ok(false),
    };
    let xobjects = match resources.get("XObject") {
        Some(x) => doc.deref(x)?.as_dict().cloned().unwrap_or_default(),
        None => return Ok(false),
    };
    for (_, x) in xobjects.iter() {
        if let Object::Stream(s) = &*doc.deref(x)?
            && (s.dict.get("SMask").is_some() || s.dict.get("Group").is_some())
        {
            return Ok(true);
        }
    }
    Ok(false)
}
