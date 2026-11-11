//! Property tests (ADR 0018 tier 5, ADR 0006 §3–§4): random sequences of `SetInfo`,
//! `SetMetadata`, `RemoveMetadata` and `RotatePage`, grouped into local transactions (some
//! coalescing), over files with an indirect, a direct and no Info dictionary, with and without
//! an XMP packet, a PDF 2.0 file and inherited `/Rotate`. Undoing everything leaves the overlay
//! empty; redoing everything gives the same overlay as before the undo.
//!
//! The seed is fixed (`SEED`, also printed to stderr, which the test harness shows for a
//! failing test) so failures reproduce; proptest still shrinks.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{corpus, fixture, inherited_rotate, open, page_ids, snapshot};
use laminapdf_model::{PdfDate, Rotation};
use laminapdf_ops::{
    Document, InfoPatch, MetadataKey, MetadataPatch, Op, Origin, RemoveMetadata, RotatePage, SetInfo, SetMetadata,
    UndoOutcome,
};
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const SEED: u64 = 0x1a31_0b3a;

/// A recipe for an op, resolved against a document's pages when applied.
#[derive(Debug, Clone)]
#[expect(clippy::option_option, reason = "InfoPatch's fields: leave, clear or set")]
enum Recipe {
    Info([Option<Option<&'static str>>; 3], Option<u8>),
    Metadata([Option<Option<&'static str>>; 3], Option<u8>),
    Remove(Vec<usize>),
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
        (
            [text_field(), text_field(), text_field()],
            proptest::option::of(0u8..30)
        )
            .prop_map(|(f, d)| Recipe::Metadata(f, d)),
        proptest::collection::vec(0usize..8, 1..3).prop_map(Recipe::Remove),
        (0usize..8, rotation).prop_map(|(p, r)| Recipe::Rotate(p, r)),
    ]
}

fn to_op(doc: &laminapdf_model::Document, recipe: &Recipe) -> Op {
    match recipe {
        Recipe::Info([title, author, keywords], day) => {
            let mut patch = InfoPatch::new();
            patch.title = title.map(|t| t.map(str::to_owned));
            patch.author = author.map(|t| t.map(str::to_owned));
            patch.keywords = keywords.map(|t| t.map(str::to_owned));
            SetInfo::new(patch, day.map(date)).into()
        }
        Recipe::Metadata([title, author, keywords], day) => {
            let mut patch = MetadataPatch::new();
            patch.title = title.map(|t| t.map(str::to_owned));
            patch.authors = author.map(|t| t.map(|a| vec![a.to_owned(), "Second".to_owned()]));
            patch.keywords = keywords.map(|t| t.map(str::to_owned));
            SetMetadata::new(patch, day.map(date)).into()
        }
        Recipe::Remove(keys) => RemoveMetadata::new(
            keys.iter()
                .filter_map(|k| MetadataKey::ALL.get(k % 8).copied())
                .collect(),
        )
        .into(),
        Recipe::Rotate(index, rotation) => {
            let ids = page_ids(doc);
            let page = ids.get(index % ids.len()).unwrap().clone();
            RotatePage::new(page, *rotation).into()
        }
    }
}

fn date(day: u8) -> PdfDate {
    format!("D:202610{:02}120000Z", day % 28 + 1).parse().unwrap()
}

/// A named way to open a test document.
type Make = (&'static str, fn() -> Document);

fn documents() -> Vec<Make> {
    vec![
        ("inherited-rotate (indirect Info)", inherited_rotate),
        ("hello.pdf (no Info)", || corpus("hello.pdf")),
        ("text-example.pdf (direct Info)", || corpus("text-example.pdf")),
        ("outline.pdf", || corpus("outline.pdf")),
        ("metadata.pdf (Info and XMP)", || open(&fixture("golden/metadata.pdf"))),
        ("pdf20-xmp-only.pdf", || open(&fixture("metadata/pdf20-xmp-only.pdf"))),
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
        let run = |tx: &mut laminapdf_ops::Tx<'_>| {
            for recipe in recipes {
                let op = to_op(tx.document(), recipe);
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
