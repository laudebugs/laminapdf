//! A document's comments as an [`Exchange`]: each annotation's dictionary without what names
//! other objects, its reply named by `/NM`, its Popup and its attached file.

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{Annot, AnnotDetails, AnnotId, AnnotSubtype};
use laminapdf_syntax::text::encode_text_string;

use crate::Result;
use crate::exchange::{Comment, Exchange, ExchangedFile, Popup, text};
use crate::map::{IMPORTED, LEFT_OUT};

/// The comments of `doc` on `pages` (every page when `None`), as an exchange made for `file`.
///
/// Only the kinds an import makes are written: links, widgets, sounds, redactions and the rest
/// stay in the PDF. A comment without `/NM` is written with its ID as its name, so its replies
/// can name it.
///
/// # Errors
/// Reading the document.
///
/// Added in 0.7.0.
pub fn exchange_of(doc: &laminapdf_model::Document, pages: Option<&[u32]>, file: Option<&str>) -> Result<Exchange> {
    let mut out = Exchange {
        file: file.map(str::to_owned),
        ids: file_ids(doc),
        ..Exchange::default()
    };
    let all = doc.pages()?;
    let mut per_page = Vec::new();
    for (index, page) in all.iter().enumerate() {
        let index = u32::try_from(index).unwrap_or(u32::MAX);
        if pages.is_some_and(|wanted| !wanted.contains(&index)) {
            continue;
        }
        per_page.push((index, doc.annotations(&page)?));
    }
    // Every annotation's name, so a reply on another page can name its parent. A Popup isn't
    // written, so a reply to one names the comment it pops up for.
    let all_annots = || per_page.iter().flat_map(|(_, annots)| annots);
    let mut names: BTreeMap<String, String> = all_annots()
        .filter(|a| a.subtype != AnnotSubtype::Popup)
        .map(|a| (a.id.to_string(), a.name.clone().unwrap_or_else(|| a.id.to_string())))
        .collect();
    let for_popups: Vec<(String, String)> = all_annots()
        .filter(|a| a.subtype == AnnotSubtype::Popup)
        .filter_map(|p| Some((p.id.to_string(), names.get(&p.parent.as_ref()?.to_string())?.clone())))
        .collect();
    names.extend(for_popups);
    let popups: BTreeMap<String, &Annot> = per_page
        .iter()
        .flat_map(|(_, annots)| annots)
        .filter(|a| a.subtype == AnnotSubtype::Popup)
        .map(|a| (a.id.to_string(), a))
        .collect();
    for (page, annots) in &per_page {
        for annot in annots {
            if let Some(comment) = comment(doc, *page, annot, &names, &popups)? {
                out.comments.push(comment);
            }
        }
    }
    Ok(out)
}

/// The trailer's `/ID`.
/// The document's `/ID`, its permanent and its changing identifier, which XFDF and FDF record
/// so an import can tell when it is given to another PDF.
///
/// Added in 0.7.0.
#[must_use]
pub fn file_ids(doc: &laminapdf_model::Document) -> Option<[Vec<u8>; 2]> {
    match doc.cos().trailer().get("ID")?.as_array()? {
        [Object::String(a), Object::String(b)] => Some([a.bytes.clone(), b.bytes.clone()]),
        _ => None,
    }
}

fn comment(
    doc: &laminapdf_model::Document,
    page: u32,
    annot: &Annot,
    names: &BTreeMap<String, String>,
    popups: &BTreeMap<String, &Annot>,
) -> Result<Option<Comment>> {
    let Some(dict) = dict_of(doc, &annot.id)? else {
        return Ok(None);
    };
    let subtype = dict
        .get_name("Subtype")
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .unwrap_or_default();
    if !IMPORTED.contains(&subtype.as_str()) {
        return Ok(None);
    }
    let mut comment = Comment::new(page, &subtype);
    for (key, value) in dict.iter() {
        if !LEFT_OUT.contains(&String::from_utf8_lossy(key.as_bytes()).as_ref()) {
            comment.entries.insert(key.clone(), value.clone());
        }
    }
    if text(&comment.entries, "NM").is_none()
        && let Some(name) = names.get(&annot.id.to_string())
    {
        comment
            .entries
            .insert("NM", Object::String(PdfString::literal(encode_text_string(name))));
    }
    comment.in_reply_to = annot
        .in_reply_to
        .as_ref()
        .and_then(|parent| names.get(&parent.to_string()).cloned());
    comment.popup = annot
        .popup
        .as_ref()
        .and_then(|p| popups.get(&p.to_string()))
        .and_then(|p| Some(Popup::new(p.rect?, popup_open(doc, &p.id))));
    if let AnnotDetails::FileAttachment {
        file: Some(id),
        name,
        mime,
        description,
        ..
    } = &annot.details
    {
        let Some(bytes) = doc.attachment_bytes(id)? else {
            return Ok(None);
        };
        comment.file = Some(ExchangedFile {
            name: name.clone().unwrap_or_default(),
            mime: mime.clone().unwrap_or_default(),
            bytes: bytes.bytes,
            description: description.clone(),
            created: None,
            modified: None,
        });
    }
    Ok(Some(comment))
}

/// Annotation `id`'s dictionary as the document holds it now.
fn dict_of(doc: &laminapdf_model::Document, id: &AnnotId) -> Result<Option<Dict>> {
    let Some(r) = id.as_lamina_id().obj_ref() else {
        return Ok(None);
    };
    resolved(doc, r)
}

fn resolved(doc: &laminapdf_model::Document, r: ObjRef) -> Result<Option<Dict>> {
    Ok(laminapdf_cos::tolerant(doc.cos().resolve(r))?
        .as_deref()
        .and_then(Object::as_dict)
        .cloned())
}

/// A Popup's `/Open`.
fn popup_open(doc: &laminapdf_model::Document, id: &AnnotId) -> bool {
    matches!(
        dict_of(doc, id).ok().flatten().and_then(|d| d.get("Open").cloned()),
        Some(Object::Bool(true))
    )
}
