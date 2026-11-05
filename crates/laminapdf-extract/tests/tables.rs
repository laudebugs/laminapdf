//! Tables on pages drawn for each rule: a ruled grid with a bold header, a merged cell, a table
//! split across two pages, a page border and a form's field boxes (no table), rules that don't
//! describe rectangles (a warning), and unruled columns (only on request).
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::fmt::Write as _;

use laminapdf_extract::{BlockKind, ExtractOptions, Extraction, TableDetector, TableMode, extract};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::CancelToken;
use laminapdf_test_fixtures::{classic, stream_body};

/// Pages of 600 × 800 pt drawing `contents`, with Helvetica as `/F1` and Helvetica-Bold as
/// `/F2`.
fn pages(contents: &[String]) -> Document {
    let n = contents.len();
    let mut kids = String::new();
    for i in 0..n {
        write!(kids, "{} 0 R ", 3 + 2 * i).unwrap();
    }
    let font1 = 3 + 2 * n;
    let mut objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!("<< /Type /Pages /Kids [{kids}] /Count {n} >>").into_bytes(),
    ];
    for (i, content) in contents.iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents {} 0 R \
                 /Resources << /Font << /F1 {font1} 0 R /F2 {} 0 R >> >> >>",
                4 + 2 * i,
                font1 + 1
            )
            .into_bytes(),
        );
        objects.push(stream_body("", content.as_bytes()));
    }
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>".to_vec());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>".to_vec());
    let refs: Vec<&[u8]> = objects.iter().map(Vec::as_slice).collect();
    let trailer = format!("<< /Size {} /Root 1 0 R >>", objects.len() + 1);
    Document::open(classic(&refs, &trailer), OpenOptions::default()).unwrap()
}

/// Text at `(x, y)` (PDF user space, y up) in `/F1` (or bold `/F2`), 10 pt.
fn text(out: &mut String, s: &str, x: f64, y: f64, bold: bool) {
    let font = if bold { "F2" } else { "F1" };
    writeln!(out, "BT /{font} 10 Tf {x} {y} Td ({s}) Tj ET").unwrap();
}

/// A stroked line from `(x0, y0)` to `(x1, y1)`.
fn line(out: &mut String, x0: f64, y0: f64, x1: f64, y1: f64) {
    writeln!(out, "{x0} {y0} m {x1} {y1} l S").unwrap();
}

/// A grid ruled at `xs` and `ys` (every line full length), with `cells` (row by row) written
/// in each cell's top left, the first row in bold.
fn grid(out: &mut String, xs: &[f64], ys: &[f64], cells: &[&[&str]]) {
    let (x0, x1) = (xs[0], xs[xs.len() - 1]);
    let (y0, y1) = (ys[ys.len() - 1], ys[0]);
    for &y in ys {
        line(out, x0, y, x1, y);
    }
    for &x in xs {
        line(out, x, y0, x, y1);
    }
    for (r, row) in cells.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            text(out, cell, xs[c] + 5.0, ys[r] - 14.0, r == 0);
        }
    }
}

fn run(doc: &Document, options: &ExtractOptions) -> Extraction {
    extract(doc, options, &CancelToken::new()).unwrap()
}

fn cell_texts(out: &Extraction, block: usize) -> Vec<&str> {
    out.blocks[block].cells.iter().map(|c| c.text.as_str()).collect()
}

const XS: [f64; 3] = [50.0, 200.0, 350.0];
const YS: [f64; 4] = [700.0, 680.0, 660.0, 640.0];

#[test]
fn a_ruled_grid_is_a_table_with_its_cells_header_and_place_in_reading_order() {
    let mut c = String::new();
    text(&mut c, "Prices this week", 50.0, 740.0, false);
    grid(
        &mut c,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Tea", "2.50"], &["Cake", "4.00"]],
    );
    text(&mut c, "Prices change daily.", 50.0, 600.0, false);
    let out = run(&pages(&[c]), &ExtractOptions::new());
    let kinds: Vec<&BlockKind> = out.blocks.iter().map(|b| &b.kind).collect();
    assert!(matches!(kinds[0], BlockKind::Paragraph));
    assert!(matches!(
        kinds[1],
        BlockKind::Table {
            rows: 3,
            cols: 2,
            detector: TableDetector::Lattice
        }
    ));
    assert!(matches!(kinds[2], BlockKind::Paragraph));
    assert_eq!(cell_texts(&out, 1), ["Item", "Price", "Tea", "2.50", "Cake", "4.00"]);
    let headers: Vec<bool> = out.blocks[1].cells.iter().map(|c| c.header).collect();
    assert_eq!(headers, [true, true, false, false, false, false]);
    assert_eq!(out.blocks[1].text, "Item\tPrice\nTea\t2.50\nCake\t4.00");
    // The cells' text is in no other block.
    assert!(
        out.blocks
            .iter()
            .filter(|b| b.cells.is_empty())
            .all(|b| !b.text.contains("Tea"))
    );
    // Top-left page space: the table's top is 800 − 700 = 100 pt from the top.
    assert!((out.blocks[1].bbox.y0.get() - 100.0).abs() < 0.01);
}

#[test]
fn a_missing_inner_rule_merges_cells_into_one_spanning_them() {
    let mut c = String::new();
    // The middle vertical rule stops under the first row: the header spans both columns.
    for &y in &YS {
        line(&mut c, 50.0, y, 350.0, y);
    }
    line(&mut c, 50.0, 640.0, 50.0, 700.0);
    line(&mut c, 350.0, 640.0, 350.0, 700.0);
    line(&mut c, 200.0, 640.0, 200.0, 680.0);
    text(&mut c, "Prices", 55.0, 686.0, true);
    for (r, (a, b)) in [("Tea", "2.50"), ("Cake", "4.00")].iter().enumerate() {
        let y = 666.0 - 20.0 * f64::from(u8::try_from(r).unwrap());
        text(&mut c, a, 55.0, y, false);
        text(&mut c, b, 205.0, y, false);
    }
    let out = run(&pages(&[c]), &ExtractOptions::new());
    let table = &out.blocks[0];
    assert_eq!(table.cells.len(), 5);
    assert_eq!((table.cells[0].row_span, table.cells[0].col_span), (1, 2));
    assert_eq!(table.cells[0].text, "Prices");
    assert!(table.cells[0].header && !table.cells[1].header);
}

#[test]
fn a_table_split_across_a_page_break_is_two_tables() {
    let mut first = String::new();
    grid(
        &mut first,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Tea", "2.50"], &["Cake", "4.00"]],
    );
    let mut second = String::new();
    grid(
        &mut second,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Bread", "3.00"], &["Jam", "1.50"]],
    );
    let out = run(&pages(&[first, second]), &ExtractOptions::new());
    let tables: Vec<(usize, &str)> = out
        .blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::Table { .. }))
        .map(|b| (b.page, b.cells[2].text.as_str()))
        .collect();
    assert_eq!(tables, [(0, "Tea"), (1, "Bread")]);
}

#[test]
fn a_page_border_and_a_forms_field_boxes_are_not_tables() {
    let mut c = String::new();
    writeln!(c, "20 20 560 760 re S").unwrap();
    // Two field boxes side by side, each with its label: one row, not a grid.
    writeln!(c, "50 640 150 30 re S 200 640 150 30 re S").unwrap();
    text(&mut c, "Name", 55.0, 655.0, false);
    text(&mut c, "Date", 205.0, 655.0, false);
    let out = run(&pages(&[c]), &ExtractOptions::new());
    assert!(out.blocks.iter().all(|b| b.cells.is_empty()), "{:?}", out.blocks);
}

#[test]
fn rules_that_describe_no_rectangular_cells_keep_the_grid_with_a_warning() {
    let mut c = String::new();
    // An L-shaped merge: no middle vertical on row 0, no middle horizontal under column 0.
    line(&mut c, 50.0, 700.0, 350.0, 700.0);
    line(&mut c, 200.0, 680.0, 350.0, 680.0);
    line(&mut c, 50.0, 660.0, 350.0, 660.0);
    line(&mut c, 50.0, 660.0, 50.0, 700.0);
    line(&mut c, 200.0, 660.0, 200.0, 680.0);
    line(&mut c, 350.0, 660.0, 350.0, 700.0);
    text(&mut c, "A", 55.0, 686.0, false);
    text(&mut c, "B", 205.0, 686.0, false);
    text(&mut c, "C", 205.0, 666.0, false);
    let doc = pages(&[c]);
    let out = run(&doc, &ExtractOptions::new());
    assert_eq!(out.blocks[0].cells.len(), 4);
    assert!(doc.warnings().iter().any(|w| w.code() == "extract-table-ambiguous"));
}

#[test]
fn unruled_columns_are_a_table_only_when_stream_tables_are_asked_for() {
    let mut c = String::new();
    for (r, (a, b)) in [("Item", "Price"), ("Tea", "2.50"), ("Cake", "4.00")]
        .iter()
        .enumerate()
    {
        let y = 700.0 - 14.0 * f64::from(u8::try_from(r).unwrap());
        text(&mut c, a, 50.0, y, false);
        text(&mut c, b, 300.0, y, false);
    }
    let doc = pages(&[c]);
    let plain = run(&doc, &ExtractOptions::new());
    assert!(plain.blocks.iter().all(|b| b.cells.is_empty()));
    let stream = run(&doc, &ExtractOptions::new().with_tables(TableMode::LatticeAndStream));
    let table = stream.blocks.iter().find(|b| !b.cells.is_empty()).unwrap();
    assert!(matches!(
        table.kind,
        BlockKind::Table {
            rows: 3,
            cols: 2,
            detector: TableDetector::Stream
        }
    ));
    assert!(table.confidence < 0.5);
}

#[test]
fn tables_off_gives_text_blocks_only() {
    let mut c = String::new();
    grid(
        &mut c,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Tea", "2.50"], &["Cake", "4.00"]],
    );
    let out = run(&pages(&[c]), &ExtractOptions::new().with_tables(TableMode::Off));
    assert!(out.blocks.iter().all(|b| b.cells.is_empty()));
    assert!(out.blocks.iter().any(|b| b.text.contains("Tea")));
}

#[test]
fn a_table_is_a_pipe_table_in_markdown_between_its_paragraphs() {
    let mut c = String::new();
    text(&mut c, "Prices this week", 50.0, 740.0, false);
    grid(
        &mut c,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Tea", "2.50"], &["Cake", "4.00"]],
    );
    let out = run(&pages(&[c]), &ExtractOptions::new());
    let markdown = out.to_markdown(&laminapdf_extract::MarkdownOptions::new()).text;
    assert_eq!(
        markdown,
        "Prices this week\n\n| Item | Price |\n| --- | --- |\n| Tea | 2.50 |\n| Cake | 4.00 |\n"
    );
}

#[test]
fn a_grid_past_the_cell_limit_gives_no_table_and_a_warning() {
    let mut c = String::new();
    grid(
        &mut c,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Tea", "2.50"], &["Cake", "4.00"]],
    );
    let doc = pages(&[c]);
    let mut options = ExtractOptions::new();
    options.limits.max_table_cells = 4;
    let out = run(&doc, &options);
    assert!(out.blocks.iter().all(|b| b.cells.is_empty()));
    assert!(
        out.blocks.iter().any(|b| b.text.contains("Tea")),
        "the text stays in the paragraphs"
    );
    assert!(doc.warnings().iter().any(|w| w.code() == "extract-table-limit"));
}
