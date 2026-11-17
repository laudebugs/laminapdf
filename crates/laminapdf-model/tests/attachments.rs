//! `Document::attachments`, `attachment_bytes` and `collection`: the files a document carries
//! with what holds each, their bytes against `Limits` and their checksum, and a portfolio's
//! schema (ISO 32000-2 §7.11, §14.13, §12.3.5), on `fixtures/attachments/`.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf_model::{
    AfRelationship, AttachmentHolder, Checksum, CollectionFieldKind, CollectionValue, CollectionView, Document, Limit,
    Limits, OpenOptions,
};
use laminapdf_test_fixtures::{classic_with_root, fixture};

fn open(name: &str) -> Document {
    Document::open(fixture(&format!("attachments/{name}")), OpenOptions::default()).unwrap()
}

#[test]
fn each_file_is_listed_once_with_everything_that_holds_it() {
    let doc = open("associated.pdf");
    let pages = doc.pages().unwrap();
    let (first, second) = (pages.get(0).unwrap().id().clone(), pages.get(1).unwrap().id().clone());
    let list = doc.attachments().unwrap();
    let names: Vec<_> = list.iter().map(|a| a.name.as_deref().unwrap()).collect();
    assert_eq!(names, ["notes.txt", "readings.csv"]);
    let notes = &list[0];
    assert_eq!(notes.holders.len(), 2);
    assert_eq!(notes.holders[0], AttachmentHolder::EmbeddedFiles);
    assert!(matches!(&notes.holders[1], AttachmentHolder::FileAttachment { page, .. } if *page == first));
    assert_eq!(
        list[1].holders,
        [AttachmentHolder::EmbeddedFiles, AttachmentHolder::Page(second)]
    );
}

#[test]
fn a_file_reads_its_type_size_dates_and_relationship() {
    let readings = open("associated.pdf").attachments().unwrap().remove(1);
    assert_eq!(readings.key.as_deref(), Some("readings.csv"));
    assert_eq!(readings.mime.as_deref(), Some("text/csv"));
    assert_eq!(readings.size, Some(26));
    assert_eq!(readings.description.as_deref(), Some("The readings page 2 charts"));
    assert_eq!(readings.relationship, Some(AfRelationship::Data));
    assert_eq!(
        readings.modified.map(|d| d.to_string()).as_deref(),
        Some("D:20261101090000Z")
    );
    assert!(readings.embedded);
}

/// capability: H.09 core read
#[test]
fn bytes_whose_checksum_matches_say_so() {
    let doc = open("associated.pdf");
    let notes = doc.attachments().unwrap().remove(0);
    let bytes = doc.attachment_bytes(&notes.id).unwrap().unwrap();
    assert_eq!(bytes.bytes, b"Notes for the attachment fixtures.\n");
    assert_eq!(bytes.checksum, Checksum::Matches);
}

#[test]
fn bytes_whose_checksum_differs_are_still_returned_and_flagged() {
    let doc = open("associated.pdf");
    let readings = doc.attachments().unwrap().remove(1);
    let bytes = doc.attachment_bytes(&readings.id).unwrap().unwrap();
    assert_eq!(bytes.bytes, b"day,reading\n1,12.5\n2,13.1\n");
    assert_eq!(bytes.checksum, Checksum::Mismatch);
}

/// A file larger than the device may decode is refused from its stated size, before decoding.
#[test]
fn a_file_over_the_decode_limit_is_refused_before_it_is_read() {
    let options = OpenOptions::default().with_limits(Limits::default().with_max_stream_decoded(16));
    let doc = Document::open(fixture("attachments/associated.pdf"), options).unwrap();
    let notes = doc.attachments().unwrap().remove(0);
    let err = doc.attachment_bytes(&notes.id).unwrap_err();
    assert!(matches!(
        err,
        laminapdf_model::Error::LimitExceeded {
            limit: Limit::StreamDecoded,
            ..
        }
    ));
}

#[test]
fn an_unknown_id_has_no_bytes() {
    let doc = open("associated.pdf");
    assert_eq!(doc.attachment_bytes(&"e999.0".parse().unwrap()).unwrap(), None);
}

/// A file specification with no `/EF` names a file elsewhere: it is listed, without bytes.
#[test]
fn a_specification_without_embedded_bytes_is_listed_but_has_none() {
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AF [4 0 R] >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 800] >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Type /Filespec /F (elsewhere.txt) /AFRelationship /Supplement >>",
        ],
        "",
    );
    let doc = Document::open(bytes, OpenOptions::default()).unwrap();
    let list = doc.attachments().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].holders, [AttachmentHolder::Document]);
    assert!(!list[0].embedded);
    assert_eq!(doc.attachment_bytes(&list[0].id).unwrap(), None);
}

/// A direct file specification in an `/AF` array gets a path ID that finds it again.
#[test]
fn a_direct_specification_has_a_path_id_that_reads_its_bytes() {
    let stream = laminapdf_test_fixtures::stream_body("<< /Type /EmbeddedFile >>", b"inline");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /AF [<< /Type /Filespec /UF (inline.txt) /EF << /F 4 0 R >> >>] >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        std::str::from_utf8(&stream).unwrap(),
    ];
    let doc = Document::open(classic_with_root(&objects, ""), OpenOptions::default()).unwrap();
    let list = doc.attachments().unwrap();
    assert!(list[0].id.as_lamina_id().is_path());
    assert_eq!(doc.attachment_bytes(&list[0].id).unwrap().unwrap().bytes, b"inline");
}

#[test]
fn a_document_without_a_collection_is_not_a_portfolio() {
    assert_eq!(open("associated.pdf").collection().unwrap(), None);
}

/// capability: J.06 core read
#[test]
fn a_portfolio_reads_its_schema_in_column_order() {
    let collection = open("portfolio.pdf").collection().unwrap().unwrap();
    let columns: Vec<_> = collection
        .schema
        .iter()
        .map(|f| (f.key.as_str(), f.kind.clone(), f.visible, f.editable))
        .collect();
    assert_eq!(
        columns,
        [
            ("name", CollectionFieldKind::FileName, true, false),
            ("desc", CollectionFieldKind::Description, true, false),
            ("owner", CollectionFieldKind::Text, true, true),
            ("due", CollectionFieldKind::Date, true, false),
            ("pages", CollectionFieldKind::Number, false, false),
        ]
    );
    assert_eq!(collection.schema[0].name.as_deref(), Some("Name"));
}

#[test]
fn a_portfolio_reads_its_sort_view_and_first_file() {
    let collection = open("portfolio.pdf").collection().unwrap().unwrap();
    assert_eq!(collection.sort, [("owner".to_owned(), true), ("due".to_owned(), false)]);
    assert_eq!(collection.view, CollectionView::Details);
    assert_eq!(collection.initial.as_deref(), Some("report.pdf"));
}

/// Each file's collection item gives its values; a subitem's value comes without its prefix.
#[test]
fn a_portfolio_lists_each_file_with_its_values() {
    let collection = open("portfolio.pdf").collection().unwrap().unwrap();
    let files: Vec<_> = collection
        .files
        .iter()
        .map(|f| (f.key.as_str(), f.values.clone()))
        .collect();
    assert_eq!(
        files,
        [
            (
                "notes.txt",
                vec![
                    ("owner".to_owned(), CollectionValue::Text("Review".into())),
                    ("pages".to_owned(), CollectionValue::Number(1.0)),
                ]
            ),
            (
                "report.pdf",
                vec![
                    ("due".to_owned(), CollectionValue::Text("D:20261231000000Z".into())),
                    ("pages".to_owned(), CollectionValue::Number(12.0)),
                    ("owner".to_owned(), CollectionValue::Text("Finance".into())),
                ]
            ),
        ]
    );
}
