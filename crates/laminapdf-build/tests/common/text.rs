//! Text read back through lamina's reader and `laminapdf-content` (todo B5b): every shown string
//! decoded with its font's ToUnicode CMap (two-byte codes for Type 0 fonts, one byte for
//! simple ones), `/ActualText` spans replacing the glyphs they cover, grouped into lines by
//! baseline. A stand-in for B9b's extraction, enough to check that what the builder drew
//! reads back as its input.

use std::collections::BTreeMap;

use laminapdf_content::{MarkedContent, TextAdvance, TextItem, TextShow, Visitor, interpret_page};
use laminapdf_cos::{Object, Warning};
use laminapdf_syntax::text::decode_text_string;

/// One shown string: where its baseline starts in user space and the text it reads as.
#[derive(Debug, Clone, PartialEq)]
pub struct Shown {
    pub origin: (f64, f64),
    pub text: String,
    pub font_size: f64,
    pub render_mode: i64,
}

#[derive(Default)]
struct Reader<'d> {
    doc: Option<&'d laminapdf_cos::Document>,
    cmaps: BTreeMap<String, (BTreeMap<Vec<u8>, String>, usize)>,
    shown: Vec<Shown>,
    actual: Vec<Option<String>>,
    /// An `/ActualText` not yet given to a shown string.
    pending: Option<String>,
    warnings: Vec<&'static str>,
}

/// `bfchar`/`bfrange` entries of a ToUnicode CMap: code bytes → text, and the code width.
fn parse_cmap(cmap: &str) -> (BTreeMap<Vec<u8>, String>, usize) {
    let hex = |s: &str| -> Vec<u8> {
        let s = s.trim_matches(|c| c == '<' || c == '>');
        (0..s.len() / 2)
            .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
            .collect()
    };
    let utf16 = |b: &[u8]| {
        let units: Vec<u16> = b.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16(&units).unwrap()
    };
    let mut map = BTreeMap::new();
    let mut width = 1;
    let mut mode = "";
    for line in cmap.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        match parts.as_slice() {
            [_, "beginbfchar"] => mode = "char",
            [_, "beginbfrange"] => mode = "range",
            [lo, _, "endcodespacerange"] | [lo, _] if mode == "space" => width = hex(lo).len(),
            [_, "begincodespacerange"] => mode = "space",
            ["endcodespacerange" | "endbfchar" | "endbfrange"] => mode = "",
            [src, dst] if mode == "char" => {
                map.insert(hex(src), utf16(&hex(dst)));
            }
            [lo, hi, dst] if mode == "range" => {
                let (lo, hi, dst) = (hex(lo), hex(hi), hex(dst));
                let n = |b: &[u8]| b.iter().fold(0u32, |a, &x| (a << 8) | u32::from(x));
                for (i, code) in (n(&lo)..=n(&hi)).enumerate() {
                    let mut units = dst.clone();
                    let last = units.len() - 1;
                    let v = u16::from_be_bytes([units[last - 1], units[last]]) + u16::try_from(i).unwrap();
                    units[last - 1..].copy_from_slice(&v.to_be_bytes());
                    let bytes = code.to_be_bytes();
                    map.insert(bytes[4 - lo.len()..].to_vec(), utf16(&units));
                }
            }
            _ => {}
        }
    }
    (map, width)
}

impl Reader<'_> {
    fn cmap(&mut self, font: &laminapdf_content::FontRef) -> &(BTreeMap<Vec<u8>, String>, usize) {
        // An indirect font by its object; a direct one by its dictionary's identity.
        let key = match (font.object, &font.dict) {
            (Some(r), _) => format!("{r:?}"),
            (None, Some(dict)) => format!("direct {:p}", std::sync::Arc::as_ptr(dict)),
            (None, None) => "missing".to_owned(),
        };
        let doc = self.doc.unwrap();
        self.cmaps.entry(key).or_insert_with(|| {
            let Some(dict) = font.dict.as_deref().and_then(Object::as_dict) else {
                return (BTreeMap::new(), 1);
            };
            let resolved;
            let stream = match dict.get("ToUnicode") {
                Some(Object::Ref(r)) => {
                    resolved = doc.resolve(*r).unwrap();
                    match &*resolved {
                        Object::Stream(s) => s,
                        other => panic!("ToUnicode is {other:?}"),
                    }
                }
                Some(Object::Stream(s)) => s,
                _ => return (BTreeMap::new(), 1),
            };
            parse_cmap(&String::from_utf8(doc.decode_stream(stream).unwrap()).unwrap())
        })
    }
}

impl Visitor for Reader<'_> {
    fn show_text(&mut self, show: &TextShow<'_>) -> TextAdvance {
        // The text origin without the rise (`Ts` moves marks, not the baseline).
        let trm = show.tm.multiply(&show.state.ctm);
        let (map, width) = self.cmap(show.font().unwrap()).clone();
        let mut text = String::new();
        for item in show.items() {
            if let TextItem::Text(bytes) = item {
                for code in bytes.chunks(width) {
                    text.push_str(map.get(code).map_or("\u{FFFD}", String::as_str));
                }
            }
        }
        if self.actual.iter().any(Option::is_some) {
            text = self.pending.take().unwrap_or_default();
        }
        self.shown.push(Shown {
            origin: (trm.e, trm.f),
            text,
            font_size: show.state.text.font_size,
            render_mode: show.state.text.render_mode,
        });
        TextAdvance::default()
    }

    fn begin_marked(&mut self, marked: &MarkedContent<'_>) {
        let actual = marked
            .properties
            .and_then(|p| p.get("ActualText"))
            .and_then(|t| match t {
                Object::String(s) => Some(decode_text_string(&s.bytes)),
                _ => None,
            });
        if actual.is_some() {
            self.pending.clone_from(&actual);
        }
        self.actual.push(actual);
    }

    fn end_marked(&mut self) {
        self.actual.pop();
    }

    fn warning(&mut self, warning: &Warning) {
        self.warnings.push(warning.code());
    }
}

/// Every string shown on page `index`, in content order, and the content warnings.
pub fn shown(doc: &laminapdf_model::Document, index: usize) -> (Vec<Shown>, Vec<&'static str>) {
    let pages = doc.pages().unwrap();
    let mut reader = Reader {
        doc: Some(doc),
        ..Reader::default()
    };
    interpret_page(doc, &pages.get(index).unwrap(), &mut reader).unwrap();
    (reader.shown, reader.warnings)
}

/// The text of page `index` in content order, one entry per line: consecutive shown strings
/// on the same baseline (to a tenth of a point) are joined.
pub fn lines(doc: &laminapdf_model::Document, index: usize) -> Vec<String> {
    let (shown, _) = shown(doc, index);
    let mut out: Vec<(i64, String)> = Vec::new();
    for s in shown {
        let y = (s.origin.1 * 10.0).round() as i64;
        match out.last_mut() {
            Some((at, text)) if *at == y => text.push_str(&s.text),
            _ => out.push((y, s.text)),
        }
    }
    out.into_iter().map(|(_, t)| t).filter(|t| !t.is_empty()).collect()
}
