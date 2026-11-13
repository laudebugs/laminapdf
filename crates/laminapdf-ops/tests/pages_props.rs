//! Property tests for the page-structure ops (ADR 0006 §3, ADR 0018 tier 5): random sequences
//! of inserts, deletions, moves, rotations and reversals, each its own local transaction, over
//! `fixtures/pages/`. Undoing everything leaves the overlay empty; redoing everything gives the
//! same overlay as before the undo.
//!
//! The seed is fixed (`SEED`, printed to stderr) so failures reproduce; proptest still shrinks.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids, snapshot};
use laminapdf_model::{PagePosition, PageSize, Rotation};
use laminapdf_ops::{
    DeletePages, Document, InsertBlankPages, MovePages, Op, Origin, ReferencePolicy, ReversePages, RotatePages,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const SEED: u64 = 0x0e2b_5eed;

/// A recipe for a page op, resolved against the document's pages when applied (indices are
/// taken modulo the page count).
#[derive(Debug, Clone)]
enum Recipe {
    Insert(usize, u32),
    Delete(Vec<usize>, bool),
    Move(Vec<usize>, usize),
    Rotate(Vec<usize>, Rotation, bool),
    Reverse(Vec<usize>),
}

fn recipe() -> impl Strategy<Value = Recipe> {
    let picks = proptest::collection::vec(0usize..64, 1..4);
    let rotation = prop_oneof![
        Just(Rotation::None),
        Just(Rotation::Clockwise90),
        Just(Rotation::Half),
        Just(Rotation::Clockwise270)
    ];
    prop_oneof![
        (0usize..64, 1u32..3).prop_map(|(at, n)| Recipe::Insert(at, n)),
        (picks.clone(), any::<bool>()).prop_map(|(p, r)| Recipe::Delete(p, r)),
        (picks.clone(), 0usize..64).prop_map(|(p, to)| Recipe::Move(p, to)),
        (picks.clone(), rotation, any::<bool>()).prop_map(|(p, r, t)| Recipe::Rotate(p, r, t)),
        picks.prop_map(Recipe::Reverse),
    ]
}

fn to_op(doc: &Document, recipe: &Recipe) -> Option<Op> {
    let ids = page_ids(doc);
    let pick = |i: usize| ids[i % ids.len()].clone();
    let picks = |p: &[usize]| p.iter().map(|&i| pick(i)).collect::<Vec<_>>();
    Some(match recipe {
        Recipe::Insert(at, n) => InsertBlankPages::new(PagePosition::After(pick(*at)), *n, PageSize::LETTER).into(),
        Recipe::Delete(p, remove) => {
            let pages = picks(p);
            if pages.len() >= ids.len() {
                return None;
            }
            let policy = if *remove {
                ReferencePolicy::Remove
            } else {
                ReferencePolicy::Retarget
            };
            DeletePages::new(pages).with_references(policy).into()
        }
        Recipe::Move(p, to) => MovePages::new(picks(p), PagePosition::Before(pick(*to))).into(),
        Recipe::Rotate(p, r, turn) => {
            if *turn {
                RotatePages::turn(picks(p), *r).into()
            } else {
                RotatePages::set(picks(p), *r).into()
            }
        }
        Recipe::Reverse(p) => ReversePages::new(picks(p)).into(),
    })
}

/// Applies `recipes` one transaction each, then undoes everything and redoes everything.
fn check(name: &str, recipes: &[Recipe]) -> Result<(), TestCaseError> {
    // The first edit of a file with a page listed twice copies it; start from a file without.
    let mut doc = open(&fixture(&format!("pages/{name}")));
    for recipe in recipes {
        if let Some(op) = to_op(&doc, recipe) {
            doc.apply_all(Origin::Local, [op]).unwrap();
        }
    }
    let applied = snapshot(&doc);
    while doc.undo().unwrap().is_some() {}
    prop_assert!(!doc.is_dirty(), "{name}: overlay not empty after undoing everything");
    while doc.redo().unwrap().is_some() {}
    prop_assert_eq!(snapshot(&doc), applied, "{}: redo didn't restore the overlay", name);
    Ok(())
}

#[test]
fn undo_all_empties_the_overlay_and_redo_all_restores_it() {
    eprintln!("proptest seed: {SEED:#x}");
    for name in [
        "inherited.pdf",
        "references.pdf",
        "labels.pdf",
        "forms.pdf",
        "chapters.pdf",
    ] {
        let mut runner = TestRunner::new(Config {
            cases: 24,
            rng_seed: RngSeed::Fixed(SEED),
            failure_persistence: None,
            ..Config::default()
        });
        let result = runner.run(&proptest::collection::vec(recipe(), 1..6), |recipes| {
            check(name, &recipes)
        });
        assert!(result.is_ok(), "{name}: {result:?}");
    }
}
