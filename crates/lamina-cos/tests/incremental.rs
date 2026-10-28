//! Incremental updates beyond the spike's golden case: encrypted bases, xref-stream bases and
//! `/Size`. Regression tests for the review of PR #8.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::fixture;
use lamina_cos::{Dict, Error, ObjRef, Object, PdfString, Writer, XrefEol, incremental_update, open};

/// A compact base (object stream + xref stream): catalog 1, pages 2, page 3, info 4.
fn compact_base() -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let catalog = w.reserve();
    let pages = w.reserve();
    let page: Dict = [("Type", Object::name("Page")), ("Parent", pages.into())]
        .into_iter()
        .collect();
    let page = w.add(page);
    let info: Dict = [("Title", Object::from(PdfString::literal("base")))]
        .into_iter()
        .collect();
    let info = w.add(info);
    w.set(
        pages,
        [
            ("Type", Object::name("Pages")),
            ("Kids", vec![page.into()].into()),
            ("Count", 1.into()),
        ]
        .into_iter()
        .collect::<Dict>(),
    );
    w.set(
        catalog,
        [("Type", Object::name("Catalog")), ("Pages", pages.into())]
            .into_iter()
            .collect::<Dict>(),
    );
    let trailer: Dict = [("Root", catalog), ("Info", info)].into_iter().collect();
    w.finish_compact(&trailer).unwrap()
}

fn title_update() -> [(ObjRef, Object); 1] {
    let info: Dict = [("Title", Object::from(PdfString::literal("updated")))]
        .into_iter()
        .collect();
    [(ObjRef::new(4, 0), info.into())]
}

#[test]
fn incremental_update_refuses_encrypted_base() {
    let base = fixture("corpus/enc.pdf");
    let result = incremental_update(&base, &title_update(), XrefEol::CrLf);
    assert_eq!(result, Err(Error::EncryptedBase));
}

#[test]
fn incremental_update_of_xref_stream_base_appends_an_xref_stream() {
    let base = compact_base();
    let updated = incremental_update(&base, &title_update(), XrefEol::CrLf).unwrap();
    let appended = String::from_utf8_lossy(&updated[base.len()..]).into_owned();
    assert!(
        appended.contains("/Type /XRef") && !appended.contains("\ntrailer"),
        "{appended}"
    );
}

#[test]
fn incremental_update_of_xref_stream_base_keeps_compressed_objects_and_applies_change() {
    let base = compact_base();
    let updated = incremental_update(&base, &title_update(), XrefEol::CrLf).unwrap();
    let doc = open(&updated).unwrap();
    let title = doc
        .resolve(ObjRef::new(4, 0))
        .unwrap()
        .as_dict()
        .and_then(|d| d.get("Title").cloned());
    assert_eq!(
        (doc.was_repaired(), doc.revisions(), doc.page_count(), title),
        (false, 2, Ok(1), Some(Object::String(PdfString::literal("updated"))))
    );
}

#[test]
fn incremental_update_twice_on_xref_stream_base_chains_three_revisions() {
    let once = incremental_update(&compact_base(), &title_update(), XrefEol::CrLf).unwrap();
    let twice = incremental_update(&once, &[(ObjRef::new(9, 0), Object::Null)], XrefEol::CrLf).unwrap();
    let doc = open(&twice).unwrap();
    assert_eq!(
        (doc.was_repaired(), doc.revisions(), doc.page_count()),
        (false, 3, Ok(1))
    );
}

#[test]
fn incremental_update_never_shrinks_size_when_base_trailer_lacks_size() {
    let mut base = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for body in [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [] /Count 0 >>",
        "(three)",
        "(four)",
        "(five)",
    ] {
        offsets.push(base.len());
        base.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", offsets.len()).as_bytes());
    }
    let xref = base.len();
    base.extend_from_slice(b"xref\n0 6\n0000000000 65535 f\r\n");
    for off in offsets {
        base.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    base.extend_from_slice(format!("trailer\n<< /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    let updated = incremental_update(&base, &[(ObjRef::new(3, 0), Object::Null)], XrefEol::CrLf).unwrap();
    assert_eq!(open(&updated).unwrap().trailer().get_int("Size"), Some(6));
}
