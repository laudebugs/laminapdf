//! Property test (ADR 0018 tier 5): random `SetInfo` / `RotatePage` sequences, saved with
//! every writer, reopen to the same model facts as the in-memory view, and saving the same
//! state twice gives the same bytes. Seed fixed (`SEED`) so failures reproduce.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::ModeSaves as _;

use common::save::{Facts, corpus_doc, editable, facts, page_ids, reopen, two_pages};
use laminapdf_model::Rotation;
use laminapdf_ops::{Document, InfoPatch, Op, Origin, RotatePage, SetInfo};
use laminapdf_save::{Renumber, SaveOptions, XrefKind};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const SEED: u64 = 0xb3b_5a7e;

#[derive(Debug, Clone)]
enum Recipe {
    Title(Option<&'static str>),
    Author(Option<&'static str>),
    Rotate(usize, Rotation),
}

const TEXTS: [&str; 4] = ["A", "Grüße", "世界", ""];

fn recipe() -> impl Strategy<Value = Recipe> {
    let text = proptest::option::of(proptest::sample::select(&TEXTS[..]));
    let rotation = prop_oneof![
        Just(Rotation::None),
        Just(Rotation::Clockwise90),
        Just(Rotation::Half),
        Just(Rotation::Clockwise270)
    ];
    prop_oneof![
        text.clone().prop_map(Recipe::Title),
        text.prop_map(Recipe::Author),
        (0usize..4, rotation).prop_map(|(p, r)| Recipe::Rotate(p, r)),
    ]
}

fn to_op(doc: &Document, recipe: &Recipe) -> Op {
    match recipe {
        Recipe::Title(t) => SetInfo::new(InfoPatch::new().title(*t), None).into(),
        Recipe::Author(a) => SetInfo::new(InfoPatch::new().author(*a), None).into(),
        Recipe::Rotate(index, rotation) => {
            let ids = page_ids(doc);
            RotatePage::new(ids[index % ids.len()].clone(), *rotation).into()
        }
    }
}

type Make = (&'static str, fn() -> Document);

fn documents() -> Vec<Make> {
    vec![
        ("two pages (classic, /ID)", || editable(&two_pages())),
        ("hello.pdf (no Info)", || corpus_doc("hello.pdf")),
        ("hello-xrefstm.pdf (xref stream)", || corpus_doc("hello-xrefstm.pdf")),
        ("text-example.pdf (direct Info)", || corpus_doc("text-example.pdf")),
        ("outline.pdf", || corpus_doc("outline.pdf")),
    ]
}

#[derive(Debug, Clone, Copy)]
enum How {
    Incremental,
    Full(SaveOptions),
}

const HOWS: [How; 4] = [
    How::Incremental,
    How::Full(SaveOptions::new()),
    How::Full(SaveOptions::new().with_xref(XrefKind::Classic)),
    How::Full(SaveOptions::new().with_renumber(Renumber::Compact)),
];

/// Applies `steps` (each a transaction) and saves with `how`: the facts before saving and
/// the bytes.
fn run(make: fn() -> Document, steps: &[Vec<Recipe>], how: How) -> (Facts, Vec<u8>) {
    let mut doc = make();
    for step in steps {
        let ops: Vec<Op> = step.iter().map(|r| to_op(&doc, r)).collect();
        doc.apply_all(Origin::Local, ops).unwrap();
    }
    let expected = facts(&doc);
    let mut out = Vec::new();
    match how {
        How::Incremental => doc.save_incremental(&mut out, &SaveOptions::new()).map(|_| ()),
        How::Full(options) => doc.save_full_and_rebase(Vec::new(), &options).map(|_| {
            out = doc.data().unwrap().to_vec();
        }),
    }
    .unwrap();
    // The rebased document reads the same as the in-memory view did.
    assert_eq!(facts(&doc), expected);
    (expected, out)
}

fn check(make: fn() -> Document, steps: &[Vec<Recipe>]) -> Result<(), TestCaseError> {
    for how in HOWS {
        let (expected, first) = run(make, steps, how);
        let (_, second) = run(make, steps, how);
        prop_assert_eq!(&first, &second, "{:?} is not deterministic", how);
        prop_assert_eq!(facts(&reopen(&first)), expected, "{:?}", how);
    }
    Ok(())
}

#[test]
fn saved_and_reopened_documents_match_the_in_memory_view() {
    for (name, make) in documents() {
        eprintln!("proptest seed: {SEED:#x}");
        let mut runner = TestRunner::new(Config {
            cases: 24,
            rng_seed: RngSeed::Fixed(SEED),
            failure_persistence: None,
            ..Config::default()
        });
        let steps = proptest::collection::vec(proptest::collection::vec(recipe(), 1..4), 0..6);
        let result = runner.run(&steps, |steps| check(make, &steps));
        assert!(result.is_ok(), "{name}: {result:?}");
    }
}
