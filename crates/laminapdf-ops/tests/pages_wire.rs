//! The page ops' frozen v1 samples (`fixtures/ops/v1/`, ADR 0006 §6): each deserialises,
//! serialises back to the same op, and unknown fields are rejected.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use laminapdf_model::{PagePosition, Rotation};
use laminapdf_ops::{Op, ReferencePolicy, RotateMode};

const SAMPLES: [&str; 6] = [
    "insert-blank-pages",
    "delete-pages",
    "move-pages",
    "rotate-pages",
    "reverse-pages",
    "duplicate-pages",
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
        r#"{ "op": "insertBlankPages", "args": { "at": "end", "count": 1, "size": { "width": 1, "height": 1, "depth": 1 }, "rotate": 0 } }"#,
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
