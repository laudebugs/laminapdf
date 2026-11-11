//! Saving the metadata ops (E1 plan §2, §7): each op's verdict on the signed fixtures, the
//! save-time stamp and when it is left out, undo, determinism and the conformance warnings.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::classic_with_root;
use common::fixture;
use common::save::editable;
use laminapdf_model::xmp::{Packet, PacketState};
use laminapdf_model::{PdfDate, Rotation};
use laminapdf_ops::{
    ConformanceWarning, Document, MetadataKey, MetadataPatch, ModificationClass, Op, Origin, RemoveMetadata,
    RotatePage, SetMetadata, UpdateSaveMetadata,
};
use laminapdf_save::{
    ConstraintKind, DocMdpPermission, MetadataUpdate, Restriction, Save, SaveMode, SaveOptions, SavePlanner,
};

fn signed(name: &str) -> Document {
    editable(&fixture(&format!("signed/{name}")))
}

fn date(s: &str) -> PdfDate {
    s.parse().unwrap()
}

fn the_metadata_ops() -> [Op; 3] {
    [
        SetMetadata::new(MetadataPatch::new().title(Some("Changed")), None).into(),
        RemoveMetadata::new(vec![MetadataKey::Producer]).into(),
        UpdateSaveMetadata::new(Some(date("D:20261110093000Z")), "uuid:x".to_owned()).into(),
    ]
}

/// A usage-rights signature granting form filling and annotations (UR3, §12.8.2.3), built by
/// hand since pyHanko doesn't sign usage rights.
fn usage_rights() -> Document {
    editable(&classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R /Perms 6 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Fields [5 0 R] >>",
            "<< /FT /Sig /T (UR) /V 7 0 R >>",
            "<< /UR3 7 0 R >>",
            "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> /Reference [<< /TransformMethod /UR3 \
             /TransformParams << /Annots [/Create /Delete] /Form [/FillIn] >> >>] >>",
        ],
        "",
    ))
}

/// The restrictions each metadata op breaks on `doc`, and the constraint's kind as it is.
fn verdicts(doc: &Document) -> (ConstraintKind, Vec<Vec<Restriction>>) {
    let planner = SavePlanner::new(doc).unwrap();
    let ops = the_metadata_ops();
    for op in &ops {
        assert_eq!(
            (op.class(), op.requires_rewrite()),
            (ModificationClass::Metadata, false)
        );
    }
    let restrictions = ops
        .iter()
        .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
        .collect();
    (planner.constraint().kind(), restrictions)
}

mod would_violate {
    use super::*;

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(), Vec::new(), Vec::new()])
    }

    fn every_op(restriction: Restriction) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![vec![restriction]; 3])
    }

    #[test]
    fn an_approval_signature_allows_them() {
        assert_eq!(verdicts(&signed("approval.pdf")), none());
    }

    #[test]
    fn two_approval_signatures_allow_them() {
        assert_eq!(verdicts(&signed("two-signatures.pdf")), none());
    }

    #[test]
    fn a_fieldmdp_lock_allows_them() {
        assert_eq!(verdicts(&signed("locked.pdf")), none());
    }

    #[test]
    fn docmdp_p1_forbids_them() {
        assert_eq!(
            verdicts(&signed("certified-p1.pdf")),
            every_op(Restriction::DocMdp(DocMdpPermission::NoChanges))
        );
    }

    #[test]
    fn docmdp_p2_forbids_them() {
        assert_eq!(
            verdicts(&signed("certified-p2.pdf")),
            every_op(Restriction::DocMdp(DocMdpPermission::FormFill))
        );
    }

    #[test]
    fn docmdp_p3_forbids_them() {
        assert_eq!(
            verdicts(&signed("certified-p3.pdf")),
            every_op(Restriction::DocMdp(DocMdpPermission::Annotate))
        );
    }

    #[test]
    fn usage_rights_forbid_them() {
        assert_eq!(verdicts(&usage_rights()), every_op(Restriction::UsageRights));
    }

    #[test]
    fn an_applied_edit_on_a_certified_file_is_a_conflict() {
        let mut doc = signed("certified-p2.pdf");
        let [op, ..] = the_metadata_ops();
        doc.apply_all(Origin::Local, [op]).unwrap();
        assert_eq!(
            SavePlanner::new(&doc).unwrap().constraint().kind(),
            ConstraintKind::Conflict
        );
    }
}

fn packet_of(bytes: &[u8]) -> Packet {
    match editable(bytes).xmp().unwrap() {
        PacketState::Parsed(p) => p,
        other => panic!("no packet: {other:?}"),
    }
}

fn mod_date(bytes: &[u8]) -> Option<String> {
    editable(bytes)
        .info()
        .unwrap()
        .and_then(|i| i.mod_date().unwrap())
        .map(|d| d.raw)
}

fn edit_title(doc: &mut Document) {
    doc.apply_all(
        Origin::Local,
        [SetMetadata::new(MetadataPatch::new().title(Some("Edited")), None).into()],
    )
    .unwrap();
}

const AT: &str = "D:20261110093000Z";

mod stamp {
    use super::*;

    #[test]
    fn a_title_edit_on_an_unsigned_file_stamps_the_dates_and_a_new_instance_id() {
        let base = fixture("golden/metadata.pdf");
        let mut doc = editable(&base);
        edit_title(&mut doc);
        let mut out = Vec::new();
        let report = doc
            .save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
            .unwrap();
        let p = packet_of(&out);
        assert_eq!(
            (
                report.metadata_updated(),
                p.title().as_deref(),
                p.metadata_date(),
                p.modify_date(),
                mod_date(&out).as_deref(),
            ),
            (true, Some("Edited"), Some(date(AT)), Some(date(AT)), Some(AT))
        );
        let id = p.instance_id().unwrap();
        assert!(
            id.starts_with("uuid:") && Some(&id) != packet_of(&base).instance_id().as_ref(),
            "{id}"
        );
    }

    #[test]
    fn the_shells_instance_id_is_written_as_given() {
        let mut doc = editable(&fixture("golden/metadata.pdf"));
        edit_title(&mut doc);
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_instance_id([0xab; 16]))
            .unwrap();
        assert_eq!(
            packet_of(&out).instance_id().as_deref(),
            Some("uuid:abababab-abab-abab-abab-abababababab")
        );
    }

    #[test]
    fn without_a_date_or_an_id_a_save_stamps_nothing() {
        let mut doc = editable(&fixture("golden/metadata.pdf"));
        edit_title(&mut doc);
        let mut out = Vec::new();
        let report = doc.save(&mut out, &SaveOptions::new()).unwrap();
        assert_eq!(
            (report.metadata_updated(), packet_of(&out).metadata_date()),
            (false, None)
        );
    }

    #[test]
    fn a_save_that_changed_nothing_stamps_nothing() {
        let base = fixture("golden/metadata.pdf");
        let mut doc = editable(&base);
        let mut out = Vec::new();
        let report = doc
            .save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
            .unwrap();
        assert_eq!((report.metadata_updated(), out), (false, base));
    }

    #[test]
    fn the_stamp_is_not_on_the_undo_history() {
        let mut doc = editable(&fixture("golden/metadata.pdf"));
        edit_title(&mut doc);
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
            .unwrap();
        assert_eq!(doc.history().undo_len(), 1);
    }

    #[test]
    fn an_approval_signed_file_is_stamped_in_an_incremental_update() {
        let base = fixture("signed/approval.pdf");
        let mut doc = editable(&base);
        edit_title(&mut doc);
        let mut out = Vec::new();
        let report = doc
            .save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
            .unwrap();
        assert_eq!(
            (
                report.mode(),
                report.metadata_updated(),
                out.starts_with(&base),
                mod_date(&out).as_deref()
            ),
            (SaveMode::Incremental, true, true, Some(AT))
        );
    }

    #[test]
    fn a_rotation_on_a_certified_p2_file_is_a_conflict_whose_override_still_stamps() {
        // The rotation already breaks the certification, so the stamp adds nothing new.
        let mut doc = signed("certified-p2.pdf");
        let page = doc.pages().unwrap().get(0).unwrap().id().clone();
        doc.apply_all(Origin::Local, [RotatePage::new(page, Rotation::Half).into()])
            .unwrap();
        let options = SaveOptions::new()
            .with_metadata_date(date(AT))
            .with_invalidate_signatures(true);
        let plan = doc.plan(&options).unwrap();
        assert_eq!(*plan.metadata_update(), MetadataUpdate::Stamp);
    }

    #[test]
    fn a_failed_write_leaves_the_document_without_the_stamp() {
        let mut doc = editable(&fixture("golden/metadata.pdf"));
        edit_title(&mut doc);
        let before: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
        let mut not_empty = vec![1u8];
        let result = doc.save(&mut not_empty, &SaveOptions::new().with_metadata_date(date(AT)));
        let after: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
        assert_eq!(
            (result.is_err(), doc.in_transaction(), after == before),
            (true, false, true)
        );
    }
}

mod undo_and_determinism {
    use super::*;

    #[test]
    fn each_edit_undone_saves_byte_identical_to_the_original() {
        let base = fixture("golden/metadata.pdf");
        for op in the_metadata_ops().into_iter().take(2) {
            let mut doc = editable(&base);
            doc.apply_all(Origin::Local, [op]).unwrap();
            doc.undo().unwrap();
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
                .unwrap();
            assert_eq!(out, base);
        }
    }

    #[test]
    fn redo_saves_the_same_bytes_as_the_edit() {
        let base = fixture("golden/metadata.pdf");
        let save = |doc: &mut Document| {
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
                .unwrap();
            out
        };
        let mut edited = editable(&base);
        edit_title(&mut edited);
        let mut redone = editable(&base);
        edit_title(&mut redone);
        redone.undo().unwrap();
        redone.redo().unwrap();
        assert_eq!(save(&mut redone), save(&mut edited));
    }

    #[test]
    fn saving_the_same_state_twice_gives_the_same_bytes() {
        let base = fixture("golden/metadata.pdf");
        let save = || {
            let mut doc = editable(&base);
            edit_title(&mut doc);
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new().with_metadata_date(date(AT)))
                .unwrap();
            out
        };
        assert_eq!(save(), save());
    }
}

mod conformance {
    use super::*;

    #[test]
    fn the_planner_warns_before_a_save_that_breaks_a_pdfua_claim() {
        let mut doc = editable(&fixture("golden/examples/pdf_ua_report.pdf"));
        doc.apply_all(Origin::Local, [RemoveMetadata::new(vec![MetadataKey::Title]).into()])
            .unwrap();
        let planner = SavePlanner::new(&doc).unwrap();
        assert_eq!(planner.conformance_warnings(), [ConformanceWarning::PdfuaTitleMissing]);
    }

    #[test]
    fn a_title_change_on_a_pdfa_file_raises_nothing() {
        let mut doc = editable(&fixture("golden/examples/pdf_a_archive-2b.pdf"));
        edit_title(&mut doc);
        assert_eq!(SavePlanner::new(&doc).unwrap().conformance_warnings(), []);
    }

    #[test]
    fn a_warning_never_blocks_the_save() {
        let mut doc = editable(&fixture("golden/examples/pdf_ua_report.pdf"));
        doc.apply_all(Origin::Local, [RemoveMetadata::new(vec![MetadataKey::Title]).into()])
            .unwrap();
        assert!(doc.save(&mut Vec::new(), &SaveOptions::new()).is_ok());
    }
}
