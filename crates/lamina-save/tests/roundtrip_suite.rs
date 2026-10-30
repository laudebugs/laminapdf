//! The lamina-only half of the round-trip suite (ADR 0018 tier 7; todo B3c), in the fast lane:
//! every fixture × proving op × mode × overrides saves (or is refused) as expected and reopens
//! with the expected facts. The full lane also checks the outputs with qpdf, pdf.js and `PDFium`
//! (`cargo xtask roundtrip`).

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

/// What the builder's B5b goldens hold that a save must keep: per page, the fonts and
/// annotations; the outline's titles, the named destinations, the page labels and the XMP
/// stream.
fn built_features(doc: &lamina_model::Document) -> Vec<String> {
    let mut out = Vec::new();
    let pages = doc.pages().unwrap();
    for page in pages.iter() {
        let fonts = page
            .resources()
            .and_then(|r| r.get("Font"))
            .and_then(lamina_cos::Object::as_dict)
            .map_or(0, lamina_cos::Dict::len);
        out.push(format!("page fonts={fonts} annots={}", page.annots().unwrap().len()));
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
                .map(|(k, _)| format!("dest {}", String::from_utf8_lossy(&k))),
        );
    }
    let labels = doc.page_labels().unwrap();
    out.extend(
        (0..pages.len())
            .filter_map(|i| labels.label(i))
            .map(|l| format!("label {l}")),
    );
    out.push(format!("xmp {}", doc.catalog().unwrap().contains_key("Metadata")));
    out
}

/// The builder's goldens (B5b; `crates/lamina-build/tests/goldens.rs` proves they are what it
/// writes, byte for byte) are in the suite's fixtures, so the full lane checks them and their
/// saves with qpdf, pdf.js and `PDFium`; here, an edit and a save in each mode keep their text
/// fonts, links, outline, named destinations, page labels and XMP.
/// capability: A.01 core write
/// capability: A.09 core write
/// capability: I.02 core write
/// capability: I.03 core write
/// capability: I.04 core write
/// capability: I.10 core write
/// capability: J.04 core write
#[test]
fn built_text_links_outlines_labels_and_xmp_survive_a_save() {
    use lamina_cos::OpenOptions;
    use lamina_ops::{Document, InfoPatch, Origin, SetInfo};
    use lamina_save::{Save as _, SaveMode, SaveOptions};

    for name in common::roundtrip::PLAIN_BUILT.iter().skip(2) {
        let bytes = std::fs::read(common::fixtures_dir().join("golden").join(name)).unwrap();
        let before = built_features(&lamina_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap());
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            doc.apply_all(
                Origin::Local,
                [SetInfo::new(InfoPatch::new().title(Some("edited")), None).into()],
            )
            .unwrap();
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
            let after = lamina_model::Document::open(out, OpenOptions::default()).unwrap();
            assert_eq!(built_features(&after), before, "{name} {mode:?}");
            assert_eq!(after.warnings(), [], "{name} {mode:?}");
        }
    }
}
