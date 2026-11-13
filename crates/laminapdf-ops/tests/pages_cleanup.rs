//! Cleanup cases a fixture doesn't hold, on hand-built files: a surviving element whose kids mix
//! a removed element, marked content on the deleted page and a live element (dropped in one
//! pass, so no index shifts); a grouping element with no `/Pg` left empty; a `/Dests` name that
//! isn't UTF-8; and the values `InsertBlankPages` refuses.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{open, page_ids};
use laminapdf_cos::{Dict, Name, ObjRef, Object, PdfString};
use laminapdf_model::{PagePosition, PageSize, Pt};
use laminapdf_ops::{DeletePages, Document, Error, InsertBlankPages, MovePages, Origin};
use laminapdf_save::test_support::Writer;

fn dict(entries: Vec<(&str, Object)>) -> Dict {
    entries.into_iter().collect()
}

/// Three pages; a `Sect` holds `[E (page 2), MCR on page 2, P (page 1)]`, a `Table` with no
/// `/Pg` holds one `TR` on page 2; `/Dests` holds a name with the byte 0xE9.
fn built() -> (Document, [ObjRef; 4]) {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let p: Vec<ObjRef> = (0..3)
        .map(|_| w.add(dict(vec![("Type", Object::name("Page")), ("Parent", pages.into())])))
        .collect();
    w.set(
        pages,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", p.iter().map(|r| Object::Ref(*r)).collect::<Vec<_>>().into()),
            ("Count", 3.into()),
            ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ]),
    );
    let root = w.reserve();
    let doc_elem = w.reserve();
    let sect = w.reserve();
    let table = w.reserve();
    let elem = |w: &mut Writer, s: &str, parent: ObjRef, page: ObjRef| {
        w.add(dict(vec![
            ("Type", Object::name("StructElem")),
            ("S", Object::name(s)),
            ("P", parent.into()),
            ("Pg", page.into()),
            ("K", 0.into()),
        ]))
    };
    let e = elem(&mut w, "P", sect, p[1]);
    let live = elem(&mut w, "P", sect, p[0]);
    let tr = elem(&mut w, "TR", table, p[1]);
    let mcr = dict(vec![
        ("Type", Object::name("MCR")),
        ("Pg", p[1].into()),
        ("MCID", 1.into()),
    ]);
    w.set(
        sect,
        dict(vec![
            ("Type", Object::name("StructElem")),
            ("S", Object::name("Sect")),
            ("P", doc_elem.into()),
            ("K", vec![e.into(), Object::Dict(mcr), live.into()].into()),
        ]),
    );
    w.set(
        table,
        dict(vec![
            ("Type", Object::name("StructElem")),
            ("S", Object::name("Table")),
            ("P", doc_elem.into()),
            ("K", vec![tr.into()].into()),
        ]),
    );
    w.set(
        doc_elem,
        dict(vec![
            ("Type", Object::name("StructElem")),
            ("S", Object::name("Document")),
            ("P", root.into()),
            ("K", vec![sect.into(), table.into()].into()),
        ]),
    );
    w.set(
        root,
        dict(vec![("Type", Object::name("StructTreeRoot")), ("K", doc_elem.into())]),
    );
    let mut dests = Dict::new();
    dests.insert(
        Name(b"Caf\xE9".to_vec()),
        Object::Array(vec![p[1].into(), Object::name("Fit")]),
    );
    let catalog = w.add(dict(vec![
        ("Type", Object::name("Catalog")),
        ("Pages", pages.into()),
        ("StructTreeRoot", root.into()),
        ("Dests", Object::Dict(dests)),
    ]));
    let bytes = w.finish(&dict(vec![("Root", catalog.into())])).unwrap();
    (open(&bytes), [sect, live, doc_elem, catalog])
}

fn kids(doc: &Document, r: ObjRef) -> Vec<Object> {
    doc.resolve(r)
        .unwrap()
        .as_dict()
        .unwrap()
        .get("K")
        .and_then(Object::as_array)
        .unwrap_or_default()
        .to_vec()
}

#[test]
fn a_surviving_element_keeps_its_live_kid_when_others_go() {
    let (mut doc, [sect, live, ..]) = built();
    let ids = page_ids(&doc);
    doc.apply_all(Origin::Local, [DeletePages::new(vec![ids[1].clone()]).into()])
        .unwrap();
    assert_eq!(kids(&doc, sect), [Object::Ref(live)]);
}

#[test]
fn a_grouping_element_left_empty_goes_too() {
    let (mut doc, [sect, _, doc_elem, _]) = built();
    let ids = page_ids(&doc);
    doc.apply_all(Origin::Local, [DeletePages::new(vec![ids[1].clone()]).into()])
        .unwrap();
    assert_eq!(kids(&doc, doc_elem), [Object::Ref(sect)]);
}

#[test]
fn a_dests_name_that_isnt_utf8_is_retargeted_in_place() {
    let (mut doc, [.., catalog]) = built();
    let ids = page_ids(&doc);
    doc.apply_all(Origin::Local, [DeletePages::new(vec![ids[1].clone()]).into()])
        .unwrap();
    let object = doc.resolve(catalog).unwrap();
    let dests = object
        .as_dict()
        .unwrap()
        .get("Dests")
        .unwrap()
        .as_dict()
        .unwrap()
        .clone();
    let entries: Vec<(Vec<u8>, Object)> = dests.iter().map(|(k, v)| (k.0.clone(), v.clone())).collect();
    let third = ids[2].as_lamina_id().obj_ref().unwrap();
    assert_eq!(
        entries,
        [(
            b"Caf\xE9".to_vec(),
            Object::Array(vec![third.into(), Object::name("Fit")])
        )]
    );
}

#[test]
fn inserting_refuses_counts_and_sizes_out_of_range() {
    let (mut doc, _) = built();
    let bad = [
        InsertBlankPages::new(PagePosition::End, 0, PageSize::LETTER),
        InsertBlankPages::new(PagePosition::End, 10_001, PageSize::LETTER),
        InsertBlankPages::new(PagePosition::End, 1, PageSize::new(Pt::new(0.0), Pt::new(792.0))),
        InsertBlankPages::new(PagePosition::End, 1, PageSize::new(Pt::new(612.0), Pt::new(f64::NAN))),
        InsertBlankPages::new(PagePosition::End, 1, PageSize::new(Pt::new(20_000.0), Pt::new(792.0))),
    ];
    for op in bad {
        let err = doc.apply_all(Origin::Local, [op.into()]).unwrap_err();
        assert!(matches!(err, Error::InvalidOp(_)), "{err:?}");
    }
}

#[test]
fn moving_lists_the_moved_pages_as_touched() {
    let (mut doc, _) = built();
    let ids = page_ids(&doc);
    let applied = doc
        .apply_all(
            Origin::Local,
            [MovePages::new(vec![ids[2].clone()], PagePosition::Start).into()],
        )
        .unwrap();
    let undone = doc.undo().unwrap().unwrap();
    assert_eq!(
        (
            applied.touched_pages.iter().collect::<Vec<_>>(),
            undone.touched_pages.iter().collect::<Vec<_>>()
        ),
        (vec![&ids[2]], vec![&ids[2]])
    );
}

#[test]
fn a_field_whose_indirect_kids_empty_leaves_the_form() {
    let mut w = Writer::new(1, 7);
    let pages = w.reserve();
    let field = w.reserve();
    let kids = w.reserve();
    let widget = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Widget")),
        ("Parent", field.into()),
        ("Rect", vec![0.into(), 0.into(), 10.into(), 10.into()].into()),
    ]));
    let p1 = w.add(dict(vec![("Type", Object::name("Page")), ("Parent", pages.into())]));
    let p2 = w.add(dict(vec![
        ("Type", Object::name("Page")),
        ("Parent", pages.into()),
        ("Annots", vec![widget.into()].into()),
    ]));
    w.set(kids, Object::Array(vec![widget.into()]));
    w.set(
        field,
        dict(vec![
            ("FT", Object::name("Tx")),
            ("T", Object::String(PdfString::literal("name"))),
            ("Kids", kids.into()),
        ]),
    );
    w.set(
        pages,
        dict(vec![
            ("Type", Object::name("Pages")),
            ("Kids", vec![p1.into(), p2.into()].into()),
            ("Count", 2.into()),
            ("MediaBox", vec![0.into(), 0.into(), 612.into(), 792.into()].into()),
        ]),
    );
    let form = w.add(dict(vec![("Fields", vec![field.into()].into())]));
    let catalog = w.add(dict(vec![
        ("Type", Object::name("Catalog")),
        ("Pages", pages.into()),
        ("AcroForm", form.into()),
    ]));
    let mut doc = open(&w.finish(&dict(vec![("Root", catalog.into())])).unwrap());
    let ids = page_ids(&doc);
    doc.apply_all(Origin::Local, [DeletePages::new(vec![ids[1].clone()]).into()])
        .unwrap();
    let fields = doc.resolve(form).unwrap().as_dict().unwrap().get("Fields").cloned();
    assert_eq!(fields, Some(Object::Array(Vec::new())));
}
