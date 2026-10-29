//! Property tests (ADR 0018 tier 5, ADR 0006 §3–§4): random sequences of `SetInfo` and
//! `RotatePage`, grouped into local transactions (some coalescing), over files with an indirect, a direct and
//! no Info dictionary and with inherited `/Rotate`. Undoing everything leaves the overlay
//! empty; redoing everything gives the same overlay as before the undo.
//!
//! The seed is fixed (printed on every run) so failures reproduce; proptest still shrinks.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{corpus, inherited_rotate, page_ids, snapshot};
use lamina_model::{PdfDate, Rotation};
use lamina_ops::{Document, InfoPatch, Op, Origin, RotatePage, SetInfo, UndoOutcome};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const SEED: u64 = 0x1a31_0b3a;

/// A recipe for an op, resolved against a document's pages when applied.
#[derive(Debug, Clone)]
#[expect(clippy::option_option, reason = "InfoPatch's fields: leave, clear or set")]
enum Recipe {
    Info([Option<Option<&'static str>>; 3], Option<u8>),
    Rotate(usize, Rotation),
}

const TEXTS: [&str; 5] = ["A", "Report", "Grüße", "世界", ""];

fn text_field() -> impl Strategy<Value = Option<Option<&'static str>>> {
    prop_oneof![
        Just(None),
        Just(Some(None)),
        proptest::sample::select(&TEXTS[..]).prop_map(|t| Some(Some(t))),
    ]
}

fn recipe() -> impl Strategy<Value = Recipe> {
    let rotation = prop_oneof![
        Just(Rotation::None),
        Just(Rotation::Clockwise90),
        Just(Rotation::Half),
        Just(Rotation::Clockwise270)
    ];
    prop_oneof![
        (
            [text_field(), text_field(), text_field()],
            proptest::option::of(0u8..30)
        )
            .prop_map(|(f, d)| Recipe::Info(f, d)),
        (0usize..8, rotation).prop_map(|(p, r)| Recipe::Rotate(p, r)),
    ]
}

fn to_op(doc: &Document, recipe: &Recipe) -> Op {
    to_op_in(doc, recipe)
}

fn to_op_in(doc: &lamina_model::Document, recipe: &Recipe) -> Op {
    match recipe {
        Recipe::Info([title, author, keywords], day) => {
            let mut patch = InfoPatch::new();
            patch.title = title.map(|t| t.map(str::to_owned));
            patch.author = author.map(|t| t.map(str::to_owned));
            patch.keywords = keywords.map(|t| t.map(str::to_owned));
            let at = day.map(|d| format!("D:202610{:02}120000Z", d % 28 + 1).parse::<PdfDate>().unwrap());
            SetInfo::new(patch, at).into()
        }
        Recipe::Rotate(index, rotation) => {
            let ids = page_ids(doc);
            let page = ids.get(index % ids.len()).unwrap().clone();
            RotatePage::new(page, *rotation).into()
        }
    }
}

/// A named way to open a test document.
type Make = (&'static str, fn() -> Document);

fn documents() -> Vec<Make> {
    vec![
        ("inherited-rotate (indirect Info)", inherited_rotate),
        ("hello.pdf (no Info)", || corpus("hello.pdf")),
        ("text-example.pdf (direct Info)", || corpus("text-example.pdf")),
        ("outline.pdf", || corpus("outline.pdf")),
    ]
}

fn runner() -> TestRunner {
    eprintln!("proptest seed: {SEED:#x}");
    let config = Config {
        cases: 64,
        rng_seed: RngSeed::Fixed(SEED),
        failure_persistence: None,
        ..Config::default()
    };
    TestRunner::new(config)
}

/// One transaction: its ops and, for a coalescing one, its key.
type Transaction = (Vec<Recipe>, Option<u8>);

fn transactions() -> impl Strategy<Value = Vec<Transaction>> {
    proptest::collection::vec(
        (proptest::collection::vec(recipe(), 1..4), proptest::option::of(0u8..2)),
        0..10,
    )
}

fn check(make: fn() -> Document, transactions: &[Transaction]) -> Result<(), TestCaseError> {
    let mut doc = make();
    for (recipes, key) in transactions {
        let run = |tx: &mut lamina_ops::Tx<'_>| {
            for recipe in recipes {
                let op = to_op_in(tx.document(), recipe);
                tx.apply(op)?;
            }
            Ok(())
        };
        match key {
            Some(key) => doc.transact_coalescing(&key.to_string(), run),
            None => doc.transact(Origin::Local, run),
        }
        .unwrap();
    }
    let applied = snapshot(&doc);
    while let Some(reverted) = doc.undo().unwrap() {
        prop_assert_eq!(reverted.outcome, UndoOutcome::Applied);
    }
    prop_assert!(
        !doc.is_dirty(),
        "overlay not empty after undoing everything: {:?}",
        snapshot(&doc)
    );
    prop_assert!(!doc.has_unsaved_changes());
    while doc.redo().unwrap().is_some() {}
    prop_assert_eq!(snapshot(&doc), applied);
    Ok(())
}

#[test]
fn undo_all_empties_the_overlay_and_redo_all_restores_it() {
    for (name, make) in documents() {
        let mut runner = runner();
        let result = runner.run(&transactions(), |txs| check(make, &txs));
        assert!(result.is_ok(), "{name}: {result:?}");
    }
}

#[test]
fn every_op_alone_then_undone_leaves_the_overlay_empty() {
    for (name, make) in documents() {
        let mut runner = runner();
        let result = runner.run(&recipe(), |recipe| {
            let mut doc = make();
            let op = to_op(&doc, &recipe);
            doc.apply_all(Origin::Local, [op]).unwrap();
            doc.undo().unwrap();
            prop_assert!(!doc.is_dirty());
            Ok(())
        });
        assert!(result.is_ok(), "{name}: {result:?}");
    }
}
