//! The façade's metadata calls: `metadata`, the metadata ops through `apply` and `apply_log`,
//! `plan` and `would_violate`, with the error and warning codes they surface.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{
    ConstraintKind, Document, MetadataKey, MetadataPatch, MetadataUpdate, Op, OpenOptions, RemoveMetadata, SaveOptions,
    SetMetadata,
};
use laminapdf_test_fixtures::fixture;

fn open(rel: &str) -> Document {
    Document::open_bytes(fixture(rel), OpenOptions::new()).unwrap()
}

fn title(text: &str) -> Op {
    SetMetadata::new(MetadataPatch::new().title(Some(text)), None).into()
}

#[test]
fn an_edit_reads_back_from_metadata_with_both_sides_in_step() {
    let mut doc = open("golden/metadata.pdf");
    doc.apply(vec![title("Edited")]).unwrap();
    let meta = doc.metadata().unwrap();
    assert_eq!(
        (
            meta.title.as_deref(),
            meta.disagreements.len(),
            doc.info().unwrap().title.as_deref()
        ),
        (Some("Edited"), 0, Some("Edited"))
    );
}

#[test]
fn an_op_log_with_the_new_ops_applies() {
    let mut doc = open("golden/metadata.pdf");
    let edited = doc
        .apply_log(r#"{ "v": 1, "ops": [{ "op": "removeMetadata", "args": { "keys": ["title"] } }] }"#)
        .unwrap();
    assert!(edited.still_in_earlier_revisions);
}

#[test]
fn a_malformed_packet_is_refused_with_its_code() {
    let packet = b"<x:xmpmeta";
    let bytes = laminapdf_test_fixtures::classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R /Metadata 3 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 >>",
            &laminapdf_test_fixtures::stream_body("/Type /Metadata /Subtype /XML", packet),
        ],
        "<< /Size 4 /Root 1 0 R >>",
    );
    let mut doc = Document::open_bytes(bytes, OpenOptions::new()).unwrap();
    let err = doc.apply(vec![title("T")]).unwrap_err();
    assert_eq!((err.code(), err.kind().as_str()), ("malformed-xmp", "input"));
}

#[test]
fn plan_shows_the_constraint_and_the_stamp_before_saving() {
    let mut doc = open("signed/approval.pdf");
    doc.apply(vec![title("Edited")]).unwrap();
    let options = SaveOptions::new().with_metadata_date("D:20261110093000Z".parse().unwrap());
    let plan = doc.plan(&options).unwrap();
    assert_eq!(
        (plan.constraint, plan.refusal, plan.metadata, plan.signatures.len()),
        (ConstraintKind::AppendOnly, None, MetadataUpdate::Stamp, 1)
    );
}

#[test]
fn plan_names_the_refusal_instead_of_failing() {
    let mut doc = open("signed/certified-p1.pdf");
    doc.apply(vec![title("Edited")]).unwrap();
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    assert_eq!(
        (plan.constraint, plan.refusal.map(laminapdf::ErrorCode::as_str), plan.violations.len()),
        (ConstraintKind::Conflict, Some("would-invalidate-signatures"), 1)
    );
}

#[test]
fn would_violate_warns_before_an_edit_on_a_certified_file() {
    let doc = open("signed/certified-p3.pdf");
    let op = RemoveMetadata::new(vec![MetadataKey::Title]).into();
    assert_eq!(doc.would_violate(&op).unwrap().len(), 1);
}

#[test]
fn conformance_warnings_come_as_warnings_with_their_codes() {
    let mut doc = open("golden/examples/pdf_ua_report.pdf");
    doc.apply(vec![RemoveMetadata::new(vec![MetadataKey::Title]).into()])
        .unwrap();
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    let codes: Vec<&str> = plan.conformance_warnings.iter().map(laminapdf::Warning::code).collect();
    assert_eq!(codes, ["pdfua-title-missing"]);
}
