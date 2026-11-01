//! Attached files (todo B8b; ISO 32000-2 §7.11, §14.13; ISO 19005-3 §6.8): each attachment is
//! an `/EmbeddedFile` stream with its MIME type, size and modification date, a file
//! specification with `/F`, `/UF`, `/Desc` and `/AFRelationship`, listed by name in
//! `/Names /EmbeddedFiles` and in the catalog's `/AF`; PDF/A-3b accepts them.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{reopen, save};
use lamina_build::{AfRelationship, Attachment, BuildOptions, Builder, Fit, PageSize, Profile};
use lamina_cos::{Dict, Object};

fn options() -> BuildOptions {
    BuildOptions::default()
        .with_id([9; 16])
        .with_creation_date("D:20261031120000Z".parse().unwrap())
}

fn xml() -> Attachment {
    Attachment::new("invoice.xml", b"<invoice/>".to_vec(), "text/xml")
        .with_relationship(AfRelationship::Alternative)
        .with_description("The invoice as XML")
}

/// The document `attach` built, reopened, with its catalog.
fn built(options: BuildOptions, attach: impl FnOnce(&mut Builder)) -> (lamina_model::Document, Dict) {
    let mut b = Builder::new(options);
    b.page(PageSize::A4).finish().unwrap();
    attach(&mut b);
    let doc = reopen(&save(b.finish().unwrap()));
    let cat = doc.catalog().unwrap();
    (doc, cat)
}

fn dict(doc: &lamina_model::Document, o: &Object) -> Dict {
    match o {
        Object::Dict(d) => d.clone(),
        Object::Ref(r) => doc.resolve(*r).unwrap().as_dict().unwrap().clone(),
        _ => panic!("not a dictionary: {o:?}"),
    }
}

/// The `/EmbeddedFiles` tree's (name, file specification) pairs.
fn files(doc: &lamina_model::Document, cat: &Dict) -> Vec<(String, Dict)> {
    let names = dict(doc, cat.get("Names").unwrap());
    let tree = dict(doc, names.get("EmbeddedFiles").unwrap());
    tree.get("Names")
        .and_then(Object::as_array)
        .unwrap()
        .chunks(2)
        .map(|pair| {
            let Object::String(name) = &pair[0] else {
                panic!("not a name")
            };
            (String::from_utf8(name.bytes.clone()).unwrap(), dict(doc, &pair[1]))
        })
        .collect()
}

#[test]
fn an_attachment_is_listed_in_the_embedded_files_tree() {
    let (doc, cat) = built(options(), |b| {
        b.attach(xml());
    });
    assert_eq!(
        files(&doc, &cat).iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["invoice.xml"]
    );
}

#[test]
fn the_file_specification_has_f_uf_desc_and_its_relationship() {
    let (doc, cat) = built(options(), |b| {
        b.attach(xml());
    });
    let (_, spec) = &files(&doc, &cat)[0];
    let text = |k| match spec.get(k) {
        Some(Object::String(s)) => String::from_utf8(s.bytes.clone()).unwrap(),
        other => panic!("{k}: {other:?}"),
    };
    assert_eq!(
        (text("F"), text("UF"), text("Desc"), spec.get_name("AFRelationship")),
        (
            "invoice.xml".into(),
            "invoice.xml".into(),
            "The invoice as XML".into(),
            Some(&b"Alternative"[..])
        )
    );
}

#[test]
fn the_embedded_file_stream_has_its_mime_type_size_and_date() {
    let (doc, cat) = built(options(), |b| {
        b.attach(xml());
    });
    let (_, spec) = &files(&doc, &cat)[0];
    let ef = dict(&doc, spec.get("EF").unwrap());
    let Object::Stream(stream) = &*doc.resolve(ef.get("F").and_then(Object::as_ref).unwrap()).unwrap() else {
        panic!("not a stream")
    };
    let params = dict(&doc, stream.dict.get("Params").unwrap());
    let date = match params.get("ModDate") {
        Some(Object::String(s)) => String::from_utf8(s.bytes.clone()).unwrap(),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        (
            stream.dict.get_name("Subtype"),
            params.get_int("Size"),
            date.as_str(),
            doc.decode_stream(stream).unwrap()
        ),
        (
            Some(&b"text/xml"[..]),
            Some(10),
            "D:20261031120000Z",
            b"<invoice/>".to_vec()
        )
    );
}

#[test]
fn a_given_modification_date_wins_over_the_creation_date() {
    let (doc, cat) = built(options(), |b| {
        b.attach(xml().with_mod_date("D:20250101000000Z".parse().unwrap()));
    });
    let (_, spec) = &files(&doc, &cat)[0];
    let ef = dict(&doc, spec.get("EF").unwrap());
    let Object::Stream(stream) = &*doc.resolve(ef.get("UF").and_then(Object::as_ref).unwrap()).unwrap() else {
        panic!("not a stream")
    };
    let params = dict(&doc, stream.dict.get("Params").unwrap());
    assert_eq!(
        params.get("ModDate"),
        Some(&Object::String(lamina_cos::PdfString::literal(
            b"D:20250101000000Z".to_vec()
        )))
    );
}

#[test]
fn the_catalog_associates_every_attachment_with_the_document() {
    let (doc, cat) = built(options(), |b| {
        b.attach(xml())
            .attach(Attachment::new("data.csv", b"a\n".to_vec(), "text/csv"));
    });
    let af: Vec<_> = cat.get("AF").and_then(Object::as_array).unwrap().to_vec();
    let listed: Vec<Object> = files(&doc, &cat)
        .into_iter()
        .map(|(_, spec)| Object::Dict(spec))
        .collect();
    let resolved: Vec<Object> = af.iter().map(|o| Object::Dict(dict(&doc, o))).collect();
    assert_eq!(resolved, listed);
}

#[test]
fn attachments_are_written_sorted_by_name() {
    let (doc, cat) = built(options(), |b| {
        b.attach(Attachment::new("z.txt", b"z".to_vec(), "text/plain"))
            .attach(Attachment::new("a.txt", b"a".to_vec(), "text/plain"));
    });
    assert_eq!(
        files(&doc, &cat).iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(),
        ["a.txt", "z.txt"]
    );
}

#[test]
fn attaching_a_name_again_replaces_the_file() {
    let (doc, cat) = built(options(), |b| {
        b.attach(xml())
            .attach(Attachment::new("invoice.xml", b"<v2/>".to_vec(), "text/xml"));
    });
    let listed = files(&doc, &cat);
    assert_eq!(
        (listed.len(), listed[0].1.get_name("AFRelationship")),
        (1, Some(&b"Unspecified"[..]))
    );
}

#[test]
fn attachments_keep_the_named_destinations() {
    let (doc, cat) = built(options(), |b| {
        b.destination("start", 0, Fit::Page).attach(xml());
    });
    let names = dict(&doc, cat.get("Names").unwrap());
    assert!(names.get("Dests").is_some() && names.get("EmbeddedFiles").is_some());
}

#[test]
fn pdfa_3b_accepts_attachments() {
    let mut b = Builder::new(options().with_profile(Profile::PdfA3b));
    b.attach(xml());
    assert!(b.finish().is_ok());
}

#[test]
fn a_document_without_attachments_has_no_af() {
    let (_, cat) = built(options(), |_| {});
    assert!(cat.get("AF").is_none() && cat.get("Names").is_none());
}
