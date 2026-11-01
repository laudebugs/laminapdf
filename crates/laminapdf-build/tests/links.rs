//! Link annotations and named destinations (todo B5b): what the golden's links resolve to
//! through Lamina PDF's reader and `laminapdf-model`, refusals, and links on pages appended to an
//! open document.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::b5b::URI;
use common::text::lines;
use common::{fixture, reopen, save};
use laminapdf_build::{BuildOptions, Builder, Fit, LinkTarget, PageSize, Standard14};
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{PointPt, Pt, RectPt};
use laminapdf_save::{IncrementalWriter, SaveOptions};

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

fn annots(doc: &laminapdf_model::Document, page: usize) -> Vec<Dict> {
    let pages = doc.pages().unwrap();
    pages
        .get(page)
        .unwrap()
        .annots()
        .unwrap()
        .into_iter()
        .map(|r| doc.resolve(r).unwrap().as_dict().unwrap().clone())
        .collect()
}

/// The page index an explicit destination array names.
fn dest_page(doc: &laminapdf_model::Document, dest: &Object) -> usize {
    let Some(Object::Ref(r)) = dest.as_array().and_then(|a| a.first()) else {
        panic!("{dest:?}")
    };
    let pages = doc.pages().unwrap();
    pages.iter().position(|p| p.obj_ref() == Some(*r)).unwrap()
}

/// The `/Dests` name tree's destination for `name`.
fn named(doc: &laminapdf_model::Document, name: &str) -> Object {
    doc.name_tree("Dests")
        .unwrap()
        .unwrap()
        .get(name.as_bytes())
        .unwrap()
        .unwrap()
}

#[test]
fn the_golden_links_resolve_to_their_targets() {
    let doc = reopen(&fixture("golden/links-and-outlines.pdf"));
    let links = annots(&doc, 0);
    assert_eq!(links.len(), 4);
    for link in &links {
        assert_eq!(
            (link.get_name("Subtype"), link.get_int("F")),
            (Some(&b"Link"[..]), Some(4))
        );
    }
    let uri = links[0].get("A").unwrap().as_dict().unwrap().get("URI").unwrap();
    assert_eq!(uri, &Object::String(laminapdf_cos::PdfString::literal(URI.as_bytes())));
    assert_eq!(dest_page(&doc, links[1].get("Dest").unwrap()), 2);
    let Object::String(name) = links[2].get("Dest").unwrap() else {
        panic!()
    };
    assert_eq!(
        dest_page(&doc, &named(&doc, &String::from_utf8(name.bytes.clone()).unwrap())),
        3
    );
    assert_eq!(dest_page(&doc, links[3].get("Dest").unwrap()), 1);
}

#[test]
fn the_linked_run_covers_its_text() {
    let doc = reopen(&fixture("golden/links-and-outlines.pdf"));
    let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
    let r: Vec<f64> = annots(&doc, 0)[3]
        .get("Rect")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    // The run's baseline (page space y 185) lies inside the rectangle.
    let baseline = frame.to_user_space(PointPt::from_points(40.0, 185.0));
    assert!(
        r[0] <= baseline.x && baseline.x <= r[2] && r[1] < baseline.y && baseline.y < r[3],
        "{r:?}"
    );
    assert_eq!(lines(&doc, 0).last().unwrap(), "Contents (a linked run)");
}

#[test]
fn named_destinations_are_a_sorted_name_tree() {
    let doc = reopen(&fixture("golden/links-and-outlines.pdf"));
    let tree = doc.name_tree("Dests").unwrap().unwrap();
    let names: Vec<Vec<u8>> = tree.entries().unwrap().into_iter().map(|(k, _)| k).collect();
    assert_eq!(
        names,
        [b"appendix".to_vec(), b"chapter-2".to_vec(), b"contents".to_vec()]
    );
    let xyz = named(&doc, "chapter-2");
    let a = xyz.as_array().unwrap();
    let frame = doc.pages().unwrap().get(3).unwrap().frame().unwrap();
    let top = frame.to_user_space(PointPt::from_points(0.0, 100.0));
    assert_eq!(a[1], Object::name("XYZ"));
    assert!((a[3].as_f64().unwrap() - top.y).abs() < 1e-5);
}

fn finish_err(build: impl FnOnce(&mut Builder)) -> &'static str {
    let mut b = Builder::new(BuildOptions::default());
    b.page(PageSize::A4).finish().unwrap();
    build(&mut b);
    b.finish().map(|_| ()).unwrap_err().code()
}

#[test]
fn a_link_to_a_missing_page_or_name_fails_the_build() {
    let page = finish_err(|b| {
        let mut p = b.page(PageSize::A4);
        p.surface().link(rect(0.0, 0.0, 10.0, 10.0), LinkTarget::page(7), None);
        p.finish().unwrap();
    });
    let name = finish_err(|b| {
        let mut p = b.page(PageSize::A4);
        p.surface()
            .link(rect(0.0, 0.0, 10.0, 10.0), LinkTarget::Named("nowhere".into()), None);
        p.finish().unwrap();
    });
    let dest = finish_err(|b| {
        b.destination("x", 9, Fit::Page);
    });
    assert_eq!(
        (page, name, dest),
        (
            "page-index-out-of-range",
            "unknown-destination",
            "page-index-out-of-range"
        )
    );
}

#[test]
fn a_non_finite_destination_is_invalid_geometry() {
    let code = finish_err(|b| {
        b.destination(
            "x",
            0,
            Fit::Xyz {
                left: Some(Pt::new(f64::NAN)),
                top: None,
                zoom: None,
            },
        );
    });
    assert_eq!(code, "invalid-geometry");
}

#[test]
fn appended_pages_get_text_and_links_to_existing_pages() {
    let base = fixture("corpus/hello.pdf");
    let mut doc = reopen(&base);
    let first = doc.pages().unwrap().get(0).unwrap().obj_ref().unwrap();
    let mut appender = Builder::on(&mut doc, &BuildOptions::default()).unwrap();
    let f = appender.standard_font(Standard14::Helvetica).unwrap();
    let mut page = appender.page(PageSize::LETTER);
    let _ = page
        .surface()
        .text("Back to page 1", &f, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap()
        .link(LinkTarget::page(0), None);
    page.finish().unwrap();
    appender.finish().unwrap();
    let mut out = Vec::new();
    IncrementalWriter::new(doc.cos(), SaveOptions::default())
        .write(&mut out)
        .unwrap();
    let after = reopen(&out);
    let links = annots(&after, 1);
    let dest = links[0].get("Dest").unwrap().as_array().unwrap();
    assert_eq!(
        (out.starts_with(&base), dest[0].clone(), lines(&after, 1)),
        (true, Object::Ref(first), vec!["Back to page 1".to_owned()])
    );
}

#[test]
fn an_appended_link_to_a_missing_page_fails() {
    let mut doc = reopen(&fixture("corpus/hello.pdf"));
    let mut appender = Builder::on(&mut doc, &BuildOptions::default()).unwrap();
    let mut page = appender.page(PageSize::LETTER);
    page.surface().link(rect(0.0, 0.0, 9.0, 9.0), LinkTarget::page(5), None);
    page.finish().unwrap();
    assert_eq!(appender.finish().unwrap_err().code(), "page-index-out-of-range");
}

#[test]
fn links_are_deterministic() {
    let build = || {
        let mut b = Builder::new(BuildOptions::default().with_id([3; 16]));
        for i in 0..3_u32 {
            let mut p = b.page(PageSize::A6);
            p.surface()
                .link(rect(10.0, 10.0, 50.0, 20.0), LinkTarget::page((i + 1) % 3), None);
            p.finish().unwrap();
        }
        save(b.finish().unwrap())
    };
    let bytes = build();
    assert_eq!(bytes, build());
    let doc = reopen(&bytes);
    let pages: Vec<ObjRef> = doc.pages().unwrap().iter().map(|p| p.obj_ref().unwrap()).collect();
    let Object::Ref(target) = annots(&doc, 2)[0].get("Dest").unwrap().as_array().unwrap()[0] else {
        panic!()
    };
    assert_eq!(target, pages[0]);
}

#[test]
fn a_failed_append_leaves_the_page_tree_alone() {
    let base = fixture("corpus/hello.pdf");
    let mut doc = reopen(&base);
    let mut appender = Builder::on(&mut doc, &BuildOptions::default()).unwrap();
    let f = appender.standard_font(Standard14::Helvetica).unwrap();
    let mut page = appender.page(PageSize::LETTER);
    let _ = page
        .surface()
        .text("nowhere", &f, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap()
        .link(LinkTarget::page(9), None);
    page.finish().unwrap();
    assert_eq!(appender.finish().unwrap_err().code(), "page-index-out-of-range");
    // The new page was never linked into the tree.
    assert_eq!(doc.pages().unwrap().len(), 1);
    let mut out = Vec::new();
    IncrementalWriter::new(doc.cos(), SaveOptions::default())
        .write(&mut out)
        .unwrap();
    assert_eq!(reopen(&out).pages().unwrap().len(), 1);
}

/// A one-page document whose page is a direct dictionary in `/Kids` (and has no usable
/// `/MediaBox`).
fn direct_page_document() -> Vec<u8> {
    let mut w = laminapdf_save::Writer::new(1, 7);
    let page: Dict = [("Type", Object::name("Page"))].into_iter().collect();
    let pages = w.add(
        [
            ("Type", Object::name("Pages")),
            ("Kids", Object::Array(vec![Object::Dict(page)])),
            ("Count", 1.into()),
        ]
        .into_iter()
        .collect::<Dict>(),
    );
    let root = w.add(
        [("Type", Object::name("Catalog")), ("Pages", pages.into())]
            .into_iter()
            .collect::<Dict>(),
    );
    w.finish(&[("Root", root)].into_iter().collect()).unwrap()
}

#[test]
fn appending_reads_only_the_pages_links_target() {
    let base = direct_page_document();
    // No link to the direct page: appending works.
    let mut doc = reopen(&base);
    let mut appender = Builder::on(&mut doc, &BuildOptions::default()).unwrap();
    let mut page = appender.page(PageSize::A6);
    page.surface().link(rect(0.0, 0.0, 9.0, 9.0), LinkTarget::page(1), None);
    page.finish().unwrap();
    assert_eq!(appender.finish().unwrap().len(), 1);
    // A link to it can't be written: refused before anything changes.
    let mut doc = reopen(&base);
    let mut appender = Builder::on(&mut doc, &BuildOptions::default()).unwrap();
    let mut page = appender.page(PageSize::A6);
    page.surface().link(rect(0.0, 0.0, 9.0, 9.0), LinkTarget::page(0), None);
    page.finish().unwrap();
    assert_eq!(appender.finish().unwrap_err().code(), "invalid-structure");
    assert_eq!(doc.pages().unwrap().len(), 1);
}

#[test]
fn a_negative_zoom_is_invalid_geometry() {
    let code = finish_err(|b| {
        b.destination(
            "x",
            0,
            Fit::Xyz {
                left: None,
                top: None,
                zoom: Some(-2.0),
            },
        );
    });
    assert_eq!(code, "invalid-geometry");
}
