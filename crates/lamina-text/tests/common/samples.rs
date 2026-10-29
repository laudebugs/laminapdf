//! The documents behind the `fixtures/golden/text-*.pdf` goldens, with the text each line
//! should extract to (checked against pypdf's output in `fixtures/text/extracted.json`).

use lamina_text::{Font, GlyphId, LayoutOptions, Standard14, Tag};

use super::{TestPdf, fixture, font};

/// A generated document (its lines are the `LATIN`, `CFF`, … constants).
pub struct Sample {
    pub bytes: Vec<u8>,
}

/// The code that builds one sample.
pub type Build = fn() -> Sample;

/// The lines of [`latin`].
pub const LATIN: [&str; 4] = [
    "Hello, world!",
    "Café crème brûlée, naïve façade: “curly” and ‘single’ quotes",
    "The office staff fill the affine waffle flask first.",
    "Bold maths: \u{1D400}\u{1D401}\u{1D402} = ABC",
];
/// The lines of [`cff`].
pub const CFF: [&str; 2] = ["CFF outlines: Ådne’s naïve café", "你好世界，这是中文字体子集测试。"];
/// The lines of [`variable_and_collection`].
pub const VARIABLE_COLLECTION: [&str; 3] = [
    "Variable font, default instance",
    "Variable font, wght 700 instance",
    "Collection member 1: Noto Sans Bold",
];

/// The lines of [`std14`]: one per standard 14 font, in [`Standard14::ALL`] order, then
/// [`STD14_DIFFERENCES`].
pub const STD14: [&str; 16] = [
    "Courier: Hello, world! 0123456789",
    "Courier-Bold: {braces} [brackets] <angles>",
    "Courier-Oblique: café, naïve, façade",
    "Courier-BoldOblique: 50% off ~ $9.99",
    "Helvetica: “Curly” ‘quotes’ — and dashes – too",
    "Helvetica-Bold: Euro € pound £ yen ¥",
    "Helvetica-Oblique: Ærøskøbing, Þórshöfn, Œuvre",
    "Helvetica-BoldOblique: ½ ¼ ¾ © ® ™ § ¶",
    "Times-Roman: The quick brown fox jumps over the lazy dog.",
    "Times-Bold: Grüße aus Köln • München",
    "Times-Italic: ¿Qué tal? ¡Muy bien!",
    "Times-BoldItalic: Ça va… très «bien» ‰",
    "αβγ π ∑ ≤ ∞ €",
    "✔ ✈ ❤ ☎",
    "Łódź, Kraków − Şişli",
    "Țară, Ğğ İı őű",
];

/// How many of [`STD14`]'s lines are Helvetica lines outside `WinAnsiEncoding` (written through
/// `/Differences`), after the one line per font: together more such glyphs than
/// `WinAnsiEncoding`'s five undefined high codes, so control codes are assigned too.
pub const STD14_DIFFERENCES: usize = 2;

/// The paragraphs of [`shaped`]: (fonts in order of preference, box width, size, text).
pub const SHAPED_PARAGRAPHS: [(&[&str], f32, f32, &str); 7] = [
    (
        &["NotoSans-Regular.ttf"],
        300.0,
        12.0,
        "The office staff fill the affine waffle flask first. AVATAR Wave: To Yo!",
    ),
    (&["NotoSans-Regular.otf"], 450.0, 12.0, "CFF outlines: office, AVATAR."),
    (
        &["NotoSansArabic-Regular.ttf", "NotoSans-Regular.ttf"],
        450.0,
        16.0,
        "مرحبا بالعالم، السلام عليكم",
    ),
    (
        &["NotoSansHebrew-Regular.ttf", "NotoSans-Regular.ttf"],
        450.0,
        16.0,
        "שָׁלוֹם עוֹלָם, שלום עולם",
    ),
    (
        &["NotoSansDevanagari-Regular.ttf"],
        450.0,
        16.0,
        "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण कर्म",
    ),
    (
        &["NotoSansSC-Regular.otf"],
        450.0,
        14.0,
        "你好世界，这是中文字体子集测试。汉字排版与文本提取",
    ),
    (
        &[
            "NotoSans-Regular.ttf",
            "NotoSansHebrew-Regular.ttf",
            "NotoSansArabic-Regular.ttf",
        ],
        450.0,
        12.0,
        "The words שלום עולם and مرحبا بالعالم mean hello.",
    ),
];

/// The lines of [`shaped`], as `break_lines` cut [`SHAPED_PARAGRAPHS`].
pub const SHAPED: [&str; 8] = [
    "The office staff fill the affine waffle flask first. AVATAR",
    "Wave: To Yo!",
    "CFF outlines: office, AVATAR.",
    "مرحبا بالعالم، السلام عليكم",
    "שָׁלוֹם עוֹלָם, שלום עולם",
    "नमस्ते दुनिया। हिन्दी पाठ का परीक्षण कर्म",
    "你好世界，这是中文字体子集测试。汉字排版与文本提取",
    "The words שלום עולם and مرحبا بالعالم mean hello.",
];

/// Every golden: its file name under `fixtures/golden/`, the code that builds it and the
/// lines it draws.
pub const GOLDENS: [(&str, Build, &[&str]); 5] = [
    ("text-latin.pdf", latin, &LATIN),
    ("text-cff.pdf", cff, &CFF),
    (
        "text-variable-collection.pdf",
        variable_and_collection,
        &VARIABLE_COLLECTION,
    ),
    ("text-shaped.pdf", shaped, &SHAPED),
    ("text-std14.pdf", std14, &STD14),
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
    let lines = LATIN;
    pdf.line(s, 18.0, lines[0]).unwrap();
    pdf.line(s, 12.0, lines[1]).unwrap();
    glyph_line(&mut pdf, s, 12.0, &with_ligatures(&sans, lines[2]));
    pdf.line(m, 12.0, lines[3]).unwrap();
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample { bytes }
}

/// CFF outlines: Noto Sans (name-keyed CFF) and a Noto Sans SC subset (CID-keyed CFF).
pub fn cff() -> Sample {
    let mut pdf = TestPdf::new();
    let latin = pdf.add_font(font("NotoSans-Regular.otf")).unwrap();
    let cjk = pdf.add_font(font("NotoSansSC-Regular.otf")).unwrap();
    let lines = CFF;
    pdf.line(latin, 14.0, lines[0]).unwrap();
    pdf.line(cjk, 14.0, lines[1]).unwrap();
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample { bytes }
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
    let lines = VARIABLE_COLLECTION;
    pdf.line(v, 14.0, lines[0]).unwrap();
    pdf.line(b, 14.0, lines[1]).unwrap();
    pdf.line(t, 14.0, lines[2]).unwrap();
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample { bytes }
}

/// Shaped paragraphs in five scripts (todo B4b): HarfRust glyphs, bidi reordering, greedy line
/// breaking, `TJ` kerning and offsets, rises and `/ActualText`.
pub fn shaped() -> Sample {
    shaped_with_lines().0
}

/// [`shaped`] and the text of every line it drew.
pub fn shaped_with_lines() -> (Sample, Vec<String>) {
    let mut pdf = TestPdf::new();
    let mut keys = std::collections::BTreeMap::new();
    let mut lines = Vec::new();
    for (fonts, width, size, text) in SHAPED_PARAGRAPHS {
        let paragraph_keys: Vec<_> = fonts
            .iter()
            .map(|&name| *keys.entry(name).or_insert_with(|| pdf.add_font(font(name)).unwrap()))
            .collect();
        lines.extend(
            pdf.paragraph(&paragraph_keys, text, &LayoutOptions::new(width, size))
                .unwrap(),
        );
    }
    let (bytes, _, _) = pdf.finish().unwrap();
    (Sample { bytes }, lines)
}

/// Each standard 14 font by name (not embedded), and Helvetica again for glyphs outside
/// `WinAnsiEncoding`.
pub fn std14() -> Sample {
    let mut pdf = TestPdf::new();
    let keys: Vec<(Standard14, _)> = Standard14::ALL
        .into_iter()
        .map(|f| (f, pdf.add_font(Font::standard(f)).unwrap()))
        .collect();
    let (per_font, differences) = STD14.split_at(STD14.len() - STD14_DIFFERENCES);
    assert_eq!(per_font.len(), keys.len());
    for ((_, key), line) in keys.iter().zip(per_font) {
        pdf.line(*key, 12.0, line).unwrap();
    }
    let (_, helvetica) = keys.iter().find(|(f, _)| *f == Standard14::Helvetica).unwrap();
    for line in differences {
        pdf.line(*helvetica, 12.0, line).unwrap();
    }
    let (bytes, _, _) = pdf.finish().unwrap();
    Sample { bytes }
}
