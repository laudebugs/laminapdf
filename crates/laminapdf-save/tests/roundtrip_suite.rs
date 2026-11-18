//! The lamina-only half of the round-trip suite (ADR 0018 tier 7; todo B3c), in the fast lane:
//! every fixture × proving op × mode × overrides saves (or is refused) as expected and reopens
//! with the expected facts, one test per family of ops and per kind of other case, so each stays
//! well inside nextest's per-test timeout. The full lane also checks the outputs with qpdf,
//! pdf.js and `PDFium` (`cargo xtask roundtrip`).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "tests and their helpers: a panic is a test failure"
)]

mod common;

use std::path::Path;

use common::roundtrip::{OpFamily, OtherCases, PLAIN_BUILT, PLAIN_CORPUS, Suite};

/// Runs `cases` over the fixtures and checks that every case saved, or was refused, as expected
/// and reopened with the expected facts, and that some case saved.
fn check(cases: impl FnOnce(&Path, &mut Suite)) -> Suite {
    let mut suite = Suite::default();
    cases(&common::fixtures_dir().join(".."), &mut suite);
    assert_eq!(suite.failures, Vec::<String>::new());
    assert!(suite.saved > 0, "no case saved");
    suite
}

/// How many plain fixtures `op` applies to. A plain fixture saves in all 12 mode × override
/// cases, so the op must save at least 12 times this many cases. Most ops apply to every plain
/// fixture; deleting, moving and reversing pages need more than one page, which 5 of them have,
/// and 9 have a run of text that can be rewritten.
fn plain_fixtures_for(op: &str) -> usize {
    match op {
        "deletePages" | "movePages" | "reversePages" => 5,
        "editText" => 9,
        _ => PLAIN_CORPUS.len() + PLAIN_BUILT.len(),
    }
}

/// [`check`] over every fixture × `family`'s ops × mode × overrides. Each op must save at least
/// its plain fixtures' cases, so an op that silently skips fixtures fails. The signed, certified
/// and encrypted fixtures refuse some modes, so some case must be refused too.
fn check_family(family: OpFamily) {
    let suite = check(|root, suite| common::roundtrip::op_cases(root, family, suite));
    for &op in family.ops() {
        let saved = suite.saved_by_op.get(op).copied().unwrap_or_default();
        let floor = 12 * plain_fixtures_for(op);
        assert!(saved >= floor, "{op}: {saved} cases saved, expected at least {floor}");
    }
    assert!(suite.refused > 0, "{family:?}: {} saved, none refused", suite.saved);
}

/// The test that runs `family`. The match is exhaustive, so a new family can't build until it
/// has a test of its own.
fn test_of_family(family: OpFamily) -> fn() {
    match family {
        OpFamily::PageOrder => page_order_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::RotationAndLabels => rotation_and_label_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::Import => import_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::Metadata => metadata_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::Xmp => xmp_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::InitialView => initial_view_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::PageBoxes => page_box_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::PageSize => page_size_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::Content => content_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
        OpFamily::Attachments => attachment_cases_save_or_refuse_as_expected_and_reopen_in_lamina,
    }
}

/// The test that runs `other`, exhaustively as [`test_of_family`].
fn test_of_other(other: OtherCases) -> fn() {
    match other {
        OtherCases::Copies => page_copies_save_and_reopen_in_lamina,
        OtherCases::Images => images_made_into_pdfs_save_and_reopen_in_lamina,
        OtherCases::Extracted => extracted_and_split_documents_save_and_reopen_in_lamina,
        OtherCases::Outlines => outline_edits_save_and_reopen_in_lamina,
        OtherCases::Links => link_edits_save_and_reopen_in_lamina,
        OtherCases::Marks => marks_save_and_reopen_in_lamina,
        OtherCases::Flattened => flattened_marks_save_and_reopen_in_lamina,
        OtherCases::Attachments => attachment_edits_save_and_reopen_in_lamina,
        OtherCases::Summarised => comment_summaries_save_and_reopen_in_lamina,
    }
}

/// Every family and every kind of other case that [`common::roundtrip::run`] runs for the full
/// lane has a test here. The proof is that [`test_of_family`] and [`test_of_other`] build; this
/// test names one for each.
#[test]
fn every_family_and_kind_of_case_has_a_test() {
    let families: Vec<fn()> = OpFamily::ALL.into_iter().map(test_of_family).collect();
    let others: Vec<fn()> = OtherCases::ALL.into_iter().map(test_of_other).collect();
    assert_eq!(
        (families.len(), others.len()),
        (OpFamily::ALL.len(), OtherCases::ALL.len())
    );
}

/// capability: C.01 core round-trip
/// capability: C.03 core round-trip
/// capability: C.04 core round-trip
/// capability: C.13 core round-trip
#[test]
fn page_order_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::PageOrder);
}

/// capability: C.08 core round-trip
/// capability: I.10 core round-trip
#[test]
fn rotation_and_label_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::RotationAndLabels);
}

/// capability: C.02 core round-trip
/// capability: C.16 core round-trip
/// capability: C.06 core round-trip
#[test]
fn import_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::Import);
}

/// The encrypted fixtures save only with `remove_encryption`, as a plaintext full rewrite (G.05),
/// as in every family.
/// capability: J.01 core round-trip
/// capability: G.05 core round-trip
#[test]
fn metadata_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::Metadata);
}

/// capability: J.04 core round-trip
#[test]
fn xmp_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::Xmp);
}

/// capability: I.06 core round-trip
/// capability: I.11 core round-trip
#[test]
fn initial_view_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::InitialView);
}

/// capability: C.09 core round-trip
/// capability: C.11 core round-trip
#[test]
fn page_box_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::PageBoxes);
}

/// capability: C.10 core round-trip
/// capability: C.12 core round-trip
#[test]
fn page_size_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::PageSize);
}

#[test]
fn content_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::Content);
}

/// A file attached as the document's associated file reopens byte for byte, in every fixture
/// that saves.
/// capability: J.05 core round-trip
/// capability: M.09 core round-trip
#[test]
fn attachment_cases_save_or_refuse_as_expected_and_reopen_in_lamina() {
    check_family(OpFamily::Attachments);
}

/// capability: C.02 core round-trip
#[test]
fn page_copies_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Copies.run(root, suite));
}

/// capability: H.02 core write
#[test]
fn images_made_into_pdfs_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Images.run(root, suite));
}

/// capability: C.05 core write
/// capability: C.07 core write
/// capability: H.10 core write
#[test]
fn extracted_and_split_documents_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Extracted.run(root, suite));
}

/// capability: I.01 core round-trip
/// capability: I.04 core round-trip
#[test]
fn outline_edits_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Outlines.run(root, suite));
}

/// capability: A.09 core round-trip
/// capability: I.02 core round-trip
/// capability: I.03 core round-trip
#[test]
fn link_edits_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Links.run(root, suite));
}

/// capability: B.06 core round-trip
/// capability: D.01 core round-trip
/// capability: D.02 core round-trip
/// capability: D.03 core round-trip
/// capability: D.04 core round-trip
/// capability: D.05 core round-trip
/// capability: D.06 core round-trip
/// capability: D.07 core round-trip
/// capability: D.08 core round-trip
/// capability: D.10 core round-trip
/// capability: D.11 core round-trip
/// capability: D.16 core round-trip
#[test]
fn marks_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Marks.run(root, suite));
}

/// capability: D.14 core round-trip
#[test]
fn flattened_marks_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Flattened.run(root, suite));
}

/// capability: D.15 core round-trip
#[test]
fn comment_summaries_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Summarised.run(root, suite));
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
            let page = link_page(doc, dict);
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

/// The page a link goes to: its `/Dest`, or its `GoTo` action's `/D`, resolved by [`dest_page`].
fn link_page(doc: &laminapdf_model::Document, link: &laminapdf_cos::Dict) -> Option<usize> {
    use laminapdf_cos::Object;
    let dest = link
        .get("Dest")
        .or_else(|| link.get("A").and_then(Object::as_dict).and_then(|a| a.get("D")))?;
    dest_page(doc, dest)
}

/// The pages page `index`'s links go to, in annotation order ([`link_page`]).
fn link_pages(doc: &laminapdf_model::Document, index: usize) -> Vec<Option<usize>> {
    let pages = doc.pages().unwrap();
    pages
        .get(index)
        .unwrap()
        .annots()
        .unwrap()
        .into_iter()
        .map(|annot| link_page(doc, doc.resolve(annot).unwrap().as_dict().unwrap()))
        .collect()
}

/// The façade's contents page (E3l; `laminapdf`'s `tests/contents_page.rs` checks how it is made)
/// on `links-and-outlines.pdf`, whose bookmarks two levels deep go to pages 1 to 4 once the page
/// is in: after an edit and a save in each mode, each entry's link still lands on its
/// bookmark's page, and the contents page's own bookmark comes first and goes to it. The suite's
/// families save it with every other op too, and the full lane checks those saves with qpdf,
/// pdf.js and `PDFium`.
/// capability: I.05 core round-trip
#[test]
fn a_contents_pages_links_land_on_their_bookmarks_pages_after_a_save() {
    use laminapdf_cos::OpenOptions;
    use laminapdf_model::Rotation;
    use laminapdf_ops::{Document, Origin, RotatePage};
    use laminapdf_save::{Save as _, SaveMode, SaveOptions};

    let path = common::fixtures_dir()
        .join("golden")
        .join(common::roundtrip::CONTENTS_PAGE);
    let bytes = std::fs::read(path).unwrap();
    let expected = vec![Some(1), Some(2), Some(3), Some(4)];
    let before = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
    assert_eq!(link_pages(&before, 0), expected);
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
        let last = doc.pages().unwrap().iter().last().unwrap().id().clone();
        doc.apply_all(Origin::Local, [RotatePage::new(last, Rotation::Clockwise90).into()])
            .unwrap();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        let after = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
        assert_eq!(link_pages(&after, 0), expected, "{mode:?}");
        let outline = after.outline().unwrap();
        let first = outline.iter().next().unwrap();
        assert_eq!(
            (
                first.title().unwrap(),
                first.target().unwrap().and_then(|d| d.page_index)
            ),
            ("Contents".to_owned(), Some(0)),
            "{mode:?}"
        );
        assert_eq!(after.warnings(), [], "{mode:?}");
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

/// capability: A.02 core write
/// capability: A.03 core round-trip
#[test]
fn text_edits_round_trip_and_read_back_as_typed() {
    use laminapdf_cos::OpenOptions;
    use laminapdf_ops::{Document, EditText, Op, Origin, text_runs};
    use laminapdf_save::{Save as _, SaveMode, SaveOptions};

    fn texts(doc: &laminapdf_model::Document) -> Vec<String> {
        let page = doc.pages().unwrap().get(0).unwrap().id().clone();
        text_runs(doc, &page).unwrap().into_iter().map(|r| r.text).collect()
    }

    let root = common::fixtures_dir();
    // Helvetica, not embedded, and the invoice's embedded subsets in a tagged file.
    for name in ["golden/examples/hello.pdf", "golden/examples/invoice.pdf"] {
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let bytes = std::fs::read(root.join(name)).unwrap();
            let mut doc = Document::open(bytes, OpenOptions::default()).unwrap();
            let pages = doc.pages().unwrap().len();
            let page = doc.pages().unwrap().get(0).unwrap().id().clone();
            let runs = text_runs(&doc, &page).unwrap();
            let editable: Vec<_> = runs.iter().filter(|r| r.read_only.is_none()).collect();
            // The first run's text reversed: characters its font already draws.
            let first = editable[0];
            let reversed: String = first.text.chars().rev().collect();
            let mut ops = vec![Op::from(EditText::new(page.clone(), first.run, reversed.clone()))];
            // The last run deleted, when it is in another text object.
            let last = editable.last().filter(|r| r.run.object != first.run.object);
            if let Some(last) = last {
                ops.push(EditText::new(page.clone(), last.run, String::new()).into());
            }
            doc.apply_all(Origin::Local, ops).unwrap();
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
            let reopened = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
            let again = texts(&reopened);
            assert!(again.contains(&reversed), "{name} {mode:?}: {again:?}");
            if let Some(last) = last {
                assert_eq!(
                    again.iter().filter(|t| **t == last.text).count() + 1,
                    runs.iter().filter(|r| r.text == last.text).count(),
                    "{name} {mode:?}"
                );
            }
            assert_eq!(reopened.pages().unwrap().len(), pages, "{name} {mode:?}");
            assert_eq!(reopened.warnings(), [], "{name} {mode:?}");
        }
    }
}

/// capability: A.07 core round-trip
/// capability: B.01 core round-trip
/// capability: B.02 core round-trip
/// capability: B.08 core round-trip
/// capability: B.12 core round-trip
#[test]
fn content_edits_round_trip_with_the_rest_of_the_page_unchanged() {
    use laminapdf_content::{ContentObjectKind, ContentOps, ContentOwner, interpret_page};
    use laminapdf_cos::OpenOptions;
    use laminapdf_model::{Matrix, RectPt};
    use laminapdf_ops::{ContentEdit, Document, EditContent, ObjectRef, Op, Origin, StackPosition};
    use laminapdf_save::{Save as _, SaveMode, SaveOptions};

    /// The first page's objects: kind, name, box (rounded to a hundredth of a point) and whether
    /// it can be edited.
    fn objects(doc: &laminapdf_model::Document) -> Vec<(ContentObjectKind, ObjectRef, Option<[i64; 4]>, bool)> {
        let pages = doc.pages().unwrap();
        let page = pages.get(0).unwrap();
        let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(page.id().clone())).unwrap();
        interpret_page(doc, &page, &mut ir.recorder(page.frame().unwrap())).unwrap();
        #[expect(clippy::cast_possible_truncation, reason = "rounded points, well inside i64")]
        let round = |r: RectPt| [r.x0, r.y0, r.x1, r.y1].map(|v| (v.get() * 100.0).round() as i64);
        ir.objects()
            .iter()
            .map(|o| {
                let name = ObjectRef::from(&ir.reference(o.id()).unwrap());
                (o.kind(), name, o.bounds().map(round), ir.edit_refusal(o.id()).is_none())
            })
            .collect()
    }

    let root = common::fixtures_dir();
    let jpeg = std::fs::read(root.join("images/photo-rgb.jpg")).unwrap();
    for (name, kinds) in [
        // Shapes and images: an image replaced and cropped, a shape scaled, a shape to the back.
        (
            "golden/examples/images_and_vectors.pdf",
            [ContentObjectKind::Image, ContentObjectKind::Path],
        ),
        // Text: a line of text moved.
        (
            "golden/examples/multilingual.pdf",
            [ContentObjectKind::Text, ContentObjectKind::Text],
        ),
    ] {
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let bytes = std::fs::read(root.join(name)).unwrap();
            let mut doc = Document::open(bytes, OpenOptions::default())
                .unwrap()
                .with_images(laminapdf_build::ImageXObjectWriter);
            let blob = doc.add_blob(jpeg.clone());
            let before = objects(&doc);
            let first = before.iter().position(|o| o.0 == kinds[0] && o.3).unwrap();
            // The third shape, the dashed curve, sits outside any transparency, so it can go
            // to the back.
            let last = (0..before.len())
                .filter(|&i| before[i].0 == kinds[1] && before[i].3)
                .nth(2)
                .unwrap();
            let mut edits = vec![ContentEdit::Transform {
                object: before[first].1,
                matrix: Matrix::translate(-10.0, 0.0).then(&Matrix::scale(0.5, 0.5)),
            }];
            if kinds[0] == ContentObjectKind::Image {
                edits.push(ContentEdit::ReplaceImage {
                    object: before[first].1,
                    blob,
                });
                edits.push(ContentEdit::Reorder {
                    object: before[last].1,
                    to: StackPosition::Back,
                });
            }
            let page = doc.pages().unwrap().get(0).unwrap().id().clone();
            doc.apply_all(Origin::Local, [Op::from(EditContent::new(page, edits))])
                .unwrap();
            let edited = objects(&doc);
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
            let reopened = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
            let again = objects(&reopened);
            assert_eq!(again, edited, "{name} {mode:?}");
            assert_eq!(reopened.warnings(), [], "{name} {mode:?}");
            // Every object kept its box but the one moved and halved, x' = (x - 10) / 2 and
            // y' = y / 2, in hundredths of a point and to within one.
            let mut order: Vec<usize> = (0..before.len()).collect();
            if kinds[0] == ContentObjectKind::Image {
                let back = order.remove(last);
                order.insert(0, back);
            }
            assert_eq!(order.len(), edited.len(), "{name} {mode:?}");
            for (now, &was) in edited.iter().zip(&order) {
                assert_eq!(now.0, before[was].0, "{name} {mode:?}");
                if was != first {
                    assert_eq!(now.2, before[was].2, "{name} {mode:?}: object {was}");
                    continue;
                }
                let (Some(now), Some(was)) = (now.2, before[was].2) else {
                    panic!("{name} {mode:?}: the moved object has no box");
                };
                let want = [was[0] / 2 - 500, was[1] / 2, was[2] / 2 - 500, was[3] / 2];
                assert!(
                    now.iter().zip(want).all(|(n, w)| (n - w).abs() <= 1),
                    "{name} {mode:?}: {now:?}, expected {want:?}"
                );
            }
        }
    }
}

/// A file attached to a page, one renamed and one deleted reopen as the tree lists them, byte for
/// byte.
/// capability: J.05 core round-trip
#[test]
fn attachment_edits_save_and_reopen_in_lamina() {
    check(|root, suite| OtherCases::Attachments.run(root, suite));
}
