//! `Builder::page_forms`: a page of another document copied as a Form XObject with its crop box,
//! its resources (inherited ones too) and its transparency group, drawn the way it displays;
//! what two copies share copied once; marked content and hidden layers left out; a page whose
//! content can't be read to its end refused; a page past the end an error.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{rect, reopen, save};
use laminapdf_build::{BuildOptions, Builder, PageForm, PageSize};
use laminapdf_cos::{Dict, Object, OpenOptions, Stream};
use laminapdf_test_fixtures::{classic_with_root, fixture, stream_body};

fn open(bytes: Vec<u8>) -> laminapdf_model::Document {
    laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap()
}

fn review() -> laminapdf_model::Document {
    open(fixture("annotations/review.pdf"))
}

/// A one-page document: the page tree's `pages` entries (inherited), the page's `page` entries,
/// and its content.
fn one_page(pages: &str, page: &str, content: &[u8]) -> laminapdf_model::Document {
    let content = String::from_utf8(stream_body("", content)).unwrap();
    open(classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [5 0 R 6 0 R] /D << /OFF [6 0 R] >> >> >>",
            &format!("<< /Type /Pages /Kids [3 0 R] /Count 1 {pages} >>"),
            &format!("<< /Type /Page /Parent 2 0 R /Contents 4 0 R {page} >>"),
            &content,
            "<< /Type /OCG /Name (On) >>",
            "<< /Type /OCG /Name (Off) >>",
        ],
        "",
    ))
}

/// The dictionary `value` holds, resolved.
fn dict(doc: &laminapdf_model::Document, value: &Object) -> Dict {
    match &*doc.deref(value).unwrap() {
        Object::Dict(d) => d.clone(),
        Object::Stream(s) => s.dict.clone(),
        other => panic!("not a dictionary: {other:?}"),
    }
}

/// `pages` of `source` drawn side by side on a new page, and the forms.
fn drawn(source: &laminapdf_model::Document, pages: &[usize]) -> (laminapdf_model::Document, Vec<PageForm>) {
    let mut b = Builder::new(BuildOptions::new().with_id([5; 16]));
    let forms: Vec<PageForm> = b.page_forms(source, pages).unwrap().into_iter().flatten().collect();
    let mut page = b.page(PageSize::A4);
    let mut x = 20.0;
    for form in &forms {
        page.surface().page_form(form, rect(x, 20.0, 150.0, 150.0));
        x += 160.0;
    }
    page.finish().unwrap();
    (reopen(&save(b.finish().unwrap())), forms)
}

/// The new page's form `name` (`Fm1`, `Fm2`).
fn form(doc: &laminapdf_model::Document, name: &str) -> Stream {
    let resources = doc.pages().unwrap().get(0).unwrap().resources().unwrap().clone();
    let xobjects = dict(doc, resources.get("XObject").unwrap());
    let object = doc.deref(xobjects.get(name).unwrap()).unwrap();
    let Object::Stream(stream) = &*object else {
        panic!("not a stream")
    };
    stream.clone()
}

fn bbox(stream: &Stream) -> Option<Object> {
    stream.dict.get("BBox").cloned()
}

fn numbers(values: [i64; 4]) -> Object {
    Object::Array(values.into_iter().map(Object::from).collect())
}

#[test]
fn a_page_is_copied_as_a_form() {
    let (doc, _) = drawn(&review(), &[0]);
    assert_eq!(form(&doc, "Fm1").dict.get_name("Subtype"), Some(&b"Form"[..]));
}

#[test]
fn a_form_is_bounded_by_its_pages_box() {
    let (doc, _) = drawn(&review(), &[0]);
    assert_eq!(bbox(&form(&doc, "Fm1")), Some(numbers([0, 0, 300, 300])));
}

#[test]
fn a_form_holds_its_pages_content() {
    let (doc, _) = drawn(&review(), &[0]);
    assert!(
        doc.decode_stream(&form(&doc, "Fm1"))
            .unwrap()
            .starts_with(b"BT /F1 12 Tf")
    );
}

#[test]
fn a_form_keeps_its_pages_fonts() {
    let (doc, _) = drawn(&review(), &[0]);
    let resources = dict(&doc, form(&doc, "Fm1").dict.get("Resources").unwrap());
    assert!(dict(&doc, resources.get("Font").unwrap()).get("F1").is_some());
}

#[test]
fn what_two_copies_share_is_copied_once() {
    let (doc, _) = drawn(&review(), &[0, 0]);
    let font = |name: &str| {
        let resources = dict(&doc, form(&doc, name).dict.get("Resources").unwrap());
        dict(&doc, resources.get("Font").unwrap())
            .get("F1")
            .and_then(Object::as_ref)
    };
    assert_eq!(font("Fm1"), font("Fm2"));
}

#[test]
fn a_turned_page_is_as_wide_as_it_displays() {
    let source = one_page("", "/MediaBox [0 0 200 100] /Rotate 90", b"0 0 m 10 10 l S");
    let (_, forms) = drawn(&source, &[0]);
    assert_eq!((forms[0].width(), forms[0].height()), (100.0, 200.0));
}

#[test]
fn a_crop_box_smaller_than_the_media_box_bounds_the_form() {
    let source = one_page(
        "",
        "/MediaBox [0 0 200 200] /CropBox [10 20 110 70]",
        b"0 0 m 10 10 l S",
    );
    let (doc, forms) = drawn(&source, &[0]);
    assert_eq!(
        (bbox(&form(&doc, "Fm1")), forms[0].width(), forms[0].height()),
        (Some(numbers([10, 20, 110, 70])), 100.0, 50.0)
    );
}

#[test]
fn inherited_resources_come_with_the_form() {
    let font = "<< /Font << /F9 << /Type /Font /Subtype /Type1 /BaseFont /Courier >> >> >>";
    let source = one_page(
        &format!("/Resources {font}"),
        "/MediaBox [0 0 100 100]",
        b"BT /F9 9 Tf (x) Tj ET",
    );
    let (doc, _) = drawn(&source, &[0]);
    let resources = dict(&doc, form(&doc, "Fm1").dict.get("Resources").unwrap());
    assert!(dict(&doc, resources.get("Font").unwrap()).get("F9").is_some());
}

#[test]
fn the_pages_transparency_group_becomes_the_forms() {
    let source = one_page(
        "",
        "/MediaBox [0 0 100 100] /Group << /S /Transparency /CS /DeviceRGB /K true >>",
        b"0 0 m 10 10 l S",
    );
    let (doc, _) = drawn(&source, &[0]);
    let group = dict(&doc, form(&doc, "Fm1").dict.get("Group").unwrap());
    assert_eq!(
        (group.get_name("S"), group.get("K").cloned()),
        (Some(&b"Transparency"[..]), Some(Object::Bool(true)))
    );
}

#[test]
fn a_layer_that_is_off_stays_hidden_and_one_that_is_on_is_drawn() {
    let resources = "/Resources << /Properties << /On 5 0 R /Off 6 0 R >> >>";
    let content = b"/OC /On BDC BT (shown) Tj ET EMC /OC /Off BDC BT (hidden) Tj ET EMC";
    let source = one_page("", &format!("/MediaBox [0 0 100 100] {resources}"), content);
    let (doc, _) = drawn(&source, &[0]);
    let drawn = String::from_utf8(doc.decode_stream(&form(&doc, "Fm1")).unwrap()).unwrap();
    assert_eq!(
        (
            drawn.contains("(shown)"),
            drawn.contains("(hidden)"),
            drawn.contains("BDC")
        ),
        (true, false, false)
    );
}

#[test]
fn a_page_whose_content_cannot_be_read_to_its_end_gives_no_form() {
    let content = [b"[".repeat(1000), b"] Tj".to_vec()].concat();
    let source = one_page("", "/MediaBox [0 0 100 100]", &content);
    let mut b = Builder::new(BuildOptions::new());
    assert_eq!(b.page_forms(&source, &[0]).unwrap(), [None]);
}

#[test]
fn a_page_past_the_end_is_refused() {
    let source = review();
    let mut b = Builder::new(BuildOptions::new());
    let err = b.page_forms(&source, &[2]).unwrap_err();
    assert_eq!(err.code(), "page-index-out-of-range");
}
