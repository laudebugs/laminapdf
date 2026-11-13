//! Property tests (ADR 0018 tier 5, ADR 0006 §3–§4): random sequences of `SetInfo`,
//! `SetMetadata`, `RemoveMetadata`, `RotatePage` and the outline and named-destination ops,
//! grouped into local transactions (some
//! coalescing), over files with an indirect, a direct and no Info dictionary, with and without
//! an XMP packet, a PDF 2.0 file and inherited `/Rotate`. Undoing everything leaves the overlay
//! empty; redoing everything gives the same overlay as before the undo.
//!
//! The seed is fixed (`SEED`, also printed to stderr, which the test harness shows for a
//! failing test) so failures reproduce; proptest still shrinks.
#![expect(
    deprecated,
    reason = "these tests check that SetInfo, deprecated but still on the wire, keeps working"
)]
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{corpus, fixture, inherited_rotate, open, page_ids, snapshot};
use laminapdf_model::{OutlineTree, PdfDate, Rotation, SiblingPosition};
use laminapdf_ops::{
    AddOutlineItem, ChildItems, DeleteNamedDestination, DeleteOutlineItem, DestinationView, Document,
    ExplicitDestination, InfoPatch, MetadataKey, MetadataPatch, MoveOutlineItem, Op, Origin, OutlineStyle,
    OutlineTarget, RemoveMetadata, RenameNamedDestination, Rgb, RotatePage, SetInfo, SetMetadata, SetNamedDestination,
    UndoOutcome, UpdateOutlineItem,
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
    /// Add under the nth item (or the top level), at a position, to the nth page.
    AddItem(Option<usize>, u8, usize),
    /// Retitle, restyle and open or close the nth item.
    UpdateItem(usize, bool),
    /// Move the nth item under the mth (or the top level).
    MoveItem(usize, Option<usize>, u8),
    /// Delete the nth item, promoting its children or not.
    DeleteItem(usize, bool),
    /// Point the nth of `NAMES` at the nth page.
    SetName(usize, usize),
    /// Rename the nth existing name to the nth of `NAMES`.
    RenameName(usize, usize),
    /// Delete the nth existing name.
    DeleteName(usize),
}

const NAMES: [&str; 4] = ["chapter2", "appendix", "results", "Grüße"];

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
        (proptest::option::of(0usize..8), 0u8..4, 0usize..8).prop_map(|(p, a, g)| Recipe::AddItem(p, a, g)),
        (0usize..8, any::<bool>()).prop_map(|(i, o)| Recipe::UpdateItem(i, o)),
        (0usize..8, proptest::option::of(0usize..8), 0u8..4).prop_map(|(i, p, a)| Recipe::MoveItem(i, p, a)),
        (0usize..8, any::<bool>()).prop_map(|(i, p)| Recipe::DeleteItem(i, p)),
        (0usize..4, 0usize..8).prop_map(|(n, p)| Recipe::SetName(n, p)),
        (0usize..4, 0usize..4).prop_map(|(f, t)| Recipe::RenameName(f, t)),
        (0usize..4).prop_map(Recipe::DeleteName),
    ]
}

/// The names the document defines, in either store.
fn names(doc: &laminapdf_model::Document) -> Vec<String> {
    let mut out: Vec<String> = doc
        .name_tree("Dests")
        .unwrap()
        .map(|t| t.entries().unwrap())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(k, _)| String::from_utf8(k).ok())
        .collect();
    let dests = match doc.catalog().unwrap().get("Dests") {
        Some(laminapdf_cos::Object::Ref(r)) => doc.resolve(*r).unwrap().as_dict().cloned(),
        Some(laminapdf_cos::Object::Dict(d)) => Some(d.clone()),
        _ => None,
    };
    if let Some(d) = dests {
        out.extend(
            d.iter()
                .filter_map(|(k, _)| String::from_utf8(k.as_bytes().to_vec()).ok()),
        );
    }
    out
}

/// A position among `parent`'s children that exists.
fn position(tree: &OutlineTree, parent: Option<&laminapdf_model::OutlineItemId>, at: u8) -> SiblingPosition {
    let siblings: Vec<_> = tree
        .items()
        .into_iter()
        .filter(|i| tree.parent(i) == Some(parent))
        .collect();
    match (at, siblings.first()) {
        (0, _) => SiblingPosition::First,
        (2, Some(s)) => SiblingPosition::Before(s.clone()),
        (3, Some(s)) => SiblingPosition::After(s.clone()),
        _ => SiblingPosition::Last,
    }
}

/// An outline or name recipe as an op, or `None` when the document has nothing it applies to
/// (no items, a move into the item's own branch, a name that doesn't exist).
fn outline_op(doc: &laminapdf_model::Document, recipe: &Recipe) -> Option<Op> {
    let tree = OutlineTree::read(doc).unwrap();
    let items = tree.items();
    let pick = |i: usize| (!items.is_empty()).then(|| items[i % items.len()].clone());
    let ids = page_ids(doc);
    let dest =
        |p: usize| ExplicitDestination::new(ids[p % ids.len()].clone(), DestinationView::FitH { top: Some(700.0) });
    Some(match recipe {
        Recipe::AddItem(parent, at, p) => {
            let parent = parent.and_then(pick);
            let at = position(&tree, parent.as_ref(), *at);
            AddOutlineItem::new(parent, at, "Added", OutlineTarget::Destination(dest(*p))).into()
        }
        Recipe::UpdateItem(i, open) => {
            let mut op = UpdateOutlineItem::new(pick(*i)?);
            op.title = Some(if *open { "Opened" } else { "Closed" }.to_owned());
            op.style = Some(OutlineStyle::new(open.then(|| Rgb::new(0, 0, 200)), *open, false));
            op.open = Some(*open);
            op.into()
        }
        Recipe::MoveItem(i, parent, at) => {
            let item = pick(*i)?;
            let parent = parent.and_then(pick);
            if parent.as_ref().is_some_and(|p| tree.subtree(&item).contains(p)) {
                return None;
            }
            let at = position(&tree, parent.as_ref(), *at);
            if matches!(&at, SiblingPosition::Before(s) | SiblingPosition::After(s) if *s == item) {
                return None;
            }
            MoveOutlineItem::new(item, parent, at).into()
        }
        Recipe::DeleteItem(i, promote) => {
            let children = if *promote {
                ChildItems::Promote
            } else {
                ChildItems::Delete
            };
            DeleteOutlineItem::new(pick(*i)?, children).into()
        }
        Recipe::SetName(n, p) => SetNamedDestination::new(NAMES[n % NAMES.len()], dest(*p)).into(),
        Recipe::RenameName(f, t) => {
            let existing = names(doc);
            let from = existing.get(f % existing.len().max(1))?.clone();
            let to = NAMES[t % NAMES.len()];
            if existing.iter().any(|n| n == to) {
                return None;
            }
            RenameNamedDestination::new(from, to).into()
        }
        Recipe::DeleteName(n) => {
            let existing = names(doc);
            DeleteNamedDestination::new(existing.get(n % existing.len().max(1))?.clone()).into()
        }
        _ => return None,
    })
}

fn to_op(doc: &laminapdf_model::Document, recipe: &Recipe) -> Op {
    if let Some(op) = outline_op(doc, recipe) {
        return op;
    }
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
        // An outline recipe the document has nothing for: a rotation instead.
        _ => RotatePage::new(page_ids(doc).first().unwrap().clone(), Rotation::Half).into(),
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
