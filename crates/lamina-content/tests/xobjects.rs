//! Form and image XObjects (§8.8, §8.10): recursion with the form matrix, resource
//! inheritance, depth limits and cycle detection (ADR 0023 §4).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{Event, dict, font_resources, form, page_doc, page_doc_with, run};
use lamina_content::{Error, Limit, Limits, Matrix};
use lamina_cos::{Dict, Object, Stream, Writer};
use lamina_model::OpenOptions;

/// Resources with one XObject `/X` = `obj`.
fn xobject(obj: Object) -> Dict {
    dict([("XObject", dict([("X", obj)]).into())])
}

/// A chain of `n` forms, each drawing the next by `/X Do`; the last fills a rectangle.
fn chain(w: &mut Writer, n: usize) -> Dict {
    let mut next: Option<Object> = None;
    for _ in 0..n {
        let (content, extra) = match next.take() {
            Some(inner) => (&b"/X Do"[..], dict([("Resources", xobject(inner).into())])),
            None => (&b"0 0 1 1 re f"[..], Dict::new()),
        };
        next = Some(w.add(form(content, extra)).into());
    }
    next.map(xobject).unwrap_or_default()
}

#[test]
fn a_form_runs_with_its_matrix_and_reports_enter_and_leave() {
    let doc = page_doc(|w| {
        let matrix: Object = vec![1.into(), 0.into(), 0.into(), 1.into(), 50.into(), 60.into()].into();
        let fm = w.add(form(b"0 0 1 1 re f", dict([("Matrix", matrix)])));
        (b"2 0 0 2 0 0 cm /X Do".to_vec(), xobject(fm.into()))
    });
    let (result, rec) = run(&doc);
    assert_eq!(result.map(|s| s.forms), Ok(1));
    let Some(Event::Enter { depth, ctm, .. }) = rec.events.first() else {
        panic!("enter first: {:?}", rec.events)
    };
    // [1 0 0 1 50 60] × [2 0 0 2 0 0] = [2 0 0 2 100 120]
    assert_eq!(
        (*depth, *ctm, rec.events.last().map(|e| matches!(e, Event::Leave(_)))),
        (1, Matrix::new(2.0, 0.0, 0.0, 2.0, 100.0, 120.0), Some(true))
    );
}

#[test]
fn a_form_without_resources_uses_the_page_s() {
    let doc = page_doc(|w| {
        let fm = w.add(form(b"BT /F1 10 Tf (a) Tj ET", Dict::new()));
        let mut resources = font_resources(w);
        resources.insert("XObject", dict([("X", fm.into())]));
        (b"/X Do".to_vec(), resources)
    });
    let (_, rec) = run(&doc);
    assert!(rec.warnings.is_empty(), "{:?}", rec.warnings);
}

#[test]
fn a_form_with_resources_does_not_see_the_page_s() {
    let doc = page_doc(|w| {
        let fm = w.add(form(
            b"BT /F1 10 Tf (a) Tj ET",
            dict([("Resources", Dict::new().into())]),
        ));
        let mut resources = font_resources(w);
        resources.insert("XObject", dict([("X", fm.into())]));
        (b"/X Do".to_vec(), resources)
    });
    let (_, rec) = run(&doc);
    assert_eq!(rec.codes(), ["content-resource-missing"]);
}

#[test]
fn a_form_restores_the_graphics_state_after_it() {
    let doc = page_doc(|w| {
        let fm = w.add(form(b"5 0 0 5 0 0 cm q", Dict::new()));
        (b"/X Do BT (a) Tj ET".to_vec(), xobject(fm.into()))
    });
    let (_, rec) = run(&doc);
    let Some(Event::Show { ctm, .. }) = rec.shows().first().copied() else {
        panic!("one show")
    };
    // The form's unbalanced `q` is closed with a warning; the page's CTM is untouched.
    assert_eq!((*ctm, rec.codes()), (Matrix::IDENTITY, vec!["content-unbalanced-q"]));
}

#[test]
fn text_inside_a_form_reports_its_depth() {
    let doc = page_doc(|w| {
        let fm = w.add(form(b"BT (a) Tj ET", Dict::new()));
        (b"/X Do".to_vec(), xobject(fm.into()))
    });
    let (_, rec) = run(&doc);
    assert!(matches!(rec.shows().first(), Some(Event::Show { depth: 1, .. })));
}

#[test]
fn nested_forms_at_the_default_depth_limit_succeed() {
    let doc = page_doc(|w| (b"/X Do".to_vec(), chain(w, 32)));
    let (result, _) = run(&doc);
    assert_eq!(result.map(|s| s.forms), Ok(32));
}

#[test]
fn nested_forms_one_past_the_default_depth_limit_fail() {
    let doc = page_doc(|w| (b"/X Do".to_vec(), chain(w, 33)));
    let (result, _) = run(&doc);
    assert!(matches!(
        result,
        Err(Error::LimitExceeded {
            limit: Limit::XObjectDepth,
            ..
        })
    ));
}

#[test]
fn the_depth_limit_is_configurable() {
    let options = OpenOptions::default().with_limits(Limits::default().with_max_xobject_depth(2));
    let doc = page_doc_with(options, |w| (b"/X Do".to_vec(), chain(w, 2)));
    assert!(run(&doc).0.is_ok());
}

#[test]
fn a_self_referencing_form_reports_a_cycle_and_continues() {
    let doc = page_doc(|w| {
        let fm = w.reserve();
        w.set(
            fm,
            form(b"/X Do 0 0 1 1 re f", dict([("Resources", xobject(fm.into()).into())])),
        );
        (b"/X Do 0 0 m".to_vec(), xobject(fm.into()))
    });
    let (result, rec) = run(&doc);
    assert_eq!((result.map(|s| s.forms), rec.codes()), (Ok(1), vec!["xobject-cycle"]));
}

#[test]
fn a_form_drawn_twice_is_not_a_cycle() {
    let doc = page_doc(|w| {
        let fm = w.add(form(b"0 0 1 1 re f", Dict::new()));
        (b"/X Do /X Do".to_vec(), xobject(fm.into()))
    });
    let (result, rec) = run(&doc);
    assert_eq!((result.map(|s| s.forms), rec.warnings.len()), (Ok(2), 0));
}

#[test]
fn an_image_xobject_is_reported_with_the_ctm() {
    let doc = page_doc(|w| {
        let image = w.add(Stream::new(
            dict([
                ("Subtype", Object::name("Image")),
                ("Width", 1.into()),
                ("Height", 1.into()),
                ("BitsPerComponent", 8.into()),
                ("ColorSpace", Object::name("DeviceGray")),
            ]),
            vec![0],
        ));
        (b"100 0 0 50 10 10 cm /X Do".to_vec(), xobject(image.into()))
    });
    let (_, rec) = run(&doc);
    assert_eq!(
        rec.events,
        [Event::Image {
            inline: false,
            ctm: Matrix::new(100.0, 0.0, 0.0, 50.0, 10.0, 10.0)
        }]
    );
}

#[test]
fn a_missing_xobject_warns() {
    let doc = page_doc(|_| (b"/X Do".to_vec(), Dict::new()));
    let (result, rec) = run(&doc);
    assert_eq!((result.is_ok(), rec.codes()), (true, vec!["content-resource-missing"]));
}

#[test]
fn a_form_whose_content_does_not_decode_is_skipped_with_a_warning() {
    let doc = page_doc(|w| {
        let mut broken = form(b"not flate", Dict::new());
        broken.dict.insert("Filter", Object::name("FlateDecode"));
        let fm = w.add(broken);
        (b"/X Do 0 0 m".to_vec(), xobject(fm.into()))
    });
    let (result, rec) = run(&doc);
    assert_eq!((result.is_ok(), rec.codes()), (true, vec!["content-resource-missing"]));
}
