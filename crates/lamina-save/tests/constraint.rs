//! `SaveConstraint` detection (ADR 0007 §2): signatures, `DocMDP`, `FieldMDP`, UR3, rewrite
//! reasons and violations, over the signed fixtures (`fixtures/signed/`, pyHanko), the corpus
//! (`signed.pdf`, `tampered.pdf`) and hand-built files.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::{classic, corpus_doc, editable, rotate, set_title, tampered_and_repaired};
use lamina_model::Rotation;
use lamina_ops::{Document, ModificationClass};
use lamina_save::{ConstraintKind, DocMdpPermission, Restriction, RewriteReason, SavePlanner, SignatureKind};

fn signed(name: &str) -> Document {
    editable(&fixture(&format!("signed/{name}")))
}

fn planner(doc: &Document) -> SavePlanner {
    SavePlanner::new(doc).unwrap()
}

fn kinds(doc: &Document) -> Vec<SignatureKind> {
    planner(doc).constraint().signatures.iter().map(|s| s.kind).collect()
}

#[test]
fn an_approval_signature_is_append_only() {
    let doc = signed("approval.pdf");
    let p = planner(&doc);
    let sig = &p.constraint().signatures[0];
    assert_eq!(
        (
            p.constraint().kind(),
            sig.kind,
            sig.field.is_some(),
            sig.value.is_some(),
            sig.revision,
            p.constraint().doc_mdp.is_none()
        ),
        (
            ConstraintKind::AppendOnly,
            SignatureKind::Approval,
            true,
            true,
            Some(2),
            true
        )
    );
}

#[test]
fn the_signature_covers_the_whole_signed_file() {
    let bytes = fixture("signed/approval.pdf");
    let doc = editable(&bytes);
    let covers = planner(&doc).constraint().signatures[0].covers;
    assert_eq!(covers, Some(u64::try_from(bytes.len()).unwrap()));
}

#[test]
fn certified_files_report_their_docmdp_permission() {
    for (name, permission) in [
        ("certified-p1.pdf", DocMdpPermission::NoChanges),
        ("certified-p2.pdf", DocMdpPermission::FormFill),
        ("certified-p3.pdf", DocMdpPermission::Annotate),
    ] {
        let doc = signed(name);
        let p = planner(&doc);
        let doc_mdp = p.constraint().doc_mdp.clone().unwrap();
        assert_eq!(
            (
                p.constraint().kind(),
                kinds(&doc),
                doc_mdp.permission,
                doc_mdp.signature.kind
            ),
            (
                ConstraintKind::AppendOnly,
                vec![SignatureKind::Certification],
                permission,
                SignatureKind::Certification
            ),
            "{name}"
        );
    }
}

#[test]
fn a_document_timestamp_is_found() {
    assert_eq!(kinds(&signed("timestamp.pdf")), [SignatureKind::DocTimeStamp]);
}

#[test]
fn a_field_lock_restricts_form_changes_only() {
    let doc = signed("locked.pdf");
    let p = planner(&doc);
    let restrictions = |class| -> Vec<Restriction> {
        p.would_violate_class(class)
            .into_iter()
            .map(|v| v.restriction)
            .collect()
    };
    assert_eq!(
        (
            restrictions(ModificationClass::FormFill),
            restrictions(ModificationClass::Metadata)
        ),
        (vec![Restriction::FieldMdp], vec![])
    );
}

#[test]
fn the_corpus_signature_is_found() {
    assert_eq!(kinds(&corpus_doc("signed.pdf")), [SignatureKind::Approval]);
}

#[test]
fn the_tampered_corpus_file_is_signed_and_not_repaired() {
    // One signed byte changed: the signature no longer verifies, but the structure is intact.
    let doc = corpus_doc("tampered.pdf");
    assert_eq!(planner(&doc).constraint().kind(), ConstraintKind::AppendOnly);
}

#[test]
fn a_signed_and_repaired_file_is_a_conflict_with_its_signature_found_by_scanning() {
    let doc = editable(&tampered_and_repaired());
    assert!(doc.was_repaired());
    let p = planner(&doc);
    assert_eq!(
        (
            p.constraint().kind(),
            p.constraint().signatures.len(),
            p.constraint().rewrite_reasons.clone()
        ),
        (ConstraintKind::Conflict, 1, vec![RewriteReason::RepairedBase])
    );
}

#[test]
fn unsigned_corpus_files_are_free_or_need_a_rewrite() {
    for name in [
        "hello.pdf",
        "hello-incremental.pdf",
        "hello-xrefstm.pdf",
        "imaging-demo.pdf",
        "text-example.pdf",
        "raw.pdf",
        "pdflib.pdf",
        "fpdf2.pdf",
        "reportlab.pdf",
        "outline.pdf",
    ] {
        assert_eq!(
            planner(&corpus_doc(name)).constraint().kind(),
            ConstraintKind::Free,
            "{name}"
        );
    }
    assert_eq!(
        planner(&corpus_doc("hello-broken.pdf")).constraint().rewrite_reasons,
        [RewriteReason::RepairedBase]
    );
}

#[test]
fn docmdp_p1_forbids_set_info() {
    let mut doc = signed("certified-p1.pdf");
    set_title(&mut doc, "Changed");
    let p = planner(&doc);
    let v = &p.constraint().violations;
    assert_eq!(
        (p.constraint().kind(), v.len(), v[0].class, v[0].restriction),
        (
            ConstraintKind::Conflict,
            1,
            ModificationClass::Metadata,
            Restriction::DocMdp(DocMdpPermission::NoChanges)
        )
    );
}

#[test]
fn docmdp_p2_forbids_rotate_page() {
    let mut doc = signed("certified-p2.pdf");
    rotate(&mut doc, 0, Rotation::Clockwise90);
    let v = planner(&doc).constraint().violations.clone();
    assert_eq!(
        (v.len(), v[0].class, v[0].restriction),
        (
            1,
            ModificationClass::PageStructure,
            Restriction::DocMdp(DocMdpPermission::FormFill)
        )
    );
}

#[test]
fn docmdp_p3_allows_annotations_and_p2_does_not() {
    let p3 = signed("certified-p3.pdf");
    let p2 = signed("certified-p2.pdf");
    assert_eq!(
        (
            planner(&p3).would_violate_class(ModificationClass::Annotation).len(),
            planner(&p2).would_violate_class(ModificationClass::Annotation).len(),
            planner(&p2).would_violate_class(ModificationClass::FormFill).len(),
            planner(&p2).would_violate_class(ModificationClass::Signature).len(),
        ),
        (0, 1, 0, 0)
    );
}

#[test]
fn would_violate_checks_an_op_before_it_is_applied() {
    let doc = signed("certified-p1.pdf");
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let op = lamina_ops::RotatePage::new(page, Rotation::Half).into();
    assert_eq!(planner(&doc).would_violate(&op).len(), 1);
}

#[test]
fn an_approval_signature_restricts_nothing() {
    let mut doc = signed("approval.pdf");
    set_title(&mut doc, "Changed");
    let p = planner(&doc);
    assert_eq!(
        (p.constraint().kind(), p.constraint().violations.len()),
        (ConstraintKind::AppendOnly, 0)
    );
}

#[test]
fn undoing_back_to_the_base_clears_the_violation() {
    let mut doc = signed("certified-p1.pdf");
    set_title(&mut doc, "Changed");
    doc.undo().unwrap();
    assert_eq!(planner(&doc).constraint().kind(), ConstraintKind::AppendOnly);
}

/// Catalog 1 with `/Perms` 6, pages 2, page 3, form 4 with field 5, signature 7.
fn hand_signed(field: &str, sig: &str, perms: &str) -> Vec<u8> {
    classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R /Perms 6 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Fields [5 0 R] >>",
            field,
            perms,
            sig,
        ],
        "",
    )
}

#[test]
fn usage_rights_allow_only_what_they_grant() {
    let bytes = hand_signed(
        "<< /FT /Sig /T (UR) /V 7 0 R >>",
        "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> /Reference [<< /TransformMethod /UR3 \
         /TransformParams << /Annots [/Create /Delete] /Form [/FillIn] >> >>] >>",
        "<< /UR3 7 0 R >>",
    );
    let doc = editable(&bytes);
    let p = planner(&doc);
    assert_eq!(
        (
            kinds(&doc),
            p.would_violate_class(ModificationClass::Annotation).len(),
            p.would_violate_class(ModificationClass::FormFill).len(),
            p.would_violate_class(ModificationClass::Metadata)
                .iter()
                .map(|v| v.restriction)
                .collect::<Vec<_>>()
        ),
        (vec![SignatureKind::UsageRights], 0, 0, vec![Restriction::UsageRights])
    );
}

#[test]
fn a_perms_docmdp_without_a_transform_defaults_to_p2() {
    let bytes = hand_signed(
        "<< /FT /Sig /T (Cert) /V 7 0 R >>",
        "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> >>",
        "<< /DocMDP 7 0 R >>",
    );
    let doc = editable(&bytes);
    assert_eq!(
        planner(&doc).constraint().doc_mdp.as_ref().map(|d| d.permission),
        Some(DocMdpPermission::FormFill)
    );
}

#[test]
fn an_inherited_field_type_and_a_direct_kid_are_followed() {
    let bytes = hand_signed(
        "<< /FT /Sig /T (Parent) /Kids [<< /T (Kid) /V 7 0 R >>] >>",
        "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> >>",
        "<< >>",
    );
    let doc = editable(&bytes);
    let p = planner(&doc);
    let sig = &p.constraint().signatures[0];
    // The kid is a direct dictionary: no field ID.
    assert_eq!(
        (kinds(&doc), sig.field.is_none()),
        (vec![SignatureKind::Approval], true)
    );
}

#[test]
fn an_unsigned_signature_field_is_not_a_signature() {
    let bytes = hand_signed("<< /FT /Sig /T (Empty) >>", "<< /Type /Sig >>", "<< >>");
    assert_eq!(planner(&editable(&bytes)).constraint().kind(), ConstraintKind::Free);
}

#[test]
fn a_field_cycle_ends() {
    let bytes = hand_signed("<< /FT /Sig /T (Loop) /Kids [5 0 R] >>", "<< >>", "<< >>");
    assert_eq!(planner(&editable(&bytes)).constraint().kind(), ConstraintKind::Free);
}
