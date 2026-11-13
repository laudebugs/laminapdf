//! The page ops' frozen v1 samples (`fixtures/ops/v1/`, ADR 0006 §6): each deserialises,
//! serialises back to the same op, and unknown fields are rejected.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use laminapdf_model::{PagePosition, Rotation};
use laminapdf_ops::{Op, ReferencePolicy, RotateMode};

const SAMPLES: [&str; 9] = [
    "insert-blank-pages",
    "delete-pages",
    "move-pages",
    "rotate-pages",
    "reverse-pages",
    "duplicate-pages",
    "insert-pages-from",
    "replace-pages",
    "merge-documents",
];

fn sample(name: &str) -> Op {
    serde_json::from_slice(&fixture(&format!("ops/v1/{name}.json"))).unwrap()
}

#[test]
fn every_sample_serialises_back_to_the_same_op() {
    for name in SAMPLES {
        let op = sample(name);
        let again: Op = serde_json::from_str(&serde_json::to_string(&op).unwrap()).unwrap();
        assert_eq!(again, op, "{name}");
    }
}

#[test]
fn the_samples_read_their_fields() {
    let Op::InsertBlankPages(insert) = sample("insert-blank-pages") else {
        panic!("not insertBlankPages");
    };
    let Op::DeletePages(delete) = sample("delete-pages") else {
        panic!("not deletePages");
    };
    let Op::RotatePages(rotate) = sample("rotate-pages") else {
        panic!("not rotatePages");
    };
    let Op::MovePages(moved) = sample("move-pages") else {
        panic!("not movePages");
    };
    assert_eq!(
        (
            insert.at,
            insert.count,
            insert.size.width.get(),
            delete.references,
            rotate.angle,
            rotate.mode,
            moved.to
        ),
        (
            PagePosition::After("p3.0".parse().unwrap()),
            2,
            612.0,
            ReferencePolicy::Retarget,
            Rotation::Clockwise90,
            RotateMode::Turn,
            PagePosition::Start
        )
    );
}

#[test]
fn defaults_fill_the_optional_fields() {
    let delete: Op = serde_json::from_str(r#"{ "op": "deletePages", "args": { "pages": ["p7.0"] } }"#).unwrap();
    let rotate: Op =
        serde_json::from_str(r#"{ "op": "rotatePages", "args": { "pages": ["p7.0"], "angle": 180 } }"#).unwrap();
    assert!(matches!(delete, Op::DeletePages(d) if d.references == ReferencePolicy::Retarget));
    assert!(matches!(rotate, Op::RotatePages(r) if r.mode == RotateMode::Set));
}

#[test]
fn unknown_fields_and_positions_are_rejected() {
    for json in [
        r#"{ "op": "movePages", "args": { "pages": ["p7.0"], "to": "start", "extra": 1 } }"#,
        r#"{ "op": "movePages", "args": { "pages": ["p7.0"], "to": { "beside": "p3.0" } } }"#,
        r#"{ "op": "rotatePages", "args": { "pages": ["p7.0"], "angle": 45 } }"#,
        r#"{ "op": "duplicatePages", "args": { "pages": ["p7.0"], "at": "end", "count": 2 } }"#,
        r#"{ "op": "duplicatePages", "args": { "pages": ["p7.0"] } }"#,
        r#"{ "op": "insertPagesFrom", "args": { "source": "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d", "pages": ["p3.0"], "at": "end", "copy": true } }"#,
        r#"{ "op": "insertPagesFrom", "args": { "source": "not-a-hash", "pages": ["p3.0"], "at": "end" } }"#,
        r#"{ "op": "insertPagesFrom", "args": { "source": "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d", "pages": ["p3.0"], "at": "end", "carry": { "fields": "merge" } } }"#,
        r#"{ "op": "replacePages", "args": { "targets": ["p7.0"], "source": "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d" } }"#,
        r#"{ "op": "insertBlankPages", "args": { "at": "end", "count": 1, "size": { "width": 1, "height": 1, "depth": 1 }, "rotate": 0 } }"#,
        r#"{ "op": "mergeDocuments", "args": { "sources": [{ "source": "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d", "name": "a.pdf" }], "at": "end" } }"#,
        r#"{ "op": "mergeDocuments", "args": { "sources": [], "at": "end", "options": { "outline": "flat" } } }"#,
        r#"{ "op": "mergeDocuments", "args": { "sources": [] } }"#,
    ] {
        assert!(serde_json::from_str::<Op>(json).is_err(), "{json}");
    }
}

#[test]
fn the_duplicate_sample_names_its_pages_and_place() {
    let Op::DuplicatePages(dup) = sample("duplicate-pages") else {
        panic!("not duplicatePages");
    };
    assert_eq!(
        (dup.pages, dup.at),
        (
            vec!["p7.0".parse().unwrap()],
            PagePosition::After("p7.0".parse().unwrap())
        )
    );
}

#[test]
fn the_import_samples_name_their_source_pages_and_carry() {
    let Op::InsertPagesFrom(insert) = sample("insert-pages-from") else {
        panic!("not insertPagesFrom");
    };
    assert_eq!(
        (insert.source.to_string(), insert.pages.len(), insert.carry.fields),
        (
            "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d".to_owned(),
            2,
            laminapdf_ops::FieldClash::Link
        )
    );
    let Op::ReplacePages(replace) = sample("replace-pages") else {
        panic!("not replacePages");
    };
    assert_eq!(
        (replace.targets.len(), replace.carry, replace.pages.len()),
        (1, laminapdf_ops::Carry::default(), 1)
    );
}

#[test]
fn the_merge_sample_names_its_sources_titles_and_pages() {
    let Op::MergeDocuments(merge) = sample("merge-documents") else {
        panic!("not mergeDocuments");
    };
    let sources: Vec<(Option<&str>, Option<usize>)> = merge
        .sources
        .iter()
        .map(|s| (s.title.as_deref(), s.pages.as_ref().map(Vec::len)))
        .collect();
    assert_eq!(
        (sources, merge.at, merge.options.outline),
        (
            vec![(Some("Chapters"), None), (None, Some(2))],
            PagePosition::End,
            laminapdf_ops::OutlineMerge::PerFile
        )
    );
}

#[test]
fn a_merge_without_options_or_pages_reads_with_the_defaults() {
    let op: Op = serde_json::from_str(
        r#"{ "op": "mergeDocuments", "args": { "sources": [{ "source": "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d" }], "at": "start" } }"#,
    )
    .unwrap();
    let Op::MergeDocuments(merge) = op else {
        panic!("not mergeDocuments");
    };
    assert_eq!(
        (merge.sources[0].pages.clone(), merge.options),
        (None, laminapdf_ops::MergeOptions::default())
    );
}
