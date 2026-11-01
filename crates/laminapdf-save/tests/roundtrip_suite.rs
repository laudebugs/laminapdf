//! The lamina-only half of the round-trip suite (ADR 0018 tier 7; todo B3c), in the fast lane:
//! every fixture × proving op × mode × overrides saves (or is refused) as expected and reopens
//! with the expected facts. The full lane also checks the outputs with qpdf, pdf.js and `PDFium`
//! (`cargo xtask roundtrip`).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests and their helpers: a panic is a test failure"
)]

mod common;

/// capability: C.08 core round-trip
/// capability: J.01 core round-trip
/// capability: G.05 core round-trip
#[test]
fn every_case_saves_or_refuses_as_expected_and_reopens_in_lamina() {
    let suite = common::roundtrip::run(&common::fixtures_dir().join(".."));
    assert_eq!(suite.failures, Vec::<String>::new());
    assert!(
        suite.saved > 300 && suite.refused > 100,
        "{} saved, {} refused",
        suite.saved,
        suite.refused
    );
}

/// The page index a destination names: an explicit array's page, or a name looked up in
/// `/Names /Dests`.
fn dest_page(doc: &laminapdf_model::Document, dest: &laminapdf_cos::Object) -> Option<usize> {
    use laminapdf_cos::Object;
    let array = match dest {
        Object::String(name) => doc.name_tree("Dests").unwrap()?.get(&name.bytes).unwrap()?,
        other => other.clone(),
    };
    let Some(Object::Ref(r)) = array.as_array().and_then(|a| a.first()).cloned() else {
        return None;
    };
    doc.pages().unwrap().iter().position(|p| p.obj_ref() == Some(r))
}

/// What the builder's B5b goldens hold that a save must keep: per page, its decoded content,
/// fonts and each link's resolved target (URI, or the page a destination or name leads to);
/// the outline's titles and pages; each named destination's page; the page labels; the XMP
/// packet's bytes.
fn built_features(doc: &laminapdf_model::Document) -> Vec<String> {
    use laminapdf_cos::Object;
    let mut out = Vec::new();
    let pages = doc.pages().unwrap();
    let cos: &laminapdf_cos::Document = doc;
    for (i, page) in cos.pages().unwrap().iter().enumerate() {
        out.push(format!("page {i} content {:?}", cos.page_content(page).unwrap()));
    }
    for page in pages.iter() {
        let fonts = page
            .resources()
            .and_then(|r| r.get("Font"))
            .and_then(Object::as_dict)
            .map_or(0, laminapdf_cos::Dict::len);
        out.push(format!("page fonts={fonts}"));
        for annot in page.annots().unwrap() {
            let annot = doc.resolve(annot).unwrap();
            let dict = annot.as_dict().unwrap();
            let uri = dict
                .get("A")
                .and_then(Object::as_dict)
                .and_then(|a| a.get("URI"))
                .map(|u| format!("{u:?}"));
            let page = dict.get("Dest").and_then(|d| dest_page(doc, d));
            out.push(format!("link uri={uri:?} page={page:?} rect={:?}", dict.get("Rect")));
        }
    }
    let outline = doc.outline().unwrap();
    out.extend(outline.iter().map(|i| {
        format!(
            "outline {} -> {:?}",
            i.title().unwrap(),
            i.target().unwrap().and_then(|d| d.page_index)
        )
    }));
    if let Some(tree) = doc.name_tree("Dests").unwrap() {
        out.extend(
            tree.entries()
                .unwrap()
                .into_iter()
                .map(|(k, v)| format!("dest {} -> {:?}", String::from_utf8_lossy(&k), dest_page(doc, &v))),
        );
    }
    let labels = doc.page_labels().unwrap();
    out.extend(
        (0..pages.len())
            .filter_map(|i| labels.label(i))
            .map(|l| format!("label {l}")),
    );
    let catalog = doc.catalog().unwrap();
    if let Some(Object::Ref(r)) = catalog.get("Metadata") {
        let Object::Stream(s) = &*doc.resolve(*r).unwrap() else {
            panic!("/Metadata is a stream")
        };
        out.push(format!(
            "xmp {}",
            String::from_utf8_lossy(&doc.decode_stream(s).unwrap())
        ));
    }
    out
}

/// The builder's goldens (B5b; `crates/laminapdf-build/tests/goldens.rs` proves they are what it
/// writes, byte for byte) are in the suite's fixtures, so the full lane checks them and their
/// saves with qpdf, pdf.js and `PDFium`; here, an edit and a save in each mode keep their text
/// (content and fonts), each link's resolved target, the outline, the named destinations'
/// pages, the page labels and the XMP packet byte for byte. The edit is `SetInfo`, which
/// changes Info but not XMP: keeping the two in sync on an existing document is E1's.
/// capability: A.01 core write
/// capability: A.09 core write
/// capability: I.02 core write
/// capability: I.03 core write
/// capability: I.04 core write
/// capability: I.10 core write
/// capability: J.04 core write
#[test]
fn built_text_links_outlines_labels_and_xmp_survive_a_save() {
    use laminapdf_cos::OpenOptions;
    use laminapdf_ops::{Document, InfoPatch, Origin, SetInfo};
    use laminapdf_save::{Save as _, SaveMode, SaveOptions};

    for name in common::roundtrip::B5B_BUILT {
        let bytes = std::fs::read(common::fixtures_dir().join("golden").join(name)).unwrap();
        let before = built_features(&laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap());
        assert!(
            before
                .iter()
                .any(|f| f.starts_with("page fonts=") && f != "page fonts=0"),
            "{name}"
        );
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            doc.apply_all(
                Origin::Local,
                [SetInfo::new(InfoPatch::new().title(Some("edited")), None).into()],
            )
            .unwrap();
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
            let after = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
            assert_eq!(built_features(&after), before, "{name} {mode:?}");
            assert_eq!(after.warnings(), [], "{name} {mode:?}");
        }
    }
}
