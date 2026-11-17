//! `Builder::page_forms`: a page of another document copied as a Form XObject with its crop box
//! and resources, what two copies share copied once, a page past the end refused, and the copy
//! drawn on a page of the new document.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{rect, reopen, save};
use laminapdf_build::{BuildOptions, Builder, PageSize};
use laminapdf_cos::{Dict, Object};
use laminapdf_test_fixtures::fixture;

fn review() -> laminapdf_model::Document {
    laminapdf_model::Document::open(fixture("annotations/review.pdf"), laminapdf_cos::OpenOptions::default()).unwrap()
}

/// The dictionary `value` holds, resolved.
fn dict(doc: &laminapdf_model::Document, value: &Object) -> Dict {
    match &*doc.deref(value).unwrap() {
        Object::Dict(d) => d.clone(),
        Object::Stream(s) => s.dict.clone(),
        other => panic!("not a dictionary: {other:?}"),
    }
}

/// The new document's first page drawing `pages` of review.pdf side by side.
fn drawn(pages: &[usize]) -> laminapdf_model::Document {
    let source = review();
    let mut b = Builder::new(BuildOptions::new().with_id([5; 16]));
    let forms = b.page_forms(&source, pages).unwrap();
    let mut page = b.page(PageSize::A4);
    let mut x = 20.0;
    for form in &forms {
        page.surface().page_form(form, rect(x, 20.0, 150.0, 150.0));
        x += 160.0;
    }
    page.finish().unwrap();
    reopen(&save(b.finish().unwrap()))
}

#[test]
fn a_page_is_copied_with_its_crop_box_and_resources() {
    let doc = drawn(&[0]);
    let page = doc.pages().unwrap().get(0).unwrap().resources().unwrap().clone();
    let xobjects = dict(&doc, page.get("XObject").unwrap());
    let form = doc.deref(xobjects.get("Fm1").unwrap()).unwrap();
    let Object::Stream(form) = &*form else {
        panic!("not a stream")
    };
    let fonts = dict(
        &doc,
        dict(&doc, form.dict.get("Resources").unwrap()).get("Font").unwrap(),
    );
    assert_eq!(
        (
            form.dict.get_name("Subtype"),
            form.dict.get("BBox").cloned(),
            fonts.get("F1").is_some(),
            doc.decode_stream(form).unwrap().starts_with(b"BT /F1 12 Tf")
        ),
        (
            Some(&b"Form"[..]),
            Some(Object::Array(vec![0.into(), 0.into(), 300.into(), 300.into()])),
            true,
            true
        )
    );
}

#[test]
fn what_two_copies_share_is_copied_once() {
    let doc = drawn(&[0, 0]);
    let page = doc.pages().unwrap().get(0).unwrap().resources().unwrap().clone();
    let xobjects = dict(&doc, page.get("XObject").unwrap());
    let font = |name: &str| {
        let form = dict(&doc, xobjects.get(name).unwrap());
        dict(&doc, form.get("Resources").unwrap()).get("Font").cloned()
    };
    let (a, b) = (font("Fm1").unwrap(), font("Fm2").unwrap());
    let refs = |o: &Object| dict(&doc, o).get("F1").and_then(Object::as_ref);
    assert_eq!(refs(&a), refs(&b));
}

#[test]
fn a_page_past_the_end_is_refused() {
    let source = review();
    let mut b = Builder::new(BuildOptions::new());
    let err = b.page_forms(&source, &[2]).unwrap_err();
    assert_eq!(err.code(), "page-index-out-of-range");
}
