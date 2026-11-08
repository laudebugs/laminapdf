//! The `covered` flag (todo T2a, ADR 0030 §4): text painted over by a later opaque fill, image or
//! annotation leaves the blocks, and nothing translucent, earlier, optional or masked covers.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf_build::{BuildOptions, Builder, Image, PageSize, Standard14};
use laminapdf_extract::{DropReason, Dropped, ExtractOptions, Extraction, extract};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt, RectPt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::CancelToken;
use laminapdf_test_fixtures::{classic, fixture, fixtures_dir, stream_body};

/// "Pay John Smith now." in Helvetica 12 at (72, 700): "Pay " is 24.01 pt wide, so "John Smith"
/// runs from x 96.01 to 156.04, its glyph boxes from y 697.5 to 708.6.
const SENTENCE: &[u8] = b"BT /F1 12 Tf 72 700 Td (Pay John Smith now.) Tj ET";

/// A black box over "John Smith".
const BOX: &[u8] = b"95 694 62 16 re f";

/// A page drawing `content` with Helvetica as `/F1` and Helvetica-Bold as `/F2`, more resource
/// entries `resources`, the annotations `annots` (an array's contents) and objects from 7 on.
fn page(content: &[u8], resources: &str, annots: &str, objects: &[Vec<u8>]) -> Document {
    let stream = stream_body("", content);
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Annots [{annots}] \
         /Resources << /Font << /F1 5 0 R /F2 6 0 R >> {resources} >> >>"
    );
    let mut bodies: Vec<&[u8]> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page.as_bytes(),
        &stream,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>",
    ];
    bodies.extend(objects.iter().map(Vec::as_slice));
    let trailer = format!("<< /Size {} /Root 1 0 R >>", bodies.len() + 1);
    Document::open(classic(&bodies, &trailer), OpenOptions::default()).unwrap()
}

fn content(parts: &[&[u8]]) -> Vec<u8> {
    parts.join(&b' ')
}

fn run(doc: &Document, options: &ExtractOptions) -> Extraction {
    extract(doc, options, &CancelToken::new()).unwrap()
}

fn texts(out: &Extraction) -> Vec<&str> {
    out.blocks.iter().map(|b| b.text.as_str()).collect()
}

fn covered(out: &Extraction) -> Vec<&Dropped> {
    out.dropped.iter().filter(|d| d.reason == DropReason::Covered).collect()
}

/// The blocks' text, and the covered entries' character counts.
fn read(doc: &Document) -> (Vec<String>, Vec<usize>) {
    let out = run(doc, &ExtractOptions::new());
    let texts = texts(&out).into_iter().map(str::to_owned).collect();
    (texts, covered(&out).iter().map(|d| d.chars).collect())
}

/// `doc` reads `Pay now.`, with "John Smith" (9 characters) covered.
fn hidden(doc: &Document) {
    assert_eq!(read(doc), (vec!["Pay now.".to_owned()], vec![9]));
}

/// `doc` reads the whole sentence, nothing covered.
fn kept(doc: &Document) {
    assert_eq!(read(doc), (vec!["Pay John Smith now.".to_owned()], vec![]));
}

#[test]
fn a_black_box_over_two_words_drops_them_and_keeps_the_line() {
    let doc = page(&content(&[SENTENCE, b"0 g", BOX]), "", "", &[]);
    let out = run(&doc, &ExtractOptions::new());
    assert_eq!(texts(&out), ["Pay now."]);
    let entry = covered(&out);
    assert_eq!(entry.len(), 1);
    assert_eq!((entry[0].page, entry[0].chars), (0, 9));
    // The box around the two words: x 96.01 → 156.04 in page space.
    let b = entry[0].bbox;
    assert!(
        (b.x0.get() - 96.01).abs() < 0.1 && (b.x1.get() - 156.04).abs() < 0.1,
        "{b:?}"
    );
}

#[test]
fn keep_spans_shows_the_covered_words_to_an_auditor() {
    let doc = page(&content(&[SENTENCE, b"0 g", BOX]), "", "", &[]);
    let mut options = ExtractOptions::new();
    options.drop.keep_spans = true;
    let out = run(&doc, &options);
    let spans: Vec<_> = covered(&out)
        .iter()
        .flat_map(|d| d.spans.iter())
        .map(|s| s.text.as_str())
        .collect();
    assert_eq!(spans, ["John Smith"]);
    assert!(covered(&out)[0].spans[0].flags.covered);
}

#[test]
fn covered_words_in_a_run_of_their_own_leave_one_line() {
    let doc = page(
        &content(&[
            b"BT /F1 12 Tf 72 700 Td (Pay ) Tj /F2 12 Tf (John Smith) Tj /F1 12 Tf ( now.) Tj ET 0 g",
            b"95 694 66 16 re f",
        ]),
        "",
        "",
        &[],
    );
    assert_eq!(read(&doc).0, ["Pay now."]);
}

#[test]
fn a_white_box_covers_too() {
    hidden(&page(&content(&[SENTENCE, b"1 g", BOX]), "", "", &[]));
}

#[test]
fn a_near_black_multiply_fill_covers_black_text() {
    let doc = page(
        &content(&[SENTENCE, b"/GSm gs 0.05 g", BOX]),
        "/ExtGState << /GSm << /BM /Multiply >> >>",
        "",
        &[],
    );
    hidden(&doc);
}

#[test]
fn text_drawn_twice_with_the_box_over_the_first_copy_comes_out_once() {
    let doc = page(
        &content(&[SENTENCE, b"0 g 70 694 120 16 re f 1 g", SENTENCE]),
        "",
        "",
        &[],
    );
    let (texts, chars) = read(&doc);
    assert_eq!(texts, ["Pay John Smith now."]);
    assert_eq!(chars, [16]);
}

/// A Square annotation over "John Smith" whose appearance fills its box in black, with
/// `extra` entries.
fn square(extra: &str) -> Document {
    let appearance = stream_body("/Type /XObject /Subtype /Form /BBox [0 0 62 16]", b"0 g 0 0 62 16 re f");
    let annot = format!("<< /Type /Annot /Subtype /Square /Rect [95 694 157 710] /AP << /N 8 0 R >> {extra} >>");
    page(SENTENCE, "", "7 0 R", &[annot.into_bytes(), appearance])
}

#[test]
fn an_opaque_annotation_appearance_covers() {
    hidden(&square(""));
}

#[test]
fn hidden_and_translucent_annotations_cover_nothing() {
    kept(&square("/F 2"));
    kept(&square("/F 32"));
    kept(&square("/CA 0.5"));
}

#[test]
fn a_translucent_box_covers_nothing() {
    kept(&page(
        &content(&[SENTENCE, b"/GSh gs 0 g", BOX]),
        "/ExtGState << /GSh << /ca 0.5 >> >>",
        "",
        &[],
    ));
}

#[test]
fn a_yellow_multiply_highlight_covers_nothing() {
    kept(&page(
        &content(&[SENTENCE, b"/GSm gs 1 1 0 rg", BOX]),
        "/ExtGState << /GSm << /BM /Multiply >> >>",
        "",
        &[],
    ));
}

#[test]
fn a_box_in_optional_content_covers_nothing() {
    kept(&page(
        &content(&[SENTENCE, b"/OC /oc1 BDC 0 g", BOX, b"EMC"]),
        "/Properties << /oc1 << /Type /OCG /Name (Layer) >> >>",
        "",
        &[],
    ));
}

#[test]
fn a_box_in_a_group_drawn_translucently_covers_nothing() {
    let form = stream_body(
        "/Type /XObject /Subtype /Form /BBox [0 0 600 800] /Group << /S /Transparency >> \
         /Resources << /ExtGState << /GSa << /ca 1 >> >> >>",
        b"/GSa gs 0 g 95 694 62 16 re f",
    );
    kept(&page(
        &content(&[SENTENCE, b"/GSh gs /Fm1 Do"]),
        "/ExtGState << /GSh << /ca 0.5 >> >> /XObject << /Fm1 7 0 R >>",
        "",
        &[form],
    ));
}

#[test]
fn shading_drawn_before_a_row_and_under_the_descenders_above_covers_nothing() {
    let doc = page(
        &content(&[
            b"0.9 g 60 694 300 16 re f 0 g",
            SENTENCE,
            // The next row's shading reaches 1 pt into the descenders above, then its text.
            b"0.9 g 60 680 300 18.5 re f 0 g",
            b"BT /F1 12 Tf 72 685 Td (Next row.) Tj ET",
        ]),
        "",
        "",
        &[],
    );
    assert_eq!(read(&doc), (vec!["Pay John Smith now. Next row.".to_owned()], vec![]));
}

/// "Pay John Smith now." built at (72, 100) on A4, then `image` drawn in `rect`.
fn built_with_image(image: &Image, rect: RectPt) -> Document {
    let mut b = Builder::new(BuildOptions::default());
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.text(
        "Pay John Smith now.",
        &font,
        Pt::new(12.0),
        PointPt::from_points(72.0, 100.0),
    )
    .unwrap();
    s.image(image, rect);
    page.finish().unwrap();
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

/// Over "John Smith" in the built page: x 95 → 157, y 89 → 105.
fn over_two_words() -> RectPt {
    RectPt::from_points(95.0, 89.0, 62.0, 16.0)
}

#[test]
fn an_opaque_jpeg_covers() {
    let jpeg = Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap();
    hidden(&built_with_image(&jpeg, over_two_words()));
}

#[test]
fn an_image_with_a_soft_mask_covers_nothing() {
    let rgba = Image::rgba(2, 2, &[0, 0, 0, 128].repeat(4)).unwrap();
    kept(&built_with_image(&rgba, over_two_words()));
}

#[test]
fn a_full_page_scan_covers_nothing() {
    let jpeg = Image::jpeg(fixture("images/photo-rgb.jpg")).unwrap();
    kept(&built_with_image(&jpeg, RectPt::from_points(0.0, 0.0, 595.28, 841.89)));
}

/// Every PDF under `dir`, recursively.
fn pdfs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            pdfs(&path, out);
        } else if path.extension().is_some_and(|x| x == "pdf") {
            out.push(path);
        }
    }
}

#[test]
fn no_fixture_has_covered_text() {
    let mut files = Vec::new();
    for dir in ["corpus", "golden", "reading-order"] {
        pdfs(&fixtures_dir().join(dir), &mut files);
    }
    files.sort();
    assert!(files.len() > 20, "found {} files", files.len());
    for path in files {
        let Ok(doc) = Document::open(std::fs::read(&path).unwrap(), OpenOptions::default()) else {
            continue;
        };
        let Ok(out) = extract(&doc, &ExtractOptions::new(), &CancelToken::new()) else {
            continue;
        };
        assert_eq!(covered(&out), Vec::<&Dropped>::new(), "{}", path.display());
    }
}

// Review round 1.

#[test]
fn an_l_shape_covers_nothing_in_its_notch() {
    // The L's box holds "John Smith", but the notch at its top right is left unpainted.
    let l = b"0 g 95 694 m 157 694 l 157 700 l 100 700 l 100 710 l 95 710 l f";
    kept(&page(&content(&[SENTENCE, l]), "", "", &[]));
}

#[test]
fn a_pattern_fill_covers_nothing() {
    kept(&page(
        &content(&[SENTENCE, b"/Pattern cs /P1 scn", BOX]),
        "/Pattern << /P1 << /PatternType 2 /Shading << /ShadingType 2 /ColorSpace /DeviceGray \
         /Coords [0 0 1 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [0] /C1 [1] /N 1 >> >> >> >>",
        "",
        &[],
    ));
}

#[test]
fn a_scan_stored_as_strips_covers_nothing() {
    // Two images, each half the page, drawn over the text layer.
    let image = stream_body(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
        b"\x80",
    );
    let doc = page(
        &content(&[
            SENTENCE,
            b"q 600 0 0 400 0 0 cm /Im1 Do Q q 600 0 0 400 0 400 cm /Im1 Do Q",
        ]),
        "/XObject << /Im1 7 0 R >>",
        "",
        &[image],
    );
    kept(&doc);
}

#[test]
fn an_annotation_in_optional_content_or_another_blend_mode_covers_nothing() {
    kept(&square("/OC << /Type /OCG /Name (Layer) >>"));
    kept(&square("/BM /Multiply"));
}

#[test]
fn a_cover_far_past_the_page_is_cut_to_it() {
    // Without the cut, the grid index would hold 1e11 cells.
    let doc = page(
        &content(&[SENTENCE, b"0 g -10000000 -10000000 20000000 20000000 re f"]),
        "",
        "",
        &[],
    );
    let (texts, chars) = read(&doc);
    assert_eq!((texts.len(), chars), (0, vec![16]));
}

#[test]
fn a_cover_past_a_gutter_never_joins_the_columns() {
    // One run: "Left" at x 50, then the pen jumps 250 pt to "SECRET more", with "SECRET"
    // under a box.
    let doc = page(
        b"BT /F1 10 Tf 50 700 Td [(Left) -25000 (SECRET more)] TJ ET 0 g 314 694 46 16 re f",
        "",
        "",
        &[],
    );
    assert_eq!(read(&doc).0, ["Left", "more"]);
}

#[test]
fn a_cover_away_from_a_gap_adds_no_space_to_it() {
    // "here" and "." are runs of their own that touch; the box sits over "SECRET" before them.
    let doc = page(
        b"BT /F1 12 Tf 72 700 Td (The SECRET was here) Tj /F2 12 Tf (.) Tj ET 0 g 94 694 56 16 re f",
        "",
        "",
        &[],
    );
    assert_eq!(read(&doc).0, ["The was here."]);
}

#[test]
fn hidden_text_lists_the_covered_entry_extraction_drops() {
    let doc = page(&content(&[SENTENCE, b"0 g", BOX]), "", "", &[]);
    let hidden = laminapdf_extract::hidden_text(&doc, &ExtractOptions::new(), &CancelToken::new()).unwrap();
    let out = run(&doc, &ExtractOptions::new());
    assert_eq!(hidden, out.dropped);
    assert_eq!(
        hidden.iter().map(|d| (d.reason, d.chars)).collect::<Vec<_>>(),
        [(DropReason::Covered, 9)]
    );
}

#[test]
fn hidden_text_stops_when_cancelled() {
    let doc = page(SENTENCE, "", "", &[]);
    let cancel = CancelToken::new();
    cancel.cancel();
    let got = laminapdf_extract::hidden_text(&doc, &ExtractOptions::new(), &cancel);
    assert!(matches!(got, Err(laminapdf_extract::ExtractError::Cancelled)));
}

#[test]
fn hidden_text_finds_a_box_over_an_artifact() {
    let doc = page(&content(&[b"/Artifact BMC", SENTENCE, b"EMC 0 g", BOX]), "", "", &[]);
    let hidden = laminapdf_extract::hidden_text(&doc, &ExtractOptions::new(), &CancelToken::new()).unwrap();
    assert_eq!(
        hidden.iter().map(|d| (d.reason, d.chars)).collect::<Vec<_>>(),
        [(DropReason::Covered, 9)]
    );
}
