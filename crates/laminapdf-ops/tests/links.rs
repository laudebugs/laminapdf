//! The link ops: `AddLink` on an area and on text, `UpdateLink`, `DeleteLink`, their refusals,
//! and a tagged link's place in the structure tree (`golden/tagged-report.pdf`: a `Link`
//! element holding the content it covers, which goes back when the link is deleted).
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open};
use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{AnnotId, LinkAction, PageId, UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddLink, Applied, DeleteLink, DestinationView, Document, ExplicitDestination, ModificationClass, Op, Origin,
    OutlineTarget, UpdateLink,
};

fn apply(doc: &mut Document, op: impl Into<Op>) -> Applied {
    doc.apply_all(Origin::Local, [op.into()]).unwrap()
}

fn refused(doc: &mut Document, op: impl Into<Op>) -> &'static str {
    doc.apply_all(Origin::Local, [op.into()]).unwrap_err().code()
}

fn page(doc: &Document, index: usize) -> PageId {
    doc.pages().unwrap().get(index).unwrap().id().clone()
}

fn rect() -> UserSpaceRect {
    UserSpaceRect::from_corners(72.0, 700.0, 180.0, 714.0)
}

fn web() -> OutlineTarget {
    OutlineTarget::Uri("https://example.com/".into())
}

fn dict(doc: &Document, r: ObjRef) -> Dict {
    doc.resolve(r).unwrap().as_dict().unwrap().clone()
}

fn obj(id: &AnnotId) -> ObjRef {
    id.as_lamina_id().obj_ref().unwrap()
}

/// The links of page `index`, as the model reads them.
fn links(doc: &Document, index: usize) -> Vec<laminapdf_model::Link> {
    let pages = doc.pages().unwrap();
    pages.links(&pages.get(index).unwrap()).unwrap()
}

mod add {
    use super::*;

    #[test]
    fn a_link_on_an_area_goes_where_it_says_and_has_no_border() {
        let mut doc = open(&fixture("golden/links-and-outlines.pdf"));
        let before = links(&doc, 0).len();
        let mut add = AddLink::new(page(&doc, 0), rect(), web());
        add.contents = Some("Example site".into());
        let id = apply(&mut doc, add).created_annotations.pop().unwrap();
        let link = links(&doc, 0).pop().unwrap();
        let annot = dict(&doc, obj(&id));
        assert_eq!(
            (
                links(&doc, 0).len(),
                link.action,
                link.rect,
                link.contents.as_deref(),
                annot.get("Border").cloned(),
            ),
            (
                before + 1,
                LinkAction::Uri(b"https://example.com/".to_vec()),
                rect(),
                Some("Example site"),
                Some(Object::Array(vec![Object::Real(0.0); 3])),
            )
        );
    }

    #[test]
    fn a_link_on_text_bounds_its_quads() {
        let mut doc = open(&fixture("golden/links-and-outlines.pdf"));
        let quad = [
            UserSpacePoint::new(60.0, 690.0),
            UserSpacePoint::new(200.0, 690.0),
            UserSpacePoint::new(60.0, 704.0),
            UserSpacePoint::new(200.0, 704.0),
        ];
        let mut add = AddLink::new(page(&doc, 0), rect(), web());
        add.quads = vec![quad];
        apply(&mut doc, add);
        let link = links(&doc, 0).pop().unwrap();
        assert_eq!(
            (link.rect, link.quads),
            (UserSpaceRect::from_corners(60.0, 690.0, 200.0, 714.0), vec![quad])
        );
    }

    #[test]
    fn a_page_target_and_its_view_are_written() {
        let mut doc = open(&fixture("golden/links-and-outlines.pdf"));
        let to = ExplicitDestination::new(page(&doc, 1), DestinationView::Fit);
        let op = AddLink::new(page(&doc, 0), rect(), OutlineTarget::Destination(to));
        apply(&mut doc, op);
        let LinkAction::GoTo(dest) = links(&doc, 0).pop().unwrap().action else {
            panic!("not a GoTo");
        };
        assert_eq!(dest.page_index, Some(1));
    }

    #[test]
    fn a_script_address_and_a_coordinate_that_isnt_finite_are_refused() {
        let mut doc = open(&fixture("golden/links-and-outlines.pdf"));
        let script = AddLink::new(page(&doc, 0), rect(), OutlineTarget::Uri("javascript:alert(1)".into()));
        let mut nan = AddLink::new(page(&doc, 0), rect(), web());
        nan.quads = vec![[UserSpacePoint::new(f64::NAN, 0.0); 4]];
        assert_eq!(
            (refused(&mut doc, script), refused(&mut doc, nan)),
            ("link-uri-refused", "invalid-op")
        );
    }
}

mod update_and_delete {
    use super::*;

    fn with_link() -> (Document, AnnotId) {
        let mut doc = open(&fixture("golden/links-and-outlines.pdf"));
        let mut add = AddLink::new(page(&doc, 0), rect(), web());
        add.contents = Some("Example site".into());
        let id = apply(&mut doc, add).created_annotations.pop().unwrap();
        (doc, id)
    }

    #[test]
    fn an_update_changes_the_target_and_removes_the_description() {
        let (mut doc, id) = with_link();
        let mut update = UpdateLink::new(id);
        update.target = Some(OutlineTarget::Uri("mailto:team@example.com".into()));
        update.contents = Some(None);
        apply(&mut doc, update);
        let link = links(&doc, 0).pop().unwrap();
        assert_eq!(
            (link.action, link.contents),
            (LinkAction::Uri(b"mailto:team@example.com".to_vec()), None)
        );
    }

    #[test]
    fn an_update_that_changes_nothing_leaves_no_step() {
        let (mut doc, id) = with_link();
        let mut update = UpdateLink::new(id);
        update.contents = Some(Some("Example site".into()));
        assert!(apply(&mut doc, update).undo.is_empty());
    }

    #[test]
    fn a_delete_removes_the_link_from_its_page() {
        let (mut doc, id) = with_link();
        let before = links(&doc, 0).len();
        apply(&mut doc, DeleteLink::new(id));
        assert_eq!(links(&doc, 0).len(), before - 1);
    }

    #[test]
    fn an_annotation_that_isnt_a_link_is_refused() {
        let mut doc = open(&fixture("annotations/threads.pdf"));
        assert_eq!(
            refused(&mut doc, DeleteLink::new("a16.0".parse().unwrap())),
            "not-a-link"
        );
    }
}

mod tagged {
    use super::*;

    /// The tagged file, and page 1's MCID 0 with the element that owns it.
    fn file() -> (Document, ObjRef) {
        let doc = open(&fixture("golden/tagged-report.pdf"));
        let owner = owners(&doc)[0].as_ref().unwrap();
        (doc, owner)
    }

    /// Page 1's parent tree entry: the owner of each MCID.
    fn owners(doc: &Document) -> Vec<Object> {
        let page = doc.pages().unwrap().get(0).unwrap().obj_ref().unwrap();
        let key = dict(doc, page).get_int("StructParents").unwrap();
        let root = doc.catalog().unwrap().get("StructTreeRoot").unwrap().as_ref().unwrap();
        let tree = dict(doc, root).get("ParentTree").cloned().unwrap();
        let entries = doc.number_tree(tree).entries().unwrap();
        let value = entries.into_iter().find(|(k, _)| *k == key).unwrap().1;
        doc.deref(&value).unwrap().as_array().unwrap().to_vec()
    }

    fn kids(doc: &Document, elem: ObjRef) -> Vec<Object> {
        match dict(doc, elem).get("K").cloned() {
            Some(Object::Array(k)) => k,
            Some(k) => vec![k],
            None => Vec::new(),
        }
    }

    fn add(doc: &mut Document) -> (AnnotId, ObjRef) {
        let mut op = AddLink::new(page(doc, 0), rect(), web());
        op.marked = vec![0];
        let id = apply(doc, op).created_annotations.pop().unwrap();
        let key = dict(doc, obj(&id)).get_int("StructParent").unwrap();
        let root = doc.catalog().unwrap().get("StructTreeRoot").unwrap().as_ref().unwrap();
        let tree = dict(doc, root).get("ParentTree").cloned().unwrap();
        let elem = doc
            .number_tree(tree)
            .entries()
            .unwrap()
            .into_iter()
            .find(|(k, _)| *k == key)
            .unwrap()
            .1
            .as_ref()
            .unwrap();
        (id, elem)
    }

    #[test]
    fn the_link_element_takes_the_covered_content_and_its_place() {
        let (mut doc, owner) = file();
        let before = kids(&doc, owner);
        let (id, link) = add(&mut doc);
        let elem = dict(&doc, link);
        let after = kids(&doc, owner);
        let place = after.iter().position(|k| k.as_ref() == Some(link));
        assert_eq!(
            (
                elem.get_name("S").map(<[u8]>::to_vec),
                elem.get("P").and_then(Object::as_ref),
                owners(&doc)[0].as_ref(),
                kids(&doc, link).len(),
                after.len(),
                place.is_some(),
            ),
            (Some(b"Link".to_vec()), Some(owner), Some(link), 2, before.len(), true,),
            "the link {id} is {elem:?}"
        );
    }

    #[test]
    fn the_page_gets_the_structure_tab_order() {
        let (mut doc, _) = file();
        add(&mut doc);
        let page = doc.pages().unwrap().get(0).unwrap().obj_ref().unwrap();
        assert_eq!(dict(&doc, page).get_name("Tabs"), Some(b"S".as_slice()));
    }

    #[test]
    fn a_delete_puts_the_content_back_where_it_was() {
        let (mut doc, owner) = file();
        let before = kids(&doc, owner);
        let (id, _) = add(&mut doc);
        apply(&mut doc, DeleteLink::new(id));
        assert_eq!((kids(&doc, owner), owners(&doc)[0].as_ref()), (before, Some(owner)));
    }

    #[test]
    fn a_pdf_ua_file_without_a_structure_tree_refuses_a_link() {
        // One page and an XMP packet that claims PDF/UA-1, but no structure tree.
        let xmp = br#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" pdfuaid:part="1"/></rdf:RDF></x:xmpmeta>"#;
        let metadata = common::stream_body("/Type /Metadata /Subtype /XML", xmp);
        let bytes = common::classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>",
                &metadata,
            ],
            "<< /Size 5 /Root 1 0 R >>",
        );
        let mut doc = open(&bytes);
        let op = AddLink::new(page(&doc, 0), rect(), web());
        assert_eq!(refused(&mut doc, op), "link-not-tagged");
    }
}

#[test]
fn every_op_is_an_annotation_change_saved_incrementally() {
    let id: AnnotId = "a3.0".parse().unwrap();
    let ops: Vec<Op> = vec![
        AddLink::new("p3.0".parse().unwrap(), rect(), web()).into(),
        UpdateLink::new(id.clone()).into(),
        DeleteLink::new(id).into(),
    ];
    for op in ops {
        assert_eq!(op.class(), ModificationClass::Annotation, "{op:?}");
        assert!(!op.requires_rewrite(), "{op:?}");
        assert!(op.page_changes().is_empty(), "{op:?}");
    }
}

#[test]
fn every_op_undone_leaves_the_overlay_empty() {
    for tagged in [false, true] {
        let file = if tagged {
            "golden/tagged-report.pdf"
        } else {
            "golden/links-and-outlines.pdf"
        };
        let mut doc = open(&fixture(file));
        let mut add = AddLink::new(page(&doc, 0), rect(), web());
        add.marked = vec![0];
        let id = apply(&mut doc, add).created_annotations.pop().unwrap();
        let mut update = UpdateLink::new(id.clone());
        update.rect = Some(UserSpaceRect::from_corners(0.0, 0.0, 10.0, 10.0));
        apply(&mut doc, update);
        apply(&mut doc, DeleteLink::new(id));
        for _ in 0..3 {
            doc.undo().unwrap();
        }
        assert!(!doc.is_dirty(), "{file}");
    }
}
