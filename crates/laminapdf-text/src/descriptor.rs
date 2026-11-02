//! The `FontDescriptor` (ISO 32000-2 §9.8) of every font Lamina PDF writes: embedded Type 0
//! fonts and the standard 14.

use laminapdf_cos::{Dict, Name, Object};

use crate::embed::subset::round_i64;
use crate::font::FontInfo;

/// The entries every descriptor shares, in thousandths of an em: `/FontName`, `/Flags`,
/// `/FontBBox`, `/ItalicAngle`, `/Ascent`, `/Descent`, `/CapHeight` and `/XHeight`. `bbox`
/// replaces the font's own when given: a variable-font instance's box, from its instanced
/// outlines and already in thousandths (`head` is not varied). The caller adds the stems and
/// the font file.
pub(crate) fn descriptor(info: &FontInfo, font_name: Name, bbox: Option<[f32; 4]>) -> Dict {
    let scale = 1000.0 / f64::from(info.units_per_em.max(1));
    let units = |v: f32| Object::Integer(round_i64(f64::from(v) * scale));
    let bbox: Vec<Object> = match bbox {
        Some(b) => b.iter().map(|&v| Object::Integer(round_i64(f64::from(v)))).collect(),
        None => info.bbox.iter().map(|&v| units(v)).collect(),
    };
    let mut d = Dict::new();
    d.insert("Type", Name::from("FontDescriptor"));
    d.insert("FontName", font_name);
    d.insert("Flags", i64::from(info.flags));
    d.insert("FontBBox", Object::Array(bbox));
    d.insert("ItalicAngle", Object::rounded(f64::from(info.italic_angle), 2));
    d.insert("Ascent", units(info.ascent));
    d.insert("Descent", units(info.descent));
    // §9.8.1 Table 120: `/CapHeight` is required for a font with Latin characters. A font that
    // doesn't record it (no OS/2 cap height; the Symbol and ZapfDingbats AFMs) gets its ascent,
    // a safe stand-in, so no Latin font goes without; for other fonts the entry is harmless.
    d.insert("CapHeight", units(info.cap_height.unwrap_or(info.ascent)));
    if let Some(x_height) = info.x_height {
        d.insert("XHeight", units(x_height));
    }
    d
}
