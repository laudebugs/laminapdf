//! Fonts as text extraction reads them (ISO 32000-2 §9.5–9.10; research 03 §3–5): how a string
//! splits into codes, how far each code moves the pen, and what text it stands for.
//!
//! Unicode is resolved in this order (§9.10.2, research 03 §8.1):
//!
//! 1. the font's ToUnicode CMap;
//! 2. simple fonts: the code's glyph name (`/Encoding`, its `/Differences`, else the built-in or
//!    standard encoding) through the AGL ([`crate::agl`]: `uniXXXX`, `uXXXX[XX]`, `a.sc`,
//!    `f_f_i`); composite fonts with a predefined `Uni…-UCS2-…`/`Uni…-UTF16-…` CMap: the code
//!    itself, which is UTF-16;
//! 3. an embedded TrueType/OpenType program's `cmap`, read backwards from the glyph;
//! 4. nothing: the caller extracts U+FFFD and raises `unmapped-glyph`.
//!
//! Character-collection CMaps (`Adobe-Japan1-UCS2` and the rest) are not bundled, and Type 1
//! programs are not parsed for their built-in encodings: such fonts need a ToUnicode CMap.

mod composite;
mod extent;
mod reader;
mod simple;
use composite::{CidEncoding, Composite};
use extent::{DEFAULT_EXTENT, DW2, extent};
use reader::{Reader, name_of, whole};
use simple::{Simple, fixed, glyph_text};

use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf_cos::{Document, Result, Warning};
use laminapdf_syntax::Dict;

use super::cmap::{CMap, Code};

/// How many `usecmap` / `/UseCMap` links are followed from a font's CMap (TN #5014 allows
/// one; files nest a few). Further links are dropped with `cmap-invalid`.
pub const MAX_USECMAP_DEPTH: usize = 8;

/// A font loaded for extraction.
#[derive(Debug)]
pub(crate) struct ExtractFont {
    kind: Kind,
    to_unicode: Option<CMap>,
    /// `/BaseFont` (the font's name), read lossily; shared by every glyph shown in the font.
    pub(crate) name: Arc<str>,
    /// Ascent and descent in thousandths of text space, for glyph boxes.
    pub(crate) extent: (f64, f64),
}

#[derive(Debug)]
enum Kind {
    Simple(Simple),
    Composite(Box<Composite>),
}

/// `/W` or `/W2`: CID ranges with one value each, or a run of values from a first CID.
#[derive(Debug)]
struct Widths<T> {
    ranges: BTreeMap<u32, (u32, Run<T>)>,
}

#[derive(Debug)]
enum Run<T> {
    Same(T),
    Each(Vec<T>),
}

impl<T> Default for Widths<T> {
    fn default() -> Self {
        Self {
            ranges: BTreeMap::new(),
        }
    }
}

impl<T: Copy> Widths<T> {
    fn get(&self, cid: u32) -> Option<T> {
        let (&lo, (hi, run)) = self.ranges.range(..=cid).next_back()?;
        if cid > *hi {
            return None;
        }
        match run {
            Run::Same(value) => Some(*value),
            Run::Each(values) => values.get(usize::try_from(cid - lo).ok()?).copied(),
        }
    }
}

/// Problems met while loading, reported once each by the caller.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Issues {
    pub(crate) font: bool,
    pub(crate) cmap: bool,
}

impl ExtractFont {
    /// Loads the font dictionary `dict`. Never fails on a malformed font (it decodes with
    /// defaults and returns [`Issues`]); fails only on hard limits and in strict mode.
    pub(crate) fn load(doc: &Document, dict: &Dict) -> Result<(Self, Issues)> {
        let mut reader = Reader {
            doc,
            issues: Issues::default(),
        };
        let to_unicode = match reader.get(dict, "ToUnicode")? {
            Some(object) => match reader.stream(&object)? {
                Some(bytes) => {
                    let cmap = CMap::parse(&bytes);
                    reader.issues.cmap |= cmap.skipped() > 0;
                    Some(cmap)
                }
                // A name here (`/Identity-H`, written by some producers) maps nothing.
                None => None,
            },
            None => None,
        };
        let composite = name_of(dict.get("Subtype")) == Some(b"Type0");
        let kind = if composite {
            Kind::Composite(Box::new(Composite::load(&mut reader, dict)?))
        } else {
            Kind::Simple(Simple::load(&mut reader, dict)?)
        };
        let extent = match &kind {
            Kind::Simple(s) => s.extent,
            Kind::Composite(c) => c.extent,
        };
        let name = Arc::from(String::from_utf8_lossy(
            name_of(dict.get("BaseFont")).unwrap_or_default(),
        ));
        Ok((
            Self {
                kind,
                to_unicode,
                name,
                extent,
            },
            reader.issues,
        ))
    }

    /// The position vector `(vx, vy)` of a glyph in vertical writing, thousandths of text
    /// space (§9.7.4.3): from `/W2`, else half the horizontal width and `/DW2`'s first number.
    pub(crate) fn position_vector(&self, code: Code) -> (f64, f64) {
        match &self.kind {
            Kind::Simple(_) => (0.0, 0.0),
            Kind::Composite(c) => {
                let cid = c.cid(code);
                c.vertical_widths.get(cid).map_or_else(
                    || (c.widths.get(cid).unwrap_or(c.default_width) / 2.0, c.default_vertical.0),
                    |(_, vx, vy)| (vx, vy),
                )
            }
        }
    }

    /// Whether the font writes vertically (`/WMode 1`, `Identity-V`).
    pub(crate) fn vertical(&self) -> bool {
        matches!(&self.kind, Kind::Composite(c) if c.vertical)
    }

    /// The first code of `bytes` and how many bytes it took.
    pub(crate) fn next_code(&self, bytes: &[u8]) -> Option<(Code, usize)> {
        match &self.kind {
            Kind::Simple(_) => {
                let value = u32::from(*bytes.first()?);
                Some((Code { value, len: 1 }, 1))
            }
            Kind::Composite(c) => match &c.encoding {
                CidEncoding::Identity => fixed(bytes, 2),
                CidEncoding::Utf16 => {
                    let high = bytes.first().is_some_and(|b| (0xD8..=0xDB).contains(b));
                    fixed(bytes, if high && bytes.len() >= 4 { 4 } else { 2 })
                }
                CidEncoding::Embedded(cmap) => cmap.next_code(bytes, 2),
            },
        }
    }

    /// The text `code` stands for (see the module docs for the order); `None` when nothing
    /// maps it. An empty string is a mapping too: a glyph that stands for no text.
    pub(crate) fn unicode(&self, code: Code) -> Option<String> {
        if let Some(text) = self.to_unicode.as_ref().and_then(|cmap| cmap.unicode(code)) {
            return Some(text);
        }
        match &self.kind {
            Kind::Simple(s) => s.unicode(code),
            Kind::Composite(c) => c.unicode(code),
        }
    }

    /// The glyph's displacement `(w0, w1)` in thousandths of text space (§9.4.4): `w0` for
    /// horizontal writing, `w1` for vertical.
    pub(crate) fn displacement(&self, code: Code) -> (f64, f64) {
        match &self.kind {
            Kind::Simple(s) => (
                usize::try_from(code.value)
                    .ok()
                    .and_then(|i| s.widths.get(i))
                    .copied()
                    .unwrap_or(0.0),
                0.0,
            ),
            Kind::Composite(c) => {
                let cid = c.cid(code);
                let w0 = c.widths.get(cid).unwrap_or(c.default_width);
                let w1 = c.vertical_widths.get(cid).map_or(c.default_vertical.1, |(w1, _, _)| w1);
                (w0, w1)
            }
        }
    }

    /// Whether `Tw` applies to `code`: the single-byte code 32 (§9.3.3).
    pub(crate) const fn is_word_space(code: Code) -> bool {
        code.len == 1 && code.value == 32
    }
}

/// Fonts already loaded in one document, by font object: each is parsed once.
#[derive(Debug, Default)]
pub(crate) struct FontCache {
    fonts: BTreeMap<laminapdf_syntax::ObjRef, Arc<ExtractFont>>,
    /// Direct font dictionaries (no object to key by), compared by value: their streams are
    /// indirect, so a comparison is cheap. At most [`DIRECT_FONTS`].
    direct: Vec<(Dict, Arc<ExtractFont>)>,
}

/// Direct font dictionaries cached per extractor; further ones are loaded on each use.
const DIRECT_FONTS: usize = 64;

impl FontCache {
    /// The font of `dict` (object `object`, when indirect), loading it on first use and
    /// reporting its issues once, on the document.
    pub(crate) fn get(
        &mut self,
        doc: &Document,
        object: Option<laminapdf_syntax::ObjRef>,
        dict: &Dict,
    ) -> Result<Arc<ExtractFont>> {
        if let Some(font) = object.and_then(|r| self.fonts.get(&r)) {
            return Ok(Arc::clone(font));
        }
        if object.is_none()
            && let Some((_, font)) = self.direct.iter().find(|(d, _)| d == dict)
        {
            return Ok(Arc::clone(font));
        }
        let (font, issues) = ExtractFont::load(doc, dict)?;
        let num = object.map(|r| r.num);
        if issues.font {
            doc.report(Warning::FontInvalid { object: num })?;
        }
        if issues.cmap {
            doc.report(Warning::CMapInvalid { object: num })?;
        }
        let font = Arc::new(font);
        match object {
            Some(r) => {
                self.fonts.insert(r, Arc::clone(&font));
            }
            None if self.direct.len() < DIRECT_FONTS => self.direct.push((dict.clone(), Arc::clone(&font))),
            None => {}
        }
        Ok(font)
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::{Document, OpenOptions};
    use laminapdf_syntax::{Dict, Name, Object, Stream, parse_object};

    use super::simple::standard_font;
    use super::{Code, ExtractFont, Issues, MAX_USECMAP_DEPTH};
    use crate::Standard14;

    fn dict(source: &str) -> Dict {
        match parse_object(source.as_bytes()) {
            Ok(Object::Dict(d)) => d,
            other => panic!("not a dictionary: {other:?}"),
        }
    }

    fn stream(dict_source: &str, data: &[u8]) -> Object {
        Object::Stream(Stream::new(dict(dict_source), data.to_vec()))
    }

    fn load(font: &Dict) -> (ExtractFont, Issues) {
        let doc = Document::empty(None, &OpenOptions::default());
        ExtractFont::load(&doc, font).expect("loads")
    }

    fn text(font: &ExtractFont, bytes: &[u8]) -> Vec<Option<String>> {
        let mut rest = bytes;
        let mut out = Vec::new();
        while let Some((code, used)) = font.next_code(rest) {
            out.push(font.unicode(code));
            rest = &rest[used..];
        }
        out
    }

    const fn byte(value: u32) -> Code {
        Code { value, len: 1 }
    }

    const fn two(value: u32) -> Code {
        Code { value, len: 2 }
    }

    fn type0(encoding: &str, descendant: &str) -> Dict {
        dict(&format!(
            "<< /Type /Font /Subtype /Type0 /BaseFont /F /Encoding {encoding} /DescendantFonts [{descendant}] >>"
        ))
    }

    #[test]
    fn differences_override_the_base_encoding_and_go_through_the_agl() {
        let (font, issues) = load(&dict(
            "<< /Subtype /Type1 /BaseFont /Helvetica /Encoding << /BaseEncoding /WinAnsiEncoding
               /Differences [128 /Lslash /uni0105 39 /quoteright] >> >>",
        ));
        assert_eq!(
            (text(&font, &[0x41, 0x80, 0x81, 0x27, 0xE9]), issues.font),
            (
                vec![
                    Some("A".into()),
                    Some("Ł".into()),
                    Some("ą".into()),
                    Some("’".into()),
                    Some("é".into())
                ],
                false
            )
        );
    }

    #[test]
    fn standard_14_fonts_without_widths_use_the_afm_metrics() {
        let (font, _) = load(&dict(
            "<< /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        ));
        assert_eq!(
            [font.displacement(byte(0x57)), font.displacement(byte(0x20))],
            [(944.0, 0.0), (278.0, 0.0)]
        );
    }

    #[test]
    fn widths_cover_their_codes_and_missing_width_the_rest() {
        let mut font = dict(
            "<< /Subtype /TrueType /BaseFont /Arial /FirstChar 65 /Widths [500 600]
               /FontDescriptor << /Flags 32 /MissingWidth 250 >> >>",
        );
        font.insert("Encoding", Object::Name(Name::from("WinAnsiEncoding")));
        let (font, _) = load(&font);
        assert_eq!(
            [byte(65), byte(66), byte(0x57)].map(|c| font.displacement(c).0),
            [500.0, 600.0, 250.0]
        );
    }

    #[test]
    fn a_nonsymbolic_font_without_encoding_reads_standard_encoding() {
        let (font, _) = load(&dict("<< /Subtype /Type1 /BaseFont /Unknown >>"));
        assert_eq!(text(&font, &[0x27, 0x41]), [Some("’".into()), Some("A".into())]);
    }

    #[test]
    fn arbitrary_glyph_names_map_to_nothing() {
        let (font, _) = load(&dict(
            "<< /Subtype /Type1 /BaseFont /X /Encoding << /Differences [1 /g37] >> >>",
        ));
        assert_eq!(text(&font, &[1]), [None]);
    }

    #[test]
    fn zapf_dingbats_names_use_their_own_list() {
        let (font, _) = load(&dict("<< /Subtype /Type1 /BaseFont /ZapfDingbats >>"));
        assert_eq!(text(&font, &[0x34]), [Some("✔".into())]);
    }

    #[test]
    fn type3_widths_go_through_the_font_matrix() {
        let (font, _) = load(&dict(
            "<< /Subtype /Type3 /FontMatrix [0.01 0 0 0.01 0 0] /FirstChar 1 /Widths [50]
               /Encoding << /Differences [1 /a] >> >>",
        ));
        assert_eq!(
            (font.displacement(byte(1)), text(&font, &[1])),
            ((500.0, 0.0), vec![Some("a".into())])
        );
    }

    #[test]
    fn to_unicode_wins_over_the_encoding() {
        let mut font = dict("<< /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>");
        font.insert("ToUnicode", stream("<< >>", b"1 beginbfchar <41> <0042> endbfchar"));
        let (font, _) = load(&font);
        assert_eq!(text(&font, b"AC"), [Some("B".into()), Some("C".into())]);
    }

    #[test]
    fn identity_h_widths_come_from_w_runs_ranges_and_dw() {
        let (font, issues) = load(&type0(
            "/Identity-H",
            "<< /Subtype /CIDFontType2 /DW 400 /W [1 [500 600] 10 20 700] >>",
        ));
        assert_eq!(
            (
                [1, 2, 15, 30].map(|cid| font.displacement(two(cid)).0),
                font.vertical(),
                issues.font
            ),
            ([500.0, 600.0, 700.0, 400.0], false, false)
        );
    }

    #[test]
    fn identity_v_writes_vertically_with_w2_and_dw2() {
        let (font, _) = load(&type0(
            "/Identity-V",
            "<< /Subtype /CIDFontType0 /W2 [5 [-900 500 800]] /DW2 [800 -1000] >>",
        ));
        assert_eq!(
            (
                font.vertical(),
                font.displacement(two(5)).1,
                font.displacement(two(6)).1
            ),
            (true, -900.0, -1000.0)
        );
    }

    #[test]
    fn ucs2_cmaps_read_codes_as_utf16() {
        let (font, issues) = load(&type0("/UniGB-UCS2-H", "<< /Subtype /CIDFontType0 >>"));
        assert_eq!(
            (text(&font, &[0x4F, 0x60, 0x59, 0x7D]), issues.cmap),
            (vec![Some("你".into()), Some("好".into())], false)
        );
    }

    #[test]
    fn utf16_cmaps_read_surrogate_pairs_as_one_code() {
        let (font, _) = load(&type0("/UniJIS-UTF16-H", "<< /Subtype /CIDFontType0 >>"));
        assert_eq!(text(&font, &[0xD8, 0x40, 0xDC, 0x0B]), [Some("\u{2000B}".into())]);
    }

    #[test]
    fn other_predefined_cmaps_are_an_issue_and_read_as_identity() {
        let (font, issues) = load(&type0("/90ms-RKSJ-H", "<< /Subtype /CIDFontType0 >>"));
        assert_eq!(
            (issues.cmap, font.next_code(&[1, 2, 3]).map(|(c, n)| (c.value, n))),
            (true, Some((0x0102, 2)))
        );
    }

    #[test]
    fn an_embedded_cmap_splits_codes_and_maps_cids() {
        let mut font = type0("null", "<< /Subtype /CIDFontType2 /W [500 [250]] >>");
        font.insert(
            "Encoding",
            stream(
                "<< /Type /CMap >>",
                b"/WMode 1 def 2 begincodespacerange <00> <7F> <8000> <FFFF> endcodespacerange
                  1 begincidrange <8000> <80FF> 400 endcidrange",
            ),
        );
        let (font, _) = load(&font);
        let (code, used) = font.next_code(&[0x80, 0x64]).expect("a code");
        assert_eq!(
            (code, used, font.displacement(code).0, font.vertical()),
            (two(0x8064), 2, 250.0, true)
        );
    }

    /// A Type 0 font whose CMap has a `/UseCMap` chain `depth` streams deep below it.
    fn chained(depth: usize) -> Dict {
        let mut parent = stream("<< >>", b"1 begincidchar <0001> 77 endcidchar");
        for _ in 0..depth {
            let Object::Stream(mut s) = stream("<< >>", b"1 begincodespacerange <0000> <FFFF> endcodespacerange")
            else {
                unreachable!()
            };
            s.dict.insert("UseCMap", parent);
            parent = Object::Stream(s);
        }
        let mut font = type0("null", "<< /Subtype /CIDFontType2 /W [77 [123]] >>");
        font.insert("Encoding", parent);
        font
    }

    #[test]
    fn usecmap_chains_are_followed_to_the_depth_limit() {
        let (font, issues) = load(&chained(MAX_USECMAP_DEPTH));
        assert_eq!((font.displacement(two(1)).0, issues.cmap), (123.0, false));
    }

    #[test]
    fn usecmap_chains_past_the_depth_limit_are_cut() {
        let (font, issues) = load(&chained(MAX_USECMAP_DEPTH + 1));
        assert_eq!((font.displacement(two(1)).0, issues.cmap), (1000.0, true));
    }

    #[test]
    fn a_predefined_use_cmap_in_the_dictionary_is_inherited_with_its_wmode() {
        let mut font = type0("null", "<< /Subtype /CIDFontType2 /W [9 [300]] >>");
        font.insert(
            "Encoding",
            stream(
                "<< /Type /CMap /UseCMap /Identity-V >>",
                b"1 begincidchar <0001> 9 endcidchar",
            ),
        );
        let (font, issues) = load(&font);
        assert_eq!(
            (
                font.displacement(two(1)).0,
                font.displacement(two(9)).0,
                font.vertical(),
                issues.cmap
            ),
            (300.0, 300.0, true, false)
        );
    }

    #[test]
    fn the_cmap_dictionarys_wmode_applies_when_the_source_has_none() {
        let mut font = type0("null", "<< /Subtype /CIDFontType2 >>");
        font.insert(
            "Encoding",
            stream(
                "<< /Type /CMap /WMode 1 >>",
                b"1 begincodespacerange <0000> <FFFF> endcodespacerange",
            ),
        );
        assert!(load(&font).0.vertical());
    }

    #[test]
    fn a_malformed_w_array_is_an_issue_and_keeps_what_parsed() {
        let (font, issues) = load(&type0("/Identity-H", "<< /Subtype /CIDFontType2 /W [1 [500] 2 /x] >>"));
        assert_eq!((font.displacement(two(1)).0, issues.font), (500.0, true));
    }

    #[test]
    fn a_type0_font_without_descendant_is_an_issue_with_default_widths() {
        let (font, issues) = load(&dict("<< /Subtype /Type0 /Encoding /Identity-H >>"));
        assert_eq!((font.displacement(two(9)).0, issues.font), (1000.0, true));
    }

    #[test]
    fn a_malformed_to_unicode_is_an_issue_and_keeps_its_good_entries() {
        let mut font = dict("<< /Subtype /Type1 /BaseFont /Helvetica >>");
        font.insert(
            "ToUnicode",
            stream("<< >>", b"2 beginbfchar <41> <0042> <42> 7 endbfchar"),
        );
        let (font, issues) = load(&font);
        assert_eq!((text(&font, b"A"), issues.cmap), (vec![Some("B".into())], true));
    }

    #[test]
    fn extents_come_from_the_descriptor_then_its_bbox_then_the_afm() {
        let ascent = |source: &str| load(&dict(source)).0.extent;
        assert_eq!(
            [
                ascent("<< /Subtype /TrueType /BaseFont /X /FontDescriptor << /Ascent 900 /Descent -100 >> >>"),
                ascent(
                    "<< /Subtype /TrueType /BaseFont /X /FontDescriptor << /Ascent 0 /Descent 0 /FontBBox [0 -300 1000 950] >> >>"
                ),
                ascent("<< /Subtype /Type1 /BaseFont /Helvetica >>"),
                ascent("<< /Subtype /Type1 /BaseFont /Unknown >>"),
                ascent("<< /Subtype /Type3 /FontMatrix [0.01 0 0 0.01 0 0] /FontBBox [0 -10 100 90] >>"),
            ],
            [
                (900.0, -100.0),
                (950.0, -300.0),
                (718.0, -207.0),
                (800.0, -200.0),
                (900.0, -100.0)
            ]
        );
    }

    #[test]
    fn a_positive_descent_counts_as_negative() {
        let (font, _) = load(&dict(
            "<< /Subtype /TrueType /BaseFont /X /FontDescriptor << /Ascent 900 /Descent 200 >> >>",
        ));
        assert_eq!(font.extent, (900.0, -200.0));
    }

    #[test]
    fn a_type0_extent_comes_from_its_descendants_descriptor() {
        let (font, _) = load(&type0(
            "/Identity-H",
            "<< /Subtype /CIDFontType2 /FontDescriptor << /Ascent 1069 /Descent -293 >> >>",
        ));
        assert_eq!((font.extent, &*font.name), ((1069.0, -293.0), "F"));
    }

    #[test]
    fn position_vectors_come_from_w2_else_half_the_width_and_dw2() {
        let (font, _) = load(&type0(
            "/Identity-V",
            "<< /Subtype /CIDFontType0 /W [6 [600]] /W2 [5 [-900 500 800]] /DW2 [870 -1000] >>",
        ));
        assert_eq!(
            [font.position_vector(two(5)), font.position_vector(two(6))],
            [(500.0, 800.0), (300.0, 870.0)]
        );
    }

    #[test]
    fn word_spacing_applies_to_the_single_byte_code_32_only() {
        assert_eq!(
            [
                ExtractFont::is_word_space(byte(32)),
                ExtractFont::is_word_space(two(32))
            ],
            [true, false]
        );
    }

    #[test]
    fn standard_names_aliases_and_subset_tags_resolve() {
        assert_eq!(
            [
                standard_font(b"Helvetica-Bold"),
                standard_font(b"ABCDEF+Times-Roman"),
                standard_font(b"Arial,BoldItalic"),
                standard_font(b"TimesNewRomanPSMT"),
                standard_font(b"NotoSans"),
            ],
            [
                Some(Standard14::HelveticaBold),
                Some(Standard14::TimesRoman),
                Some(Standard14::HelveticaBoldOblique),
                Some(Standard14::TimesRoman),
                None
            ]
        );
    }
}
