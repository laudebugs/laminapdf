//! The mode table of ADR 0007 §3, row by row (4 kinds × 3 requested modes, with and without
//! the override), then the same rules on real files: the signed fixtures and a signed file
//! that is also a repaired base.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::{corpus_doc, editable, rotate, set_title, tampered_and_repaired};
use lamina_model::Rotation;
use lamina_ops::{Document, ModificationClass, SaveState};
use lamina_save::{
    ConstraintKind, DocMdpPermission, Restriction, RewriteReason, Save, SaveConstraint, SaveError, SaveMode,
    SaveOptions, SavePlan, SignatureInfo, SignatureKind, Violation,
};

fn approval() -> SignatureInfo {
    SignatureInfo::new(SignatureKind::Approval)
}

fn constraint(kind: ConstraintKind, with_reasons: bool) -> SaveConstraint {
    let mut c = SaveConstraint::default();
    match kind {
        ConstraintKind::Free => {}
        ConstraintKind::AppendOnly => c.signatures.push(approval()),
        ConstraintKind::RequiresRewrite => c.rewrite_reasons.push(RewriteReason::RepairedBase),
        _ => {
            let cert = SignatureInfo::new(SignatureKind::Certification);
            c.signatures.extend([approval(), cert.clone()]);
            if with_reasons {
                c.rewrite_reasons.push(RewriteReason::RepairedBase);
            } else {
                c.violations.push(Violation::new(
                    ModificationClass::Metadata,
                    Restriction::DocMdp(DocMdpPermission::NoChanges),
                    cert,
                ));
            }
        }
    }
    assert_eq!(c.kind(), kind);
    c
}

/// What the table says: a mode, or an error code.
type Outcome = Result<SaveMode, &'static str>;

fn resolve(c: &SaveConstraint, mode: SaveMode, overridden: bool) -> Outcome {
    let options = SaveOptions::new()
        .with_mode(mode)
        .with_invalidate_signatures(overridden);
    SavePlan::resolve(c.clone(), &options)
        .map(|p| p.mode())
        .map_err(|e| e.code())
}

const REFUSED: &str = "would-invalidate-signatures";
const REWRITE: &str = "requires-rewrite";
use SaveMode::{Auto, FullRewrite as Full, Incremental as Inc};

fn check(kind: ConstraintKind, reasons: bool, rows: [(SaveMode, bool, Outcome); 6]) {
    let c = constraint(kind, reasons);
    for (mode, overridden, expected) in rows {
        assert_eq!(
            resolve(&c, mode, overridden),
            expected,
            "{kind:?} (reasons {reasons}), {mode:?}, override {overridden}"
        );
    }
}

#[test]
fn free_row() {
    check(
        ConstraintKind::Free,
        false,
        [
            (Auto, false, Ok(Inc)),
            (Auto, true, Ok(Inc)),
            (Inc, false, Ok(Inc)),
            (Inc, true, Ok(Inc)),
            (Full, false, Ok(Full)),
            (Full, true, Ok(Full)),
        ],
    );
}

#[test]
fn without_a_base_auto_is_a_full_rewrite() {
    let mut c = SaveConstraint::default();
    c.rewrite_reasons.push(RewriteReason::NoBase);
    assert_eq!(resolve(&c, Auto, false), Ok(Full));
}

#[test]
fn without_a_base_an_incremental_save_is_refused_up_front() {
    let mut c = SaveConstraint::default();
    c.rewrite_reasons.push(RewriteReason::NoBase);
    assert_eq!(resolve(&c, Inc, false), Err(REWRITE));
}

#[test]
fn append_only_row() {
    check(
        ConstraintKind::AppendOnly,
        false,
        [
            (Auto, false, Ok(Inc)),
            (Auto, true, Ok(Inc)),
            (Inc, false, Ok(Inc)),
            (Inc, true, Ok(Inc)),
            (Full, false, Err(REFUSED)),
            (Full, true, Ok(Full)),
        ],
    );
}

#[test]
fn requires_rewrite_row() {
    check(
        ConstraintKind::RequiresRewrite,
        false,
        [
            (Auto, false, Ok(Full)),
            (Auto, true, Ok(Full)),
            (Inc, false, Err(REWRITE)),
            (Inc, true, Err(REWRITE)),
            (Full, false, Ok(Full)),
            (Full, true, Ok(Full)),
        ],
    );
}

#[test]
fn conflict_row_with_rewrite_reasons() {
    check(
        ConstraintKind::Conflict,
        true,
        [
            (Auto, false, Err(REFUSED)),
            (Auto, true, Ok(Full)),
            (Inc, false, Err(REFUSED)),
            (Inc, true, Err(REWRITE)),
            (Full, false, Err(REFUSED)),
            (Full, true, Ok(Full)),
        ],
    );
}

#[test]
fn conflict_row_with_violations_only() {
    check(
        ConstraintKind::Conflict,
        false,
        [
            (Auto, false, Err(REFUSED)),
            (Auto, true, Ok(Inc)),
            (Inc, false, Err(REFUSED)),
            (Inc, true, Ok(Inc)),
            (Full, false, Err(REFUSED)),
            (Full, true, Ok(Full)),
        ],
    );
}

#[test]
fn a_refusal_carries_the_whole_constraint() {
    let c = constraint(ConstraintKind::Conflict, true);
    let err = SavePlan::resolve(c.clone(), &SaveOptions::new()).unwrap_err();
    assert_eq!(
        err,
        SaveError::WouldInvalidateSignatures {
            signatures: c.signatures,
            rewrite_reasons: c.rewrite_reasons,
            violations: c.violations,
            signatures_unknown: false,
        }
    );
}

#[test]
fn an_overridden_incremental_save_invalidates_only_the_violated_signatures() {
    let c = constraint(ConstraintKind::Conflict, false);
    let plan = SavePlan::resolve(c, &SaveOptions::new().with_invalidate_signatures(true)).unwrap();
    let kinds: Vec<_> = plan.would_invalidate().iter().map(|s| s.kind).collect();
    assert_eq!(kinds, [SignatureKind::Certification]);
}

#[test]
fn an_overridden_full_rewrite_invalidates_every_signature() {
    let c = constraint(ConstraintKind::Conflict, false);
    let options = SaveOptions::new().with_mode(Full).with_invalidate_signatures(true);
    assert_eq!(SavePlan::resolve(c, &options).unwrap().would_invalidate().len(), 2);
}

#[test]
fn nothing_is_invalidated_without_a_need() {
    let c = constraint(ConstraintKind::AppendOnly, false);
    let plan = SavePlan::resolve(c, &SaveOptions::new().with_invalidate_signatures(true)).unwrap();
    assert_eq!(plan.would_invalidate(), []);
}

// --- the same rules on files ---------------------------------------------------------------

const SIGNED: [&str; 6] = [
    "signed/approval.pdf",
    "signed/certified-p1.pdf",
    "signed/certified-p2.pdf",
    "signed/certified-p3.pdf",
    "signed/locked.pdf",
    "signed/timestamp.pdf",
];

fn open(path: &str) -> Document {
    editable(&fixture(path))
}

#[test]
fn auto_saves_a_signed_file_incrementally_and_keeps_the_signed_bytes() {
    for path in SIGNED
        .iter()
        .copied()
        .chain(["corpus/signed.pdf", "corpus/tampered.pdf"])
    {
        let bytes = fixture(path);
        let mut doc = editable(&bytes);
        rotate(&mut doc, 0, Rotation::Half);
        if path.contains("certified") {
            // Page structure is outside every DocMDP permission: refused without the override.
            assert_eq!(
                doc.save(&mut Vec::new(), &SaveOptions::new()).map_err(|e| e.code()),
                Err(REFUSED),
                "{path}"
            );
            continue;
        }
        let mut out = Vec::new();
        let report = doc.save(&mut out, &SaveOptions::new()).unwrap();
        assert_eq!(
            (report.mode(), out.starts_with(&bytes), report.invalidated().len()),
            (Inc, true, 0),
            "{path}"
        );
    }
}

#[test]
fn a_requested_full_rewrite_of_a_signed_file_names_every_signature() {
    for path in SIGNED
        .iter()
        .copied()
        .chain(["corpus/signed.pdf", "corpus/tampered.pdf"])
    {
        let mut doc = open(path);
        let mut out = Vec::new();
        let err = doc.save(&mut out, &SaveOptions::new().with_mode(Full)).unwrap_err();
        let SaveError::WouldInvalidateSignatures { signatures, .. } = err else {
            panic!("{path}: {err:?}");
        };
        assert_eq!(
            (signatures.len(), out.len(), doc.save_state()),
            (1, 0, SaveState::Clean),
            "{path}"
        );
    }
}

#[test]
fn a_signed_and_repaired_file_refuses_every_mode_without_the_override() {
    for mode in [Auto, Inc, Full] {
        let mut doc = editable(&tampered_and_repaired());
        set_title(&mut doc, "Changed");
        let err = doc
            .save(&mut Vec::new(), &SaveOptions::new().with_mode(mode))
            .unwrap_err();
        let SaveError::WouldInvalidateSignatures {
            signatures,
            rewrite_reasons,
            ..
        } = err
        else {
            panic!("{mode:?}: {err:?}");
        };
        assert_eq!(
            (signatures.len(), rewrite_reasons),
            (1, vec![RewriteReason::RepairedBase]),
            "{mode:?}"
        );
    }
}

#[test]
fn a_repaired_unsigned_base_is_saved_only_as_a_full_rewrite() {
    let mut doc = corpus_doc("hello-broken.pdf");
    rotate(&mut doc, 0, Rotation::Half);
    assert_eq!(
        doc.save(&mut Vec::new(), &SaveOptions::new().with_mode(Inc)).err(),
        Some(SaveError::RequiresRewrite {
            reasons: vec![RewriteReason::RepairedBase]
        })
    );
    let report = doc.save_and_rebase(Vec::new(), &SaveOptions::new()).unwrap();
    assert_eq!((report.mode(), doc.save_state()), (Full, SaveState::Clean));
}

#[test]
fn plan_matches_what_save_does() {
    let mut doc = open("signed/certified-p1.pdf");
    set_title(&mut doc, "Changed");
    let options = SaveOptions::new().with_invalidate_signatures(true);
    let plan = doc.plan(&options).unwrap();
    let report = doc.save(&mut Vec::new(), &options).unwrap();
    assert_eq!(
        (plan.mode(), plan.would_invalidate()),
        (report.mode(), report.invalidated())
    );
}

#[test]
fn a_save_is_refused_while_a_transaction_is_open() {
    let mut doc = corpus_doc("hello.pdf");
    doc.begin(lamina_ops::Origin::Local).unwrap();
    assert_eq!(
        doc.save(&mut Vec::new(), &SaveOptions::new()).err(),
        Some(SaveError::Ops(lamina_ops::Error::NestedTransaction))
    );
}
