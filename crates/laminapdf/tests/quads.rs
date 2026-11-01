//! Glyph quads against PDFium's character boxes (todo B9c, ADR 0018 tier 6). PDFium's loose
//! boxes (`FPDFText_GetLooseCharBox`, from the font's ascent to its descent, as Lamina PDF's quads
//! are) are precomputed in `fixtures/text/charboxes.json` (`cargo xtask quads --update`,
//! `tools/render/charbox.py`); this test needs no PDFium.
//!
//! **Rule (what the two boxes have in common):** Lamina PDF's quad runs along the glyph's advance,
//! from the font program's ascent to its descent; PDFium's loose box covers the ink too (an `f`'s
//! overhang) and takes ascent and descent from the PDF's font descriptor. So, for every glyph with
//! one character of text, matched by position with PDFium's nearest unused character of the same
//! text (order isn't used: PDFium reorders right-to-left text):
//!
//! - horizontally, Lamina PDF's box lies within PDFium's (give or take the tolerance: 1 pt, or 15 % of
//!   the font size when that is more), and at least one edge agrees within the tolerance: PDFium's
//!   box may extend past the advance on one side only, by the ink's overhang (an `f`, the Arabic
//!   lam whose stroke runs under the next alef);
//! - the vertical extents overlap by at least 80 % of the shorter one.
//!
//! Glyphs without a one-to-one character are counted and skipped, not compared: ligatures and
//! clusters (several characters in one glyph), combining marks (zero-width in Lamina PDF, ink boxes in
//! PDFium) and the Indic blocks, where PDFium's box for a consonant spans the shaped cluster.
//! Pages are unrotated (PDFium reports boxes in unrotated user space).
#![cfg(feature = "extract")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_arithmetic,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    reason = "test code: a panic is a test failure; geometry is arithmetic on box edges (l, b, r, t); one comparison loop"
)]

use std::path::{Path, PathBuf};

use laminapdf::extract::TextOptions;
use laminapdf::{Document, OpenOptions};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Compared {
    pages: usize,
    glyphs: usize,
    unaligned: usize,
    skipped: usize,
}

/// A combining mark, or a character of the Indic blocks (U+0900–U+0DFF), whose glyph has no
/// one-to-one box in PDFium.
fn is_cluster_part(c: char) -> bool {
    matches!(c,
        '\u{0300}'..='\u{036F}'
        | '\u{0483}'..='\u{0489}'
        | '\u{0591}'..='\u{05C7}'
        | '\u{0610}'..='\u{061A}'
        | '\u{064B}'..='\u{065F}'
        | '\u{0670}'
        | '\u{06D6}'..='\u{06ED}'
        | '\u{0900}'..='\u{0DFF}'
        | '\u{1AB0}'..='\u{1AFF}'
        | '\u{1DC0}'..='\u{1DFF}'
        | '\u{20D0}'..='\u{20FF}'
        | '\u{FE20}'..='\u{FE2F}')
}

/// The text compared: whitespace and format characters dropped (PDFium inserts U+200C in
/// Arabic), the compatibility ligatures U+FB00–U+FB06 expanded (PDFium splits them).
fn normal(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            c if c.is_whitespace() => {}
            '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2064}' | '\u{FEFF}' => {}
            '\u{FB00}' => out.push_str("ff"),
            '\u{FB01}' => out.push_str("fi"),
            '\u{FB02}' => out.push_str("fl"),
            '\u{FB03}' => out.push_str("ffi"),
            '\u{FB04}' => out.push_str("ffl"),
            '\u{FB05}' | '\u{FB06}' => out.push_str("st"),
            c => out.push(c),
        }
    }
    out
}

/// The centre of four points.
fn centre(points: &[(f64, f64); 4]) -> (f64, f64) {
    let (x, y) = points.iter().fold((0.0, 0.0), |(x, y), (a, b)| (x + a, y + b));
    (x / 4.0, y / 4.0)
}

#[test]
fn glyph_quads_lie_within_tolerance_of_pdfium_char_boxes() {
    let boxes: Value =
        serde_json::from_slice(&std::fs::read(root().join("fixtures/text/charboxes.json")).unwrap()).unwrap();
    let mut failures = Vec::new();
    let mut unmatched = Vec::new();
    let mut compared = Compared {
        pages: 0,
        glyphs: 0,
        unaligned: 0,
        skipped: 0,
    };
    for (file, pages) in boxes["files"].as_object().unwrap() {
        let bytes = std::fs::read(root().join(file)).unwrap();
        let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
        let texts = doc.page_texts(&TextOptions::new()).unwrap();
        assert_eq!(
            texts.len(),
            pages.as_array().unwrap().len(),
            "{file}: lamina's and PDFium's page counts"
        );
        for (index, (page, text)) in pages.as_array().unwrap().iter().zip(&texts).enumerate() {
            assert_eq!(
                page["rotation"].as_i64(),
                Some(0),
                "{file} page {index}: only unrotated pages"
            );
            let crop: Vec<f64> = page["cropbox"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            let (left, top) = (crop[0], crop[3]);
            let theirs: Vec<(String, [f64; 4])> = page["chars"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|c| {
                    let c = c.as_array().unwrap();
                    let text = normal(c[0].as_str().unwrap());
                    let edges = [1, 2, 3, 4].map(|i| c[i].as_f64().unwrap());
                    (!text.is_empty()).then_some((text, edges))
                })
                .collect();
            let ours: Vec<_> = text.glyphs.iter().filter(|g| !normal(&g.text).is_empty()).collect();
            // Match by position, not order (PDFium reorders right-to-left text, lamina keeps
            // the drawing order).
            let mut used = vec![false; theirs.len()];
            for glyph in &ours {
                let want = normal(&glyph.text);
                let mut chars = want.chars();
                let (Some(c), None) = (chars.next(), chars.next()) else {
                    compared.skipped += 1;
                    continue;
                };
                if is_cluster_part(c) {
                    compared.skipped += 1;
                    continue;
                }
                let (cx, cy) = centre(&glyph.quad.map(|p| (p.x.get() + left, top - p.y.get())));
                let nearest = theirs
                    .iter()
                    .enumerate()
                    .filter(|(k, (t, _))| !used[*k] && t.chars().eq([c]))
                    .min_by(|(_, (_, a)), (_, (_, b))| {
                        let d = |e: &[f64; 4]| (f64::midpoint(e[0], e[2]) - cx).hypot(f64::midpoint(e[1], e[3]) - cy);
                        d(a).total_cmp(&d(b))
                    });
                let Some((k, (_, [l, b, r, tp]))) = nearest else {
                    compared.unaligned += 1;
                    unmatched.push(format!("{file} page {index}: glyph {want:?}"));
                    continue;
                };
                used[k] = true;
                compared.glyphs += 1;
                let xs = glyph.quad.map(|p| p.x.get() + left);
                let ys = glyph.quad.map(|p| top - p.y.get());
                let min = |v: [f64; 4]| v.into_iter().fold(f64::INFINITY, f64::min);
                let max = |v: [f64; 4]| v.into_iter().fold(f64::NEG_INFINITY, f64::max);
                let (ol, ob, or, ot) = (min(xs), min(ys), max(xs), max(ys));
                let tolerance = (glyph.size.abs() * 0.15).max(1.0);
                // Within PDFium's box, and one edge on PDFium's edge.
                let inside = ol >= l - tolerance && or <= r + tolerance;
                let horizontal = (ol - l).abs().min((or - r).abs());
                let overlap = (ot.min(*tp) - ob.max(*b)).max(0.0);
                let shorter = (ot - ob).min(tp - b);
                if !inside || horizontal > tolerance || overlap < 0.8 * shorter {
                    failures.push(format!(
                        "{file} page {index} {want:?}: lamina [{ol:.2}, {ob:.2}, {or:.2}, {ot:.2}] vs PDFium [{l:.2}, {b:.2}, {r:.2}, {tp:.2}]: inside {inside}, nearer edge off by {horizontal:.2} (tolerance {tolerance:.2}), vertical overlap {overlap:.2} of {shorter:.2}"
                    ));
                }
            }
            compared.pages += 1;
        }
    }
    // A few glyphs PDFium decodes differently can't be placed (it reads a non-BMP letter such as
    // U+1D400 as another character); they must stay rare.
    assert!(
        compared.unaligned * 50 <= compared.glyphs,
        "{} of {} glyphs couldn't be aligned with PDFium's characters:\n{}",
        compared.unaligned,
        compared.glyphs,
        unmatched.join("\n")
    );
    assert!(
        failures.is_empty(),
        "{} of {} glyphs out of tolerance:\n{}",
        failures.len(),
        compared.glyphs,
        failures.join("\n")
    );
    assert!(
        compared.glyphs > 1000,
        "compared only {} glyphs on {} pages",
        compared.glyphs,
        compared.pages
    );
}
