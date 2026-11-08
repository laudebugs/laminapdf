//! `Glyph::paints_before` and `TextExtractor::page_with`'s forwarded events (todo T2a): the
//! count that orders glyphs against paints, and the same text with a visitor or without one.
#![allow(clippy::expect_used, reason = "tests: a panic is a test failure")]

use laminapdf_content::{FormXObject, ImageDraw, MarkedContent, PathPaint, ShadingPaint, Visitor};
use laminapdf_cos::OpenOptions;
use laminapdf_text::TextExtractor;

mod common;

use common::extract::{extract, helvetica, page};

#[test]
fn paints_before_counts_the_fills_and_strokes_before_each_glyph() {
    // `n` ends a path without painting it, so it doesn't count.
    let content = b"BT /F1 10 Tf 72 700 Td (A) Tj ET \
        0 0 10 10 re f 0 0 10 10 re n \
        BT /F1 10 Tf 72 680 Td (B) Tj ET \
        0 0 m 10 10 l S 0 0 10 10 re B \
        BT /F1 10 Tf 72 660 Td (C) Tj ET";
    let text = extract(&page(content, vec![helvetica()], OpenOptions::default())).expect("extracts");
    let counts: Vec<_> = text.glyphs.iter().map(|g| (g.text.as_str(), g.paints_before)).collect();
    assert_eq!(counts, [("A", 0), ("B", 1), ("C", 3)]);
}

/// Counts what `page_with` forwards and checks marked content and forms are balanced.
#[derive(Debug, Default)]
struct Forwarded {
    paints: u32,
    marked: i64,
    forms: i64,
}

impl Visitor for Forwarded {
    fn paint_path(&mut self, paint: &PathPaint<'_>) {
        if paint.paint.draws() {
            self.paints += 1;
        }
    }

    fn image(&mut self, _: &ImageDraw<'_>) {
        self.paints += 1;
    }

    fn paint_shading(&mut self, _: &ShadingPaint<'_>) {
        self.paints += 1;
    }

    fn begin_marked(&mut self, _: &MarkedContent<'_>) {
        self.marked += 1;
    }

    fn end_marked(&mut self) {
        self.marked -= 1;
    }

    fn enter_xobject(&mut self, _: &FormXObject<'_>) {
        self.forms += 1;
    }

    fn leave_xobject(&mut self, _: &FormXObject<'_>) {
        self.forms -= 1;
    }
}

#[test]
fn page_and_page_with_return_the_same_text_on_every_corpus_file() {
    for (name, bytes) in laminapdf_test_fixtures::corpus() {
        let Ok(doc) = laminapdf_model::Document::open(bytes, OpenOptions::default()) else {
            continue;
        };
        let Ok(pages) = doc.pages() else {
            continue;
        };
        for (index, page) in pages.iter().enumerate() {
            let plain = TextExtractor::new().page(&doc, &page);
            let mut forwarded = Forwarded::default();
            let with = TextExtractor::new().page_with(&doc, &page, &mut forwarded);
            assert_eq!(plain.ok(), with.as_ref().ok().cloned(), "{name} page {index}");
            if with.is_ok() {
                assert_eq!(
                    (forwarded.marked, forwarded.forms),
                    (0, 0),
                    "{name} page {index}: balanced"
                );
            }
            // Every glyph's count is at most the paints the visitor saw.
            let last = with.ok().and_then(|t| t.glyphs.last().map(|g| g.paints_before));
            assert!(last.is_none_or(|n| n <= forwarded.paints), "{name} page {index}");
        }
    }
}
