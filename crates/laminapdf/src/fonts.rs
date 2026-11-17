//! The fonts a document's pages use, for a properties view and the inspector: what
//! `laminapdf-model` reads of each (type, embedded or subset, encoding, ToUnicode), and with the
//! `extract` feature how much of the font its ToUnicode CMap covers (ISO 32000-2 §9.10.3).

use std::collections::BTreeSet;

use laminapdf_model::FontResource;

use crate::document::Document;
use crate::error::Result;
use crate::properties::{FontSummary, ToUnicodeCoverage};

impl Document {
    /// The fonts the pages `pages` use (0-based; `None` for every page), once each in the order
    /// first met: their resources', the forms, patterns and Type 3 fonts those reach, and their
    /// annotations' appearances'.
    ///
    /// ```
    /// # use laminapdf::{Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/reportlab.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// for font in doc.fonts(None)? {
    ///     println!("{:?} {:?} embedded: {}", font.name, font.kind, font.embedded);
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page the document doesn't have; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn fonts(&self, pages: Option<&[u32]>) -> Result<Vec<FontSummary>> {
        let model = self.ops();
        let all = model.pages()?;
        let resources = match pages {
            None => all.fonts()?,
            Some(indexes) => {
                let count = self.page_count()?;
                let mut seen = BTreeSet::new();
                let mut out = Vec::new();
                for &index in indexes {
                    let page = usize::try_from(index)
                        .ok()
                        .and_then(|i| all.get(i))
                        .ok_or(crate::Error::PageIndexOutOfRange { index, count })?;
                    for font in page.fonts()? {
                        let key = (font.object, font.subtype.clone(), font.name.clone());
                        if seen.insert(key) {
                            out.push(font);
                        }
                    }
                }
                out
            }
        };
        resources.into_iter().map(|font| self.font_summary(font)).collect()
    }

    fn font_summary(&self, font: FontResource) -> Result<FontSummary> {
        let to_unicode_coverage = if font.to_unicode {
            self.to_unicode_coverage(&font)?
        } else {
            None
        };
        Ok(FontSummary {
            name: font.name,
            kind: font.subtype,
            embedded: font.embedded,
            subset: font.subset,
            encoding: font.encoding,
            object: font.object,
            to_unicode: font.to_unicode,
            to_unicode_coverage,
        })
    }

    #[cfg(not(feature = "extract"))]
    #[expect(
        clippy::unused_self,
        clippy::unnecessary_wraps,
        reason = "the extract feature's signature"
    )]
    fn to_unicode_coverage(&self, _font: &FontResource) -> Result<Option<ToUnicodeCoverage>> {
        Ok(None)
    }

    /// The codes `font` defines and how many its ToUnicode CMap maps; `None` for a direct
    /// font or one whose codes aren't known.
    #[cfg(feature = "extract")]
    fn to_unicode_coverage(&self, font: &FontResource) -> Result<Option<ToUnicodeCoverage>> {
        use laminapdf_cos::{Object, tolerant};
        let model = self.ops();
        let Some(object) = font.object else { return Ok(None) };
        let Some(dict) = tolerant(model.resolve(object))? else {
            return Ok(None);
        };
        let Some(dict) = dict.as_dict() else { return Ok(None) };
        let Some(cmap) = dict.get("ToUnicode") else {
            return Ok(None);
        };
        let Some(cmap) = tolerant(model.deref(cmap))? else {
            return Ok(None);
        };
        let Object::Stream(cmap) = &*cmap else { return Ok(None) };
        let Some(source) = tolerant(model.decode_stream(cmap))? else {
            return Ok(None);
        };
        let codes = match font.subtype.as_deref() {
            Some("Type0") => cid_codes(model, dict)?,
            _ => simple_codes(model, dict)?,
        };
        let Some(codes) = codes else { return Ok(None) };
        let count = u32::try_from(codes.len()).unwrap_or(u32::MAX);
        let mapped = laminapdf_text::to_unicode_coverage(&source, codes);
        Ok(Some(ToUnicodeCoverage { mapped, codes: count }))
    }
}

/// The most codes counted for one font: a two-byte font defines at most 65,536.
#[cfg(feature = "extract")]
const MAX_CODES: usize = 1 << 16;

/// A simple font's one-byte codes from `/FirstChar` to `/LastChar` whose `/Widths` entry isn't
/// 0 (§9.6.2.1); `None` without `/FirstChar` or `/Widths`.
#[cfg(feature = "extract")]
fn simple_codes(
    model: &laminapdf_ops::Document,
    font: &laminapdf_cos::Dict,
) -> Result<Option<Vec<laminapdf_text::Code>>> {
    let Some(first) = font.get_int("FirstChar").and_then(|n| u32::try_from(n).ok()) else {
        return Ok(None);
    };
    let Some(widths) = font.get("Widths") else {
        return Ok(None);
    };
    let Some(widths) = laminapdf_cos::tolerant(model.deref(widths))? else {
        return Ok(None);
    };
    let Some(widths) = widths.as_array() else {
        return Ok(None);
    };
    let codes = widths
        .iter()
        .zip(first..=0xff)
        .filter(|(width, _)| width.as_f64().is_some_and(|w| w != 0.0))
        .map(|(_, value)| laminapdf_text::Code { value, len: 1 })
        .collect();
    Ok(Some(codes))
}

/// A Type 0 font's codes when its encoding is `Identity-H` or `Identity-V` (code = CID): the
/// CIDs its descendant's `/W` array lists (§9.7.4.3) but CID 0, `.notdef`, which stands for no
/// character; `None` for any other encoding.
#[cfg(feature = "extract")]
fn cid_codes(model: &laminapdf_ops::Document, font: &laminapdf_cos::Dict) -> Result<Option<Vec<laminapdf_text::Code>>> {
    use laminapdf_cos::{Object, tolerant};
    if !matches!(font.get_name("Encoding"), Some(b"Identity-H" | b"Identity-V")) {
        return Ok(None);
    }
    let resolve = |value: &Object| -> Result<Option<std::sync::Arc<Object>>> {
        Ok(tolerant(model.deref(value))?.map(laminapdf_cos::Resolved::into_arc))
    };
    let Some(descendants) = font.get("DescendantFonts").map(resolve).transpose()?.flatten() else {
        return Ok(None);
    };
    let Some(first) = descendants.as_array().and_then(<[Object]>::first) else {
        return Ok(None);
    };
    let Some(cid_font) = resolve(first)? else {
        return Ok(None);
    };
    let Some(widths) = cid_font.as_dict().and_then(|d| d.get("W")) else {
        return Ok(None);
    };
    let Some(widths) = resolve(widths)? else {
        return Ok(None);
    };
    let Some(items) = widths.as_array() else {
        return Ok(None);
    };
    let mut cids = BTreeSet::new();
    let mut at = 0;
    while let Some(start) = items.get(at).and_then(Object::as_int) {
        let start = u32::try_from(start).unwrap_or(u32::MAX);
        match (items.get(at + 1), items.get(at + 2)) {
            // `c [w1 w2 …]`: consecutive CIDs from c.
            (Some(list), _) if resolve(list)?.and_then(|l| l.as_array().map(<[Object]>::len)).is_some() => {
                let len = resolve(list)?
                    .and_then(|l| l.as_array().map(<[Object]>::len))
                    .unwrap_or(0);
                cids.extend((0..u32::try_from(len).unwrap_or(0)).map(|i| start.saturating_add(i)));
                at += 2;
            }
            // `first last w`: every CID from first to last.
            (Some(last), Some(_)) => {
                let last = last.as_int().and_then(|n| u32::try_from(n).ok()).unwrap_or(start);
                cids.extend(start..=last.min(start.saturating_add(u32::try_from(MAX_CODES).unwrap_or(u32::MAX))));
                at += 3;
            }
            _ => break,
        }
        if cids.len() >= MAX_CODES {
            break;
        }
    }
    Ok(Some(
        cids.into_iter()
            .filter(|&cid| cid != 0 && cid <= 0xffff)
            .map(|value| laminapdf_text::Code { value, len: 2 })
            .collect(),
    ))
}
