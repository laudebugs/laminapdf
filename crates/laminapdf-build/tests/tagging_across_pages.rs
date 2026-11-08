//! Structure elements across pages (todo T4f): `tagged-across-pages.pdf`'s paragraph and table
//! are one element each, whose content is on two pages, read back by laminapdf-model; every
//! page's `ParentTree` entries name them; a link on a later page is an `OBJR` with its own
//! `/Pg`; an element opened and closed on one page writes what [`Surface::tag`] writes; an
//! element never closed, closed twice or resumed by another builder fails with its code; a page
//! that fails takes its content of an open element with it.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::collections::BTreeMap;

use common::across_pages::{LINK, tagged_across_pages};
use common::{reopen, save, text_and_navigation::font};
use laminapdf_build::{
    ArtifactKind, BuildOptions, Builder, Error, FontHandle, LinkTarget, PageSize, Standard14, Surface, TagKind,
};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{PageId, PointPt, Pt, RectPt, StructElem, StructKid, StructTree};
use laminapdf_test_fixtures::fixture;

fn tree(doc: &laminapdf_model::Document) -> StructTree {
    doc.structure().unwrap().unwrap()
}

fn page_ids(doc: &laminapdf_model::Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

/// The pages of an element's marked content, in its `/K` order (one entry per sequence).
fn mcid_pages(elem: &StructElem, pages: &[PageId]) -> Vec<usize> {
    elem.kids
        .iter()
        .filter_map(|k| match k {
            StructKid::Mcid { page, .. } => pages.iter().position(|p| Some(p) == page.as_ref()),
            _ => None,
        })
        .collect()
}

fn find<'t>(tree: &'t StructTree, kind: &str) -> Vec<&'t StructElem> {
    tree.elements()
        .into_iter()
        .filter(|e| e.kind == kind.as_bytes())
        .collect()
}

#[test]
fn the_golden_is_rebuilt() {
    // Not `assert_eq!`: a byte diff of two PDFs is unreadable.
    assert!(
        tagged_across_pages() == fixture("golden/tagged-across-pages.pdf"),
        "stale tagged-across-pages.pdf"
    );
}

#[test]
fn the_paragraph_is_one_element_with_content_on_both_pages() {
    let doc = reopen(&fixture("golden/tagged-across-pages.pdf"));
    let tree = tree(&doc);
    let paragraphs = find(&tree, "P");
    assert_eq!(
        (paragraphs.len(), mcid_pages(paragraphs[0], &page_ids(&doc))),
        (2, vec![0, 1])
    );
}

#[test]
fn the_table_is_one_element_whose_rows_are_on_two_pages() {
    let doc = reopen(&fixture("golden/tagged-across-pages.pdf"));
    let tree = tree(&doc);
    let pages = page_ids(&doc);
    let tables = find(&tree, "Table");
    let row_pages: Vec<Option<usize>> = tables[0]
        .children()
        .map(|tr| pages.iter().position(|p| Some(p) == tr.page.as_ref()))
        .collect();
    assert_eq!((tables.len(), row_pages), (1, vec![Some(1), Some(1), Some(2), Some(2)]));
}

#[test]
fn every_page_parent_tree_entry_names_the_owner_of_its_mcid() {
    let doc = reopen(&fixture("golden/tagged-across-pages.pdf"));
    let tree = tree(&doc);
    let parent_tree = doc.number_tree(tree.parent_tree.clone().unwrap());
    let owners: BTreeMap<(PageId, i64), ObjRef> = tree
        .elements()
        .into_iter()
        .flat_map(|e| {
            e.kids.iter().filter_map(move |k| match k {
                StructKid::Mcid { page, mcid } => Some(((page.clone().unwrap(), *mcid), e.object.unwrap())),
                _ => None,
            })
        })
        .collect();
    for (i, page) in doc.pages().unwrap().iter().enumerate() {
        let key = page.dict().get("StructParents").and_then(Object::as_int).unwrap();
        let Some(Object::Array(entry)) = parent_tree.get(key).unwrap() else {
            panic!("page {i}: no array under its /StructParents key")
        };
        for (mcid, owner) in entry.iter().enumerate() {
            let expected = owners.get(&(page.id().clone(), i64::try_from(mcid).unwrap())).copied();
            assert_eq!(owner.as_ref(), expected, "page {i}, MCID {mcid}");
        }
    }
}

#[test]
fn the_link_on_the_second_page_is_an_objr_with_that_page() {
    let doc = reopen(&fixture("golden/tagged-across-pages.pdf"));
    let tree = tree(&doc);
    let pages = page_ids(&doc);
    let link = find(&tree, "Link")[0];
    let objr_page = link.kids.iter().find_map(|k| match k {
        StructKid::Object { page, .. } => pages.iter().position(|p| Some(p) == page.as_ref()),
        _ => None,
    });
    let pages = doc.pages().unwrap();
    let Some(Object::Array(annots)) = pages.get(1).unwrap().dict().get("Annots").cloned() else {
        panic!("page 2 has no /Annots")
    };
    let annot = doc.resolve(annots[0].as_ref().unwrap()).unwrap();
    let contents = match annot.as_dict().and_then(|d| d.get("Contents")) {
        Some(Object::String(s)) => Some(s.bytes.clone()),
        _ => None,
    };
    assert_eq!((objr_page, contents), (Some(1), Some(LINK.as_bytes().to_vec())));
}

#[test]
fn every_footer_is_a_pagination_artifact_with_the_footer_subtype() {
    let bytes = fixture("golden/tagged-across-pages.pdf");
    let footer: &[u8] = b"/Artifact <<\n  /Type /Pagination\n  /Subtype /Footer\n  /Attached [/Bottom]\n>> BDC";
    let count = bytes.windows(footer.len()).filter(|w| *w == footer).count();
    assert_eq!(count, 3);
}

#[test]
fn an_element_opened_and_closed_on_one_page_writes_what_tag_writes() {
    let text = |s: &mut Surface, font: &FontHandle| {
        s.text("One page", font, Pt::new(11.0), PointPt::from_points(72.0, 96.0))
            .map(|_| ())
    };
    let build = |across: bool| {
        let mut b = Builder::new(
            BuildOptions::default()
                .with_id(*b"lamina-build-T4f")
                .with_creation_date("D:20261108120000Z".parse().unwrap())
                .with_compress(false),
        );
        let font = b.standard_font(Standard14::Helvetica).unwrap();
        let mut page = b.page(PageSize::A4);
        let s = page.surface();
        s.tag(TagKind::H1, |s| text(s, &font)).unwrap();
        let id = if across {
            let id = s.open_element(TagKind::P);
            s.resume(id, |s| text(s, &font)).unwrap();
            Some(id)
        } else {
            s.tag(TagKind::P, |s| text(s, &font)).unwrap();
            None
        };
        s.tag(TagKind::P, |s| text(s, &font)).unwrap();
        page.finish().unwrap();
        if let Some(id) = id {
            b.close_element(id).unwrap();
        }
        save(b.finish().unwrap())
    };
    // Not `assert_eq!`: a byte diff of two PDFs is unreadable.
    assert!(build(true) == build(false), "the two documents differ");
}

fn builder() -> Builder {
    Builder::new(BuildOptions::default().with_tagged(true))
}

#[test]
fn an_element_never_closed_fails_the_build() {
    let mut b = builder();
    let mut page = b.page(PageSize::A4);
    page.surface().open_element(TagKind::P);
    page.finish().unwrap();
    let err = b.finish().unwrap_err();
    assert_eq!(
        (err.code(), matches!(err, Error::TagElementUnclosed { count: 1 })),
        ("tag-element-unclosed", true)
    );
}

#[test]
fn closing_an_element_twice_fails() {
    let mut b = builder();
    let mut page = b.page(PageSize::A4);
    let id = page.surface().open_element(TagKind::P);
    page.finish().unwrap();
    b.close_element(id).unwrap();
    assert_eq!(b.close_element(id).unwrap_err().code(), "tag-element-not-open");
}

#[test]
fn resuming_another_builders_element_fails_the_page() {
    let mut other = builder();
    let mut page = other.page(PageSize::A4);
    let id = page.surface().open_element(TagKind::P);
    page.finish().unwrap();
    let mut b = builder();
    let mut page = b.page(PageSize::A4);
    page.surface().resume(id, |_| Ok(())).unwrap();
    assert_eq!(page.finish().unwrap_err().code(), "tag-element-not-open");
}

#[test]
fn an_element_opened_inside_an_artifact_is_never_open() {
    let mut b = builder();
    let mut page = b.page(PageSize::A4);
    let mut id = None;
    page.surface()
        .artifact(ArtifactKind::Layout, |s| {
            id = Some(s.open_element(TagKind::P));
            Ok(())
        })
        .unwrap();
    page.finish().unwrap();
    assert_eq!(b.close_element(id.unwrap()).unwrap_err().code(), "tag-element-not-open");
}

#[test]
fn a_page_that_fails_takes_its_content_of_an_open_element_with_it() {
    let mut b = builder();
    let font = b.font(font("NotoSans-Regular.ttf")).unwrap();
    let text = |s: &mut Surface, y: f64| {
        s.text("Line", &font, Pt::new(11.0), PointPt::from_points(72.0, y))
            .map(|_| ())
    };
    let mut page = b.page(PageSize::A4);
    let id = page.surface().open_element(TagKind::P);
    page.surface().resume(id, |s| text(s, 72.0)).unwrap();
    page.finish().unwrap();
    let mut page = b.page(PageSize::A4);
    page.surface().resume(id, |s| text(s, 72.0)).unwrap();
    // An invalid rectangle fails this page's finish.
    page.surface().link(
        RectPt::from_xywh(Pt::new(f64::NAN), Pt::new(0.0), Pt::new(1.0), Pt::new(1.0)),
        LinkTarget::Uri("https://example.com".into()),
        None,
    );
    assert!(page.finish().is_err());
    let mut page = b.page(PageSize::A4);
    page.surface().resume(id, |s| text(s, 96.0)).unwrap();
    page.finish().unwrap();
    b.close_element(id).unwrap();
    let doc = reopen(&save(b.finish().unwrap()));
    let tree = tree(&doc);
    assert_eq!(mcid_pages(find(&tree, "P")[0], &page_ids(&doc)), vec![0, 1]);
}
