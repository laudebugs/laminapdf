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

/// capability: C.01 core round-trip
/// capability: C.02 core round-trip
/// capability: C.03 core round-trip
/// capability: C.04 core round-trip
/// capability: C.08 core round-trip
/// capability: C.13 core round-trip
/// capability: C.16 core round-trip
/// capability: J.01 core round-trip
/// capability: J.04 core round-trip
/// capability: G.05 core round-trip
/// capability: H.02 core write
/// capability: I.10 core round-trip
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
/// pages, the page labels and the XMP packet byte for byte. The edit turns the first page, which
/// touches none of these; the metadata edits' own round trip is
/// [`metadata_edits_round_trip_with_info_and_xmp_in_step`].
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
    use laminapdf_model::Rotation;
    use laminapdf_ops::{Document, Origin, RotatePage};
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
            let page = doc.pages().unwrap().get(0).unwrap().id().clone();
            doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Clockwise90).into()])
                .unwrap();
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
            let after = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
            assert_eq!(built_features(&after), before, "{name} {mode:?}");
            assert_eq!(after.warnings(), [], "{name} {mode:?}");
        }
    }
}

/// Each metadata edit, saved in each mode with the save-time stamp and reopened: Info and XMP
/// hold the new values and agree, the pages and every other mapped field are kept, and a
/// removal leaves neither side holding the value.
/// capability: J.01 core round-trip
/// capability: J.04 core round-trip
/// capability: J.02 core round-trip
/// capability: J.03 core round-trip
/// capability: J.10 core round-trip
#[test]
fn metadata_edits_round_trip_with_info_and_xmp_in_step() {
    use laminapdf_cos::OpenOptions;
    use laminapdf_model::PdfDate;
    use laminapdf_ops::{Document, MetadataKey, MetadataPatch, Origin, RemoveMetadata, SetMetadata, read_metadata};
    use laminapdf_save::{Save as _, SaveMode, SaveOptions};

    let date = |s: &str| s.parse::<PdfDate>().unwrap();
    let stamp = date("D:20261110093000Z");
    let created = date("D:20260101120000+01'00'");
    for name in [
        "golden/metadata.pdf",
        "corpus/outline.pdf",
        "corpus/hello.pdf",
        "metadata/pdf20-xmp-only.pdf",
        "golden/examples/pdf_a_archive-2b.pdf",
    ] {
        let bytes = std::fs::read(common::fixtures_dir().join(name)).unwrap();
        let pages = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default())
            .unwrap()
            .pages()
            .unwrap()
            .len();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let set = SetMetadata::new(
                MetadataPatch::new()
                    .title(Some("Round trip"))
                    .authors(&["Ada Lovelace", "Charles Babbage"])
                    .creator(Some("lamina round trip"))
                    .producer(Some("Lamina PDF"))
                    .created(Some(created)),
                None,
            );
            doc.apply_all(Origin::Local, [set.into()]).unwrap();
            doc.apply_all(Origin::Local, [RemoveMetadata::new(vec![MetadataKey::Keywords]).into()])
                .unwrap();
            let mut out = Vec::new();
            let options = SaveOptions::new().with_mode(mode).with_metadata_date(stamp);
            doc.save(&mut out, &options).unwrap();
            let reopened = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
            let meta = read_metadata(&reopened).unwrap();
            assert_eq!(
                (
                    reopened.pages().unwrap().len(),
                    meta.title.as_deref(),
                    meta.authors.len(),
                    meta.creator.as_deref(),
                    meta.producer.as_deref(),
                    meta.keywords.as_deref(),
                    meta.created,
                    meta.modified,
                    meta.disagreements.as_slice(),
                ),
                (
                    pages,
                    Some("Round trip"),
                    2,
                    Some("lamina round trip"),
                    Some("Lamina PDF"),
                    None,
                    Some(created),
                    Some(stamp),
                    &[][..],
                ),
                "{name} {mode:?}"
            );
            assert_eq!(reopened.warnings(), [], "{name} {mode:?}");
        }
    }
}
