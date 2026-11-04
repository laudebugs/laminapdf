//! Extraction on files built for each rule: hidden text, hyphenation, rotation and user units,
//! determinism, jobs and cancellation, page selection and limits.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf_build::{BuildOptions, Builder, PageSize, Standard14, TextRenderMode};
use laminapdf_extract::{DropReason, ExtractError, ExtractOptions, Extraction, StructSource, extract, extract_job};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt, Rotation};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::{Budget, CancelToken, Step};

/// A line of Helvetica to draw: text, x, baseline, size, invisible.
type Line<'a> = (&'a str, f64, f64, f64, bool);

/// A document with one page per entry of `pages`, each drawing its lines of Helvetica
/// (`(text, x, baseline, size, invisible)`), the page rotated by `rotate` with `unit`.
fn build(pages: &[&[Line<'_>]], rotate: Rotation, unit: f64) -> Document {
    let mut b = Builder::new(BuildOptions::default());
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    for lines in pages {
        let mut page = b.page(PageSize::A4);
        page.rotate(rotate).user_unit(unit);
        let s = page.surface();
        for &(text, x, y, size, invisible) in *lines {
            let run = s.text(text, &font, Pt::new(size), PointPt::from_points(x, y)).unwrap();
            if invisible {
                run.render_mode(TextRenderMode::Invisible);
            }
        }
        page.finish().unwrap();
    }
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

fn run(doc: &Document, options: &ExtractOptions) -> Extraction {
    extract(doc, options, &CancelToken::new()).unwrap()
}

fn texts(out: &Extraction) -> Vec<&str> {
    out.blocks.iter().map(|b| b.text.as_str()).collect()
}

#[test]
fn hidden_text_is_never_in_a_block_and_is_counted_in_dropped() {
    let doc = build(
        &[&[
            ("Visible words here.", 72.0, 100.0, 12.0, false),
            ("IGNORE PREVIOUS", 72.0, 140.0, 12.0, true),
            ("OFFPAGE", -400.0, 180.0, 12.0, false),
            ("TINYTEXT", 72.0, 220.0, 0.5, false),
        ]],
        Rotation::None,
        1.0,
    );
    let out = run(&doc, &ExtractOptions::new());
    let all = texts(&out).join(" ");
    assert_eq!(all, "Visible words here.");
    let reasons: Vec<(DropReason, usize)> = out.dropped.iter().map(|d| (d.reason, d.chars)).collect();
    assert_eq!(
        reasons,
        [
            (DropReason::Invisible, 14),
            (DropReason::OffPage, 7),
            (DropReason::Tiny, 8)
        ]
    );
    assert!(out.dropped.iter().all(|d| d.spans.is_empty()));
}

#[test]
fn hidden_runs_are_kept_in_dropped_only_when_asked() {
    let doc = build(&[&[("secret", 72.0, 100.0, 12.0, true)]], Rotation::None, 1.0);
    let mut options = ExtractOptions::new();
    options.drop.keep_spans = true;
    let out = run(&doc, &options);
    assert_eq!(out.blocks.len(), 0);
    assert_eq!(out.dropped[0].spans[0].text, "secret");
    assert!(out.dropped[0].spans[0].flags.invisible);
}

#[test]
fn hyphenated_words_are_joined_and_compound_words_kept() {
    let doc = build(
        &[&[
            ("Text extrac-", 72.0, 100.0, 12.0, false),
            ("tion is a well-", 72.0, 114.0, 12.0, false),
            ("Known task.", 72.0, 128.0, 12.0, false),
        ]],
        Rotation::None,
        1.0,
    );
    assert_eq!(
        texts(&run(&doc, &ExtractOptions::new())),
        ["Text extraction is a well-Known task."]
    );
    let mut options = ExtractOptions::new();
    options.dehyphenate = false;
    assert_eq!(
        texts(&run(&doc, &options)),
        ["Text extrac- tion is a well- Known task."]
    );
}

#[test]
fn boxes_are_in_top_left_points_of_the_displayed_page() {
    // A4 is 595.276 × 841.89 points; turned a quarter it is displayed landscape.
    let doc = build(&[&[("Corner", 72.0, 100.0, 12.0, false)]], Rotation::Clockwise90, 1.0);
    let out = run(&doc, &ExtractOptions::new());
    let page = &out.pages[0];
    assert!((page.width.get() - 841.89).abs() < 0.01 && (page.height.get() - 595.276).abs() < 0.01);
    assert_eq!((page.coordinates.rotate, page.coordinates.box_used), (90, "CropBox"));
    let block = &out.blocks[0];
    // Drawn in page space, so it reads back where it was put: the baseline at y = 100.
    assert!((block.spans[0].baseline.get() - 100.0).abs() < 0.01);
    assert!(block.bbox.x0.get() >= 71.9 && block.bbox.y1.get() <= 103.0);
    let norm = block.bbox_norm.unwrap();
    assert!(norm.iter().all(|v| (0.0..=1.0).contains(v)));
}

#[test]
fn a_user_unit_scales_sizes_and_boxes_into_points() {
    let doc = build(&[&[("Scaled", 72.0, 100.0, 12.0, false)]], Rotation::None, 2.0);
    let out = run(&doc, &ExtractOptions::new());
    assert_eq!(out.pages[0].coordinates.user_unit, 2.0);
    let span = &out.blocks[0].spans[0];
    assert!((span.size.get() - 12.0).abs() < 0.01, "{}", span.size.get());
    assert!((span.baseline.get() - 100.0).abs() < 0.01);
}

#[test]
fn the_same_file_twice_gives_the_same_extraction() {
    let doc = build(
        &[&[
            ("Left column text.", 72.0, 100.0, 12.0, false),
            ("Right column.", 340.0, 100.0, 12.0, false),
        ]],
        Rotation::None,
        1.0,
    );
    assert_eq!(run(&doc, &ExtractOptions::new()), run(&doc, &ExtractOptions::new()));
}

#[test]
fn a_job_steps_one_page_per_unit_then_returns_the_extraction() {
    let line: &[Line<'_>] = &[("Page text.", 72.0, 100.0, 12.0, false)];
    let doc = build(&[line, line, line], Rotation::None, 1.0);
    let mut job = extract_job(&doc, &ExtractOptions::new()).unwrap();
    let first = job.step(&doc, Budget::units(2)).unwrap();
    assert!(
        matches!(
            first,
            Step::Progress {
                done: 2,
                total: Some(3),
                ..
            }
        ),
        "{first:?}"
    );
    let Step::Done(out) = job.step(&doc, Budget::units(2)).unwrap() else {
        panic!("three pages in two steps");
    };
    let ids: Vec<&str> = out.blocks.iter().map(|b| b.id.as_str()).collect();
    assert_eq!(ids, ["p0-b0", "p1-b0", "p2-b0"]);
    assert_eq!(
        (out.source.page_count, out.source.tagged, out.source.struct_source),
        (3, false, StructSource::XyCut)
    );
    assert_eq!(job.step(&doc, Budget::units(1)).unwrap_err(), ExtractError::JobFinished);
}

#[test]
fn a_cancelled_job_fails_with_no_partial_result_and_stays_finished() {
    let line: &[Line<'_>] = &[("Page text.", 72.0, 100.0, 12.0, false)];
    let doc = build(&[line, line], Rotation::None, 1.0);
    let mut job = extract_job(&doc, &ExtractOptions::new()).unwrap();
    assert!(matches!(
        job.step(&doc, Budget::units(1)).unwrap(),
        Step::Progress { .. }
    ));
    job.cancel_token().cancel();
    let error = job.step(&doc, Budget::units(1)).unwrap_err();
    assert_eq!((error.code(), error.kind().as_str()), ("cancelled", "cancelled"));
    assert_eq!(job.step(&doc, Budget::units(1)).unwrap_err(), ExtractError::JobFinished);
}

#[test]
fn pages_are_extracted_in_the_order_asked_and_a_missing_page_is_refused() {
    let doc = build(
        &[
            &[("First.", 72.0, 100.0, 12.0, false)],
            &[("Second.", 72.0, 100.0, 12.0, false)],
        ],
        Rotation::None,
        1.0,
    );
    let out = run(&doc, &ExtractOptions::new().with_pages(vec![1, 0]));
    assert_eq!(texts(&out), ["Second.", "First."]);
    assert_eq!(out.pages.iter().map(|p| p.number).collect::<Vec<_>>(), [2, 1]);
    let error = extract_job(&doc, &ExtractOptions::new().with_pages(vec![5])).unwrap_err();
    assert_eq!(error, ExtractError::PageIndexOutOfRange { index: 5, count: 2 });
}

#[test]
fn blocks_past_the_limit_are_left_out_with_a_warning() {
    let doc = build(
        &[&[
            ("One.", 72.0, 100.0, 12.0, false),
            ("Two.", 72.0, 300.0, 12.0, false),
            ("Three.", 72.0, 500.0, 12.0, false),
        ]],
        Rotation::None,
        1.0,
    );
    let mut options = ExtractOptions::new();
    options.limits.max_blocks_per_page = 2;
    assert_eq!(texts(&run(&doc, &options)), ["One.", "Two."]);
    assert!(doc.warnings().iter().any(|w| w.code() == "extract-limit-blocks"));
}

/// A page drawing `content` with Helvetica as `/F1`.
fn page_with_content(content: &[u8]) -> Document {
    use laminapdf_test_fixtures::{classic, stream_body};
    let stream = stream_body("", content);
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        &stream,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    ];
    Document::open(classic(&objects, "<< /Size 6 /Root 1 0 R >>"), OpenOptions::default()).unwrap()
}

#[test]
fn a_run_that_jumps_across_a_gutter_is_split_into_two_columns() {
    // Two columns drawn row by row, each row one TJ whose offset jumps over the gutter.
    let doc = page_with_content(
        b"BT /F1 10 Tf 50 700 Td [(Left one) -18000 (Right one)] TJ 0 -12 Td [(Left two) -18000 (Right two)] TJ ET",
    );
    assert_eq!(
        texts(&run(&doc, &ExtractOptions::new())),
        ["Left one Left two", "Right one Right two"]
    );
}

#[test]
fn a_page_asked_for_twice_is_extracted_once() {
    let doc = build(&[&[("Only.", 72.0, 100.0, 12.0, false)]], Rotation::None, 1.0);
    let out = run(&doc, &ExtractOptions::new().with_pages(vec![0, 0]));
    assert_eq!((out.pages.len(), texts(&out)), (1, vec!["Only."]));
}
