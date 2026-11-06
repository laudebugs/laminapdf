//! Reading order and kinds from the structure tree (ADR 0030 §2–§3), on tagged files built for
//! each rule: a tree whose order differs from the layout's, a tree that covers too little,
//! untagged text beside a tree, and tagged lists and tables.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]

use laminapdf_build::{BuildOptions, Builder, FontHandle, PageSize, Standard14, Surface, TagKind};
use laminapdf_cos::Warning;
use laminapdf_extract::{BlockKind, ExtractOptions, Extraction, ReadingOrder, StructSource, TableDetector, extract};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::CancelToken;

/// A tagged one-page document drawn by `draw`.
fn build(draw: impl FnOnce(&mut Surface, &FontHandle)) -> Document {
    let mut b = Builder::new(BuildOptions::default().with_tagged(true));
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font);
    page.finish().unwrap();
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

/// Draws `text` at `(x, y)` inside a `kind` element.
fn tagged(s: &mut Surface, font: &FontHandle, kind: TagKind, text: &str, x: f64, y: f64) {
    s.tag(kind, |s| {
        s.text(text, font, Pt::new(11.0), PointPt::from_points(x, y))
            .map(|_| ())
    })
    .unwrap();
}

fn run(doc: &Document, order: ReadingOrder) -> Extraction {
    extract(
        doc,
        &ExtractOptions::new().with_reading_order(order),
        &CancelToken::new(),
    )
    .unwrap()
}

fn texts(out: &Extraction) -> Vec<&str> {
    out.blocks.iter().map(|b| b.text.as_str()).collect()
}

fn codes(doc: &Document) -> Vec<&'static str> {
    doc.warnings().iter().map(Warning::code).collect()
}

const BODY: &str = "The body of the report, long enough to outweigh its title on the page.";

#[test]
fn a_plausible_tree_gives_the_order_and_the_kinds() {
    // Drawn and tagged body first, title second: the tree reads the body first, the layout
    // would read the title first.
    let doc = build(|s, font| {
        tagged(s, font, TagKind::P, BODY, 72.0, 200.0);
        tagged(s, font, TagKind::H1, "Report", 72.0, 100.0);
    });
    let out = run(&doc, ReadingOrder::Auto);
    assert_eq!(out.source.struct_source, StructSource::Tagged);
    assert_eq!(texts(&out), [BODY, "Report"]);
    let heading = &out.blocks[1];
    assert_eq!(heading.kind, BlockKind::Heading { level: 1 });
    assert_eq!(
        (heading.confidence, heading.struct_path.as_slice()),
        (1.0, &["Document".to_owned(), "H1".to_owned()][..])
    );
    assert!(heading.struct_ref.as_ref().is_some_and(|r| r.mcids.len() == 1));
    assert_eq!(texts(&run(&doc, ReadingOrder::Layout)), ["Report", BODY]);
}

#[test]
fn a_tree_covering_too_little_is_ignored_unless_forced() {
    let doc = build(|s, font| {
        tagged(s, font, TagKind::H1, "Report", 72.0, 200.0);
        s.text(BODY, font, Pt::new(11.0), PointPt::from_points(72.0, 100.0))
            .unwrap();
    });
    let out = run(&doc, ReadingOrder::Auto);
    assert_eq!(out.source.struct_source, StructSource::XyCut);
    assert_eq!(texts(&out), [BODY, "Report"]);
    assert!(
        codes(&doc).contains(&"extract-struct-tree-ignored"),
        "{:?}",
        codes(&doc)
    );
    // Forced: the tree's block first, the untagged text after it.
    let forced = run(&doc, ReadingOrder::Tagged);
    assert_eq!(texts(&forced), ["Report", BODY]);
    assert!(codes(&doc).contains(&"extract-untagged-content"));
}

#[test]
fn untagged_text_beside_a_tree_follows_its_blocks() {
    let doc = build(|s, font| {
        s.text("DRAFT", font, Pt::new(11.0), PointPt::from_points(72.0, 50.0))
            .unwrap();
        tagged(s, font, TagKind::P, BODY, 72.0, 100.0);
        tagged(s, font, TagKind::P, BODY, 72.0, 150.0);
    });
    let out = run(&doc, ReadingOrder::Auto);
    assert_eq!(out.source.struct_source, StructSource::Tagged);
    assert_eq!(texts(&out), [BODY, BODY, "DRAFT"]);
    assert!(out.blocks[2].struct_ref.is_none() && out.blocks[2].confidence < 1.0);
    assert!(codes(&doc).contains(&"extract-untagged-content"));
}

#[test]
fn tagged_lists_and_tables_keep_their_structure() {
    let doc = build(|s, font| {
        s.tag(TagKind::L, |s| {
            for (y, label, body) in [(100.0, "1.", "Sow early."), (120.0, "2.", "Water often.")] {
                s.tag(TagKind::LI, |s| {
                    tagged(s, font, TagKind::Lbl, label, 72.0, y);
                    tagged(s, font, TagKind::LBody, body, 90.0, y);
                    Ok(())
                })?;
            }
            Ok(())
        })
        .unwrap();
        s.tag(TagKind::Table, |s| {
            for (y, kind, cells) in [
                (200.0, TagKind::TH, ["Crop", "Yield"]),
                (220.0, TagKind::TD, ["Beans", "12"]),
            ] {
                s.tag(TagKind::TR, |s| {
                    tagged(s, font, kind, cells[0], 72.0, y);
                    tagged(s, font, kind, cells[1], 200.0, y);
                    Ok(())
                })?;
            }
            Ok(())
        })
        .unwrap();
    });
    let out = run(&doc, ReadingOrder::Auto);
    let kinds: Vec<&BlockKind> = out.blocks.iter().map(|b| &b.kind).collect();
    assert_eq!(
        kinds,
        [
            &BlockKind::ListItem {
                marker: "1.".to_owned(),
                depth: 0
            },
            &BlockKind::ListItem {
                marker: "2.".to_owned(),
                depth: 0
            },
            &BlockKind::Table {
                rows: 2,
                cols: 2,
                detector: TableDetector::Tagged
            },
        ]
    );
    assert_eq!(texts(&out), ["Sow early.", "Water often.", "Crop\tYield\nBeans\t12"]);
    let cells: Vec<(usize, usize, bool, &str)> = out.blocks[2]
        .cells
        .iter()
        .map(|c| (c.row, c.col, c.header, c.text.as_str()))
        .collect();
    assert_eq!(
        cells,
        [
            (0, 0, true, "Crop"),
            (0, 1, true, "Yield"),
            (1, 0, false, "Beans"),
            (1, 1, false, "12")
        ]
    );
}
