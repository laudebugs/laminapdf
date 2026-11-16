use laminapdf_cos::{Dict, Object, PdfString};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_syntax::text::decode_text_string;

use super::super::AnnotSpec;
use super::super::freetext_keys::{parse_css, parse_da, read_rc, write_rc};
use super::super::freetext_patch::FreeTextPatch;
use super::*;

fn rect() -> UserSpaceRect {
    UserSpaceRect::from_corners(100.0, 600.0, 300.0, 650.0)
}

fn styled() -> RichText {
    RichText {
        paragraphs: vec![
            vec![
                TextSpan::plain("Q2 is "),
                TextSpan::plain("late").with_bold(true),
                TextSpan::plain(" & <over>").with_italic(true),
            ],
            vec![TextSpan::plain("Second line")],
        ],
    }
}

fn text_key(dict: &Dict, key: &str) -> String {
    match dict.get(key) {
        Some(Object::String(s)) => decode_text_string(&s.bytes),
        other => panic!("{key}: {other:?}"),
    }
}

/// `text`'s keys written over a dictionary whose `/Rect` is the box grown by 2 on each side,
/// then read back.
fn round_trip(text: &FreeText, color: Option<AnnotColor>) -> (Dict, ReadBack) {
    let mut dict = Dict::new();
    dict.insert("Subtype", Object::name("FreeText"));
    let r = text.rect;
    dict.insert(
        "Rect",
        super::super::add::rect_object(&UserSpaceRect::from_corners(
            r.x0 - 2.0,
            r.y0 - 2.0,
            r.x1 + 2.0,
            r.y1 + 2.0,
        )),
    );
    super::super::add::write_spec(&mut dict, &AnnotSpec::FreeText(text.clone()), color);
    let read = read(&dict).unwrap();
    (dict, read)
}

#[test]
fn a_styled_box_reads_back_as_written() {
    let text = FreeText::new(rect(), styled())
        .with_font(AnnotFont::Standard(StandardFamily::Times), 14.0)
        .with_align(TextAlign::Center)
        .with_fill(Some(AnnotColor::Rgb(1.0, 1.0, 0.8)));
    let (_, read) = round_trip(&text, Some(AnnotColor::Rgb(1.0, 0.0, 0.0)));
    assert_eq!(
        (read.text, read.color, read.simplified),
        (text, Some(AnnotColor::Rgb(1.0, 0.0, 0.0)), false)
    );
}

#[test]
fn a_plain_box_writes_no_rich_text_and_its_text_as_contents() {
    let text = FreeText::new(rect(), RichText::plain("One\nTwo"));
    let (dict, read) = round_trip(&text, None);
    assert_eq!(
        (dict.get("RC").is_none(), text_key(&dict, "Contents"), read.text.text),
        (true, "One\rTwo".to_owned(), RichText::plain("One\nTwo"))
    );
}

#[test]
fn da_and_ds_name_the_font_size_colour_and_alignment() {
    let text = FreeText::new(rect(), RichText::plain("x"))
        .with_font(AnnotFont::Standard(StandardFamily::Courier), 9.5)
        .with_align(TextAlign::Right);
    let (dict, _) = round_trip(&text, Some(AnnotColor::Rgb(0.0, 0.0, 1.0)));
    assert_eq!(
        (text_key(&dict, "DA"), text_key(&dict, "DS")),
        (
            "/Cour 9.5 Tf 0 0 1 rg".to_owned(),
            "font: Courier,monospace 9.5pt; text-align:right; color:#0000FF".to_owned()
        )
    );
}

#[test]
fn a_callout_writes_its_intent_line_ending_and_inner_box() {
    let line = vec![
        UserSpacePoint::new(50.0, 500.0),
        UserSpacePoint::new(80.0, 620.0),
        UserSpacePoint::new(100.0, 620.0),
    ];
    let text = FreeText::new(rect(), RichText::plain("Look")).with_intent(FreeTextIntent::Callout(
        Callout::new(line).with_end(LineEnding::ClosedArrow),
    ));
    let (dict, read) = round_trip(&text, None);
    assert_eq!(
        (dict.get_name("IT"), dict.get("LE"), read.text),
        (
            Some(b"FreeTextCallout".as_slice()),
            Some(&Object::name("ClosedArrow")),
            text
        )
    );
}

#[test]
fn typewriter_text_has_no_border_or_fill() {
    let text = FreeText::new(rect(), RichText::plain("Jane Doe"))
        .with_intent(FreeTextIntent::Typewriter)
        .with_fill(Some(AnnotColor::Gray(0.9)));
    let (dict, read) = round_trip(&text, None);
    let width = dict
        .get("BS")
        .and_then(Object::as_dict)
        .and_then(|bs| bs.get("W"))
        .and_then(Object::as_f64);
    assert_eq!(
        (dict.get("C").is_none(), width, read.text.fill),
        (true, Some(0.0), None)
    );
}

#[test]
fn a_font_given_by_its_bytes_is_named_by_lamina_pdf_s_key() {
    let id = BlobId::of(b"font");
    let text = FreeText::new(rect(), RichText::plain("x")).with_font(AnnotFont::Font(id), 12.0);
    let (dict, read) = round_trip(&text, None);
    assert_eq!(
        (text_key(&dict, FONT_KEY), read.text.font),
        (id.to_string(), AnnotFont::Font(id))
    );
}

#[test]
fn acrobat_s_default_appearance_reads_with_the_colour_before_the_font() {
    let da = parse_da("0 0 1 rg /Helv 12 Tf");
    assert_eq!(
        (da.family, da.size, da.color, da.unknown_font),
        (
            Some(StandardFamily::Helvetica),
            Some(12.0),
            Some(AnnotColor::Rgb(0.0, 0.0, 1.0)),
            false
        )
    );
}

#[test]
fn a_font_lamina_pdf_cant_name_is_unknown_and_an_auto_size_is_none() {
    let da = parse_da("/F3 0 Tf 0.5 g");
    assert_eq!(
        (da.family, da.size, da.color, da.unknown_font),
        (None, None, Some(AnnotColor::Gray(0.5)), true)
    );
}

#[test]
fn the_default_style_reads_the_font_shorthand_alignment_and_colour() {
    let ds = parse_css("font: Times New Roman,serif 11.0pt; text-align:center; color:#F00");
    assert_eq!(
        (ds.family, ds.size, ds.align, ds.color),
        (
            Some(StandardFamily::Times),
            Some(11.0),
            Some(TextAlign::Center),
            Some(AnnotColor::Rgb(1.0, 0.0, 0.0))
        )
    );
}

#[test]
fn rich_text_escapes_xml_s_special_characters() {
    let text = FreeText::new(rect(), styled());
    let rc = write_rc(&text, None);
    assert!(
        rc.contains("<span style=\"font-style:italic\"> &amp; &lt;over&gt;</span>"),
        "{rc}"
    );
}

#[test]
fn acrobat_s_rich_text_reads_its_styles_and_paragraphs() {
    let rc = r#"<?xml version="1.0"?><body xmlns="http://www.w3.org/1999/xhtml" xmlns:xfa="http://www.xfa.org/schema/xfa-data/1.0/" xfa:APIVersion="Acrobat:23.0.0" xfa:spec="2.0.2" style="font-size:12.0pt;text-align:left;color:#000000;font-weight:normal;font-style:normal;font-family:Helvetica,sans-serif;font-stretch:normal"><p dir="ltr"><span style="font-family:Helvetica">Plain </span><b>bold</b> <i>it&#233;</i></p><p dir="ltr"><span style="font-weight:bold">Next</span></p></body>"#;
    let read = read_rc(rc);
    let expected = RichText {
        paragraphs: vec![
            vec![
                TextSpan::plain("Plain "),
                TextSpan::plain("bold").with_bold(true),
                TextSpan::plain(" "),
                TextSpan::plain("ité").with_italic(true),
            ],
            vec![TextSpan::plain("Next").with_bold(true)],
        ],
    };
    assert_eq!(
        (read.text, read.body.size, read.body.family, read.simplified),
        (expected, Some(12.0), Some(StandardFamily::Helvetica), false)
    );
}

#[test]
fn a_span_coloured_on_its_own_is_simplified() {
    let rc = r#"<body style="color:#000000"><p>a <span style="color:#FF0000">red</span></p></body>"#;
    assert!(read_rc(rc).simplified);
}

#[test]
fn an_element_outside_the_subset_is_simplified_and_its_text_kept() {
    let read = read_rc("<body><p>a <u>b</u></p></body>");
    assert_eq!((read.simplified, read.text.to_plain()), (true, "a b".to_owned()));
}

#[test]
fn a_line_break_starts_a_paragraph() {
    let read = read_rc("<body><p>one<br/>two</p></body>");
    assert_eq!(read.text.to_plain(), "one\rtwo");
}

#[test]
fn rich_text_that_isn_t_xml_reads_as_text() {
    assert_eq!(read_rc("5 < 6 & so on").text.to_plain(), "5 < 6 & so on");
}

#[test]
fn a_patch_replaces_only_what_it_names() {
    let text = FreeText::new(rect(), styled()).with_fill(Some(AnnotColor::Gray(0.9)));
    let patch = FreeTextPatch {
        text: Some(RichText::plain("New")),
        fill: Some(None),
        ..FreeTextPatch::default()
    };
    let changed = patch.applied_to(&text);
    assert_eq!(
        (changed.text, changed.fill, changed.rect),
        (RichText::plain("New"), None, rect())
    );
}

#[test]
fn a_box_reads_from_the_wire_with_its_defaults() {
    let spec: AnnotSpec = serde_json::from_str(
        r#"{ "type": "freeText", "rect": { "x0": 0, "y0": 0, "x1": 100, "y1": 20 }, "text": [[{ "text": "Hi", "bold": true }]] }"#,
    )
    .unwrap();
    let AnnotSpec::FreeText(text) = spec else { panic!() };
    assert_eq!(
        (text.font, text.size, text.border, text.intent),
        (AnnotFont::default(), 12.0, Border::default(), FreeTextIntent::TextBox)
    );
}

#[test]
fn a_font_is_a_family_or_a_blob_id_on_the_wire() {
    let id = BlobId::of(b"f");
    let fonts: Vec<AnnotFont> = serde_json::from_str(&format!(r#"["times", "{id}"]"#)).unwrap();
    assert_eq!(fonts, [AnnotFont::Standard(StandardFamily::Times), AnnotFont::Font(id)]);
}

#[test]
fn an_unknown_font_name_is_refused() {
    assert!(serde_json::from_str::<AnnotFont>(r#""comic-sans""#).is_err());
}

#[test]
fn out_of_range_boxes_are_invalid() {
    let base = FreeText::new(rect(), RichText::plain("x"));
    let callout = |n: usize| {
        base.clone().with_intent(FreeTextIntent::Callout(Callout::new(vec![
            UserSpacePoint::new(0.0, 0.0);
            n
        ])))
    };
    let cases = [
        base.clone().with_font(AnnotFont::default(), 0.5),
        base.clone().with_rotation(45),
        callout(1),
        callout(4),
        FreeText::new(UserSpaceRect::from_corners(0.0, 0.0, 0.0, 10.0), RichText::plain("x")),
    ];
    assert!(cases.iter().all(|t| !t.is_valid()) && base.is_valid() && callout(3).is_valid());
}

#[test]
fn rd_is_how_far_the_box_sits_inside_the_rect() {
    let (dict, _) = round_trip(&FreeText::new(rect(), RichText::plain("x")), None);
    assert_eq!(dict.get("RD").and_then(geom::numbers_of), Some(vec![2.0; 4]));
}

#[test]
fn a_rotated_box_writes_and_reads_its_rotation() {
    let text = FreeText::new(rect(), RichText::plain("x")).with_rotation(90);
    let (dict, read) = round_trip(&text, None);
    assert_eq!(
        (dict.get("Rotate"), read.text.rotation),
        (Some(&Object::Integer(90)), 90)
    );
}

#[test]
fn contents_with_carriage_returns_reads_as_paragraphs() {
    let mut dict = Dict::new();
    dict.insert("Rect", super::super::add::rect_object(&rect()));
    dict.insert("Contents", Object::String(PdfString::literal(b"a\r\nb\rc".to_vec())));
    assert_eq!(read(&dict).unwrap().text.text, RichText::plain("a\nb\nc"));
}
