//! `InsertPagesFrom` on a hand-built source that holds what the fixtures don't: a link to a
//! name object from the catalog's `/Dests` with an indirect view operand, a push button whose
//! `GoTo` goes to a page left behind, a Form XObject with its own `/StructParents`, and a form
//! with `/DA` and `/DR`; and what an import leaves behind in the overlay.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids};
use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::PagePosition;
use laminapdf_ops::{Carry, Document, Error, InsertPagesFrom, Origin};
use laminapdf_save::test_support::Writer;

fn dict(entries: Vec<(&str, Object)>) -> Dict {
    entries.into_iter().collect()
}

fn text(s: &str) -> Object {
    Object::String(PdfString::literal(s))
}

/// Two pages; the first holds the link, the button and the form XObject.
fn source() -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let node = w.reserve();
    let (first, second) = (w.reserve(), w.reserve());
    let top = w.add(Object::from(792));
    let form = w.add(Object::Stream(laminapdf_cos::Stream::new(
        dict(vec![
            ("Type", Object::name("XObject")),
            ("Subtype", Object::name("Form")),
            ("BBox", vec![0.into(), 0.into(), 10.into(), 10.into()].into()),
            ("StructParents", 5.into()),
        ]),
        b"0 0 m".to_vec(),
    )));
    let link = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Link")),
        ("Rect", vec![0.into(), 0.into(), 10.into(), 10.into()].into()),
        ("Dest", Object::name("Intro")),
    ]));
    let button = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Widget")),
        ("FT", Object::name("Btn")),
        ("Ff", 65_536.into()),
        ("T", text("go")),
        ("Rect", vec![0.into(), 20.into(), 10.into(), 30.into()].into()),
        (
            "A",
            Object::Dict(dict(vec![
                ("S", Object::name("GoTo")),
                ("D", vec![second.into(), Object::name("Fit")].into()),
            ])),
        ),
    ]));
    let resources = dict(vec![("XObject", Object::Dict(dict(vec![("Fm1", form.into())])))]);
    w.set(
        first,
        dict(vec![
            ("Type", Object::name("Page")),
            ("Parent", node.into()),
            ("Resources", Object::Dict(resources)),
            ("Annots", vec![link.into(), button.into()].into()),
        ]),
    );
    w.set(
        second,
        dict(vec![("Type", Object::name("Page")), ("Parent", node.into())]),
    );
    w.set(
        node,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", vec![first.into(), second.into()].into()),
            ("Count", 2.into()),
            ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ]),
    );
    let helv = w.add(dict(vec![
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]));
    let form_dict = dict(vec![
        ("Fields", vec![button.into()].into()),
        ("DA", text("/Helv 0 Tf 0 g")),
        (
            "DR",
            Object::Dict(dict(vec![("Font", Object::Dict(dict(vec![("Helv", helv.into())])))])),
        ),
    ]);
    let dests = dict(vec![(
        "Intro",
        vec![first.into(), Object::name("XYZ"), 0.into(), top.into(), 0.into()].into(),
    )]);
    let root = w.add(dict(vec![
        ("Type", Object::name("Catalog")),
        ("Pages", node.into()),
        ("AcroForm", Object::Dict(form_dict)),
        ("Dests", Object::Dict(dests)),
    ]));
    w.finish(&dict(vec![("Root", root.into())])).unwrap()
}

/// `hello.pdf` with the source's first page copied in at the end, with `carry`.
fn imported(carry: Carry) -> (Document, ObjRef) {
    let mut doc = open(&fixture("corpus/hello.pdf"));
    let id = doc.add_source(source(), None).unwrap();
    let first = page_ids(doc.source_document(&id).unwrap())[0].clone();
    doc.apply_all(
        Origin::Local,
        [InsertPagesFrom::new(id, vec![first], PagePosition::End)
            .with_carry(carry)
            .into()],
    )
    .unwrap();
    let copy = page_ids(&doc).last().unwrap().as_lamina_id().obj_ref().unwrap();
    (doc, copy)
}

fn resolved(doc: &Document, value: &Object) -> Object {
    match value {
        Object::Ref(r) => (*doc.resolve(*r).unwrap()).clone(),
        other => other.clone(),
    }
}

/// The copy's annotations, by subtype.
fn annot(doc: &Document, page: ObjRef, subtype: &[u8]) -> Option<Dict> {
    let page = doc.resolve(page).unwrap();
    let annots = resolved(doc, page.as_dict().unwrap().get("Annots")?);
    annots.as_array().unwrap().iter().find_map(|a| {
        let d = resolved(doc, a).as_dict().cloned()?;
        (d.get_name("Subtype") == Some(subtype)).then_some(d)
    })
}

#[test]
fn a_link_to_a_name_object_names_a_string_in_the_tree_with_direct_operands() {
    let (doc, copy) = imported(Carry::default());
    let link = annot(&doc, copy, b"Link").unwrap();
    assert_eq!(link.get("Dest"), Some(&text("Intro")));
    let tree = doc.name_tree("Dests").unwrap().unwrap();
    let dest = tree.get(b"Intro").unwrap().unwrap();
    let items = dest.as_array().unwrap();
    assert_eq!(items[0].as_ref(), Some(copy));
    assert_eq!(items[3], Object::from(792), "the indirect operand became a value");
}

#[test]
fn a_button_whose_goto_goes_to_a_page_left_behind_loses_the_action() {
    let (doc, copy) = imported(Carry::default());
    let button = annot(&doc, copy, b"Widget").unwrap();
    assert!(!button.contains_key("A"));
}

#[test]
fn the_form_gets_the_sources_default_appearance_and_resources() {
    let (doc, _) = imported(Carry::default());
    let form = resolved(&doc, doc.catalog().unwrap().get("AcroForm").unwrap());
    let form = form.as_dict().unwrap();
    assert_eq!(form.get("DA"), Some(&text("/Helv 0 Tf 0 g")));
    let dr = resolved(&doc, form.get("DR").unwrap());
    let fonts = resolved(&doc, dr.as_dict().unwrap().get("Font").unwrap());
    assert!(fonts.as_dict().unwrap().contains_key("Helv"));
}

#[test]
fn a_copied_form_xobject_loses_its_structure_key() {
    let (doc, copy) = imported(Carry::default());
    let page = doc.resolve(copy).unwrap();
    let resources = resolved(&doc, page.as_dict().unwrap().get("Resources").unwrap());
    let xobjects = resolved(&doc, resources.as_dict().unwrap().get("XObject").unwrap());
    let form = resolved(&doc, xobjects.as_dict().unwrap().get("Fm1").unwrap());
    let Object::Stream(form) = form else { panic!("a stream") };
    assert!(!form.dict.contains_key("StructParents"));
}

#[test]
fn annotations_left_behind_leave_nothing_in_the_overlay() {
    let (doc, _) = imported(Carry::default().with_annotations(false));
    let links = doc
        .overlay()
        .iter()
        .filter_map(|(_, entry)| entry.object().cloned())
        .filter(|o| o.as_dict().is_some_and(|d| d.get_name("Subtype") == Some(b"Link")))
        .count();
    assert_eq!(links, 0);
}

#[test]
fn extracting_no_pages_is_refused() {
    let doc = open(&fixture("corpus/hello.pdf"));
    assert_eq!(doc.extract_pages(&[]).err(), Some(Error::NoPagesLeft));
}
