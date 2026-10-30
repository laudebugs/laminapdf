//! Regression tests for the first `/code-review` round of PR #55 (todo B4b). Each test names
//! the finding it covers (groups A–D of the review).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{TestPdf, expand_widths, font, read_fonts};
use lamina_text::{
    Alignment, BidiParagraph, EmbedPolicy, Error, FontRegistry, HarfRust, LayoutOptions, ShapeOptions, Shaper, Tag,
    TjElement, Warning, break_lines, itemize, pdf_run,
};

fn latin() -> lamina_text::Font {
    font("NotoSans-Regular.ttf")
}

/// A1: a line feed ends a bidi paragraph (UAX #9 P1); the next paragraph finds its own
/// direction, so a Latin paragraph after a Hebrew one starts on the left.
#[test]
fn a1_each_paragraph_resolves_its_own_direction() {
    let fonts = [font("NotoSansHebrew-Regular.ttf"), latin()];
    let lines = break_lines(&HarfRust, &fonts, "שלום\nabc", &LayoutOptions::new(300.0, 10.0)).unwrap();
    assert_eq!(lines.len(), 2);
    assert!(
        (lines[0].x + lines[0].width - 300.0).abs() < 1e-9,
        "Hebrew starts on the right"
    );
    assert_eq!(lines[1].x, 0.0, "Latin starts on the left");
    assert_eq!(lines[1].runs[0].level, 0);
}

/// A1: an isolate left open in one paragraph does not leak into the next (X8 closes it at
/// the paragraph end).
#[test]
fn a1_open_isolates_end_with_their_paragraph() {
    let fonts = [latin(), font("NotoSansHebrew-Regular.ttf")];
    let text = "a\u{2067}b\nc";
    let lines = break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(300.0, 10.0)).unwrap();
    assert_eq!(lines[1].runs.iter().map(|r| r.level).collect::<Vec<_>>(), [0]);
}

/// A2: a tab needs no glyph of its own: it is shaped as a space.
#[test]
fn a2_tabs_are_shaped_as_spaces() {
    let font = latin();
    let run = HarfRust.shape(&font, "a\tb", &ShapeOptions::new()).unwrap();
    assert!(!run.has_notdef());
    assert_eq!(run.glyphs[1].gid, font.glyph_for(' ').unwrap());
    let lines = break_lines(&HarfRust, &[font], "a\tb", &LayoutOptions::new(300.0, 10.0)).unwrap();
    assert_eq!(lines.len(), 1);
}

/// A3: a cluster wider than the box followed by a space never leaves a line holding only
/// the hanging space.
#[test]
fn a3_no_line_is_only_hanging_spaces() {
    let text = "ab c";
    let lines = break_lines(&HarfRust, &[latin()], text, &LayoutOptions::new(5.0, 12.0)).unwrap();
    for line in &lines {
        assert!(!line.content.is_empty(), "{lines:?}");
    }
    let drawn: String = lines.iter().map(|l| &text[l.content.clone()]).collect();
    assert_eq!(drawn, "abc");
}

/// A4: after a fallback font, characters the primary font covers go back to it.
#[test]
fn a4_fallback_fonts_do_not_stick() {
    let fonts = [latin(), font("NotoSansSC-Regular.otf")];
    let text = "中a";
    let items = itemize(&BidiParagraph::new(text, None), &fonts).unwrap();
    let got: Vec<(&str, usize)> = items.iter().map(|i| (&text[i.range.clone()], i.font)).collect();
    assert_eq!(got, [("中", 1), ("a", 0)]);
}

/// A5: lines are measured as they are drawn, so mixed-direction text with separators never
/// exceeds the box.
#[test]
fn a5_drawn_lines_fit_for_mixed_direction_text() {
    let fonts = [
        latin(),
        font("NotoSansHebrew-Regular.ttf"),
        font("NotoSansArabic-Regular.ttf"),
    ];
    let text = "One שלום\tעולם two مرحبا\tبالعالم three שלום עולם four, five six seven.";
    for width in [40.0_f32, 61.0, 77.5, 90.0, 133.0, 170.0] {
        for line in break_lines(&HarfRust, &fonts, text, &LayoutOptions::new(width, 10.0)).unwrap() {
            assert!(line.width <= f64::from(width) + 1e-9, "{width}: {line:?}");
        }
    }
}

/// A6: text with nothing to draw lays out with no fonts.
#[test]
fn a6_breaks_and_ignorables_need_no_font() {
    assert!(itemize(&BidiParagraph::new("\n\u{200D}", None), &[]).is_ok());
    assert!(break_lines(&HarfRust, &[], "\n", &LayoutOptions::new(100.0, 10.0)).is_ok());
}

/// A7: justification is reserved: asking for it is an error, not silently ignored.
#[test]
fn a7_justify_is_refused() {
    let mut options = LayoutOptions::new(100.0, 10.0);
    options.justify = true;
    let err = break_lines(&HarfRust, &[latin()], "a b", &options).unwrap_err();
    assert_eq!(err.code(), "unsupported-layout-option");
}

/// B1: empty text is a text like any other: a glyph first used for nothing keeps that, and a
/// later real text is a conflict (not a silent overwrite).
#[test]
fn b1_empty_then_real_text_conflicts() {
    let font = latin();
    let a = font.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let cid = fonts.use_glyph(key, a, "").unwrap();
    fonts.use_glyph(key, a, "a").unwrap();
    assert_eq!(
        fonts
            .usage(key)
            .unwrap()
            .embedded()
            .unwrap()
            .text()
            .get(&cid.cid().unwrap())
            .map(String::as_str),
        Some("")
    );
    assert!(matches!(fonts.warnings(), [Warning::ToUnicodeConflict { .. }]));
}

/// B1: a real text then an empty one is a conflict too.
#[test]
fn b1_real_then_empty_text_conflicts() {
    let font = latin();
    let a = font.glyph_for('a').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    fonts.use_glyph(key, a, "a").unwrap();
    fonts.use_glyph(key, a, "").unwrap();
    assert!(matches!(fonts.warnings(), [Warning::ToUnicodeConflict { .. }]));
}

/// B2: a default-ignorable character (ZWJ, drawn as a hidden space glyph) never becomes a
/// glyph's ToUnicode text, and the space glyph keeps " ".
#[test]
fn b2_default_ignorables_get_no_glyph_text() {
    let font = latin();
    let space = font.glyph_for(' ').unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font.clone(), EmbedPolicy::Respect).unwrap();
    for text in ["a\u{200D}b", "a b"] {
        let run = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
        pdf_run(&run, text, &mut fonts, key).unwrap();
    }
    let usage = fonts.usage(key).unwrap().embedded().unwrap();
    assert!(
        usage.text().values().all(|t| !t.contains('\u{200D}')),
        "{:?}",
        usage.text()
    );
    let space_cid = usage.glyphs().iter().find(|(_, g)| *g == space).unwrap().0;
    assert_eq!(usage.text()[&space_cid], " ");
}

/// B3: a cluster with a glyph that stands for no text gets `/ActualText`, so extractors that
/// honour it don't depend on the empty ToUnicode entry.
#[test]
fn b3_clusters_with_textless_glyphs_get_actual_text() {
    let font = font("NotoSansArabic-Regular.ttf");
    let text = "بالعالم";
    let run = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&run, text, &mut fonts, key).unwrap();
    let empty: Vec<u16> = fonts
        .usage(key)
        .unwrap()
        .embedded()
        .unwrap()
        .text()
        .iter()
        .filter(|(_, t)| t.is_empty())
        .map(|(c, _)| *c)
        .collect();
    assert!(!empty.is_empty(), "the fixture draws the beh's dot as its own glyph");
    for piece in &pdf.pieces {
        let codes: Vec<u16> = piece
            .segments
            .iter()
            .flat_map(|s| &s.tj)
            .filter_map(|e| match e {
                TjElement::Codes(c) => Some(
                    c.chunks(2)
                        .map(|p| u16::from_be_bytes([p[0], p[1]]))
                        .collect::<Vec<_>>(),
                ),
                TjElement::Adjust(_) => None,
            })
            .flatten()
            .collect();
        if codes.iter().any(|c| empty.contains(c)) {
            assert!(piece.actual_text.is_some(), "{piece:?}");
        }
    }
}

/// B4: the `TJ` corrections use the widths `/W` is written with, also for a variable-font
/// instance: drawing a run then reading `/W` back gives the shaper's advances.
#[test]
fn b4_tj_widths_match_w_for_a_variable_instance() {
    let instance = || font("NotoSans-VF.ttf").instance(&[(Tag::WEIGHT, 700.0)]).unwrap();
    let text = "Wave AVATAR office";
    let run = HarfRust.shape(&instance(), text, &ShapeOptions::new()).unwrap();
    let mut pdf = TestPdf::new();
    let key = pdf.add_font(instance()).unwrap();
    let lines = pdf.paragraph(&[key], text, &LayoutOptions::new(500.0, 10.0)).unwrap();
    assert_eq!(lines, [text]);
    let (bytes, _, _) = pdf.finish().unwrap();
    let fonts = read_fonts(&bytes);
    let widths = expand_widths(fonts[0].cid_font.get("W").unwrap().as_array().unwrap());
    let dw = fonts[0].cid_font.get("DW").map_or(1000.0, |d| d.as_f64().unwrap());
    // Recompute the pen from /W and the TJ numbers: it must land on the shaper's advance.
    let mut registry = FontRegistry::new();
    let key = registry.add(instance(), EmbedPolicy::Respect).unwrap();
    let pdf = pdf_run(&run, text, &mut registry, key).unwrap();
    let mut pen = 0.0;
    for element in pdf.pieces.iter().flat_map(|p| &p.segments).flat_map(|s| &s.tj) {
        match element {
            TjElement::Codes(codes) => {
                for pair in codes.chunks(2) {
                    let cid = u16::from_be_bytes([pair[0], pair[1]]);
                    pen += widths.get(&cid).copied().unwrap_or(dw);
                }
            }
            TjElement::Adjust(a) => pen -= a,
        }
    }
    let shaped: f64 =
        run.glyphs.iter().map(|g| f64::from(g.x_advance)).sum::<f64>() * 1000.0 / f64::from(run.units_per_em);
    assert!((pen - shaped).abs() < 1e-6, "{pen} vs {shaped}");
}

/// B5: a run shaped with another font is refused before anything is recorded.
#[test]
fn b5_runs_from_another_font_are_refused_without_side_effects() {
    let hebrew = font("NotoSansHebrew-Regular.ttf");
    let run = HarfRust.shape(&hebrew, "שלום", &ShapeOptions::new()).unwrap();
    let mut fonts = FontRegistry::new();
    let key = fonts.add(latin(), EmbedPolicy::Respect).unwrap();
    let err = pdf_run(&run, "שלום", &mut fonts, key).unwrap_err();
    assert_eq!(err.code(), "font-mismatch");
    assert_eq!(
        fonts.usage(key).unwrap().embedded().unwrap().glyphs().len(),
        1,
        "only .notdef"
    );
}

/// B6: a missing glyph names the character the font lacks, not the cluster's first.
#[test]
fn b6_missing_glyph_names_the_missing_character() {
    let font = latin();
    let text = "a\u{0360}";
    let run = HarfRust.shape(&font, text, &ShapeOptions::new()).unwrap();
    assert!(run.has_notdef());
    let mut fonts = FontRegistry::new();
    let key = fonts.add(font, EmbedPolicy::Respect).unwrap();
    match pdf_run(&run, text, &mut fonts, key).unwrap_err() {
        Error::MissingGlyph { ch, .. } => assert_eq!(ch, '\u{0360}'),
        other => panic!("{other:?}"),
    }
}

/// A7 (dead check) and C2: visual runs of a sub-range of a line match the reference
/// reordering of that range alone.
#[test]
fn c2_visual_runs_of_a_line_slice() {
    let text = "abc שלום def עולם";
    let para = BidiParagraph::new(text, None);
    let line = 4..text.len();
    let runs: Vec<(&str, u8)> = para
        .visual_runs(line)
        .unwrap()
        .iter()
        .map(|r| (&text[r.range.clone()], r.level))
        .collect();
    assert_eq!(runs, [("שלום", 1), (" def ", 0), ("עולם", 1)]);
}

/// Alignment still works with paragraph splitting: centred lines of both paragraphs.
#[test]
fn a1_alignment_applies_per_paragraph() {
    let lines = break_lines(
        &HarfRust,
        &[latin()],
        "ab\ncd",
        &LayoutOptions::new(100.0, 10.0).with_align(Alignment::Center),
    )
    .unwrap();
    for line in &lines {
        assert!((line.x * 2.0 + line.width - 100.0).abs() < 1e-9);
    }
}
