//! The page-tree editor (`Document::edit_page_tree`) and `PageLabels::remap`, on
//! `fixtures/pages/`: random insert, remove and move sequences against a flat model of the same
//! edits (page order, `/Count`s and `/Parent`s, effective attributes), saved and checked by
//! qpdf when it is installed. Seeds are fixed so failures reproduce.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use std::collections::BTreeSet;
use std::process::Command;

use laminapdf_cos::{Dict, ObjRef, Object, Warning};
use laminapdf_model::{
    Document, LabelRange, LabelStyle, Limits, OpenOptions, PageId, PageLabels, PagePosition, PageTreeEditor,
    PageTreeError, Rotation, UserSpaceRect,
};
use laminapdf_save::{FullWriter, SaveOptions};
use laminapdf_test_fixtures::fixture;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const SEED: u64 = 0x9a6e_7ee5;

/// What a page shows: the attributes inheritance decides. Empty resources count as none.
#[derive(Debug, Clone, PartialEq)]
struct Shown {
    rotate: Rotation,
    media: Option<UserSpaceRect>,
    crop: Option<UserSpaceRect>,
    resources: Option<Dict>,
}

fn open(name: &str) -> Document {
    Document::open(fixture(&format!("pages/{name}")), OpenOptions::default()).unwrap()
}

/// The pages in order with what each shows.
fn shown(doc: &Document) -> Vec<(PageId, Shown)> {
    let pages = doc.pages().unwrap();
    pages
        .iter()
        .map(|p| {
            let shown = Shown {
                rotate: p.rotate().unwrap(),
                media: p.media_box().unwrap(),
                crop: p.crop_box().unwrap(),
                resources: p.resources().filter(|r| !r.is_empty()).cloned(),
            };
            (p.id().clone(), shown)
        })
        .collect()
}

/// Walks the page tree from the catalog: every node's `/Count` is the pages under it, every
/// kid's `/Parent` is its node, no page is listed twice. Returns the deepest page's depth and
/// the widest node's kid count.
fn check_tree(doc: &Document) -> (usize, usize) {
    let root = doc.catalog().unwrap().get("Pages").and_then(Object::as_ref).unwrap();
    let mut pages = BTreeSet::new();
    let (mut depth, mut width) = (0, 0);
    let count = walk(doc, root, 1, &mut pages, &mut depth, &mut width);
    assert_eq!(count, pages.len());
    (depth, width)
}

fn walk(
    doc: &Document,
    node: ObjRef,
    depth: usize,
    pages: &mut BTreeSet<ObjRef>,
    deepest: &mut usize,
    widest: &mut usize,
) -> usize {
    let object = doc.resolve(node).unwrap();
    let dict = object.as_dict().unwrap();
    let kids = doc.deref(dict.get("Kids").unwrap()).unwrap();
    let kids = kids.as_array().unwrap();
    *widest = (*widest).max(kids.len());
    let mut count = 0;
    for kid in kids {
        let r = kid.as_ref().expect("kids are indirect");
        let kid_obj = doc.resolve(r).unwrap();
        let kid_dict = kid_obj.as_dict().unwrap();
        assert_eq!(kid_dict.get("Parent"), Some(&Object::Ref(node)), "/Parent of {r:?}");
        if kid_dict.get_name("Type") == Some(b"Pages") {
            count += walk(doc, r, depth + 1, pages, deepest, widest);
        } else {
            assert!(pages.insert(r), "page {r:?} listed twice");
            *deepest = (*deepest).max(depth);
            count += 1;
        }
    }
    assert_eq!(
        dict.get("Count").and_then(Object::as_int),
        Some(i64::try_from(count).unwrap()),
        "/Count of {node:?}"
    );
    count
}

/// Saves `doc` with the full writer and runs `qpdf --check` on the output, when qpdf is on
/// PATH.
fn qpdf_check(doc: &Document, label: &str) {
    let mut out = Vec::new();
    FullWriter::new(doc.cos(), SaveOptions::new()).write(&mut out).unwrap();
    let dir = std::env::temp_dir().join("laminapdf-model-page-tree");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{label}-{}.pdf", std::process::id()));
    std::fs::write(&path, &out).unwrap();
    match Command::new("qpdf").arg("--check").arg(&path).output() {
        Ok(result) => assert!(
            result.status.success(),
            "qpdf --check {label}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        ),
        Err(_) => eprintln!("qpdf not on PATH: skipping qpdf --check of {label}"),
    }
    let _ = std::fs::remove_file(path);
}

/// A new page's dictionary: its own media box, so its rotation and crop box are the defaults.
fn new_page(n: u32) -> Dict {
    let media = Object::Array(vec![
        0.into(),
        0.into(),
        300.into(),
        Object::Integer(400 + i64::from(n)),
    ]);
    [("Type", Object::name("Page")), ("MediaBox", media)]
        .into_iter()
        .collect()
}

fn new_shown(n: u32) -> Shown {
    let media = UserSpaceRect::from_corners(0.0, 0.0, 300.0, 400.0 + f64::from(n));
    Shown {
        rotate: Rotation::None,
        media: Some(media),
        crop: Some(media),
        resources: None,
    }
}

#[derive(Debug, Clone)]
enum Edit {
    Insert(Pos),
    Remove(usize),
    Move { first: usize, len: usize, to: Pos },
}

#[derive(Debug, Clone, Copy)]
enum Pos {
    Start,
    End,
    Before(usize),
    After(usize),
}

fn pos() -> impl Strategy<Value = Pos> {
    prop_oneof![
        Just(Pos::Start),
        Just(Pos::End),
        any::<usize>().prop_map(Pos::Before),
        any::<usize>().prop_map(Pos::After),
    ]
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        pos().prop_map(Edit::Insert),
        any::<usize>().prop_map(Edit::Remove),
        (any::<usize>(), 1usize..4, pos()).prop_map(|(first, len, to)| Edit::Move { first, len, to }),
    ]
}

fn position(model: &[(PageId, Shown)], pos: Pos) -> PagePosition {
    let pick = |i: usize| model[i % model.len()].0.clone();
    match pos {
        _ if model.is_empty() => PagePosition::End,
        Pos::Start => PagePosition::Start,
        Pos::End => PagePosition::End,
        Pos::Before(i) => PagePosition::Before(pick(i)),
        Pos::After(i) => PagePosition::After(pick(i)),
    }
}

/// Where `at` puts pages in `model` (the moved pages already out of it).
fn model_index(model: &[(PageId, Shown)], at: &PagePosition) -> usize {
    match at {
        PagePosition::Start => 0,
        PagePosition::Before(id) => model.iter().position(|(p, _)| p == id).unwrap(),
        PagePosition::After(id) => model.iter().position(|(p, _)| p == id).unwrap() + 1,
        _ => model.len(),
    }
}

/// Applies `edit` to the editor and to the flat model.
fn apply(editor: &mut PageTreeEditor<'_>, model: &mut Vec<(PageId, Shown)>, edit: &Edit, made: &mut u32) {
    match *edit {
        Edit::Insert(pos) => {
            let at = position(model, pos);
            *made += 1;
            let id = editor.insert_new(new_page(*made), &at).unwrap();
            let index = model_index(model, &at);
            model.insert(index, (id, new_shown(*made)));
        }
        Edit::Remove(i) if model.len() > 1 => {
            let (id, _) = model.remove(i % model.len());
            editor.remove(&id).unwrap();
        }
        Edit::Move { first, len, to } if !model.is_empty() => {
            let first = first % model.len();
            let len = len.min(model.len() - first);
            let moving: Vec<(PageId, Shown)> = model[first..first + len].to_vec();
            let ids: Vec<PageId> = moving.iter().map(|(id, _)| id.clone()).collect();
            let at = position(model, to);
            editor.move_pages(&ids, &at).unwrap();
            model.drain(first..first + len);
            // An anchor that moves gives way to the nearest page that stays.
            let at = match &at {
                PagePosition::Before(a) if ids.contains(a) => model
                    .get(first)
                    .map_or(PagePosition::End, |(p, _)| PagePosition::Before(p.clone())),
                PagePosition::After(a) if ids.contains(a) => first
                    .checked_sub(1)
                    .and_then(|i| model.get(i))
                    .map_or(PagePosition::Start, |(p, _)| PagePosition::After(p.clone())),
                other => other.clone(),
            };
            let index = model_index(model, &at);
            model.splice(index..index, moving);
        }
        _ => {}
    }
}

/// Runs `edits` on `doc`, finishing the editor where `breaks` says, and compares the result
/// with the flat model.
fn check_edits(mut doc: Document, edits: &[(Edit, bool)], label: &str) -> Result<(), TestCaseError> {
    let mut model = shown(&doc);
    let mut made = 0;
    let mut editor = doc.edit_page_tree().unwrap();
    for (edit, finish) in edits {
        apply(&mut editor, &mut model, edit, &mut made);
        prop_assert_eq!(
            editor.page_ids(),
            model.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>()
        );
        if *finish {
            editor.finish().unwrap();
            editor = doc.edit_page_tree().unwrap();
        }
    }
    editor.finish().unwrap();
    prop_assert_eq!(shown(&doc), model);
    let (depth, width) = check_tree(&doc);
    prop_assert!(depth <= PageTreeEditor::MAX_DEPTH, "depth {}", depth);
    prop_assert!(width <= PageTreeEditor::MAX_KIDS, "width {}", width);
    qpdf_check(&doc, label);
    Ok(())
}

/// `unbalanced.pdf` with its page listed twice copied, as any first write does.
fn repaired_unbalanced() -> Document {
    let mut doc = open("unbalanced.pdf");
    doc.edit_page_tree().unwrap().finish().unwrap();
    doc
}

#[test]
fn random_edits_match_a_flat_model() {
    type Make = fn() -> Document;
    let documents: [(&str, Make); 2] = [
        ("inherited", || open("inherited.pdf")),
        ("unbalanced", repaired_unbalanced),
    ];
    for (name, make) in documents {
        eprintln!("proptest seed: {SEED:#x}");
        let mut runner = TestRunner::new(Config {
            cases: 48,
            rng_seed: RngSeed::Fixed(SEED),
            failure_persistence: None,
            ..Config::default()
        });
        let edits = proptest::collection::vec((edit(), proptest::bool::weighted(0.2)), 0..12);
        let result = runner.run(&edits, |edits| check_edits(make(), &edits, name));
        assert!(result.is_ok(), "{name}: {result:?}");
    }
}

#[test]
fn a_page_listed_twice_should_be_copied_with_a_warning() {
    let mut doc = open("unbalanced.pdf");
    let editor = doc.edit_page_tree().unwrap();
    let ids = editor.page_ids();
    assert_eq!(ids.len(), 12);
    assert_eq!(ids[2], ids[11], "page 3's dictionary is listed again last");
    editor.finish().unwrap();
    let after = shown(&doc);
    assert_eq!(after.len(), 12);
    assert_ne!(after[11].0, after[2].0, "the second listing is a copy");
    assert_eq!(after[11].1, after[2].1, "the copy shows what the page shows");
    assert!(
        doc.warnings()
            .iter()
            .any(|w| matches!(w, Warning::InvalidEntry { key: "Kids", .. }))
    );
    check_tree(&doc);
}

#[test]
fn a_deep_chain_should_be_rebuilt_keeping_what_each_page_shows() {
    let mut doc = open("unbalanced.pdf");
    let mut editor = doc.edit_page_tree().unwrap();
    // Page 1 goes; the chain's pages (7 to 11) show Rotate 90 from level 10 down and a
    // 420 x 595 media box from level 20 down; page 12 is page 3 listed again.
    let ids = editor.page_ids();
    editor.remove(&ids[0]).unwrap();
    editor.finish().unwrap();
    let after = shown(&doc);
    let rotations: Vec<Rotation> = after.iter().map(|(_, s)| s.rotate).collect();
    let (r, o) = (Rotation::Clockwise90, Rotation::None);
    assert_eq!(rotations, [o, o, o, o, o, o, r, r, r, r, o]);
    let a5 = UserSpaceRect::from_corners(0.0, 0.0, 420.0, 595.0);
    let letter = UserSpaceRect::from_corners(0.0, 0.0, 612.0, 792.0);
    let media: Vec<Option<UserSpaceRect>> = after.iter().map(|(_, s)| s.media).collect();
    assert_eq!(media[6], Some(letter), "page 8, level 15");
    assert_eq!(media[7], Some(a5), "page 9, level 25");
    let (depth, _) = check_tree(&doc);
    assert!(depth <= 2, "rebuilt with depth {depth}");
}

#[test]
fn a_small_move_should_keep_the_tree_shape() {
    let mut doc = open("inherited.pdf");
    let ids: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    let before = shown(&doc);
    let mut editor = doc.edit_page_tree().unwrap();
    editor
        .move_pages(&ids[1..2], &PagePosition::After(ids[6].clone()))
        .unwrap();
    editor.finish().unwrap();
    // The moved page, its old node A and the root changed; nothing else.
    let changed: BTreeSet<u32> = doc.overlay().iter().map(|(num, _)| num).collect();
    let page = ids[1].as_lamina_id().obj_ref().unwrap().num;
    let root = doc
        .catalog()
        .unwrap()
        .get("Pages")
        .and_then(Object::as_ref)
        .unwrap()
        .num;
    let a = doc.resolve(ObjRef::new(page, 0)).unwrap();
    assert_eq!(
        a.as_dict().unwrap().get("Parent"),
        Some(&Object::Ref(ObjRef::new(root, 0)))
    );
    assert_eq!(changed.len(), 3, "{changed:?}");
    assert!(changed.contains(&page) && changed.contains(&root));
    let mut expected = before;
    let moved = expected.remove(1);
    expected.insert(6, moved);
    assert_eq!(shown(&doc), expected);
}

#[test]
fn a_node_past_the_fan_out_threshold_should_get_the_tree_rebuilt() {
    let mut doc = open("inherited.pdf");
    let mut editor = doc.edit_page_tree().unwrap();
    let mut model = Vec::new();
    for n in 0..u32::try_from(PageTreeEditor::MAX_KIDS).unwrap() + 10 {
        editor.insert_new(new_page(n), &PagePosition::End).unwrap();
        model.push(new_shown(n));
    }
    editor.finish().unwrap();
    let (depth, width) = check_tree(&doc);
    assert!(width <= PageTreeEditor::FAN_OUT, "width {width}");
    assert!(depth <= 3, "depth {depth}");
    let after = shown(&doc);
    assert_eq!(after.len(), 8 + model.len());
    assert_eq!(after[8..].iter().map(|(_, s)| s.clone()).collect::<Vec<_>>(), model);
}

#[test]
fn inserting_an_object_that_is_not_a_page_should_fail() {
    let mut doc = open("inherited.pdf");
    let font = doc
        .pages()
        .unwrap()
        .get(0)
        .unwrap()
        .resources()
        .unwrap()
        .get("Font")
        .unwrap()
        .as_dict()
        .unwrap()
        .get("F1")
        .and_then(Object::as_ref)
        .unwrap();
    let catalog = doc.trailer().get("Root").and_then(Object::as_ref).unwrap();
    let mut editor = doc.edit_page_tree().unwrap();
    for r in [font, catalog] {
        assert!(
            matches!(editor.insert(r, &PagePosition::End), Err(PageTreeError::NotAPage(_))),
            "{r:?}"
        );
    }
}

#[test]
fn an_editor_dropped_without_finish_should_write_nothing() {
    let mut doc = open("inherited.pdf");
    let mut editor = doc.edit_page_tree().unwrap();
    editor.insert_new(new_page(1), &PagePosition::Start).unwrap();
    drop(editor);
    assert!(doc.overlay().is_empty());
    assert_eq!(doc.pages().unwrap().len(), 8);
}

#[test]
fn the_copy_of_a_page_listed_twice_should_share_no_annotations_or_structure() {
    let mut doc = open("unbalanced.pdf");
    doc.edit_page_tree().unwrap().finish().unwrap();
    let pages = doc.pages().unwrap();
    let (original, copy) = (pages.get(2).unwrap(), pages.get(11).unwrap());
    assert!(original.dict().contains_key("Annots") && original.dict().contains_key("StructParents"));
    assert!(!copy.dict().contains_key("Annots"), "the copy lists no annotation");
    assert!(!copy.dict().contains_key("StructParents"));
    assert_eq!(copy.dict().get("Contents"), original.dict().get("Contents"));
}

#[test]
fn a_subtree_past_the_depth_limit_should_keep_its_stored_count() {
    let options = OpenOptions::default().with_limits(Limits::default().with_max_page_tree_depth(1));
    let mut doc = Document::open(fixture("pages/inherited.pdf"), options).unwrap();
    let root = doc.catalog().unwrap().get("Pages").and_then(Object::as_ref).unwrap();
    let count = |doc: &Document, r: ObjRef| {
        doc.resolve(r)
            .unwrap()
            .as_dict()
            .unwrap()
            .get("Count")
            .and_then(Object::as_int)
    };
    let mut editor = doc.edit_page_tree().unwrap();
    // Page 7, a kid of the root, is the only page the editor reads; the nodes below it are
    // past the limit.
    let ids = editor.page_ids();
    assert_eq!(ids.len(), 1);
    editor.move_pages(&ids, &PagePosition::Start).unwrap();
    editor.finish().unwrap();
    assert_eq!(count(&doc, root), Some(8));
    let first_kid = doc
        .resolve(root)
        .unwrap()
        .as_dict()
        .unwrap()
        .get("Kids")
        .unwrap()
        .as_array()
        .unwrap()[0]
        .clone();
    assert_eq!(first_kid, Object::Ref(ids[0].as_lamina_id().obj_ref().unwrap()));
}

#[test]
fn a_refused_finish_should_leave_the_document_unchanged() {
    // Strict mode refuses the page listed twice before anything is reserved or written.
    let mut doc = Document::open(
        fixture("pages/unbalanced.pdf"),
        OpenOptions::default().with_strict(true),
    )
    .unwrap();
    let next = doc.next_number();
    let mut editor = doc.edit_page_tree().unwrap();
    editor.move_pages(&editor.page_ids()[..1], &PagePosition::End).unwrap();
    assert!(editor.finish().is_err());
    assert!(doc.overlay().is_empty());
    assert_eq!(doc.next_number(), next, "no number reserved");
}

#[test]
fn inserting_a_page_already_in_the_tree_should_fail() {
    let mut doc = open("inherited.pdf");
    let first = doc.pages().unwrap().get(0).unwrap().obj_ref().unwrap();
    let mut editor = doc.edit_page_tree().unwrap();
    assert!(editor.insert(first, &PagePosition::End).is_err());
}

#[test]
fn a_removed_page_inserted_again_should_show_what_it_showed() {
    let mut doc = open("inherited.pdf");
    let before = shown(&doc);
    let mut editor = doc.edit_page_tree().unwrap();
    let id = before[0].0.clone();
    let r = editor.remove(&id).unwrap();
    editor.insert(r, &PagePosition::End).unwrap();
    editor.finish().unwrap();
    let after = shown(&doc);
    assert_eq!(after.last(), before.first());
}

/// Random moves, deletes and inserts of `labels.pdf`'s page list: every surviving page keeps
/// its label (but the first range's, renumbered when new pages come before it), every new
/// page gets one, and the labels read back the same once written.
#[test]
fn remapped_labels_keep_every_surviving_page_label() {
    let mut doc = open("labels.pdf");
    let labels = doc.page_labels().unwrap();
    let before: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    let mut runner = TestRunner::new(Config {
        cases: 128,
        rng_seed: RngSeed::Fixed(SEED),
        failure_persistence: None,
        ..Config::default()
    });
    let edits = proptest::collection::vec(edit(), 0..16);
    let result = runner.run(&edits, |edits| {
        let mut after = before.clone();
        let mut fresh = 0u32;
        for edit in &edits {
            match *edit {
                Edit::Insert(Pos::Before(i) | Pos::After(i)) => {
                    fresh += 1;
                    after.insert(i % (after.len() + 1), PageId::object(ObjRef::new(10_000 + fresh, 0)));
                }
                Edit::Insert(Pos::Start) => {
                    fresh += 1;
                    after.insert(0, PageId::object(ObjRef::new(10_000 + fresh, 0)));
                }
                Edit::Insert(Pos::End) => {
                    fresh += 1;
                    after.push(PageId::object(ObjRef::new(10_000 + fresh, 0)));
                }
                Edit::Remove(i) if after.len() > 1 => {
                    after.remove(i % after.len());
                }
                Edit::Move {
                    first,
                    len,
                    to: Pos::Before(t) | Pos::After(t),
                } => {
                    let first = first % after.len();
                    let len = len.min(after.len() - first);
                    let moving: Vec<PageId> = after.drain(first..first + len).collect();
                    let at = t % (after.len() + 1);
                    after.splice(at..at, moving);
                }
                _ => {}
            }
        }
        let remapped = labels.remap(&before, &after);
        // New pages ahead of every old page join the range of the first old page, which may
        // number its own pages up; their labels stay apart from the new pages'.
        let leading = after.iter().take_while(|id| !before.contains(id)).count();
        let range_of = |old: usize| {
            labels
                .ranges()
                .iter()
                .rposition(|r| u64::try_from(old).is_ok_and(|o| r.start <= o))
        };
        let first_range = after
            .get(leading)
            .and_then(|id| before.iter().position(|b| b == id))
            .and_then(range_of)
            .filter(|_| leading > 0);
        let lead_labels: Vec<Option<String>> = (0..leading).map(|i| remapped.label(i)).collect();
        for (new_index, id) in after.iter().enumerate() {
            match before.iter().position(|b| b == id) {
                Some(old) if first_range.is_some() && range_of(old) == first_range => prop_assert!(
                    remapped.label(new_index).is_some()
                        && (new_index != leading || !lead_labels.contains(&remapped.label(new_index))),
                    "{} shares a label with a new page: {:?}",
                    id,
                    remapped.label(new_index)
                ),
                Some(old) => prop_assert_eq!(remapped.label(new_index), labels.label(old), "{}", id),
                None => prop_assert!(remapped.label(new_index).is_some(), "new page {} has a label", id),
            }
        }
        Ok(())
    });
    assert!(result.is_ok(), "{result:?}");
    // Written and read back.
    let mut moved = before.clone();
    moved.rotate_left(4);
    let remapped = labels.remap(&before, &moved);
    remapped.write(&mut doc.cos_mut().edit()).unwrap();
    assert_eq!(doc.page_labels().unwrap(), remapped);
}

#[test]
fn a_new_page_continues_the_range_it_lands_in() {
    let doc = open("labels.pdf");
    let labels = doc.page_labels().unwrap();
    let before: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    let mut after = before.clone();
    after.insert(5, PageId::object(ObjRef::new(9_999, 0)));
    let remapped = labels.remap(&before, &after);
    let got: Vec<String> = (0..after.len()).map(|i| remapped.label(i).unwrap()).collect();
    // The new page continues "2"; the pages after it keep their own labels.
    assert_eq!(
        got,
        ["i", "ii", "iii", "1", "2", "3", "3", "4", "A-1", "A-2", "Back cover"]
    );
}

/// The labels of `labels`' pages after `count` new pages go first.
fn labels_after_leading_pages(labels: &PageLabels, pages: usize, count: u32) -> Vec<String> {
    let before: Vec<PageId> = (1..=pages)
        .map(|n| PageId::object(ObjRef::new(u32::try_from(n).unwrap(), 0)))
        .collect();
    let mut after: Vec<PageId> = (0..count).map(|n| PageId::object(ObjRef::new(9_000 + n, 0))).collect();
    after.extend(before.iter().cloned());
    let remapped = labels.remap(&before, &after);
    (0..after.len()).map(|i| remapped.label(i).unwrap()).collect()
}

#[test]
fn a_page_before_i_starts_the_front_matter_and_numbers_it_up() {
    let doc = open("labels.pdf");
    let labels = doc.page_labels().unwrap();
    let got = labels_after_leading_pages(&labels, doc.pages().unwrap().len(), 1);
    // The new page is "i", the front matter goes up by one, the body and appendix stay.
    assert_eq!(
        got,
        ["i", "ii", "iii", "iv", "1", "2", "3", "4", "A-1", "A-2", "Back cover"]
    );
}

#[test]
fn a_page_before_1_takes_1_and_later_ranges_keep_their_labels() {
    // Paper's O11 contents page: before page 1, it takes 1; the body goes up by one and the
    // appendix keeps A-1 to A-3.
    let labels = PageLabels::from_ranges(vec![
        LabelRange::new(0, Some(LabelStyle::Decimal), "", 1),
        LabelRange::new(3, Some(LabelStyle::Decimal), "A-", 1),
    ]);
    assert_eq!(
        labels_after_leading_pages(&labels, 6, 1),
        ["1", "2", "3", "4", "A-1", "A-2", "A-3"]
    );
}

#[test]
fn new_pages_take_the_numbers_below_a_range_that_has_them() {
    let labels = PageLabels::from_ranges(vec![LabelRange::new(0, Some(LabelStyle::Decimal), "", 5)]);
    assert_eq!(labels_after_leading_pages(&labels, 2, 1), ["4", "5", "6"]);
}

#[test]
fn new_pages_short_of_numbers_below_a_range_number_it_up_by_the_shortfall() {
    let labels = PageLabels::from_ranges(vec![LabelRange::new(0, Some(LabelStyle::Decimal), "", 2)]);
    assert_eq!(labels_after_leading_pages(&labels, 2, 3), ["1", "2", "3", "4", "5"]);
}

#[test]
fn new_pages_before_a_prefix_only_range_repeat_its_prefix() {
    let labels = PageLabels::from_ranges(vec![
        LabelRange::new(0, None, "Cover", 1),
        LabelRange::new(1, Some(LabelStyle::Decimal), "", 1),
    ]);
    assert_eq!(labels_after_leading_pages(&labels, 2, 1), ["Cover", "Cover", "1"]);
}

#[test]
fn removing_every_label_range_removes_page_labels() {
    let mut doc = open("labels.pdf");
    PageLabels::default().write(&mut doc.cos_mut().edit()).unwrap();
    assert!(doc.catalog().unwrap().get("PageLabels").is_none());
}
