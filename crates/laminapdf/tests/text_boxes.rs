//! Text boxes and callouts through the façade: made from page-space boxes, upright on turned
//! pages, in a font given by its bytes, refused when they can't be drawn, an Acrobat-style box's
//! rich text simplified when its text changes, and every kind of box read back.
#![expect(clippy::unwrap_used, clippy::panic, reason = "test code: a panic is a test failure")]

use laminapdf::{
    AnnotFont, AnnotSpec, AnnotSpecPatch, Document, FreeTextIntent, FreeTextPatch, OpenOptions, PointPt, RectPt,
    RichText, SaveOptions, TextAlign, TextSpan, UpdateAnnot, Warning,
};
use laminapdf_test_fixtures::fixture;

/// A one-page document whose page is turned by `rotate`, 612 × 792 points unturned, with
/// `annots` as its annotations (objects 4 onward).
fn page(rotate: u16, annots: &[&str]) -> Document {
    let refs: Vec<String> = (0..annots.len()).map(|i| format!("{} 0 R", 4 + i)).collect();
    let page = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate {rotate} /Annots [{}] >>",
        refs.join(" ")
    );
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        page,
    ];
    objects.extend(annots.iter().map(|a| (*a).to_owned()));
    let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
    let bytes = laminapdf_test_fixtures::classic_with_root(&objects, "");
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

fn area() -> RectPt {
    RectPt::from_points(72.0, 72.0, 200.0, 40.0)
}

fn free_text(spec: &AnnotSpec) -> &laminapdf::FreeText {
    match spec {
        AnnotSpec::FreeText(t) => t,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_box_on_a_turned_page_is_upright_and_covers_the_box_shown() {
    let doc = page(90, &[]);
    let op = doc.text_box_at(0, area(), RichText::plain("Upright")).unwrap();
    let text = free_text(&op.spec);
    // On a page turned a quarter, a box 200 wide on screen is 200 tall in user space.
    assert_eq!(
        (text.rotation, text.rect.y1 - text.rect.y0, text.rect.x1 - text.rect.x0),
        (90, 200.0, 40.0)
    );
}

#[test]
fn a_box_saves_with_its_text_as_contents_and_reads_back() {
    let mut doc = page(0, &[]);
    let op = doc.text_box_at(0, area(), RichText::plain("Saved\ntext")).unwrap();
    doc.apply(vec![op.into()]).unwrap();
    let bytes = doc.save(&SaveOptions::new()).unwrap().bytes;
    let reopened = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    let annots = reopened.page_annotations(0).unwrap();
    let boxes: Vec<_> = annots.iter().filter(|a| a.subtype == "FreeText").collect();
    assert_eq!(
        boxes.iter().map(|a| a.contents.clone()).collect::<Vec<_>>(),
        [Some("Saved\rtext".to_owned())]
    );
}

#[test]
fn a_callout_takes_its_line_in_page_space() {
    let doc = page(0, &[]);
    let line = [PointPt::from_points(300.0, 300.0), PointPt::from_points(272.0, 92.0)];
    let op = doc.callout_at(0, area(), &line, RichText::plain("Look")).unwrap();
    let FreeTextIntent::Callout(callout) = &free_text(&op.spec).intent else {
        panic!("not a callout");
    };
    // y is flipped: 300 down from the top of a 792-point page is 492 up from the bottom.
    assert_eq!((callout.line[0].x, callout.line[0].y), (300.0, 492.0));
}

#[test]
fn a_box_in_a_font_given_by_its_bytes_is_embedded() {
    let mut doc = page(0, &[]);
    let font = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let mut op = doc.text_box_at(0, area(), RichText::plain("Noto Sans")).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        t.font = AnnotFont::Font(font);
    }
    doc.apply(vec![op.into()]).unwrap();
    let bytes = doc.save(&SaveOptions::new()).unwrap().bytes;
    assert!(bytes.windows(9).any(|w| w == b"FontFile2"), "no embedded font program");
}

#[test]
fn a_font_the_session_lacks_is_an_unknown_blob() {
    let mut doc = page(0, &[]);
    let mut op = doc.text_box_at(0, area(), RichText::plain("x")).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        t.font = AnnotFont::Font(laminapdf::BlobId::of(b"never added"));
    }
    assert_eq!(doc.apply(vec![op.into()]).unwrap_err().code(), "unknown-blob");
}

#[test]
fn bytes_that_aren_t_a_font_are_refused() {
    let mut doc = page(0, &[]);
    let not_a_font = doc.add_blob(b"not a font".to_vec());
    let mut op = doc.text_box_at(0, area(), RichText::plain("x")).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        t.font = AnnotFont::Font(not_a_font);
    }
    assert_eq!(
        doc.apply(vec![op.into()]).unwrap_err().code(),
        "annotation-font-invalid"
    );
}

#[test]
fn a_character_no_font_has_is_refused() {
    let mut doc = page(0, &[]);
    let op = doc.text_box_at(0, area(), RichText::plain("日本")).unwrap();
    assert_eq!(
        doc.apply(vec![op.into()]).unwrap_err().code(),
        "annotation-glyph-missing"
    );
}

#[test]
fn a_fallback_font_draws_what_helvetica_lacks() {
    let mut doc = page(0, &[]);
    let chinese = doc.add_blob(fixture("fonts/NotoSansSC-Regular.otf"));
    let mut op = doc.text_box_at(0, area(), RichText::plain("Hello 你好")).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        t.fallbacks = vec![chinese];
    }
    assert!(doc.apply(vec![op.into()]).unwrap().changed);
}

/// An Acrobat-style box: `/RC` colours one word red, which the subset can't keep.
const ACROBAT_BOX: &str = "<< /Type /Annot /Subtype /FreeText /Rect [72 600 272 640] /DA (0 0 0 rg /Helv 12 Tf) \
     /Contents (A red word) /RC (<?xml version=\"1.0\"?><body xmlns=\"http://www.w3.org/1999/xhtml\" \
     style=\"font-size:12pt;color:#000000\"><p>A <span style=\"color:#FF0000\">red</span> word</p></body>) >>";

#[test]
fn changing_the_text_of_a_box_with_styles_the_subset_drops_warns() {
    let mut doc = page(0, &[ACROBAT_BOX]);
    let id = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let mut update = UpdateAnnot::new(id);
    update.spec = Some(AnnotSpecPatch::FreeText(FreeTextPatch::text(RichText::plain(
        "A new text",
    ))));
    let edited = doc.apply(vec![update.into()]).unwrap();
    let codes: Vec<&str> = edited.warnings.iter().map(Warning::code).collect();
    assert_eq!(codes, ["rich-text-simplified"]);
}

#[test]
fn a_new_colour_alone_also_writes_the_box_again_in_the_subset_and_warns() {
    let mut doc = page(0, &[ACROBAT_BOX]);
    let id = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let mut update = UpdateAnnot::new(id);
    update.color = Some(Some(laminapdf::AnnotColor::Rgb(0.0, 0.0, 1.0)));
    let edited = doc.apply(vec![update.into()]).unwrap();
    let codes: Vec<&str> = edited.warnings.iter().map(Warning::code).collect();
    assert_eq!(codes, ["rich-text-simplified"]);
}

#[test]
fn a_box_within_the_subset_never_warns() {
    let mut doc = page(0, &[]);
    let op = doc.text_box_at(0, area(), RichText::plain("Plain")).unwrap();
    let id = doc.apply(vec![op.into()]).unwrap().created_annotations[0].clone();
    let mut update = UpdateAnnot::new(id);
    update.spec = Some(AnnotSpecPatch::FreeText(FreeTextPatch::text(RichText::plain(
        "Changed",
    ))));
    assert_eq!(doc.apply(vec![update.into()]).unwrap().warnings, Vec::<Warning>::new());
}

#[test]
fn typewriter_text_runs_past_its_box_on_one_line_and_its_form_grows() {
    // Acrobat's typewriter tool never wraps: a box narrower than its text keeps the line whole.
    let mut doc = page(
        0,
        &[
            "<< /Type /Annot /Subtype /FreeText /Rect [100 600 140 620] /IT /FreeTextTypewriter \
           /DA (/Helv 12 Tf 0 g) /Contents (short) >>",
        ],
    );
    let id = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let mut update = UpdateAnnot::new(id);
    update.spec = Some(AnnotSpecPatch::FreeText(FreeTextPatch::text(RichText::plain(
        "A line much wider than its box",
    ))));
    doc.apply(vec![update.into()]).unwrap();
    let bytes = doc.save(&SaveOptions::new()).unwrap().bytes;
    let text = String::from_utf8_lossy(&bytes);
    let at = text.rfind("/BBox [").unwrap() + "/BBox [".len();
    let numbers: Vec<f64> = text[at..]
        .split(']')
        .next()
        .unwrap()
        .split_whitespace()
        .map(|n| n.parse().unwrap())
        .collect();
    let [x0, y0, x1, y1] = numbers[..] else {
        panic!("{numbers:?}")
    };
    // Helvetica 12 sets the line about 170 points wide, on one line: no taller than the box.
    assert!(x1 - x0 > 150.0, "{numbers:?}");
    assert!(y1 - y0 < 25.0, "{numbers:?}");
}

#[test]
fn a_box_keeps_its_fallback_fonts_when_drawn_again() {
    let mut doc = page(0, &[]);
    let chinese = doc.add_blob(fixture("fonts/NotoSansSC-Regular.otf"));
    let mut op = doc.text_box_at(0, area(), RichText::plain("Hi 你好")).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        t.fallbacks = vec![chinese];
    }
    let id = doc.apply(vec![op.into()]).unwrap().created_annotations[0].clone();
    // Only the colour changes: the box is read back from its keys, fallbacks included.
    let mut update = UpdateAnnot::new(id);
    update.color = Some(Some(laminapdf::AnnotColor::Rgb(1.0, 0.0, 0.0)));
    assert!(doc.apply(vec![update.into()]).unwrap().changed);
}

#[test]
fn a_box_whose_contents_are_locked_keeps_its_text() {
    let mut doc = page(
        0,
        &[
            "<< /Type /Annot /Subtype /FreeText /Rect [72 600 272 640] /F 516 /DA (/Helv 12 Tf 0 g) \
           /Contents (Locked) >>",
        ],
    );
    let id: laminapdf::AnnotId = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let mut update = UpdateAnnot::new(id.clone());
    update.spec = Some(AnnotSpecPatch::FreeText(FreeTextPatch::text(RichText::plain("New"))));
    assert_eq!(doc.apply(vec![update.into()]).unwrap_err().code(), "annotation-locked");
    // Its colour isn't its contents.
    let mut update = UpdateAnnot::new(id);
    update.color = Some(Some(laminapdf::AnnotColor::Rgb(1.0, 0.0, 0.0)));
    assert!(doc.apply(vec![update.into()]).unwrap().changed);
}

#[test]
fn a_box_s_contents_change_only_with_its_text() {
    let mut doc = page(0, &[]);
    let op = doc.text_box_at(0, area(), RichText::plain("Text")).unwrap();
    let id = doc.apply(vec![op.into()]).unwrap().created_annotations[0].clone();
    let mut update = UpdateAnnot::new(id);
    update.contents = Some(Some("Other".to_owned()));
    assert_eq!(doc.apply(vec![update.into()]).unwrap_err().code(), "invalid-op");
}

#[test]
fn rich_text_in_a_stream_is_read() {
    let rc = laminapdf_test_fixtures::stream_body(
        "",
        "<body xmlns=\"http://www.w3.org/1999/xhtml\"><p>A <span style=\"color:#FF0000\">red</span> word</p></body>"
            .as_bytes(),
    );
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [4 0 R] >>".to_owned(),
        "<< /Type /Annot /Subtype /FreeText /Rect [72 600 272 640] /DA (/Helv 12 Tf 0 g) \
         /Contents (Plain copy) /RC 5 0 R >>"
            .to_owned(),
        String::from_utf8(rc).unwrap(),
    ];
    let objects: Vec<&str> = objects.iter().map(String::as_str).collect();
    let bytes = laminapdf_test_fixtures::classic_with_root(&objects, "");
    let mut doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    let id = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let mut update = UpdateAnnot::new(id);
    update.color = Some(Some(laminapdf::AnnotColor::Rgb(0.0, 0.0, 1.0)));
    let edited = doc.apply(vec![update.into()]).unwrap();
    // The stream's red word, which the subset drops, is read: the text is its, not /Contents'.
    let codes: Vec<&str> = edited.warnings.iter().map(Warning::code).collect();
    assert_eq!(codes, ["rich-text-simplified"]);
    let saved = doc.save(&SaveOptions::new()).unwrap().bytes;
    assert!(saved.windows(10).any(|w| w == b"A red word"));
}

/// The box `op` adds, applied to `doc` and read back.
fn applied_and_read(doc: &mut Document, op: laminapdf::AddAnnot) -> laminapdf::TextBox {
    let id = doc
        .apply(vec![op.into()])
        .unwrap()
        .created_annotations
        .first()
        .unwrap()
        .clone();
    doc.text_box(&id).unwrap().unwrap()
}

#[test]
fn a_box_reads_back_as_the_spec_it_was_written_from() {
    let mut doc = page(0, &[]);
    let text = RichText {
        paragraphs: vec![vec![
            TextSpan::plain("Check "),
            TextSpan::plain("this").with_bold(true).with_italic(true),
        ]],
    };
    let mut op = doc.text_box_at(0, area(), text).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        *t = t.clone().with_align(TextAlign::Center);
    }
    let spec = free_text(&op.spec).clone();
    let read = applied_and_read(&mut doc, op);
    assert_eq!((read.spec, read.simplified), (spec, false));
}

#[test]
fn a_box_on_a_turned_page_reads_back_where_it_is_shown() {
    let mut doc = page(90, &[]);
    let op = doc.text_box_at(0, area(), RichText::plain("Upright")).unwrap();
    let read = applied_and_read(&mut doc, op);
    assert_eq!((read.page, read.rect), (0, area()));
}

#[test]
fn a_callout_reads_back_with_its_line_in_page_space() {
    let mut doc = page(0, &[]);
    let line = [PointPt::from_points(300.0, 300.0), PointPt::from_points(272.0, 92.0)];
    let op = doc.callout_at(0, area(), &line, RichText::plain("Look")).unwrap();
    let read = applied_and_read(&mut doc, op);
    assert_eq!(read.line, line);
}

#[test]
fn typewriter_text_reads_back_as_typewriter_text() {
    let mut doc = page(0, &[]);
    let mut op = doc.text_box_at(0, area(), RichText::plain("Typed")).unwrap();
    if let AnnotSpec::FreeText(t) = &mut op.spec {
        *t = t.clone().with_intent(FreeTextIntent::Typewriter);
    }
    let read = applied_and_read(&mut doc, op);
    assert_eq!(read.spec.intent, FreeTextIntent::Typewriter);
}

#[test]
fn another_app_s_box_reads_with_styles_outside_the_subset_dropped() {
    let doc = page(0, &[ACROBAT_BOX]);
    let id = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    let read = doc.text_box(&id).unwrap().unwrap();
    assert_eq!(
        (read.spec.text.to_plain(), read.color, read.simplified),
        (
            "A red word".to_owned(),
            Some(laminapdf::AnnotColor::Rgb(0.0, 0.0, 0.0)),
            true
        )
    );
}

#[test]
fn an_annotation_of_another_kind_is_no_text_box() {
    let doc = page(0, &["<< /Type /Annot /Subtype /Square /Rect [72 600 272 640] >>"]);
    let id = doc.page_annotations(0).unwrap()[0].id.parse().unwrap();
    assert_eq!(doc.text_box(&id).unwrap(), None);
}
