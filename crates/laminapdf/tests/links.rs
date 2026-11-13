//! A page's links against an independent reader: `fixtures/links/links.pdf` (every link kind,
//! from `tools/fixtures/links.py`) against pypdf's facts in `fixtures/links/expected.json`, and
//! the links-and-outlines golden against `fixtures/build/facts.json`; then where the links sit
//! in page space.
#![expect(clippy::unwrap_used, clippy::panic, reason = "test code: a panic is a test failure")]

use laminapdf::{Document, LinkTo, OpenOptions, PageLink};
use laminapdf_test_fixtures::{fixture, json};
use serde_json::{Value, json};

fn open(path: &str) -> Document {
    Document::open_bytes(fixture(path), OpenOptions::default()).unwrap()
}

/// A link as the facts describe it: `{"page": n | null}`, `{"uri": …}`, `{"file": …}`,
/// `{"named": …}` or `{"action": …}`.
fn fact(link: &PageLink) -> Value {
    match &link.to {
        LinkTo::Destination { target, .. } => json!({ "page": target.map(|t| t.page) }),
        LinkTo::Uri(uri) => json!({ "uri": uri }),
        LinkTo::OtherFile(file) => json!({ "file": file }),
        LinkTo::NamedAction(name) => json!({ "named": name }),
        LinkTo::Unsupported(kind) => json!({ "action": kind }),
        other => panic!("a kind the facts don't describe: {other:?}"),
    }
}

fn facts_of(doc: &Document) -> Vec<Vec<Value>> {
    (0..doc.page_count().unwrap())
        .map(|i| doc.page_links(i).unwrap().iter().map(fact).collect())
        .collect()
}

#[test]
fn every_link_kind_matches_pypdf() {
    let facts = json("links/expected.json");
    // pypdf lists every link annotation; a reader leaves out hidden ones and those without a
    // rectangle.
    let expected: Vec<Vec<Value>> = facts["files"]["links.pdf"]
        .as_array()
        .unwrap()
        .iter()
        .map(|page| {
            page.as_array()
                .unwrap()
                .iter()
                .filter(|link| link["hidden"] == false && !link["rect"].is_null())
                .map(|link| {
                    let mut link = link.as_object().unwrap().clone();
                    link.remove("hidden");
                    link.remove("rect");
                    Value::Object(link)
                })
                .collect()
        })
        .collect();
    assert_eq!(facts_of(&open("links/links.pdf")), expected);
}

#[test]
fn the_golden_links_match_pypdf() {
    let facts = json("build/facts.json");
    let expected = facts["files"]["links-and-outlines.pdf"]["pypdf"]["links"].clone();
    let actual: Vec<Vec<Value>> = facts_of(&open("golden/links-and-outlines.pdf"))
        .into_iter()
        .map(|page| {
            page.into_iter()
                .map(|link| match link.get("uri") {
                    Some(uri) => json!({ "url": uri }),
                    None => link,
                })
                .collect()
        })
        .collect();
    assert_eq!(serde_json::to_value(actual).unwrap(), expected);
}

#[test]
fn links_sit_in_page_space_with_their_names() {
    let doc = open("links/links.pdf");
    let links = doc.page_links(0).unwrap();
    // The first link's /Rect is [72 736 400 754] on a 792 pt page: 38 pt from the top.
    let first = &links[0];
    assert_eq!(
        (
            first.rect.x0.get(),
            first.rect.y0.get(),
            first.rect.x1.get(),
            first.rect.y1.get()
        ),
        (72.0, 38.0, 400.0, 56.0)
    );
    assert_eq!(links[3].contents.as_deref(), Some("The project site"));
    assert!(matches!(&links[1].to, LinkTo::Destination { name: Some(name), .. } if name == "chapter"));
    assert!(matches!(&links[2].to, LinkTo::Destination { name: Some(name), .. } if name == "appendix"));
}

#[test]
fn quad_points_become_page_space_areas() {
    let doc = open("links/links.pdf");
    let quads = doc
        .page_links(0)
        .unwrap()
        .into_iter()
        .find(|l| !l.quads.is_empty())
        .unwrap()
        .quads;
    assert_eq!(quads.len(), 2);
    // Each area spans the link's height, from its top (y 578) to its bottom (y 596).
    for quad in &quads {
        let ys: Vec<f64> = quad.iter().map(|p| p.y.get()).collect();
        assert_eq!(ys.iter().copied().fold(f64::INFINITY, f64::min), 578.0);
        assert_eq!(ys.iter().copied().fold(0.0, f64::max), 596.0);
    }
}

#[test]
fn a_page_past_the_end_is_refused() {
    let err = open("links/links.pdf").page_links(3).unwrap_err();
    assert_eq!(err.code(), "page-index-out-of-range");
}

/// A one-page document whose page is turned by `rotate` (`/Rotate`), 612 × 792 points unturned.
fn turned(rotate: u16) -> Document {
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate {rotate} >>");
    let bytes = laminapdf_test_fixtures::classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &page,
        ],
        "",
    );
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

#[test]
fn a_link_on_a_box_in_page_space_reads_back_on_that_box_at_every_rotation() {
    use laminapdf::{OutlineTarget, Pt, RectPt};
    for rotate in [0, 90, 180, 270] {
        let mut doc = turned(rotate);
        let area = RectPt::new(Pt::new(40.0), Pt::new(60.0), Pt::new(200.0), Pt::new(90.0));
        let op = doc
            .link_on_area(0, area, OutlineTarget::Uri("https://example.com/".into()))
            .unwrap();
        doc.apply(vec![op.into()]).unwrap();
        let rect = doc.page_links(0).unwrap().pop().unwrap().rect;
        let near = |a: Pt, b: Pt| (a.get() - b.get()).abs() < 1e-6;
        assert!(
            near(rect.x0, area.x0) && near(rect.y0, area.y0) && near(rect.x1, area.x1) && near(rect.y1, area.y1),
            "/Rotate {rotate}: {rect:?}"
        );
    }
}
