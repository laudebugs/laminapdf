//! Generates `laminapdf-text`'s standard 14 tables (todo B4c) from the files in `data/`:
//!
//! * `metrics.rs`: per font, the AFM header values and the advance width of every glyph;
//! * `encodings.rs`: `StandardEncoding`, `WinAnsiEncoding`, `MacRomanEncoding` (ISO 32000-2
//!   Annex D) and the built-in encodings of Symbol and `ZapfDingbats`, as code → glyph;
//! * `chars.rs`: Unicode → glyph for the standard Latin character set and the two symbolic
//!   fonts (through the Adobe Glyph List), for encoding text;
//! * `agl.rs`: the Adobe Glyph List and the ITC Zapf Dingbats Glyph List (name → Unicode),
//!   for extraction.
//!
//! ```sh
//! cargo run -p std14-gen             # rewrite crates/laminapdf-text/src/std14/generated/*.rs
//! cargo run -p std14-gen -- --check  # fail if the committed files differ from a regeneration
//! ```
//!
//! Every input is checked against its SHA-256 (data/README.md) before anything is read, and
//! the output depends only on the inputs: regenerating is byte-identical.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail, ensure};
use sha2::{Digest, Sha256};

/// The inputs and their SHA-256 (also listed in data/README.md).
const INPUTS: [(&str, &str); 18] = [
    (
        "afm/Courier.afm",
        "521e0d7c7521efd4be78a5a9c5398e4c67d0771e396115b0346bc4ef74ada53d",
    ),
    (
        "afm/Courier-Bold.afm",
        "ad0150d4bedcc8877742bf94251fcec13e348dd599d4603f679d92027d1e6e99",
    ),
    (
        "afm/Courier-Oblique.afm",
        "b27103b2a2ef6030c110626597e2ab47bb8279075a039ab9170facb0aa1f70e1",
    ),
    (
        "afm/Courier-BoldOblique.afm",
        "cb82e69ef5f6d421e8f404fe00bb0d993425aae79725331d0ebf847e94e97e92",
    ),
    (
        "afm/Helvetica.afm",
        "da33f1870474c8e68bfe3e2353ff107ab6c6eea1f9836ce2aaf1e1a07b17982f",
    ),
    (
        "afm/Helvetica-Bold.afm",
        "b880d96baf56d0cc059f258f60b4d764ef49b555ab9db294b959c0016dee41f2",
    ),
    (
        "afm/Helvetica-Oblique.afm",
        "b4609b71b660a392ac09df35060271a876c2ce66617dd83bf826f742bb9d9721",
    ),
    (
        "afm/Helvetica-BoldOblique.afm",
        "69984a35ca26973a39f261cf83e0d367ea2e6517c590b0b030d4d4a219d9c269",
    ),
    (
        "afm/Times-Roman.afm",
        "768e1cabea085d489a63da3e80b96bc5abf0ec98d3073c9b4d6ba76e7bccba64",
    ),
    (
        "afm/Times-Bold.afm",
        "b4a000ed85cb22c6cdd985aa0fd3f6f78ed5079b7c6860dc4f0234e0d0e3c522",
    ),
    (
        "afm/Times-Italic.afm",
        "ed37fa2e6a67b5b17dfd47f36fc7e90df32891a4408860dbc8d4cbbe9959e242",
    ),
    (
        "afm/Times-BoldItalic.afm",
        "93c4744ba955215de02c4aae0b777133442ada2f7ab5a2af30a040b792b3c55d",
    ),
    (
        "afm/Symbol.afm",
        "3d2128a820375a10de9bc8bf6cfb15ded482c01ca0f95cc0b3277f37ec8bde66",
    ),
    (
        "afm/ZapfDingbats.afm",
        "a32565c90afd1b57a7008fc567b78d95cf1c22adff5e086094d666d88b039859",
    ),
    (
        "afm/MustRead.html",
        "be71e055bb2551a6fc8b60dcf094bf40357858980abde86f76cc9a3c144d5403",
    ),
    (
        "agl/glyphlist.txt",
        "a3b2f61ced9f3644cc0d4ecde5c59df34ca286c689d9484a43a710a81c466789",
    ),
    (
        "agl/zapfdingbats.txt",
        "f6394e3cb8a447e84a1dad75d4baaf2aa7f45dc104faf369f4720e1a774ef2dc",
    ),
    (
        "latin-encodings.txt",
        "c91fb386c1d8d31a56d94db73e9345d43943c78763c2651b53e1b55a26dc31dc",
    ),
];

/// The fonts, in the order of `laminapdf_text::Standard14` (the generated `FONTS` array).
const FONTS: [&str; 14] = [
    "Courier",
    "Courier-Bold",
    "Courier-Oblique",
    "Courier-BoldOblique",
    "Helvetica",
    "Helvetica-Bold",
    "Helvetica-Oblique",
    "Helvetica-BoldOblique",
    "Times-Roman",
    "Times-Bold",
    "Times-Italic",
    "Times-BoldItalic",
    "Symbol",
    "ZapfDingbats",
];

/// The output directory, relative to the workspace root.
const OUT: &str = "crates/laminapdf-text/src/std14/generated";

/// The notice of the Core 14 AFM files' `MustRead.html`, which derived files must keep.
const ADOBE_AFM_TERMS: &str = "This file and the 14 PostScript(R) AFM files it accompanies may be used, copied, and \
distributed for any purpose and without charge, with or without modification, provided that all copyright notices are \
retained; that the AFM files are not distributed without this file; that all modifications to this file or any of the \
AFM files are prominently noted in the modified file(s); and that this paragraph is not modified. Adobe Systems has no \
responsibility or obligation to support the use of the AFM files.";

fn main() -> Result<()> {
    let check = match std::env::args().nth(1).as_deref() {
        None => false,
        Some("--check") => true,
        Some(other) => bail!("unknown argument {other:?} (usage: std14-gen [--check])"),
    };
    let root = workspace_root();
    let files = generate(&root.join("tools/std14-gen/data"))?;
    let out = root.join(OUT);
    if !check {
        std::fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
    }
    let mut stale = Vec::new();
    for (name, text) in &files {
        let path = out.join(name);
        if check {
            let current = std::fs::read_to_string(&path).unwrap_or_default();
            if current != *text {
                stale.push(path.display().to_string());
            }
        } else {
            std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
            println!("{} ({} bytes)", path.display(), text.len());
        }
    }
    ensure!(
        stale.is_empty(),
        "stale generated tables (run `cargo run -p std14-gen`): {}",
        stale.join(", ")
    );
    Ok(())
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One AFM file: the header keys lamina uses and the character metrics.
struct Afm {
    copyright: Vec<String>,
    font_name: String,
    family_name: String,
    weight: String,
    italic_angle: String,
    is_fixed_pitch: bool,
    bbox: [i32; 4],
    cap_height: Option<i32>,
    x_height: Option<i32>,
    ascender: Option<i32>,
    descender: Option<i32>,
    std_hw: i32,
    std_vw: i32,
    /// (code, name, width), in file order; code −1 is unencoded.
    glyphs: Vec<(i32, String, i32)>,
}

fn parse_afm(text: &str) -> Result<Afm> {
    let mut header: BTreeMap<&str, &str> = BTreeMap::new();
    let mut copyright = Vec::new();
    let mut glyphs = Vec::new();
    let mut in_metrics = false;
    for line in text.lines().map(str::trim) {
        if line == "StartCharMetrics" || line.starts_with("StartCharMetrics ") {
            in_metrics = true;
            continue;
        }
        if line == "EndCharMetrics" {
            in_metrics = false;
            continue;
        }
        if in_metrics {
            let mut code = None;
            let mut name = None;
            let mut width = None;
            for field in line.split(';').map(str::trim) {
                let (key, value) = field.split_once(' ').unwrap_or((field, ""));
                match key {
                    "C" => code = Some(value.trim().parse::<i32>()?),
                    "WX" => width = Some(value.trim().parse::<i32>()?),
                    "N" => name = Some(value.trim().to_owned()),
                    _ => {}
                }
            }
            let (Some(code), Some(name), Some(width)) = (code, name, width) else {
                bail!("incomplete character metrics line {line:?}");
            };
            glyphs.push((code, name, width));
            continue;
        }
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        if key == "Comment" && value.starts_with("Copyright") {
            copyright.push(value.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        header.entry(key).or_insert(value.trim());
    }
    let get = |key: &str| header.get(key).copied().ok_or_else(|| anyhow!("no {key}"));
    let int = |key: &str| -> Result<Option<i32>> {
        header
            .get(key)
            .map(|v| v.parse::<i32>())
            .transpose()
            .map_err(Into::into)
    };
    let bbox: Vec<i32> = get("FontBBox")?
        .split_whitespace()
        .map(str::parse)
        .collect::<Result<_, _>>()?;
    Ok(Afm {
        copyright,
        font_name: get("FontName")?.to_owned(),
        family_name: get("FamilyName")?.to_owned(),
        weight: get("Weight")?.to_owned(),
        italic_angle: get("ItalicAngle")?.to_owned(),
        is_fixed_pitch: get("IsFixedPitch")? == "true",
        bbox: bbox.try_into().map_err(|_| anyhow!("FontBBox needs four numbers"))?,
        cap_height: int("CapHeight")?,
        x_height: int("XHeight")?,
        ascender: int("Ascender")?,
        descender: int("Descender")?,
        std_hw: int("StdHW")?.ok_or_else(|| anyhow!("no StdHW"))?,
        std_vw: int("StdVW")?.ok_or_else(|| anyhow!("no StdVW"))?,
        glyphs,
    })
}

/// A glyph list (`name;XXXX[ XXXX…]` lines): name → text.
fn parse_glyph_list(text: &str) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let (name, values) = line.split_once(';').ok_or_else(|| anyhow!("bad line {line:?}"))?;
        let text: String = values
            .split_whitespace()
            .map(|v| {
                u32::from_str_radix(v, 16)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| anyhow!("bad code point {v:?}"))
            })
            .collect::<Result<_>>()?;
        ensure!(out.insert(name.to_owned(), text).is_none(), "{name} listed twice");
    }
    Ok(out)
}

/// `data/latin-encodings.txt`: glyph → (STD, MAC, WIN) codes.
type LatinEncodings = BTreeMap<String, [Vec<u8>; 3]>;

fn parse_latin_encodings(text: &str) -> Result<LatinEncodings> {
    let mut out = BTreeMap::new();
    for line in text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [name, std, mac, win] = fields.as_slice() else {
            bail!("bad line {line:?}");
        };
        let codes = |s: &str| -> Result<Vec<u8>> {
            if s == "-" {
                return Ok(Vec::new());
            }
            s.split(',').map(|c| Ok(u8::from_str_radix(c, 8)?)).collect()
        };
        out.insert((*name).to_owned(), [codes(std)?, codes(mac)?, codes(win)?]);
    }
    Ok(out)
}

/// Code → index into `names`, `NONE` (`u16::MAX`) where undefined.
fn encoding_table(codes: &BTreeMap<u8, &str>, names: &[String]) -> Result<Vec<u16>> {
    let mut table = vec![u16::MAX; 256];
    for (&code, &name) in codes {
        let index = names
            .binary_search_by(|n| n.as_str().cmp(name))
            .map_err(|_| anyhow!("{name} is not in the font"))?;
        if let Some(slot) = table.get_mut(usize::from(code)) {
            *slot = u16::try_from(index)?;
        }
    }
    Ok(table)
}

/// Unicode → glyph index for the glyphs of `names` (sorted) that `keep` accepts, by `unicode`
/// (the glyph list). `encoded` ranks glyphs when two stand for the same character (an encoded
/// one wins, then the first name). A glyph `X` also stands for the letter the AGL gives
/// `Xgreek` (`Deltagreek` U+0394 → `Delta`), and `extra` adds (character, glyph) pairs; both
/// only where no glyph has the character directly.
fn char_table(
    names: &[String],
    unicode: &dyn Fn(&str) -> Option<String>,
    encoded: &dyn Fn(&str) -> bool,
    keep: &dyn Fn(&str) -> bool,
    extra: &[(char, &str)],
    agl: &BTreeMap<String, String>,
) -> Result<Vec<(char, u16)>> {
    let mut direct: BTreeMap<char, (bool, u16)> = BTreeMap::new();
    for (i, name) in names.iter().enumerate().filter(|(_, n)| keep(n)) {
        let index = u16::try_from(i)?;
        let Some(text) = unicode(name) else {
            eprintln!("note: glyph {name} has no Unicode value; it is not reachable from text");
            continue;
        };
        let mut chars = text.chars();
        let (Some(ch), None) = (chars.next(), chars.next()) else {
            continue;
        };
        let rank = (!encoded(name), index);
        match direct.get(&ch) {
            Some(&(e, j)) if (!e, j) <= rank => {}
            _ => {
                direct.insert(ch, (encoded(name), index));
            }
        }
    }
    let mut table: BTreeMap<char, u16> = direct.into_iter().map(|(c, (_, i))| (c, i)).collect();
    for (i, name) in names.iter().enumerate().filter(|(_, n)| keep(n)) {
        let Some(alias) = agl.get(&format!("{name}greek")) else {
            continue;
        };
        let mut chars = alias.chars();
        // Letters only: `questiongreek` (U+037E) is punctuation, not a Greek `question`.
        if let (Some(ch), None) = (chars.next(), chars.next())
            && ch.is_alphabetic()
        {
            table.entry(ch).or_insert(u16::try_from(i)?);
        }
    }
    for &(ch, name) in extra {
        let index = names
            .binary_search_by(|n| n.as_str().cmp(name))
            .map_err(|_| anyhow!("{name} is not in the font"))?;
        table.entry(ch).or_insert(u16::try_from(index)?);
    }
    Ok(table.into_iter().collect())
}

/// An AFM file's built-in encoding: code → glyph name.
fn builtin(afm: &Afm) -> BTreeMap<u8, &str> {
    afm.glyphs
        .iter()
        .filter_map(|(c, n, _)| u8::try_from(*c).ok().map(|c| (c, n.as_str())))
        .collect()
}

/// The generated files: (file name, contents).
#[expect(
    clippy::too_many_lines,
    reason = "one straight pipeline: read, check, then emit each file"
)]
fn generate(data: &Path) -> Result<Vec<(String, String)>> {
    let mut texts = BTreeMap::new();
    for (path, digest) in INPUTS {
        let bytes = std::fs::read(data.join(path)).with_context(|| format!("reading data/{path}"))?;
        let actual = hex(&Sha256::digest(&bytes));
        ensure!(actual == digest, "data/{path}: SHA-256 {actual}, expected {digest}");
        texts.insert(
            path,
            String::from_utf8(bytes).with_context(|| format!("data/{path} is not UTF-8"))?,
        );
    }
    let text = |path: &str| texts.get(path).map(String::as_str).ok_or_else(|| anyhow!("{path}"));
    let afms: Vec<Afm> = FONTS
        .iter()
        .map(|f| parse_afm(text(&format!("afm/{f}.afm"))?).with_context(|| format!("afm/{f}.afm")))
        .collect::<Result<_>>()?;
    for (afm, name) in afms.iter().zip(FONTS) {
        ensure!(afm.font_name == name, "afm/{name}.afm names {}", afm.font_name);
    }
    let agl = parse_glyph_list(text("agl/glyphlist.txt")?)?;
    let zapf_agl = parse_glyph_list(text("agl/zapfdingbats.txt")?)?;
    let latin_enc = parse_latin_encodings(text("latin-encodings.txt")?)?;

    let sorted_names = |afm: &Afm| -> Vec<String> {
        let set: BTreeSet<String> = afm.glyphs.iter().map(|(_, n, _)| n.clone()).collect();
        set.into_iter().collect()
    };
    let [_, _, _, _, helvetica, _, _, _, _, _, _, _, symbol_afm, zapf_afm] = afms.as_slice() else {
        bail!("expected 14 AFM files");
    };
    let latin_afms = afms.get(..12).unwrap_or_default();
    let latin = sorted_names(helvetica);
    let symbol = sorted_names(symbol_afm);
    let zapf = sorted_names(zapf_afm);
    for afm in latin_afms {
        ensure!(
            sorted_names(afm) == latin,
            "{} has another glyph set than Helvetica",
            afm.font_name
        );
    }
    ensure!(
        latin_enc.keys().cloned().collect::<Vec<_>>() == latin,
        "latin-encodings.txt lists another glyph set than the Latin AFM files"
    );

    // StandardEncoding: the AFMs' codes, which must agree with each other and with Annex D.
    let standard = builtin(helvetica);
    for afm in latin_afms {
        ensure!(
            builtin(afm) == standard,
            "{} has another built-in encoding",
            afm.font_name
        );
    }
    let column = |i: usize| -> Result<BTreeMap<u8, &str>> {
        let mut out = BTreeMap::new();
        for (name, codes) in &latin_enc {
            for &code in codes.get(i).into_iter().flatten() {
                if let Some(other) = out.insert(code, name.as_str()) {
                    bail!("latin-encodings.txt gives code {code:o} to both {other} and {name}");
                }
            }
        }
        Ok(out)
    };
    ensure!(
        column(0)? == standard,
        "latin-encodings.txt's STD column differs from the AFM files' codes"
    );
    let mac = column(1)?;
    let win = column(2)?;
    let symbol_codes = builtin(symbol_afm);
    let zapf_codes = builtin(zapf_afm);

    let mut files = Vec::new();
    files.push(("metrics.rs".to_owned(), metrics_rs(&afms, &latin, &symbol, &zapf)?));
    files.push((
        "encodings.rs".to_owned(),
        encodings_rs(&[
            (
                "STANDARD",
                "`StandardEncoding` (the Latin fonts' built-in encoding)",
                encoding_table(&standard, &latin)?,
                latin.len(),
            ),
            (
                "WIN_ANSI",
                "`WinAnsiEncoding`",
                encoding_table(&win, &latin)?,
                latin.len(),
            ),
            (
                "MAC_ROMAN",
                "`MacRomanEncoding`",
                encoding_table(&mac, &latin)?,
                latin.len(),
            ),
            (
                "SYMBOL",
                "Symbol's built-in encoding",
                encoding_table(&symbol_codes, &symbol)?,
                symbol.len(),
            ),
            (
                "ZAPF_DINGBATS",
                "`ZapfDingbats`' built-in encoding",
                encoding_table(&zapf_codes, &zapf)?,
                zapf.len(),
            ),
        ]),
    ));
    let agl_text = |name: &str| agl.get(name).cloned();
    let zapf_text = |name: &str| zapf_agl.get(name).or_else(|| agl.get(name)).cloned();
    let in_win = |name: &str| win.values().any(|n| *n == name);
    let in_symbol = |name: &str| symbol_codes.values().any(|n| *n == name);
    let in_zapf = |name: &str| zapf_codes.values().any(|n| *n == name);
    files.push((
        "chars.rs".to_owned(),
        chars_rs(&[
            (
                "LATIN_CHARS",
                "the standard Latin character set (every Latin font has all of it)",
                char_table(
                    &latin,
                    &agl_text,
                    &in_win,
                    &|_| true,
                    // Code points `WinAnsiEncoding` (Windows-1252) gives the glyphs at 240 and
                    // 255 (Annex D: `space` and `hyphen`), which the AGL names otherwise.
                    // Romanian Ț ț: the AGL gives `Tcommaaccent` the cedilla form's U+0162.
                    &[
                        ('\u{A0}', "space"),
                        ('\u{AD}', "hyphen"),
                        ('\u{21A}', "Tcommaaccent"),
                        ('\u{21B}', "tcommaaccent"),
                    ],
                    &agl,
                )?,
            ),
            (
                "SYMBOL_CHARS",
                "Symbol's encoded glyphs",
                char_table(&symbol, &agl_text, &in_symbol, &in_symbol, &[], &agl)?,
            ),
            (
                "ZAPF_DINGBATS_CHARS",
                "`ZapfDingbats`' encoded glyphs",
                char_table(&zapf, &zapf_text, &in_zapf, &in_zapf, &[], &agl)?,
            ),
        ]),
    ));
    files.push(("agl.rs".to_owned(), agl_rs(&agl, &zapf_agl)));
    Ok(files)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

const GENERATED: &str =
    "// @generated by tools/std14-gen (`cargo run -p std14-gen`) from tools/std14-gen/data/: do not edit.\n";

fn adobe_afm_header(afms: &[Afm]) -> String {
    let notices: BTreeSet<&str> = afms
        .iter()
        .flat_map(|a| a.copyright.iter().map(String::as_str))
        .collect();
    let mut s = String::from(GENERATED);
    s.push_str(
        "//\n// Derived from the Adobe Core 14 AFM files (Core14_AFMs.zip; see tools/std14-gen/data/README.md).\n",
    );
    s.push_str("// Modified: only the values lamina uses (header metrics, advance widths, built-in encodings) are\n");
    s.push_str("// kept, converted to Rust tables; kerning pairs, bounding boxes and ligatures are dropped.\n//\n");
    for notice in notices {
        let _ = writeln!(s, "// {notice}");
    }
    s.push_str("//\n");
    for line in wrap(ADOBE_AFM_TERMS, 96) {
        let _ = writeln!(s, "// {line}");
    }
    s
}

const AGL_HEADER: &str = "//\n// Derived from the Adobe Glyph List 2.0 and the ITC Zapf Dingbats Glyph List 2.0\n\
// (https://github.com/adobe-type-tools/agl-aglfn), BSD-3-Clause:\n//\n\
// Copyright 2002-2019 Adobe (http://www.adobe.com/).\n//\n\
// Redistribution and use in source and binary forms, with or without modification, are permitted\n\
// provided that the following conditions are met:\n//\n\
// Redistributions of source code must retain the above copyright notice, this list of conditions\n\
// and the following disclaimer.\n//\n\
// Redistributions in binary form must reproduce the above copyright notice, this list of\n\
// conditions and the following disclaimer in the documentation and/or other materials provided\n\
// with the distribution.\n//\n\
// Neither the name of Adobe nor the names of its contributors may be used to endorse or promote\n\
// products derived from this software without specific prior written permission.\n//\n\
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\" AND ANY EXPRESS OR\n\
// IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND\n\
// FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR\n\
// CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL\n\
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,\n\
// DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,\n\
// WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY\n\
// WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.\n";

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + 1 + word.len() > width {
            lines.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// `items` joined with ", " into lines of at most ~100 characters, indented four spaces.
fn packed(items: &[String]) -> String {
    let mut s = String::new();
    let mut line = String::new();
    for item in items {
        if !line.is_empty() && line.len() + item.len() + 2 > 96 {
            let _ = writeln!(s, "    {}", line.trim_end());
            line.clear();
        }
        line.push_str(item);
        line.push_str(", ");
    }
    if !line.is_empty() {
        let _ = writeln!(s, "    {}", line.trim_end());
    }
    s
}

fn quoted(names: &[String]) -> Vec<String> {
    names.iter().map(|n| format!("{n:?}")).collect()
}

fn metrics_rs(afms: &[Afm], latin: &[String], symbol: &[String], zapf: &[String]) -> Result<String> {
    let mut s = adobe_afm_header(afms);
    s.push_str("\nuse super::AfmMetrics;\n");
    for (name, names, doc) in [
        (
            "LATIN_NAMES",
            latin,
            "The standard Latin character set: the glyphs of every Latin font",
        ),
        ("SYMBOL_NAMES", symbol, "Symbol's glyphs"),
        ("ZAPF_DINGBATS_NAMES", zapf, "`ZapfDingbats`' glyphs"),
    ] {
        let _ = write!(
            s,
            "\n/// {doc}, sorted by name.\npub(crate) static {name}: [&str; {}] = [\n{}];\n",
            names.len(),
            packed(&quoted(names))
        );
    }
    s.push_str("\n/// Every font's metrics, in `Standard14` order.\npub(crate) static FONTS: [AfmMetrics; 14] = [\n");
    for (i, afm) in afms.iter().enumerate() {
        let (names, table) = match i {
            12 => (symbol, "SYMBOL_NAMES"),
            13 => (zapf, "ZAPF_DINGBATS_NAMES"),
            _ => (latin, "LATIN_NAMES"),
        };
        let widths: BTreeMap<&str, i32> = afm.glyphs.iter().map(|(_, n, w)| (n.as_str(), *w)).collect();
        let widths: Vec<String> = names
            .iter()
            .map(|n| {
                widths
                    .get(n.as_str())
                    .and_then(|w| u16::try_from(*w).ok())
                    .map(|w| w.to_string())
                    .ok_or_else(|| anyhow!("{}: no width for {n}", afm.font_name))
            })
            .collect::<Result<_>>()?;
        let [x0, y0, x1, y1] = afm.bbox;
        let opt = |v: Option<i32>| v.map_or_else(|| "None".to_owned(), |v| format!("Some({v})"));
        let angle = if afm.italic_angle.contains('.') {
            afm.italic_angle.clone()
        } else {
            format!("{}.0", afm.italic_angle)
        };
        let _ = write!(
            s,
            "    AfmMetrics {{\n        font_name: {:?},\n        family_name: {:?},\n        weight: {:?},\n        \
             italic_angle: {angle},\n        is_fixed_pitch: {},\n        bbox: [{}, {}, {}, {}],\n        \
             cap_height: {},\n        x_height: {},\n        ascender: {},\n        descender: {},\n        \
             std_hw: {},\n        std_vw: {},\n        names: &{table},\n        \
             widths: &[\n{}        ],\n    }},\n",
            afm.font_name,
            afm.family_name,
            afm.weight,
            afm.is_fixed_pitch,
            x0,
            y0,
            x1,
            y1,
            opt(afm.cap_height),
            opt(afm.x_height),
            opt(afm.ascender),
            opt(afm.descender),
            afm.std_hw,
            afm.std_vw,
            packed(&widths).lines().fold(String::new(), |mut out, l| {
                let _ = writeln!(out, "    {l}");
                out
            }),
        );
    }
    s.push_str("];\n");
    Ok(s)
}

/// (static name, what it is, code → glyph index, glyph count).
type EncodingTable<'a> = (&'a str, &'a str, Vec<u16>, usize);

fn encodings_rs(tables: &[EncodingTable<'_>]) -> String {
    let mut s = String::from(GENERATED);
    s.push_str(
        "//\n// Code → glyph (an index into the font's sorted glyph names in metrics.rs; `NONE` where the\n\
         // code is undefined). StandardEncoding, Symbol and ZapfDingbats: the codes of the Adobe Core 14\n\
         // AFM files (see metrics.rs for their notice); WinAnsiEncoding and MacRomanEncoding: ISO\n\
         // 32000-2 Annex D, Table D.2 (tools/std14-gen/data/latin-encodings.txt).\n\nuse super::NONE;\n",
    );
    for (name, doc, table, glyphs) in tables {
        let items: Vec<String> = table
            .iter()
            .map(|&i| {
                if i == u16::MAX {
                    "NONE".to_owned()
                } else {
                    i.to_string()
                }
            })
            .collect();
        // Glyph → its lowest code (code 0 is undefined in every one of these encodings, so 0
        // means "no code").
        let mut codes = vec![0u8; *glyphs];
        for (code, &glyph) in (0u8..=u8::MAX).zip(table).rev() {
            if let Some(slot) = codes.get_mut(usize::from(glyph)) {
                *slot = code;
            }
        }
        let codes: Vec<String> = codes.iter().map(ToString::to_string).collect();
        let _ = write!(
            s,
            "\n/// {doc}.\npub(crate) static {name}: [u16; 256] = [\n{}];\n\n/// {doc}: glyph → its lowest code (0: none).\npub(crate) static {name}_CODES: [u8; {glyphs}] = [\n{}];\n",
            packed(&items),
            packed(&codes)
        );
    }
    s
}

fn char_literal(ch: char) -> String {
    if ch.is_ascii_alphanumeric() || (ch.is_ascii_punctuation() && ch != '\'' && ch != '\\') || ch == ' ' {
        format!("'{ch}'")
    } else {
        format!("'\\u{{{:X}}}'", u32::from(ch))
    }
}

fn string_literal(text: &str) -> String {
    let mut s = String::from("\"");
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || (ch.is_ascii_punctuation() && ch != '"' && ch != '\\') || ch == ' ' {
            s.push(ch);
        } else {
            let _ = write!(s, "\\u{{{:X}}}", u32::from(ch));
        }
    }
    s.push('"');
    s
}

/// (static name, what it covers, Unicode → glyph index).
type CharTable<'a> = (&'a str, &'a str, Vec<(char, u16)>);

fn chars_rs(tables: &[CharTable<'_>]) -> String {
    let mut s = String::from(GENERATED);
    s.push_str(
        "//\n// Unicode → glyph (an index into the font's sorted glyph names in metrics.rs), sorted by\n\
         // character: each glyph name's Unicode value in the Adobe Glyph List (the ITC Zapf Dingbats\n\
         // Glyph List for ZapfDingbats), plus `<name>greek` aliases (U+0394 → `Delta`) and, for the\n\
         // Latin set, U+00A0 → `space` and U+00AD → `hyphen` (WinAnsiEncoding's codes 240 and 255).\n\
         // Where two glyphs have one value the encoded one (WinAnsiEncoding; the built-in encoding)\n\
         // wins, then the first name. Symbol's unencoded `apple` is left out.\n",
    );
    s.push_str(AGL_HEADER);
    for (name, doc, table) in tables {
        let items: Vec<String> = table
            .iter()
            .map(|&(ch, i)| format!("({}, {i})", char_literal(ch)))
            .collect();
        let _ = write!(
            s,
            "\n/// Unicode → glyph for {doc}.\npub(crate) static {name}: [(char, u16); {}] = [\n{}];\n",
            table.len(),
            packed(&items)
        );
    }
    s
}

fn agl_rs(agl: &BTreeMap<String, String>, zapf: &BTreeMap<String, String>) -> String {
    let mut s = String::from(GENERATED);
    s.push_str(AGL_HEADER);
    for (name, doc, list) in [
        ("AGL", "The Adobe Glyph List: glyph name → Unicode, sorted by name", agl),
        (
            "ZAPF_DINGBATS",
            "The ITC Zapf Dingbats Glyph List: glyph name → Unicode, sorted by name",
            zapf,
        ),
    ] {
        let items: Vec<String> = list
            .iter()
            .map(|(n, t)| format!("({n:?}, {})", string_literal(t)))
            .collect();
        let _ = write!(
            s,
            "\n/// {doc}.\npub(crate) static {name}: [(&str, &str); {}] = [\n{}];\n",
            list.len(),
            packed(&items)
        );
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The committed tables are what the generator writes from the committed data (todo B4c:
    /// "regenerating is byte-identical"). Compares only; never writes.
    #[test]
    fn committed_tables_are_current() {
        let root = workspace_root();
        let files = generate(&root.join("tools/std14-gen/data")).unwrap();
        for (name, text) in files {
            let committed = std::fs::read_to_string(root.join(OUT).join(&name)).unwrap();
            assert!(committed == text, "{name} is stale: run `cargo run -p std14-gen`");
        }
    }

    #[test]
    fn wrap_breaks_at_the_width() {
        assert_eq!(wrap("aa bb cc", 5), ["aa bb", "cc"]);
    }
}
