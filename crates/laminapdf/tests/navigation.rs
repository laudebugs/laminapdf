//! The outline and named destinations, resolved to pages and page-space views: the
//! links-and-outlines golden against pypdf's facts (`fixtures/build/facts.json`), and destinations
//! on turned pages, which the builder writes from page space, read back where they were given.
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::{
    BuildOptions, Builder, Document, Fit, OpenOptions, OutlineNode, PageSize, PageTarget, PageTargetView, Pt, Rotation,
    SaveMode, SaveOptions,
};
use laminapdf_test_fixtures::{corpus, fixture, json};

fn links_and_outlines() -> Document {
    Document::open_bytes(fixture("golden/links-and-outlines.pdf"), OpenOptions::default()).unwrap()
}

/// The outline as pypdf lists it: (title, depth, page) in document order.
fn flatten(nodes: &[OutlineNode], depth: u64, out: &mut Vec<(String, u64, Option<u64>)>) {
    for node in nodes {
        out.push((node.title.clone(), depth, node.target.map(|t| u64::from(t.page))));
        flatten(&node.children, depth + 1, out);
    }
}

/// `view` with its coordinates rounded to a thousandth of a point: the trip through user space
/// leaves rounding in the last bits.
fn rounded(view: PageTargetView) -> PageTargetView {
    let r = |v: f64| (v * 1000.0).round() / 1000.0;
    let o = |v: Option<f64>| v.map(r);
    match view {
        PageTargetView::At { x, y, zoom } => PageTargetView::At { x: o(x), y: o(y), zoom },
        PageTargetView::FitWidth { y } => PageTargetView::FitWidth { y: o(y) },
        PageTargetView::FitHeight { x } => PageTargetView::FitHeight { x: o(x) },
        PageTargetView::FitRect { x0, y0, x1, y1 } => PageTargetView::FitRect {
            x0: r(x0),
            y0: r(y0),
            x1: r(x1),
            y1: r(y1),
        },
        other => other,
    }
}

#[test]
fn the_outline_matches_pypdf() {
    let facts = json("build/facts.json");
    let expected: Vec<(String, u64, Option<u64>)> = facts["files"]["links-and-outlines.pdf"]["pypdf"]["outline"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["title"].as_str().unwrap().to_owned(),
                item["depth"].as_u64().unwrap(),
                item["page"].as_u64(),
            )
        })
        .collect();
    let mut actual = Vec::new();
    flatten(&links_and_outlines().outline().unwrap(), 0, &mut actual);
    assert_eq!(actual, expected);
}

/// Each corpus file's facts under `key` (`fixtures/corpus/expected.json`), with its bytes.
fn corpus_facts(key: &str) -> Vec<(String, Vec<u8>, serde_json::Value)> {
    let facts = json("corpus/expected.json");
    corpus()
        .into_iter()
        .filter_map(|(name, bytes)| {
            let fact = facts.get(&name)?.get(key)?.clone();
            (!fact.is_null()).then_some((name, bytes, fact))
        })
        .collect()
}

/// The outline items whose page `fixtures/corpus/exceptions.toml` excuses, per file.
fn excused_items() -> Vec<(String, Vec<usize>)> {
    let table: toml::Table = String::from_utf8(fixture("corpus/exceptions.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let entries = table
        .get("exception")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries
        .iter()
        .filter(|e| {
            e["facts"]
                .as_array()
                .is_some_and(|f| f.iter().any(|v| v.as_str() == Some("outline_pages")))
        })
        .map(|e| {
            let items = e["items"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| usize::try_from(i.as_integer().unwrap()).unwrap());
            (e["file"].as_str().unwrap().to_owned(), items.collect())
        })
        .collect()
}

#[test]
fn corpus_outline_pages_match_pypdf() {
    let files = corpus_facts("outline_pages");
    let excused = excused_items();
    assert!(
        files
            .iter()
            .any(|(_, _, f)| f.as_array().is_some_and(|a| !a.is_empty()))
    );
    for (name, bytes, fact) in files {
        let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
        let mut flat = Vec::new();
        flatten(&doc.outline().unwrap(), 0, &mut flat);
        let skip: &[usize] = excused.iter().find(|(f, _)| *f == name).map_or(&[], |(_, items)| items);
        let pages: Vec<Option<u64>> = flat.into_iter().map(|(_, _, page)| page).collect();
        let want: Vec<Option<u64>> = fact.as_array().unwrap().iter().map(serde_json::Value::as_u64).collect();
        let kept = |list: &[Option<u64>]| -> Vec<Option<u64>> {
            list.iter()
                .enumerate()
                .filter(|(i, _)| !skip.contains(i))
                .map(|(_, p)| *p)
                .collect()
        };
        assert_eq!((pages.len(), kept(&pages)), (want.len(), kept(&want)), "{name}");
    }
}

#[test]
fn corpus_page_labels_match_pypdf() {
    let files = corpus_facts("page_labels");
    assert_ne!(files.len(), 0);
    for (name, bytes, fact) in files {
        let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
        let labels: Vec<String> = doc
            .page_infos()
            .unwrap()
            .into_iter()
            .map(|p| p.label.unwrap_or_default())
            .collect();
        let want: Vec<String> = fact
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(labels, want, "{name}");
    }
}

#[test]
fn the_open_chapter_is_open() {
    let outline = links_and_outlines().outline().unwrap();
    let open: Vec<_> = outline.iter().filter(|n| n.open).map(|n| n.title.as_str()).collect();
    assert_eq!(open, ["Chapters"]);
}

#[test]
fn named_destinations_match_pypdf() {
    let facts = json("build/facts.json");
    let doc = links_and_outlines();
    let dests = facts["files"]["links-and-outlines.pdf"]["pypdf"]["destinations"]
        .as_object()
        .unwrap();
    for (name, page) in dests {
        let target = doc.named_destination(name).unwrap().unwrap();
        assert_eq!(Some(u64::from(target.page)), page.as_u64(), "{name}");
    }
}

#[test]
fn named_destinations_keep_their_page_space_views() {
    let doc = links_and_outlines();
    let view = |name: &str| rounded(doc.named_destination(name).unwrap().unwrap().view);
    assert_eq!(
        [view("chapter-2"), view("contents"), view("appendix")],
        [
            PageTargetView::At {
                x: Some(0.0),
                y: Some(100.0),
                zoom: None
            },
            PageTargetView::Fit,
            PageTargetView::FitWidth { y: Some(72.0) },
        ]
    );
}

#[test]
fn an_unknown_name_is_none() {
    assert_eq!(links_and_outlines().named_destination("nowhere").unwrap(), None);
}

#[test]
fn a_document_without_an_outline_has_none() {
    let doc = Document::open_bytes(fixture("corpus/hello.pdf"), OpenOptions::default()).unwrap();
    assert_eq!(doc.outline().unwrap(), Vec::new());
}

/// One A5 page turned by `rotation`, with a destination at (40, 120) in page space and one
/// fitting the width from 72 down.
fn turned(rotation: Rotation) -> Document {
    let mut b = Builder::new(BuildOptions::default().with_id([7; 16]));
    b.destination(
        "at",
        0,
        Fit::Xyz {
            left: Some(Pt::new(40.0)),
            top: Some(Pt::new(120.0)),
            zoom: Some(2.0),
        },
    )
    .destination(
        "width",
        0,
        Fit::Width {
            top: Some(Pt::new(72.0)),
        },
    );
    let mut page = b.page(PageSize::A5);
    page.rotate(rotation);
    page.finish().unwrap();
    let saved = b
        .finish()
        .unwrap()
        .save(&SaveOptions::new().with_mode(SaveMode::FullRewrite))
        .unwrap();
    Document::open_bytes(saved.bytes, OpenOptions::default()).unwrap()
}

#[test]
fn destinations_on_turned_pages_come_back_in_page_space() {
    for rotation in [
        Rotation::None,
        Rotation::Clockwise90,
        Rotation::Half,
        Rotation::Clockwise270,
    ] {
        let doc = turned(rotation);
        assert_eq!(
            doc.named_destination("at")
                .unwrap()
                .map(|t: PageTarget| (t.page, rounded(t.view))),
            Some((
                0,
                PageTargetView::At {
                    x: Some(40.0),
                    y: Some(120.0),
                    zoom: Some(2.0)
                }
            )),
            "{rotation:?}"
        );
        // Sideways, `/FitH` can't name the displayed top: the builder writes null.
        let top = matches!(rotation, Rotation::None | Rotation::Half).then_some(72.0);
        assert_eq!(
            doc.named_destination("width").unwrap().map(|t| rounded(t.view)),
            Some(PageTargetView::FitWidth { y: top }),
            "{rotation:?}"
        );
    }
}
