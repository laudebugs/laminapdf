//! The documents behind the `fixtures/golden/text-*.pdf` goldens, with the text each line
//! should extract to (checked against pypdf's output in `fixtures/text/extracted.json`).

use lamina_text::{Font, GlyphId, Tag};

use super::{TestPdf, fixture, font};

/// A generated document and the text of each of its lines, in order.
pub struct Sample {
    pub bytes: Vec<u8>,
    pub lines: Vec<String>,
}

/// The code that builds one sample.
pub type Build = fn() -> Sample;

/// Every golden: its file name under `fixtures/golden/` and the code that builds it.
pub const GOLDENS: [(&str, Build); 3] = [
    ("text-latin.pdf", latin),
    ("text-cff.pdf", cff),
    ("text-variable-collection.pdf", variable_and_collection),
];

/// `text` as glyphs of `font`, with the `ffi`, `ffl`, `fi` and `fl` sequences drawn as the
/// font's ligature glyphs (U+FB03, U+FB04, U+FB01, U+FB02) standing for those letters: what
/// a shaper's `liga` feature produces, without a shaper.
pub fn with_ligatures(font: &Font, text: &str) -> Vec<(GlyphId, String)> {
    const LIGATURES: [(&str, char); 4] = [
        ("ffi", '\u{FB03}'),
        ("ffl", '\u{FB04}'),
        ("fi", '\u{FB01}'),
        ("fl", '\u{FB02}'),
    ];
    let mut out = Vec::new();
    let mut rest = text;
    'outer: while let Some(ch) = rest.chars().next() {
        for (seq, lig) in LIGATURES {
            if let Some(after) = rest.strip_prefix(seq) {
                out.push((font.glyph_for(lig).unwrap(), seq.to_owned()));
                rest = after;
                continue 'outer;
            }
        }
        out.push((font.glyph_for(ch).unwrap(), ch.to_string()));
        rest = &rest[ch.len_utf8()..];
    }
    out
}

fn glyph_line(pdf: &mut TestPdf, key: lamina_text::FontKey, size: f32, glyphs: &[(GlyphId, String)]) {
    let refs: Vec<(GlyphId, &str)> = glyphs.iter().map(|(g, t)| (*g, t.as_str())).collect();
    pdf.glyphs(key, size, &refs).unwrap();
}

/// Noto Sans (TrueType): ASCII, accents, curly quotes, ligature glyphs and a non-BMP
/// character from Noto Sans Math.
pub fn latin() -> Sample {
    let sans = font("NotoSans-Regular.ttf");
    let math = font("NotoSansMath-Regular.ttf");
    let mut pdf = TestPdf::new();
    let s = pdf.add_font(sans.clone()).unwrap();
    let m = pdf.add_font(math).unwrap();
    let lines = [
        "Hello, world!",
        "Café crème brûlée, naïve façade: “curly” and ‘single’ quotes",
        "The office staff fill the affine waffle flask first.",
        "Bold maths: \u{1D400}\u{1D401}\u{1D402} = ABC",
    ];
    pdf.line(s, 18.0, lines[0]).unwrap();
    pdf.line(s, 12.0, lines[1]).unwrap();
    glyph_line(&mut pdf, s, 12.0, &with_ligatures(&sans, lines[2]));
    pdf.line(m, 12.0, lines[3]).unwrap();
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample {
        bytes,
        lines: lines.map(str::to_owned).to_vec(),
    }
}

/// CFF outlines: Noto Sans (name-keyed CFF) and a Noto Sans SC subset (CID-keyed CFF).
pub fn cff() -> Sample {
    let mut pdf = TestPdf::new();
    let latin = pdf.add_font(font("NotoSans-Regular.otf")).unwrap();
    let cjk = pdf.add_font(font("NotoSansSC-Regular.otf")).unwrap();
    let lines = ["CFF outlines: Ådne’s naïve café", "你好世界，这是中文字体子集测试。"];
    pdf.line(latin, 14.0, lines[0]).unwrap();
    pdf.line(cjk, 14.0, lines[1]).unwrap();
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample {
        bytes,
        lines: lines.map(str::to_owned).to_vec(),
    }
}

/// A variable font at its default and at `wght` 700, and the Bold member of a collection.
pub fn variable_and_collection() -> Sample {
    let variable = font("NotoSans-VF.ttf");
    let bold = variable.instance(&[(Tag::WEIGHT, 700.0)]).unwrap();
    let ttc = Font::from_collection(fixture("fonts/NotoSans-RegularBold.ttc"), 1).unwrap();
    let mut pdf = TestPdf::new();
    let v = pdf.add_font(variable).unwrap();
    let b = pdf.add_font(bold).unwrap();
    let t = pdf.add_font(ttc).unwrap();
    let lines = [
        "Variable font, default instance",
        "Variable font, wght 700 instance",
        "Collection member 1: Noto Sans Bold",
    ];
    pdf.line(v, 14.0, lines[0]).unwrap();
    pdf.line(b, 14.0, lines[1]).unwrap();
    pdf.line(t, 14.0, lines[2]).unwrap();
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample {
        bytes,
        lines: lines.map(str::to_owned).to_vec(),
    }
}
