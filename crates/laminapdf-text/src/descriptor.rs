//! The `FontDescriptor` (ISO 32000-2 §9.8) of every font Lamina PDF writes: embedded Type 0
//! fonts and the standard 14.

use laminapdf_cos::{Dict, Name, Object};

use crate::font::FontInfo;

/// The entries every descriptor shares: `/Type`, `/FontName`, `/Flags`, `/ItalicAngle`, and
/// the metrics `/FontBBox`, `/Ascent`, `/Descent`, `/CapHeight` and `/XHeight` in thousandths
/// of an em. `bbox` replaces the font's own box when given: a variable-font instance's, from
/// its instanced outlines and already in thousandths (`head` is not varied). The caller adds
/// the stems and the font file.
pub(crate) fn descriptor(info: &FontInfo, font_name: Name, bbox: Option<[f32; 4]>) -> Dict {
    let scale = 1000.0 / f64::from(info.units_per_em.max(1));
    let units = |v: f32| Object::rounded(f64::from(v) * scale, 0);
    let bbox: Vec<Object> = match bbox {
        Some(b) => b.iter().map(|&v| Object::rounded(f64::from(v), 0)).collect(),
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
    // doesn't record a cap height (no OS/2 value, or the Symbol and ZapfDingbats AFMs) gets
    // its ascent instead, one rule for every font. A non-Latin font loses nothing by having it.
    d.insert("CapHeight", units(info.cap_height.unwrap_or(info.ascent)));
    if let Some(x_height) = info.x_height {
        d.insert("XHeight", units(x_height));
    }
    d
}

#[cfg(test)]
mod tests {
    use super::descriptor;
    use crate::font::Font;
    use crate::std14::Standard14;
    use laminapdf_cos::{Name, Object};

    #[test]
    fn a_font_without_a_cap_height_gets_its_ascent() {
        let font = Font::standard(Standard14::Symbol);
        let d = descriptor(font.info(), Name::from("Symbol"), None);
        assert_eq!(d.get("CapHeight"), d.get("Ascent"));
    }

    #[test]
    fn a_given_bbox_replaces_the_fonts_own() {
        let font = Font::standard(Standard14::Helvetica);
        let d = descriptor(font.info(), Name::from("Helvetica"), Some([0.4, -1.6, 500.0, 700.5]));
        assert_eq!(
            d.get("FontBBox"),
            Some(&Object::Array(vec![0.into(), (-2).into(), 500.into(), 701.into()]))
        );
    }
}
