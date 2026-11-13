//! `DuplicatePages` on hand-built files a fixture doesn't hold: a parent element whose `/K` is
//! an indirect array, a field the form's `/Fields` doesn't reach, a page stored as a direct
//! dictionary, a page drawing a Form XObject with its own `/StructParents`; and `DeletePages`
//! on a form without `/Fields`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{open, page_ids};
use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::PagePosition;
use laminapdf_ops::{DeletePages, Document, DuplicatePages, Error, FieldOutcome, Origin};
use laminapdf_save::test_support::Writer;

fn dict(entries: Vec<(&str, Object)>) -> Dict {
    entries.into_iter().collect()
}

fn text(s: &str) -> Object {
    Object::String(PdfString::literal(s))
}

/// A document of `pages` (dictionaries without `/Parent`, set here) under one `/Pages` node,
/// with `catalog` entries added to the catalog; returns it and each page's object.
fn build(w: &mut Writer, pages: Vec<Dict>, catalog: Vec<(&str, Object)>) -> (Document, Vec<ObjRef>) {
    let node = w.reserve();
    let refs: Vec<ObjRef> = pages
        .into_iter()
        .map(|mut d| {
            d.insert("Type", Object::name("Page"));
            d.insert("Parent", Object::Ref(node));
            w.add(d)
        })
        .collect();
    w.set(
        node,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", refs.iter().map(|r| Object::Ref(*r)).collect::<Vec<_>>().into()),
            ("Count", i64::try_from(refs.len()).unwrap().into()),
            ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ]),
    );
    let mut cat = dict(vec![("Type", Object::name("Catalog")), ("Pages", node.into())]);
    for (k, v) in catalog {
        cat.insert(k, v);
    }
    let root = w.add(cat);
    let bytes = w.finish(&dict(vec![("Root", root.into())])).unwrap();
    (open(&bytes), refs)
}

fn duplicate_first(doc: &mut Document) -> laminapdf_ops::Applied {
    let first = page_ids(doc)[0].clone();
    doc.apply_all(
        Origin::Local,
        [DuplicatePages::new(vec![first], PagePosition::End).into()],
    )
    .unwrap()
}

#[test]
fn a_clone_goes_into_a_parents_indirect_kids_array() {
    let mut w = Writer::new(1, 7);
    let (st, doc_elem, para, kids) = (w.reserve(), w.reserve(), w.reserve(), w.reserve());
    let page = dict(vec![("StructParents", 0.into())]);
    // The page object is the first added after the four reserved above.
    let page_ref = ObjRef::new(6, 0);
    w.set(
        para,
        dict(vec![
            ("Type", Object::name("StructElem")),
            ("S", Object::name("P")),
            ("P", doc_elem.into()),
            ("Pg", page_ref.into()),
            ("K", 0.into()),
        ]),
    );
    w.set(kids, Object::Array(vec![para.into()]));
    w.set(
        doc_elem,
        dict(vec![
            ("Type", Object::name("StructElem")),
            ("S", Object::name("Document")),
            ("P", st.into()),
            ("K", kids.into()),
        ]),
    );
    let tree = dict(vec![("Nums", vec![0.into(), vec![para.into()].into()].into())]);
    w.set(
        st,
        dict(vec![
            ("Type", Object::name("StructTreeRoot")),
            ("K", doc_elem.into()),
            ("ParentTree", Object::Dict(tree)),
        ]),
    );
    let (mut doc, refs) = build(&mut w, vec![page], vec![("StructTreeRoot", st.into())]);
    assert_eq!(refs[0], page_ref);
    duplicate_first(&mut doc);
    let doc_kids = doc.resolve(doc_elem).unwrap().as_dict().unwrap().get("K").cloned();
    let list = doc.resolve(kids).unwrap().as_array().unwrap().to_vec();
    assert_eq!(doc_kids, Some(Object::Ref(kids)), "/K still names the array object");
    assert_eq!(list.len(), 2);
    assert_eq!(list[0], Object::Ref(para));
}

#[test]
fn a_name_only_an_orphan_field_uses_is_not_given_to_a_copy() {
    let mut w = Writer::new(1, 7);
    let listed = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Widget")),
        ("FT", Object::name("Tx")),
        ("T", text("name")),
    ]));
    // A field named "name#2" that no /Fields reaches, on page 2.
    let orphan = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Widget")),
        ("FT", Object::name("Tx")),
        ("T", text("name#2")),
    ]));
    let pages = vec![
        dict(vec![("Annots", vec![listed.into()].into())]),
        dict(vec![("Annots", vec![orphan.into()].into())]),
    ];
    let form = dict(vec![("Fields", vec![listed.into()].into())]);
    let (mut doc, _) = build(&mut w, pages, vec![("AcroForm", Object::Dict(form))]);
    let applied = duplicate_first(&mut doc);
    let names: Vec<_> = applied
        .fields
        .iter()
        .map(|f| match &f.outcome {
            FieldOutcome::Renamed { name, .. } => name.clone(),
            _ => String::new(),
        })
        .collect();
    assert_eq!(names, ["name#3"]);
}

#[test]
fn a_page_stored_inside_its_parent_is_refused() {
    let mut w = Writer::new(1, 7);
    let node = w.reserve();
    let direct = dict(vec![("Type", Object::name("Page")), ("Parent", node.into())]);
    w.set(
        node,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", vec![Object::Dict(direct)].into()),
            ("Count", 1.into()),
            ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ]),
    );
    let root = w.add(dict(vec![("Type", Object::name("Catalog")), ("Pages", node.into())]));
    let mut doc = open(&w.finish(&dict(vec![("Root", root.into())])).unwrap());
    let page = page_ids(&doc)[0].clone();
    let err = doc
        .apply_all(
            Origin::Local,
            [DuplicatePages::new(vec![page], PagePosition::End).into()],
        )
        .unwrap_err();
    assert!(matches!(err, Error::DirectPage(_)), "{err:?}");
}

#[test]
fn a_page_drawing_a_tagged_form_xobject_is_reported_incomplete() {
    let mut w = Writer::new(1, 7);
    let st = w.reserve();
    let form = w.add(Object::Stream(laminapdf_cos::Stream::new(
        dict(vec![
            ("Type", Object::name("XObject")),
            ("Subtype", Object::name("Form")),
            ("BBox", vec![0.into(), 0.into(), 10.into(), 10.into()].into()),
            ("StructParents", 1.into()),
        ]),
        b"/P <</MCID 0>> BDC EMC".to_vec(),
    )));
    let resources = dict(vec![("XObject", Object::Dict(dict(vec![("Fm1", form.into())])))]);
    let page = dict(vec![("Resources", Object::Dict(resources))]);
    w.set(
        st,
        dict(vec![
            ("Type", Object::name("StructTreeRoot")),
            (
                "ParentTree",
                Object::Dict(dict(vec![("Nums", Object::Array(Vec::new()))])),
            ),
        ]),
    );
    let (mut doc, _) = build(&mut w, vec![page], vec![("StructTreeRoot", st.into())]);
    let applied = duplicate_first(&mut doc);
    assert_eq!(applied.tags_incomplete, vec![page_ids(&doc)[1].clone()]);
}

#[test]
fn deleting_a_page_leaves_a_form_without_fields_alone() {
    let mut w = Writer::new(1, 7);
    let widget = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Widget")),
        ("FT", Object::name("Tx")),
        ("T", text("x")),
    ]));
    let form = w.add(dict(vec![("DA", text("/Helv 0 Tf 0 g"))]));
    let pages = vec![dict(vec![("Annots", vec![widget.into()].into())]), Dict::new()];
    let (mut doc, _) = build(&mut w, pages, vec![("AcroForm", form.into())]);
    let first = page_ids(&doc)[0].clone();
    doc.apply_all(Origin::Local, [DeletePages::new(vec![first]).into()])
        .unwrap();
    assert!(!doc.resolve(form).unwrap().as_dict().unwrap().contains_key("Fields"));
}
