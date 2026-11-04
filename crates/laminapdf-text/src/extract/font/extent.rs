//! A font's vertical extent (ascent and descent) for glyph quads, with the defaults when the
//! descriptor has none.

use laminapdf_cos::Result;
use laminapdf_syntax::Dict;

use super::{Reader, name_of};
use crate::Standard14;

/// The default vertical metrics of a CIDFont, `/DW2 [880 −1000]` (§9.7.4.3).
pub(super) const DW2: (f64, f64) = (880.0, -1000.0);

/// The extent used when a font gives none (thousandths of text space): about what common
/// Latin fonts have.
pub(super) const DEFAULT_EXTENT: (f64, f64) = (800.0, -200.0);

/// A font's ascent and descent in thousandths of text space, for glyph boxes: Type 3's
/// `/FontBBox` through `/FontMatrix` (from `dict`); else the descriptor's `/Ascent` and
/// `/Descent`, else its `/FontBBox`; else a standard 14 font's AFM values; else
/// [`DEFAULT_EXTENT`]. A positive descent (a sign error some producers make) counts as
/// negative. (pdf.js and PDFium differ here, so the quad tests compare within a tolerance.)
pub(super) fn extent(
    reader: &mut Reader<'_>,
    dict: &Dict,
    descriptor: Option<&Dict>,
    standard: Option<Standard14>,
) -> Result<(f64, f64)> {
    let valid = |ascent: f64, descent: f64| {
        let descent = -descent.abs();
        (ascent > 0.0 && ascent.is_finite() && descent.is_finite()).then_some((ascent, descent))
    };
    let bbox = |numbers: &[f64]| match *numbers {
        [_, y0, _, y1] => valid(y1.max(y0), y0.min(y1).min(0.0)),
        _ => None,
    };
    if name_of(dict.get("Subtype")) == Some(b"Type3") {
        let matrix = match reader.get(dict, "FontMatrix")? {
            Some(m) => reader.numbers(&m)?,
            None => Vec::new(),
        };
        let scale = matrix.get(3).copied().unwrap_or(0.001) * 1000.0;
        let numbers = match reader.get(dict, "FontBBox")? {
            Some(b) => reader.numbers(&b)?,
            None => Vec::new(),
        };
        let scaled: Vec<f64> = numbers.iter().map(|n| n * scale).collect();
        return Ok(bbox(&scaled).unwrap_or(DEFAULT_EXTENT));
    }
    if let Some(descriptor) = descriptor {
        let ascent = reader.number(descriptor, "Ascent")?;
        let descent = reader.number(descriptor, "Descent")?;
        if let Some(found) = ascent.zip(descent).and_then(|(a, d)| valid(a, d)) {
            return Ok(found);
        }
        if let Some(b) = reader.get(descriptor, "FontBBox")?
            && let Some(found) = bbox(&reader.numbers(&b)?)
        {
            return Ok(found);
        }
    }
    if let Some(font) = standard {
        let m = font.metrics();
        let afm = m.ascender.zip(m.descender).map(|(a, d)| (f64::from(a), f64::from(d)));
        let from_bbox = bbox(&m.bbox.map(f64::from));
        if let Some(found) = afm.and_then(|(a, d)| valid(a, d)).or(from_bbox) {
            return Ok(found);
        }
    }
    Ok(DEFAULT_EXTENT)
}
