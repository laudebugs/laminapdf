//! The attachment ops (ISO 32000-2 §7.11, §14.13): `AttachFile` writes the embedded file, its
//! specification and its `/AF` association; `UpdateAttachment` changes what it names;
//! `DeleteAttachment` takes it out of the tree and every `/AF` array, leaving what a
//! `FileAttachment` annotation holds; the PDF/A warnings; and the frozen samples.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

mod common;

use common::{classic_with_root, fixture, open, page_ids};
use laminapdf_model::{AfRelationship, AttachmentHolder, AttachmentId, Checksum};
use laminapdf_ops::{
    AttachFile, AttachTarget, BlobId, ConformanceWarning, DeleteAttachment, Document, EmbeddedFileRight, Error, Op,
    Origin, UpdateAttachment, pending_conformance_warnings,
};

const XML: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rsm:CrossIndustryInvoice/>\n";

fn associated() -> Document {
    open(&fixture("attachments/associated.pdf"))
}

/// `doc` holding [`XML`] as a blob, and the op that attaches it as `factur-x.xml`.
fn attach(doc: &mut Document, target: AttachTarget) -> AttachFile {
    let file = doc.add_blob(XML.to_vec());
    let mut op = AttachFile::new("factur-x.xml", file, "text/xml").with_target(target, AfRelationship::Alternative);
    op.modified = Some("D:20261117090000Z".parse().unwrap());
    op
}

fn attached(doc: &mut Document, op: AttachFile) -> AttachmentId {
    let applied = doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    applied.created_attachments[0].clone()
}

#[test]
fn an_attached_file_reads_back_byte_for_byte_with_a_matching_checksum() {
    let mut doc = associated();
    let op = attach(&mut doc, AttachTarget::Document);
    let id = attached(&mut doc, op);
    let bytes = doc.attachment_bytes(&id).unwrap().unwrap();
    assert_eq!((bytes.bytes.as_slice(), bytes.checksum), (XML, Checksum::Matches));
}

/// The attachment `op` makes in the attachments fixture.
fn attached_file(op: impl FnOnce(&mut Document) -> AttachFile) -> laminapdf_model::AttachedFile {
    let mut doc = associated();
    let op = op(&mut doc);
    let id = attached(&mut doc, op);
    doc.attachment(&id).unwrap().unwrap()
}

#[test]
fn an_attached_file_is_listed_under_its_name() {
    let a = attached_file(|doc| attach(doc, AttachTarget::Document));
    assert_eq!(
        (a.key.as_deref(), a.name.as_deref()),
        (Some("factur-x.xml"), Some("factur-x.xml"))
    );
}

#[test]
fn an_attached_file_records_its_type_and_size() {
    let a = attached_file(|doc| attach(doc, AttachTarget::Document));
    assert_eq!(
        (a.mime.as_deref(), a.size),
        (Some("text/xml"), Some(u64::try_from(XML.len()).unwrap()))
    );
}

#[test]
fn an_attached_file_records_its_relationship() {
    let a = attached_file(|doc| attach(doc, AttachTarget::Document));
    assert_eq!(a.relationship, Some(AfRelationship::Alternative));
}

#[test]
fn an_attached_file_records_the_date_the_op_gives() {
    let a = attached_file(|doc| attach(doc, AttachTarget::Document));
    assert_eq!(a.modified.map(|d| d.to_string()).as_deref(), Some("D:20261117090000Z"));
}

#[test]
fn each_target_associates_the_file_with_it() {
    let doc = associated();
    let pages = page_ids(&doc);
    let annot: laminapdf_model::AnnotId = "a6.0".parse().unwrap();
    let cases = [
        (AttachTarget::EmbeddedFiles, vec![AttachmentHolder::EmbeddedFiles]),
        (
            AttachTarget::Document,
            vec![AttachmentHolder::EmbeddedFiles, AttachmentHolder::Document],
        ),
        (
            AttachTarget::Page(pages[0].clone()),
            vec![
                AttachmentHolder::EmbeddedFiles,
                AttachmentHolder::Page(pages[0].clone()),
            ],
        ),
        (
            AttachTarget::Annotation(annot.clone()),
            vec![
                AttachmentHolder::EmbeddedFiles,
                AttachmentHolder::Annotation {
                    page: pages[0].clone(),
                    annotation: annot,
                },
            ],
        ),
    ];
    for (target, holders) in cases {
        let mut doc = associated();
        let op = attach(&mut doc, target.clone());
        let id = attached(&mut doc, op);
        assert_eq!(doc.attachment(&id).unwrap().unwrap().holders, holders, "{target:?}");
    }
}

/// A page that already has an `/AF` array keeps its files and gains the new one.
#[test]
fn a_page_keeps_its_associated_files() {
    let mut doc = associated();
    let second = page_ids(&doc)[1].clone();
    let op = attach(&mut doc, AttachTarget::Page(second.clone()));
    attached(&mut doc, op);
    let mut on_page: Vec<_> = doc
        .attachments()
        .unwrap()
        .into_iter()
        .filter(|a| a.holders.contains(&AttachmentHolder::Page(second.clone())))
        .map(|a| a.name.unwrap())
        .collect();
    on_page.sort();
    assert_eq!(on_page, ["factur-x.xml", "readings.csv"]);
}

#[test]
fn a_name_another_file_has_gets_a_free_key() {
    let mut doc = associated();
    let file = doc.add_blob(XML.to_vec());
    let id = attached(&mut doc, AttachFile::new("notes.txt", file, "text/xml"));
    let a = doc.attachment(&id).unwrap().unwrap();
    assert_eq!(
        (a.key.as_deref(), a.name.as_deref()),
        (Some("notes.txt (2)"), Some("notes.txt"))
    );
}

#[test]
fn bytes_the_session_does_not_hold_are_refused() {
    let mut doc = associated();
    let op = AttachFile::new("x.xml", BlobId::of(b"never added"), "text/xml");
    let err = doc.apply_all(Origin::Local, [Op::from(op)]).unwrap_err();
    assert_eq!(err.code(), "unknown-blob");
}

#[test]
fn an_empty_name_is_refused() {
    let mut doc = associated();
    let file = doc.add_blob(XML.to_vec());
    let err = doc
        .apply_all(Origin::Local, [Op::from(AttachFile::new("", file, "text/xml"))])
        .unwrap_err();
    assert_eq!(err.code(), "invalid-op");
}

#[test]
fn a_page_the_document_does_not_have_is_refused_before_anything_is_written() {
    let mut doc = associated();
    let op = attach(&mut doc, AttachTarget::Page("p99.0".parse().unwrap()));
    let err = doc.apply_all(Origin::Local, [Op::from(op)]).unwrap_err();
    assert_eq!(err.code(), "unknown-page");
    assert!(!doc.is_dirty());
}

/// Attaching never raises `/Version`: an `/AF` array is a second-class entry for older readers.
#[test]
fn attaching_leaves_the_version_alone() {
    let mut doc = associated();
    let op = attach(&mut doc, AttachTarget::Document);
    attached(&mut doc, op);
    assert_eq!(doc.version().unwrap().as_deref(), Some("1.7"));
    assert!(doc.catalog().unwrap().get("Version").is_none());
}

/// `readings.csv` (`e4.0`) of the attachments fixture after `change` applied to it.
fn updated(change: impl FnOnce(&mut UpdateAttachment)) -> laminapdf_model::AttachedFile {
    let mut doc = associated();
    let readings: AttachmentId = "e4.0".parse().unwrap();
    let mut op = UpdateAttachment::new(readings.clone());
    change(&mut op);
    doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    doc.attachment(&readings).unwrap().unwrap()
}

#[test]
fn an_update_renames_the_file_and_its_key() {
    let a = updated(|op| op.name = Some("readings-2026.csv".into()));
    assert_eq!(
        (a.key.as_deref(), a.name.as_deref()),
        (Some("readings-2026.csv"), Some("readings-2026.csv"))
    );
}

#[test]
fn an_update_with_null_removes_the_description() {
    assert_eq!(updated(|op| op.description = Some(None)).description, None);
}

#[test]
fn an_update_retypes_the_file() {
    assert_eq!(
        updated(|op| op.mime = Some("text/plain".into())).mime.as_deref(),
        Some("text/plain")
    );
}

#[test]
fn an_update_sets_the_relationship() {
    let a = updated(|op| op.relationship = Some(AfRelationship::Source));
    assert_eq!(a.relationship, Some(AfRelationship::Source));
}

#[test]
fn an_update_leaves_the_bytes_alone() {
    let mut doc = associated();
    let readings: AttachmentId = "e4.0".parse().unwrap();
    let before = doc.attachment_bytes(&readings).unwrap().unwrap();
    let mut op = UpdateAttachment::new(readings.clone());
    op.description = Some(Some("Renamed".into()));
    doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    assert_eq!(doc.attachment_bytes(&readings).unwrap().unwrap(), before);
}

#[test]
fn an_unknown_attachment_is_refused() {
    let mut doc = associated();
    let err = doc
        .apply_all(
            Origin::Local,
            [Op::from(DeleteAttachment::new("e99.0".parse().unwrap()))],
        )
        .unwrap_err();
    assert!(matches!(err, Error::UnknownAttachment(_)));
}

#[test]
fn a_deleted_file_leaves_the_tree_and_every_af_array() {
    let mut doc = associated();
    let readings: AttachmentId = "e4.0".parse().unwrap();
    let applied = doc
        .apply_all(Origin::Local, [Op::from(DeleteAttachment::new(readings.clone()))])
        .unwrap();
    assert_eq!(doc.attachment(&readings).unwrap(), None);
    assert!(applied.still_in_earlier_revisions);
    assert_eq!(applied.attachments_held, Vec::new());
}

/// The annotation keeps the file; deleting the annotation is an annotation edit.
#[test]
fn a_file_an_annotation_holds_stays_and_is_reported() {
    let mut doc = associated();
    let notes: AttachmentId = "e2.0".parse().unwrap();
    let applied = doc
        .apply_all(Origin::Local, [Op::from(DeleteAttachment::new(notes.clone()))])
        .unwrap();
    let held = &applied.attachments_held;
    assert_eq!(held.len(), 1);
    assert_eq!(
        (&held[0].attachment, held[0].annotation.to_string()),
        (&notes, "a6.0".to_owned())
    );
    let left = doc.attachment(&notes).unwrap().unwrap();
    assert!(matches!(
        left.holders.as_slice(),
        [AttachmentHolder::FileAttachment { .. }]
    ));
}

#[test]
fn deleting_the_last_file_removes_the_tree() {
    let mut doc = associated();
    for id in ["e2.0", "e4.0"] {
        doc.apply_all(Origin::Local, [Op::from(DeleteAttachment::new(id.parse().unwrap()))])
            .unwrap();
    }
    assert!(doc.name_tree("EmbeddedFiles").unwrap().is_none());
}

#[test]
fn each_op_needs_its_own_usage_right() {
    let mut doc = associated();
    let attach = Op::from(attach(&mut doc, AttachTarget::Document));
    let update = Op::from(UpdateAttachment::new("e4.0".parse().unwrap()));
    let delete = Op::from(DeleteAttachment::new("e4.0".parse().unwrap()));
    assert_eq!(
        [&attach, &update, &delete].map(|op| (op.class(), op.requires_rewrite(), op.embedded_file_right())),
        [
            (
                laminapdf_ops::ModificationClass::Document,
                false,
                Some(EmbeddedFileRight::Create)
            ),
            (
                laminapdf_ops::ModificationClass::Document,
                false,
                Some(EmbeddedFileRight::Modify)
            ),
            (
                laminapdf_ops::ModificationClass::Document,
                false,
                Some(EmbeddedFileRight::Delete)
            ),
        ]
    );
    assert!([&attach, &update, &delete].iter().all(|op| op.annot_right().is_none()));
}

/// A one-page document whose XMP packet claims PDF/A `part`.
fn pdfa(part: u8) -> Document {
    pdfa_with(part, "", &[])
}

/// [`pdfa`] with `catalog_extra` in its catalog and `extra` objects from 5 on.
fn pdfa_with(part: u8, catalog_extra: &str, extra: &[&str]) -> Document {
    let packet = format!(
        "<?xpacket begin=\"\" id=\"W5M0MpCehiHzreSzNTczkc9d\"?><x:xmpmeta xmlns:x=\"adobe:ns:meta/\"><rdf:RDF \
         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"><rdf:Description rdf:about=\"\" \
         xmlns:pdfaid=\"http://www.aiim.org/pdfa/ns/id/\"><pdfaid:part>{part}</pdfaid:part>\
         <pdfaid:conformance>B</pdfaid:conformance></rdf:Description></rdf:RDF></x:xmpmeta><?xpacket end=\"w\"?>"
    );
    let stream = common::stream_body("/Type /Metadata /Subtype /XML", packet.as_bytes());
    let catalog = format!("<< /Type /Catalog /Pages 2 0 R /Metadata 4 0 R {catalog_extra} >>");
    let mut objects = vec![
        catalog.as_str(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        std::str::from_utf8(&stream).unwrap(),
    ];
    objects.extend_from_slice(extra);
    open(&classic_with_root(&objects, ""))
}

fn warnings(doc: &Document) -> Vec<&'static str> {
    pending_conformance_warnings(doc, doc.pending_changes())
        .unwrap()
        .iter()
        .map(ConformanceWarning::code)
        .collect()
}

#[test]
fn attaching_to_pdfa_1_or_2_warns_that_attachments_are_forbidden() {
    for part in [1, 2] {
        let mut doc = pdfa(part);
        let op = attach(&mut doc, AttachTarget::Document);
        attached(&mut doc, op);
        assert_eq!(warnings(&doc), ["pdfa-attachment-forbidden"], "PDF/A-{part}");
    }
}

#[test]
fn a_complete_attachment_keeps_pdfa_3() {
    let mut doc = pdfa(3);
    let op = attach(&mut doc, AttachTarget::Document);
    attached(&mut doc, op);
    assert_eq!(warnings(&doc), Vec::<&str>::new());
}

#[test]
fn a_pdfa_3_attachment_without_an_association_or_a_date_is_incomplete() {
    let mut doc = pdfa(3);
    let mut op = attach(&mut doc, AttachTarget::EmbeddedFiles);
    op.modified = None;
    attached(&mut doc, op);
    assert_eq!(warnings(&doc), ["pdfa-attachment-incomplete"]);
}

#[test]
fn an_undone_attachment_warns_of_nothing() {
    let mut doc = pdfa(2);
    let op = attach(&mut doc, AttachTarget::Document);
    attached(&mut doc, op);
    doc.undo().unwrap();
    assert_eq!(warnings(&doc), Vec::<&str>::new());
}

#[test]
fn applying_then_undoing_each_op_leaves_the_overlay_empty() {
    let ops = |doc: &mut Document| -> Vec<Op> {
        let mut rename = UpdateAttachment::new("e4.0".parse().unwrap());
        rename.name = Some("renamed.csv".into());
        rename.mime = Some("text/plain".into());
        vec![
            attach(doc, AttachTarget::Page(page_ids(doc)[1].clone())).into(),
            rename.into(),
            DeleteAttachment::new("e2.0".parse().unwrap()).into(),
        ]
    };
    let mut probe = associated();
    for (i, _) in ops(&mut probe).iter().enumerate() {
        let mut doc = associated();
        let op = ops(&mut doc).remove(i);
        doc.apply_all(Origin::Local, [op]).unwrap();
        doc.undo().unwrap();
        assert!(!doc.is_dirty(), "op {i}");
    }
}

/// A tree whose key for `notes.txt` is UTF-16BE: the same name in PDFDocEncoding is taken too.
#[test]
fn a_name_taken_in_another_encoding_gets_a_free_key() {
    let stream = common::stream_body("/Type /EmbeddedFile", b"notes");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [<FEFF006E006F007400650073002E007400780074> 4 0 R] >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Filespec /UF (notes.txt) /EF << /F 5 0 R >> >>",
        std::str::from_utf8(&stream).unwrap(),
    ];
    let mut doc = open(&classic_with_root(&objects, ""));
    let file = doc.add_blob(XML.to_vec());
    let id = attached(&mut doc, AttachFile::new("notes.txt", file, "text/xml"));
    assert_eq!(
        doc.attachment(&id).unwrap().unwrap().key.as_deref(),
        Some("notes.txt (2)")
    );
}

/// A tree that lists the same key twice keeps both files when an op writes it again.
#[test]
fn a_key_listed_twice_keeps_both_files() {
    let stream = common::stream_body("/Type /EmbeddedFile", b"notes");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R /Names << /EmbeddedFiles << /Names [(a.txt) 4 0 R (a.txt) 6 0 R] >> >> >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R >>",
        "<< /Type /Filespec /UF (a.txt) /EF << /F 5 0 R >> >>",
        std::str::from_utf8(&stream).unwrap(),
        "<< /Type /Filespec /UF (a.txt) /EF << /F 5 0 R >> >>",
    ];
    let mut doc = open(&classic_with_root(&objects, ""));
    let file = doc.add_blob(XML.to_vec());
    attached(&mut doc, AttachFile::new("b.txt", file, "text/xml"));
    let keys: Vec<_> = doc.attachments().unwrap().into_iter().filter_map(|a| a.key).collect();
    assert_eq!(keys, ["a.txt", "a.txt (2)", "b.txt"]);
}

/// A direct annotation's `/AF` lives in its page's `/Annots`: deleting the file rewrites it there.
#[test]
fn deleting_a_file_takes_it_out_of_a_direct_annotations_af() {
    let stream = common::stream_body("/Type /EmbeddedFile", b"note");
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 600 800] >>",
        "<< /Type /Page /Parent 2 0 R /Annots [<< /Type /Annot /Subtype /Square /Rect [0 0 9 9] /AF [4 0 R] >>] >>",
        "<< /Type /Filespec /UF (note.txt) /EF << /F 5 0 R >> >>",
        std::str::from_utf8(&stream).unwrap(),
    ];
    let mut doc = open(&classic_with_root(&objects, ""));
    let id: AttachmentId = "e4.0".parse().unwrap();
    doc.apply_all(Origin::Local, [Op::from(DeleteAttachment::new(id.clone()))])
        .unwrap();
    assert_eq!(doc.attachment(&id).unwrap(), None);
}

/// A file the document could never read back out isn't attached.
#[test]
fn a_file_over_the_decode_limit_is_refused() {
    let options =
        laminapdf_cos::OpenOptions::default().with_limits(laminapdf_cos::Limits::default().with_max_stream_decoded(16));
    let mut doc = Document::open(fixture("attachments/associated.pdf"), options).unwrap();
    let op = attach(&mut doc, AttachTarget::Document);
    let err = doc.apply_all(Origin::Local, [Op::from(op)]).unwrap_err();
    assert!(matches!(err, Error::AttachmentTooLarge { limit: 16, .. }));
}

/// Clearing the MIME type of a file the PDF/A-3 document already carried breaks what PDF/A-3
/// asks of it.
#[test]
fn clearing_the_type_on_pdfa_3_makes_the_attachment_incomplete() {
    let file = common::stream_body(
        "/Type /EmbeddedFile /Subtype /text#2Fxml /Params << /ModDate (D:20261117090000Z) >>",
        b"<x/>",
    );
    let mut doc = pdfa_with(
        3,
        "/AF [5 0 R] /Names << /EmbeddedFiles << /Names [(x.xml) 5 0 R] >> >>",
        &[
            "<< /Type /Filespec /UF (x.xml) /EF << /F 6 0 R >> /AFRelationship /Data >>",
            std::str::from_utf8(&file).unwrap(),
        ],
    );
    assert_eq!(warnings(&doc), Vec::<&str>::new());
    let mut clear = UpdateAttachment::new("e5.0".parse().unwrap());
    clear.mime = Some(String::new());
    doc.apply_all(Origin::Local, [Op::from(clear)]).unwrap();
    assert_eq!(warnings(&doc), ["pdfa-attachment-incomplete"]);
}

mod samples {
    use super::*;

    fn sample(name: &str) -> Op {
        serde_json::from_slice(&fixture(&format!("ops/v1/{name}.json"))).unwrap()
    }

    #[test]
    fn the_attach_sample_applies_to_the_attachments_fixture() {
        let mut doc = associated();
        // The sample names its bytes by their SHA-256: these are the bytes.
        assert_eq!(
            doc.add_blob(XML.to_vec()).to_string(),
            "ad913c3c4cd479882f61d09acf510f709edd83a88df0a516cbd9afa0284a1a77"
        );
        let applied = doc.apply_all(Origin::Local, [sample("attach-file")]).unwrap();
        let a = doc.attachment(&applied.created_attachments[0]).unwrap().unwrap();
        assert_eq!(
            (a.name.as_deref(), a.holders[1].clone()),
            (Some("factur-x.xml"), AttachmentHolder::Page("p10.0".parse().unwrap()))
        );
    }

    #[test]
    fn the_update_sample_renames_and_retypes_readings() {
        let mut doc = associated();
        doc.apply_all(Origin::Local, [sample("update-attachment")]).unwrap();
        let a = doc.attachment(&"e4.0".parse().unwrap()).unwrap().unwrap();
        assert_eq!(
            (a.name.as_deref(), a.relationship),
            (Some("readings-2026.csv"), Some(AfRelationship::Source))
        );
    }

    /// The paperclip annotation still holds the deleted notes.
    #[test]
    fn the_delete_sample_leaves_notes_only_to_the_annotation() {
        let mut doc = associated();
        doc.apply_all(Origin::Local, [sample("delete-attachment")]).unwrap();
        let notes = doc.attachment(&"e2.0".parse().unwrap()).unwrap().unwrap();
        assert!(matches!(
            notes.holders.as_slice(),
            [AttachmentHolder::FileAttachment { .. }]
        ));
    }

    fn rejected(json: &str) -> bool {
        serde_json::from_str::<Op>(json).is_err()
    }

    #[test]
    fn an_unknown_field_is_rejected() {
        assert!(rejected(
            r#"{ "op": "deleteAttachment", "args": { "attachment": "e2.0", "force": true } }"#
        ));
    }

    #[test]
    fn an_unknown_relationship_is_rejected() {
        assert!(rejected(
            r#"{ "op": "updateAttachment", "args": { "attachment": "e2.0", "relationship": "cousin" } }"#
        ));
    }

    #[test]
    fn a_blob_id_that_is_not_one_is_rejected() {
        assert!(rejected(
            r#"{ "op": "attachFile", "args": { "name": "a", "file": "00", "mime": "" } }"#
        ));
    }

    #[test]
    fn an_id_of_another_kind_is_rejected() {
        assert!(rejected(
            r#"{ "op": "deleteAttachment", "args": { "attachment": "a2.0" } }"#
        ));
    }
}
