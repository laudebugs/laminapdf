//! The `extract_text` fuzz target's entry point (ADR 0018 tier 9; todo B9c).
//! `fuzz/fuzz_targets/extract_text.rs` calls it (through `fuzz/src/lib.rs`, which includes this
//! file), and `tests/regressions.rs` replays every saved crash input
//! (`fixtures/regressions/extract_text/`) through it.

use lamina_cos::{Dict, Object, Stream};
use lamina_model::{Document, OpenOptions};
use lamina_save::Writer;
use lamina_text::TextExtractor;

/// Extracts the text of `data`: a whole file (starting `%PDF`) page by page; anything else is
/// both the page content and the `/ToUnicode` CMap of [`fixture_file`], so the content
/// interpreter and the CMap parser both see it. Invariants: no panic, no hang, every run's glyph
/// range inside the page's glyphs, every quad coordinate finite, and `lines` and `visible`
/// never panic.
pub fn extract_text(data: &[u8]) {
    let bytes = if data.starts_with(b"%PDF") {
        data.to_vec()
    } else {
        fixture_file(data)
    };
    let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
        return;
    };
    let Ok(pages) = doc.pages() else {
        return;
    };
    let mut extractor = TextExtractor::new();
    for page in pages.iter() {
        let Ok(text) = extractor.page(&doc, &page) else {
            continue;
        };
        for run in &text.runs {
            assert!(run.glyphs.start <= run.glyphs.end && run.glyphs.end <= text.glyphs.len(), "run glyphs in range");
        }
        for glyph in &text.glyphs {
            for p in glyph.quad {
                assert!(p.x.get().is_finite() && p.y.get().is_finite(), "finite quad");
            }
        }
        let _ = text.lines();
        let _ = text.visible().lines();
    }
}

fn dict<const N: usize>(entries: [(&str, Object); N]) -> Dict {
    entries.into_iter().collect()
}

/// A one-page file whose page content is `data`, with a simple font `F1` (Helvetica with
/// `/Differences`) and a Type 0 font `F2` (Identity-H) whose `/ToUnicode` CMap is `data` too.
pub fn fixture_file(data: &[u8]) -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let f1 = w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
        (
            "Encoding",
            dict([
                ("Type", Object::name("Encoding")),
                ("BaseEncoding", Object::name("WinAnsiEncoding")),
                ("Differences", vec![65.into(), Object::name("Alpha"), Object::name("f_i")].into()),
            ])
            .into(),
        ),
    ]));
    let cmap = w.add(Stream::new(Dict::new(), data.to_vec()));
    let cid = w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("CIDFontType2")),
        ("BaseFont", Object::name("Fuzz")),
        (
            "CIDSystemInfo",
            dict([
                ("Registry", Object::String(lamina_cos::PdfString::literal("Adobe"))),
                ("Ordering", Object::String(lamina_cos::PdfString::literal("Identity"))),
                ("Supplement", 0.into()),
            ])
            .into(),
        ),
        ("DW", 500.into()),
    ]));
    let f2 = w.add(dict([
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type0")),
        ("BaseFont", Object::name("Fuzz")),
        ("Encoding", Object::name("Identity-H")),
        ("DescendantFonts", vec![cid.into()].into()),
        ("ToUnicode", cmap.into()),
    ]));
    let content = w.add(Stream::new(Dict::new(), data.to_vec()));
    let resources = dict([("Font", dict([("F1", f1.into()), ("F2", f2.into())]).into())]);
    let page = w.add(dict([
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ("Resources", resources.into()),
        ("Contents", content.into()),
    ]));
    w.set(
        pages,
        dict([
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]),
    );
    let catalog = w.add(dict([("Type", Object::name("Catalog")), ("Pages", pages.into())]));
    w.finish(&dict([("Root", catalog.into())])).unwrap_or_default()
}
