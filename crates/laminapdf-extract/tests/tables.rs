//! Tables on pages drawn for each rule: a ruled grid with a bold header, a merged cell, a table
//! split across two pages, a page border and a form's field boxes (no table), rules that don't
//! describe rectangles (a warning), and unruled columns (only on request).
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::fmt::Write as _;

use laminapdf_extract::{
    BlockKind, ChunkOptions, ExtractOptions, Extraction, MarkdownOptions, SplitOn, TableDetector, TableMode, extract,
};
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
    grid_with(out, xs, ys, cells, true);
}

/// [`grid`], with the first row in bold only when `bold_first`.
fn grid_with(out: &mut String, xs: &[f64], ys: &[f64], cells: &[&[&str]], bold_first: bool) {
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
            text(out, cell, xs[c] + 5.0, ys[r] - 14.0, bold_first && r == 0);
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
fn whole_tables_at_the_tops_of_two_pages_stay_apart() {
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
    // The first part ends at the top of its page, not its foot: no link.
    assert!(
        out.blocks
            .iter()
            .all(|b| b.continues.is_none() && b.continued_by.is_none())
    );
}

/// Rules at the foot of a page (the bottom one 60 pt above the page's edge) and at its top.
const FOOT: [f64; 4] = [160.0, 140.0, 120.0, 100.0];
const TOP: [f64; 4] = [760.0, 740.0, 720.0, 700.0];
const TOP_TWO_ROWS: [f64; 3] = [760.0, 740.0, 720.0];
const HEAD: &[&str] = &["Item", "Price"];

/// Page 1 ends with Item / Tea / Cake; `second` draws page 2.
fn split(second: impl FnOnce(&mut String)) -> Extraction {
    let mut first = String::new();
    grid(&mut first, &XS, &FOOT, &[HEAD, &["Tea", "2.50"], &["Cake", "4.00"]]);
    let mut next = String::new();
    second(&mut next);
    run(&pages(&[first, next]), &ExtractOptions::new())
}

fn tables(out: &Extraction) -> Vec<&laminapdf_extract::Block> {
    out.blocks
        .iter()
        .filter(|b| matches!(b.kind, BlockKind::Table { .. }))
        .collect()
}

fn linked(out: &Extraction) -> bool {
    let parts = tables(out);
    parts.len() == 2
        && parts[0].continued_by.as_deref() == Some(parts[1].id.as_str())
        && parts[1].continues.as_deref() == Some(parts[0].id.as_str())
}

const JOINED: &str =
    "| Item | Price |\n| --- | --- |\n| Tea | 2.50 |\n| Cake | 4.00 |\n| Bread | 3.00 |\n| Jam | 1.50 |\n";

#[test]
fn a_header_repeated_on_the_next_page_joins_the_parts_into_one_table() {
    let out = split(|c| grid(c, &XS, &TOP, &[HEAD, &["Bread", "3.00"], &["Jam", "1.50"]]));
    assert!(linked(&out), "{:?}", out.blocks);
    let repeated: Vec<bool> = tables(&out)[1].cells.iter().map(|c| c.header).collect();
    assert_eq!(repeated, [true, true, false, false, false, false]);
    assert_eq!(out.to_markdown(&MarkdownOptions::new()).text, JOINED);
    assert_eq!(
        out.to_text().text,
        "Item\tPrice\nTea\t2.50\nCake\t4.00\nBread\t3.00\nJam\t1.50\n"
    );
}

#[test]
fn rows_without_a_header_on_the_next_page_join() {
    let out = split(|c| grid_with(c, &XS, &TOP_TWO_ROWS, &[&["Bread", "3.00"], &["Jam", "1.50"]], false));
    assert!(linked(&out), "{:?}", out.blocks);
    assert_eq!(out.to_markdown(&MarkdownOptions::new()).text, JOINED);
}

#[test]
fn a_continued_line_between_the_parts_still_joins_them() {
    let out = split(|c| {
        text(c, "Table 1 (continued)", 50.0, 775.0, false);
        grid_with(c, &XS, &TOP_TWO_ROWS, &[&["Bread", "3.00"], &["Jam", "1.50"]], false);
    });
    assert!(linked(&out), "{:?}", out.blocks);
    assert_eq!(out.to_markdown(&MarkdownOptions::new()).text, JOINED);
    assert_eq!(out.to_text().text.matches("\n\n").count(), 0);
    // The line stays a paragraph in the blocks.
    assert!(
        out.blocks
            .iter()
            .any(|b| matches!(b.kind, BlockKind::Paragraph) && b.text == "Table 1 (continued)")
    );
}

#[test]
fn parts_that_differ_or_are_apart_stay_two_tables() {
    let narrower = [50.0, 180.0, 330.0];
    let cases: [(&str, Extraction); 3] = [
        (
            "narrower columns",
            split(|c| {
                grid_with(
                    c,
                    &narrower,
                    &TOP_TWO_ROWS,
                    &[&["Bread", "3.00"], &["Jam", "1.50"]],
                    false,
                );
            }),
        ),
        (
            "another header",
            split(|c| grid(c, &XS, &TOP, &[&["Name", "Cost"], &["Bread", "3.00"], &["Jam", "1.50"]])),
        ),
        ("a paragraph under the first part", {
            let mut first = String::new();
            grid(&mut first, &XS, &FOOT, &[HEAD, &["Tea", "2.50"], &["Cake", "4.00"]]);
            text(&mut first, "Prices include tax.", 50.0, 80.0, false);
            let mut next = String::new();
            grid_with(
                &mut next,
                &XS,
                &TOP_TWO_ROWS,
                &[&["Bread", "3.00"], &["Jam", "1.50"]],
                false,
            );
            run(&pages(&[first, next]), &ExtractOptions::new())
        }),
    ];
    for (case, out) in &cases {
        assert_eq!(tables(out).len(), 2, "{case}");
        assert!(
            out.blocks
                .iter()
                .all(|b| b.continues.is_none() && b.continued_by.is_none()),
            "{case}: {:?}",
            out.blocks
        );
    }
}

#[test]
fn a_table_over_three_pages_with_page_numbers_is_one_chain() {
    let number = |c: &mut String, n: &str| text(c, n, 300.0, 30.0, false);
    let mut first = String::new();
    grid(&mut first, &XS, &FOOT, &[HEAD, &["Tea", "2.50"], &["Cake", "4.00"]]);
    number(&mut first, "1");
    let mut middle = String::new();
    grid_with(
        &mut middle,
        &XS,
        &[760.0, 410.0, 60.0],
        &[&["Bread", "3.00"], &["Jam", "1.50"]],
        false,
    );
    number(&mut middle, "2");
    let mut last = String::new();
    grid_with(
        &mut last,
        &XS,
        &TOP_TWO_ROWS,
        &[&["Milk", "1.20"], &["Eggs", "2.80"]],
        false,
    );
    number(&mut last, "3");
    let out = run(&pages(&[first, middle, last]), &ExtractOptions::new());
    let parts = tables(&out);
    assert_eq!(parts.len(), 3, "{:?}", out.blocks);
    assert_eq!(parts[0].continued_by.as_deref(), Some(parts[1].id.as_str()));
    assert_eq!(parts[1].continues.as_deref(), Some(parts[0].id.as_str()));
    assert_eq!(parts[1].continued_by.as_deref(), Some(parts[2].id.as_str()));
    assert_eq!(parts[2].continues.as_deref(), Some(parts[1].id.as_str()));
    assert_eq!(out.to_text().text.matches("\n\n").count(), 0, "{}", out.to_text().text);
}

#[test]
fn a_chain_with_a_spanning_cell_is_one_html_table_and_its_page_marker_follows_it() {
    let mut first = String::new();
    // The middle vertical rule stops under the first row: the header spans both columns.
    for &y in &FOOT {
        line(&mut first, 50.0, y, 350.0, y);
    }
    line(&mut first, 50.0, 100.0, 50.0, 160.0);
    line(&mut first, 350.0, 100.0, 350.0, 160.0);
    line(&mut first, 200.0, 100.0, 200.0, 140.0);
    text(&mut first, "Prices", 55.0, 146.0, true);
    text(&mut first, "Tea", 55.0, 126.0, false);
    text(&mut first, "2.50", 205.0, 126.0, false);
    text(&mut first, "Cake", 55.0, 106.0, false);
    text(&mut first, "4.00", 205.0, 106.0, false);
    let mut next = String::new();
    grid_with(
        &mut next,
        &XS,
        &TOP_TWO_ROWS,
        &[&["Bread", "3.00"], &["Jam", "1.50"]],
        false,
    );
    text(&mut next, "Prices change daily.", 50.0, 600.0, false);
    let out = run(&pages(&[first, next]), &ExtractOptions::new());
    assert!(linked(&out), "{:?}", out.blocks);
    let markdown = out.to_markdown(&MarkdownOptions::new().with_page_markers(true));
    assert_eq!(markdown.text.matches("<table>").count(), 1, "{}", markdown.text);
    assert_eq!(markdown.text.matches("</table>").count(), 1, "{}", markdown.text);
    assert!(
        markdown
            .text
            .contains("<tr><td>Jam</td><td>1.50</td></tr>\n</table>\n\n<!-- page 2 -->\n\nPrices change daily."),
        "{}",
        markdown.text
    );
    for (i, range) in markdown.ranges.iter().enumerate() {
        assert_eq!(markdown.slice(*range), markdown.block(i), "block {i}");
    }
}

#[test]
fn chunks_keep_a_chain_whole_whatever_the_split() {
    let out = split(|c| grid(c, &XS, &TOP, &[HEAD, &["Bread", "3.00"], &["Jam", "1.50"]]));
    let markdown = out.to_markdown(&MarkdownOptions::new());
    for options in [
        ChunkOptions::new().with_split_on(SplitOn::Pages),
        ChunkOptions::new().with_max_tokens(1).with_overlap(1),
    ] {
        let chunks = out.chunks(&markdown, &options);
        assert_eq!(chunks.len(), 1, "{options:?}: {chunks:?}");
        assert_eq!(chunks[0].pages, [0, 1]);
        assert_eq!(markdown.slice(chunks[0].text_range), chunks[0].text);
    }
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
    let markdown = out.to_markdown(&MarkdownOptions::new()).text;
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

/// The grid of `XS` and `YS` drawn cell by cell as closed outlines painted with `s` (close and
/// stroke), which reports no `h` of its own.
fn closed_outlines(out: &mut String) {
    for r in 0..YS.len() - 1 {
        for c in 0..XS.len() - 1 {
            let (x0, x1, y0, y1) = (XS[c], XS[c + 1], YS[r + 1], YS[r]);
            writeln!(out, "{x0} {y0} m {x1} {y0} l {x1} {y1} l {x0} {y1} l s").unwrap();
        }
    }
}

#[test]
fn cells_outlined_with_close_and_stroke_are_a_table() {
    let mut c = String::new();
    closed_outlines(&mut c);
    for (r, row) in [["Item", "Price"], ["Tea", "2.50"], ["Cake", "4.00"]]
        .iter()
        .enumerate()
    {
        for (col, cell) in row.iter().enumerate() {
            text(&mut c, cell, XS[col] + 5.0, YS[r] - 14.0, false);
        }
    }
    let out = run(&pages(&[c]), &ExtractOptions::new());
    assert_eq!(cell_texts(&out, 0), ["Item", "Price", "Tea", "2.50", "Cake", "4.00"]);
}

#[test]
fn borders_drawn_as_thin_filled_shapes_are_rules() {
    let mut c = String::new();
    // Each rule a 0.5 pt quadrilateral built with m, l and h, then filled.
    for &y in &YS {
        writeln!(
            c,
            "{} {} m {} {} l {} {} l {} {} l h f",
            XS[0],
            y,
            XS[2],
            y,
            XS[2],
            y + 0.5,
            XS[0],
            y + 0.5
        )
        .unwrap();
    }
    for &x in &XS {
        writeln!(
            c,
            "{} {} m {} {} l {} {} l {} {} l h f",
            x,
            YS[3],
            x + 0.5,
            YS[3],
            x + 0.5,
            YS[0],
            x,
            YS[0]
        )
        .unwrap();
    }
    for (r, row) in [["Item", "Price"], ["Tea", "2.50"], ["Cake", "4.00"]]
        .iter()
        .enumerate()
    {
        for (col, cell) in row.iter().enumerate() {
            text(&mut c, cell, XS[col] + 5.0, YS[r] - 14.0, false);
        }
    }
    let out = run(&pages(&[c]), &ExtractOptions::new());
    assert_eq!(cell_texts(&out, 0), ["Item", "Price", "Tea", "2.50", "Cake", "4.00"]);
}

#[test]
fn each_piece_of_text_is_in_one_place_only() {
    let mut c = String::new();
    text(&mut c, "Before the table", 50.0, 740.0, false);
    grid(
        &mut c,
        &XS,
        &YS,
        &[&["Item", "Price"], &["Tea", "2.50"], &["Cake", "4.00"]],
    );
    let out = run(&pages(&[c]), &ExtractOptions::new());
    let markdown = out.to_markdown(&MarkdownOptions::new()).text;
    for word in ["Before the table", "Item", "Price", "Tea", "2.50", "Cake", "4.00"] {
        assert_eq!(markdown.matches(word).count(), 1, "{word} in {markdown}");
    }
}
