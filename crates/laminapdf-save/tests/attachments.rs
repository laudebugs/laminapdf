//! The attachment ops through a save: the planner's verdict on the signed fixtures and on usage
//! rights that grant `/EF` (ADR 0007 §2), the files surviving a save and a reopen byte for byte,
//! and undo writing the base unchanged and redo the same bytes again (ADR 0006 §3).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::{corpus_doc, editable};
use common::{classic_with_root, fixture};
use laminapdf_model::{AfRelationship, AttachmentId, Checksum};
use laminapdf_ops::{AttachFile, AttachTarget, DeleteAttachment, Document, Op, Origin, UpdateAttachment};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save, SaveMode, SaveOptions, SavePlanner};

const XML: &[u8] = b"<?xml version=\"1.0\"?>\n<invoice number=\"42\"/>\n";

/// The op attaching [`XML`] to the document, its bytes added to `doc`'s session.
fn attach(doc: &mut Document) -> Op {
    let file = doc.add_blob(XML.to_vec());
    let mut op = AttachFile::new("invoice.xml", file, "text/xml")
        .with_target(AttachTarget::Document, AfRelationship::Alternative);
    op.modified = Some("D:20261117090000Z".parse().unwrap());
    op.into()
}

/// Each attachment op on `doc`: attaching, and updating and deleting the first attachment
/// (one is attached first when the file has none, in the same transaction as the case).
fn each_op(doc: &mut Document) -> [Op; 3] {
    let existing = doc.attachments().unwrap().first().map(|a| a.id.clone());
    let id = existing.unwrap_or_else(|| "e9999.0".parse().unwrap());
    let mut update = UpdateAttachment::new(id.clone());
    update.description = Some(Some("Checked".into()));
    [attach(doc), update.into(), DeleteAttachment::new(id).into()]
}

/// A one-page document whose usage rights grant `/EF [<rights>]` and annotations, built by
/// hand since pyHanko doesn't sign usage rights.
fn granting(rights: &str) -> Document {
    let reference = format!(
        "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> /Reference [<< /TransformMethod /UR3 \
         /TransformParams << /Annots [/Create] /EF [{rights}] >> >>] >>"
    );
    editable(&classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R /Perms 6 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Fields [5 0 R] >>",
            "<< /FT /Sig /T (UR) /V 7 0 R >>",
            "<< /UR3 7 0 R >>",
            &reference,
        ],
        "",
    ))
}

/// A document-wide change: a violation under every certification level, none with approval
/// signatures or a field lock (ADR 0007 §2: P3 allows annotations, never attachments).
#[test]
fn the_planner_checks_each_op_against_each_signed_fixture() {
    let cases: [(&str, Vec<Restriction>); 5] = [
        ("approval.pdf", vec![]),
        ("locked.pdf", vec![]),
        (
            "certified-p1.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::NoChanges)],
        ),
        (
            "certified-p2.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::FormFill)],
        ),
        (
            "certified-p3.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::Annotate)],
        ),
    ];
    for (name, expected) in cases {
        let mut doc = editable(&fixture(&format!("signed/{name}")));
        let planner = SavePlanner::new(&doc).unwrap();
        for op in each_op(&mut doc) {
            let restrictions: Vec<Restriction> =
                planner.would_violate(&op).into_iter().map(|v| v.restriction).collect();
            assert_eq!(restrictions, expected, "{name}: {op:?}");
        }
    }
}

/// The usage-rights fixture grants annotations and form filling, not `/EF`.
#[test]
fn usage_rights_without_ef_refuse_every_attachment_op() {
    let mut doc = editable(&fixture("signed/usage-rights.pdf"));
    let planner = SavePlanner::new(&doc).unwrap();
    for op in each_op(&mut doc) {
        let restrictions: Vec<Restriction> = planner.would_violate(&op).into_iter().map(|v| v.restriction).collect();
        assert_eq!(restrictions, [Restriction::UsageRights], "{op:?}");
    }
}

/// Each op is held to its own `/EF` right: `/Create` allows attaching but not deleting.
#[test]
fn usage_rights_allow_each_attachment_op_only_with_its_own_ef_right() {
    for (rights, allowed) in [
        ("/Create", [true, false, false]),
        ("/Modify", [false, true, false]),
        ("/Delete", [false, false, true]),
        ("/Create /Modify /Delete", [true, true, true]),
    ] {
        let mut doc = granting(rights);
        let planner = SavePlanner::new(&doc).unwrap();
        let verdicts = each_op(&mut doc).map(|op| planner.would_violate(&op).is_empty());
        assert_eq!(verdicts, allowed, "{rights}");
    }
}

/// At save time, attaching under `/EF [/Create]` stays an incremental update after the
/// signature; deleting without `/Delete` conflicts.
#[test]
fn a_save_holds_the_pending_attachments_to_the_ef_rights() {
    let mut doc = granting("/Create");
    let op = attach(&mut doc);
    doc.apply_all(Origin::Local, [op]).unwrap();
    assert_eq!(
        SavePlanner::new(&doc).unwrap().constraint().kind(),
        ConstraintKind::AppendOnly
    );
    let id = doc.attachments().unwrap()[0].id.clone();
    doc.apply_all(Origin::Local, [Op::from(DeleteAttachment::new(id))])
        .unwrap();
    assert_eq!(
        SavePlanner::new(&doc).unwrap().constraint().kind(),
        ConstraintKind::Conflict
    );
}

/// `/EF` rights cover attachments only: another document-wide change still conflicts.
#[test]
fn ef_rights_do_not_cover_other_document_changes() {
    let mut doc = granting("/Create /Modify /Delete");
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let view = laminapdf_ops::SetInitialView::default().with_open(Some(laminapdf_ops::OpenAt::new(
        page,
        laminapdf_ops::OpenZoom::FitWidth,
    )));
    let op = attach(&mut doc);
    doc.apply_all(Origin::Local, [op, view.into()]).unwrap();
    assert_eq!(
        SavePlanner::new(&doc).unwrap().constraint().kind(),
        ConstraintKind::Conflict
    );
}

fn saved(doc: &mut Document, mode: SaveMode) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
    out
}

#[test]
fn an_attached_file_survives_a_save_and_a_reopen_byte_for_byte() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = corpus_doc("outline.pdf");
        let op = attach(&mut doc);
        doc.apply_all(Origin::Local, [op]).unwrap();
        let reopened = editable(&saved(&mut doc, mode));
        let list = reopened.attachments().unwrap();
        assert_eq!(list.len(), 1, "{mode:?}");
        let bytes = reopened.attachment_bytes(&list[0].id).unwrap().unwrap();
        assert_eq!(
            (bytes.bytes.as_slice(), bytes.checksum),
            (XML, Checksum::Matches),
            "{mode:?}"
        );
        // Everything else is kept: the pages and the bookmarks.
        assert_eq!(
            (reopened.pages().unwrap().len(), reopened.outline().unwrap().len()),
            (doc.pages().unwrap().len(), doc.outline().unwrap().len()),
            "{mode:?}"
        );
    }
}

/// A deletion saved incrementally leaves the file's bytes in the earlier revision, which a full
/// rewrite drops.
#[test]
fn a_deleted_file_stays_in_the_earlier_revision_until_a_full_rewrite() {
    let base = fixture("attachments/associated.pdf");
    let readings: AttachmentId = "e4.0".parse().unwrap();
    let needle = b"day,reading";
    let contains = |bytes: &[u8]| bytes.windows(needle.len()).any(|w| w == needle);
    let mut kept = Vec::new();
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = editable(&base);
        let applied = doc
            .apply_all(Origin::Local, [Op::from(DeleteAttachment::new(readings.clone()))])
            .unwrap();
        assert!(applied.still_in_earlier_revisions);
        let out = saved(&mut doc, mode);
        assert_eq!(editable(&out).attachment(&readings).unwrap(), None, "{mode:?}");
        kept.push(contains(&out));
    }
    assert_eq!(kept, [true, false]);
}

#[test]
fn undoing_each_op_writes_the_base_unchanged() {
    for name in [
        "attachments/associated.pdf",
        "corpus/hello.pdf",
        "corpus/hello-xrefstm.pdf",
    ] {
        let base = fixture(name);
        for i in 0..3 {
            let mut doc = editable(&base);
            // A file without attachments has none to update or delete.
            if doc.attachments().unwrap().is_empty() && i > 0 {
                continue;
            }
            let op = each_op(&mut doc)[i].clone();
            doc.apply_all(Origin::Local, [op]).unwrap();
            doc.undo().unwrap();
            assert!(!doc.is_dirty(), "{name} op {i}");
            assert_eq!(saved(&mut doc, SaveMode::Incremental), base, "{name} op {i}");
        }
    }
}

#[test]
fn redoing_each_op_writes_the_bytes_it_wrote_first() {
    let base = fixture("attachments/associated.pdf");
    for i in 0..3 {
        let mut once = editable(&base);
        let op = each_op(&mut once)[i].clone();
        once.apply_all(Origin::Local, [op.clone()]).unwrap();
        let mut again = editable(&base);
        each_op(&mut again);
        again.apply_all(Origin::Local, [op]).unwrap();
        again.undo().unwrap();
        again.redo().unwrap();
        assert_eq!(
            saved(&mut again, SaveMode::Incremental),
            saved(&mut once, SaveMode::Incremental),
            "op {i}"
        );
    }
}
