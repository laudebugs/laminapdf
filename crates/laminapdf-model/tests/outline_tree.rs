//! The outline's tree editor (`OutlineTree`): inserts, moves, removals and open states keep
//! every list whole (`/First`, `/Last`, `/Prev`, `/Next`, `/Parent`) and every `/Count` right
//! (ISO 32000-2 §12.3.3), checked by walking the written dictionaries; a tree read with broken
//! links is rewritten whole and reported; random edits keep the invariants. Seeds are fixed.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{Document, OpenOptions, OutlineItemId, OutlineTree, OutlineTreeError, SiblingPosition};
use laminapdf_save::test_support::Writer;
use laminapdf_test_fixtures::fixture;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

fn outline_pdf() -> Document {
    Document::open(fixture("corpus/outline.pdf"), OpenOptions::default()).unwrap()
}

/// An outline tree by title, as the reader shows it: `(title, open, children)`.
#[derive(Debug, Clone, PartialEq)]
struct Shape(String, bool, Vec<Shape>);

fn shape(doc: &Document) -> Vec<Shape> {
    fn of(item: &laminapdf_model::OutlineItem<'_, '_>) -> Shape {
        Shape(
            item.title().unwrap(),
            item.is_open().unwrap(),
            item.children().map(|c| of(&c)).collect(),
        )
    }
    doc.outline().unwrap().roots().map(|i| of(&i)).collect()
}

fn dict(doc: &Document, r: ObjRef) -> Dict {
    doc.resolve(r).unwrap().as_dict().unwrap().clone()
}

fn get_ref(d: &Dict, key: &str) -> Option<ObjRef> {
    d.get(key).and_then(Object::as_ref)
}

/// Walks the stored links from `/Outlines` and fails on any that the spec's rules break;
/// returns the number of items reached.
fn check_links(doc: &Document) -> usize {
    let catalog = doc.catalog().unwrap();
    let Some(root) = catalog.get("Outlines").and_then(Object::as_ref) else {
        return 0;
    };
    /// Checks the children of `parent`; returns how many of its descendants show.
    fn level(doc: &Document, parent: ObjRef, reached: &mut usize) -> i64 {
        let p = dict(doc, parent);
        let mut prev: Option<ObjRef> = None;
        let mut at = get_ref(&p, "First");
        let mut shown = 0;
        while let Some(r) = at {
            *reached += 1;
            assert!(*reached < 10_000, "a cycle");
            let d = dict(doc, r);
            assert_eq!(get_ref(&d, "Parent"), Some(parent), "{r:?}'s /Parent");
            assert_eq!(get_ref(&d, "Prev"), prev, "{r:?}'s /Prev");
            let under = level(doc, r, reached);
            let count = d.get("Count").and_then(Object::as_int);
            if get_ref(&d, "First").is_none() {
                assert!(matches!(count, None | Some(0)), "{r:?}: a leaf with a /Count");
            } else {
                let c = count.unwrap();
                assert_eq!(c.abs(), under, "{r:?}'s /Count");
                if c > 0 {
                    shown += c;
                }
            }
            shown += 1;
            prev = Some(r);
            at = get_ref(&d, "Next");
        }
        assert_eq!(get_ref(&p, "Last"), prev, "{parent:?}'s /Last");
        shown
    }
    let mut reached = 0;
    let shown = level(doc, root, &mut reached);
    let count = dict(doc, root).get("Count").and_then(Object::as_int);
    assert_eq!(count.unwrap_or(0), shown, "/Outlines /Count");
    reached
}

fn item(doc: &mut Document, title: &str) -> ObjRef {
    let d: Dict = [("Title", Object::String(PdfString::literal(title)))]
        .into_iter()
        .collect();
    doc.cos_mut().edit().create(Object::Dict(d)).unwrap()
}

fn id_of(doc: &Document, title: &str) -> OutlineItemId {
    let outline = doc.outline().unwrap();
    outline
        .iter()
        .find(|i| i.title().unwrap() == title)
        .and_then(|i| i.id().cloned())
        .unwrap()
}

fn s(title: &str, open: bool, children: Vec<Shape>) -> Shape {
    Shape(title.to_owned(), open, children)
}

#[test]
fn reading_and_writing_an_unchanged_outline_changes_nothing() {
    let mut doc = outline_pdf();
    let mut tree = OutlineTree::read(&doc).unwrap();
    assert!(!tree.repaired());
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    assert!(!doc.is_dirty());
}

#[test]
fn an_item_goes_where_its_position_says() {
    let mut doc = outline_pdf();
    let chapter = shape_ids(&doc)[1].clone();
    let mut tree = OutlineTree::read(&doc).unwrap();
    let a = item(&mut doc, "First");
    tree.insert(a, None, &SiblingPosition::First).unwrap();
    let b = item(&mut doc, "Inside, last");
    tree.insert(b, Some(&chapter), &SiblingPosition::Last).unwrap();
    let c = item(&mut doc, "Before the chapter");
    tree.insert(c, None, &SiblingPosition::Before(chapter.clone())).unwrap();
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    assert_eq!(check_links(&doc), 9);
    let titles: Vec<String> = shape(&doc).into_iter().map(|s| s.0).collect();
    assert_eq!(titles[0], "First");
    assert_eq!(titles[2], "Before the chapter");
    let chapter_children = &shape(&doc)[3].2;
    assert_eq!(chapter_children.last().unwrap().0, "Inside, last");
}

#[test]
fn a_moved_item_keeps_its_children_and_its_id() {
    let mut doc = outline_pdf();
    let appendix = shape_ids(&doc)[2].clone();
    let mut tree = OutlineTree::read(&doc).unwrap();
    tree.move_item(&appendix, None, &SiblingPosition::First).unwrap();
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    check_links(&doc);
    let top = shape(&doc);
    assert!(top[0].0.starts_with("Appendix"));
    assert_eq!(top[0].2.len(), 1);
    assert_eq!(shape_ids(&doc)[0], appendix);
}

#[test]
fn an_item_cannot_move_inside_itself() {
    let doc = outline_pdf();
    let appendix = shape_ids(&doc)[2].clone();
    let child = doc
        .outline()
        .unwrap()
        .by_id(&appendix)
        .unwrap()
        .children()
        .next()
        .unwrap()
        .id()
        .cloned()
        .unwrap();
    let mut tree = OutlineTree::read(&doc).unwrap();
    assert_eq!(
        tree.move_item(&appendix, Some(&child), &SiblingPosition::Last),
        Err(OutlineTreeError::IntoItself(appendix.clone()))
    );
    assert_eq!(
        tree.move_item(&appendix, Some(&appendix), &SiblingPosition::Last),
        Err(OutlineTreeError::IntoItself(appendix))
    );
}

#[test]
fn a_position_must_name_a_child_of_the_parent() {
    let doc = outline_pdf();
    let appendix = shape_ids(&doc)[2].clone();
    let intro = id_of(&doc, "Introduction");
    let mut tree = OutlineTree::read(&doc).unwrap();
    assert_eq!(
        tree.move_item(&intro, Some(&appendix), &SiblingPosition::Before(intro.clone())),
        Err(OutlineTreeError::IntoItself(intro.clone()))
    );
    let chapter_child = doc
        .outline()
        .unwrap()
        .iter()
        .find(|i| i.depth() == 2)
        .and_then(|i| i.id().cloned())
        .unwrap();
    assert_eq!(
        tree.move_item(&intro, None, &SiblingPosition::After(chapter_child.clone())),
        Err(OutlineTreeError::NotASibling(chapter_child))
    );
}

#[test]
fn removing_an_item_takes_its_children_or_promotes_them() {
    for promote in [false, true] {
        let mut doc = outline_pdf();
        let appendix = shape_ids(&doc)[2].clone();
        let mut tree = OutlineTree::read(&doc).unwrap();
        let removed = tree.remove(&appendix, promote).unwrap();
        assert_eq!(removed.len(), if promote { 1 } else { 2 });
        tree.write(&mut doc.cos_mut().edit()).unwrap();
        let reached = check_links(&doc);
        assert_eq!(reached, if promote { 5 } else { 4 });
    }
}

#[test]
fn removing_every_item_leaves_an_empty_outlines_dictionary() {
    let mut doc = outline_pdf();
    let mut tree = OutlineTree::read(&doc).unwrap();
    for id in shape_ids(&doc) {
        tree.remove(&id, false).unwrap();
    }
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    assert_eq!(check_links(&doc), 0);
    assert!(doc.outline().unwrap().is_empty());
}

fn shape_ids(doc: &Document) -> Vec<OutlineItemId> {
    doc.outline().unwrap().roots().filter_map(|i| i.id().cloned()).collect()
}

#[test]
fn opening_and_closing_an_item_sets_the_sign_of_its_count_and_its_ancestors_counts() {
    let mut doc = outline_pdf();
    let appendix = shape_ids(&doc)[2].clone();
    let mut tree = OutlineTree::read(&doc).unwrap();
    tree.set_open(&appendix, true).unwrap();
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    check_links(&doc);
    assert!(doc.outline().unwrap().by_id(&appendix).unwrap().is_open().unwrap());
    let mut tree = OutlineTree::read(&doc).unwrap();
    tree.set_open(&appendix, false).unwrap();
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    check_links(&doc);
    assert_eq!(
        doc.outline().unwrap().by_id(&appendix).unwrap().count().unwrap(),
        Some(-1)
    );
}

#[test]
fn a_first_item_creates_the_outlines_dictionary() {
    let mut w = Writer::new(1, 7);
    let pages = w.add(
        [
            ("Type", Object::name("Pages")),
            ("Kids", Vec::<Object>::new().into()),
            ("Count", 0.into()),
        ]
        .into_iter()
        .collect::<Dict>(),
    );
    let root = w.add(
        [("Type", Object::name("Catalog")), ("Pages", pages.into())]
            .into_iter()
            .collect::<Dict>(),
    );
    let mut doc = Document::open(
        w.finish(&[("Root", root)].into_iter().collect()).unwrap(),
        OpenOptions::default(),
    )
    .unwrap();
    let mut tree = OutlineTree::read(&doc).unwrap();
    let a = item(&mut doc, "Only");
    let b = item(&mut doc, "Child");
    let a = tree.insert(a, None, &SiblingPosition::Last).unwrap();
    tree.insert(b, Some(&a), &SiblingPosition::Last).unwrap();
    tree.set_open(&a, true).unwrap();
    tree.write(&mut doc.cos_mut().edit()).unwrap();
    assert_eq!(check_links(&doc), 2);
    assert_eq!(shape(&doc), vec![s("Only", true, vec![s("Child", false, vec![])])]);
}

/// A file whose outline lists `A → B → C`, with links broken by `breakage`.
fn broken(breakage: &str) -> Document {
    let mut w = Writer::new(1, 7);
    let pages = w.add(
        [
            ("Type", Object::name("Pages")),
            ("Kids", Vec::<Object>::new().into()),
            ("Count", 0.into()),
        ]
        .into_iter()
        .collect::<Dict>(),
    );
    let outlines = w.reserve();
    let [a, b, c] = [w.reserve(), w.reserve(), w.reserve()];
    let title = |t: &str| Object::String(PdfString::literal(t));
    let mut da: Dict = [("Title", title("A")), ("Parent", outlines.into()), ("Next", b.into())]
        .into_iter()
        .collect();
    let mut db: Dict = [
        ("Title", title("B")),
        ("Parent", outlines.into()),
        ("Prev", a.into()),
        ("Next", c.into()),
    ]
    .into_iter()
    .collect();
    let mut dc: Dict = [("Title", title("C")), ("Parent", outlines.into()), ("Prev", b.into())]
        .into_iter()
        .collect();
    let mut root: Dict = [
        ("Type", Object::name("Outlines")),
        ("First", a.into()),
        ("Last", c.into()),
        ("Count", 3.into()),
    ]
    .into_iter()
    .collect();
    match breakage {
        "wrong last" => root.insert("Last", b),
        "cycle" => dc.insert("Next", a),
        "wrong prev" => db.insert("Prev", c),
        "missing parent" => {
            da.remove("Parent");
        }
        _ => {}
    }
    w.set(a, da);
    w.set(b, db);
    w.set(c, dc);
    w.set(outlines, root);
    let catalog = w.add(
        [
            ("Type", Object::name("Catalog")),
            ("Pages", pages.into()),
            ("Outlines", outlines.into()),
        ]
        .into_iter()
        .collect::<Dict>(),
    );
    Document::open(
        w.finish(&[("Root", catalog)].into_iter().collect()).unwrap(),
        OpenOptions::default(),
    )
    .unwrap()
}

#[test]
fn a_tree_read_with_broken_links_is_reported_and_rewritten_whole() {
    for breakage in ["wrong last", "cycle", "wrong prev", "missing parent"] {
        let mut doc = broken(breakage);
        let mut tree = OutlineTree::read(&doc).unwrap();
        assert!(tree.repaired(), "{breakage}");
        tree.write(&mut doc.cos_mut().edit()).unwrap();
        assert_eq!(check_links(&doc), 3, "{breakage}");
        assert!(!OutlineTree::read(&doc).unwrap().repaired(), "{breakage}");
    }
    assert!(!OutlineTree::read(&broken("none")).unwrap().repaired());
}

/// One random edit, by index into the tree's items at the time.
#[derive(Debug, Clone)]
enum Edit {
    Insert { parent: Option<usize>, at: u8 },
    Move { item: usize, parent: Option<usize>, at: u8 },
    Remove { item: usize, promote: bool },
    Open { item: usize, open: bool },
}

fn edit() -> impl Strategy<Value = Edit> {
    prop_oneof![
        (proptest::option::of(0..16usize), 0..4u8).prop_map(|(parent, at)| Edit::Insert { parent, at }),
        (0..16usize, proptest::option::of(0..16usize), 0..4u8).prop_map(|(item, parent, at)| Edit::Move {
            item,
            parent,
            at
        }),
        (0..16usize, any::<bool>()).prop_map(|(item, promote)| Edit::Remove { item, promote }),
        (0..16usize, any::<bool>()).prop_map(|(item, open)| Edit::Open { item, open }),
    ]
}

fn position(tree: &OutlineTree, parent: Option<&OutlineItemId>, at: u8) -> SiblingPosition {
    let siblings: Vec<OutlineItemId> = tree
        .items()
        .into_iter()
        .filter(|i| tree.parent(i) == Some(parent))
        .collect();
    match (at, siblings.first(), siblings.last()) {
        (0, ..) => SiblingPosition::First,
        (2, Some(s), _) => SiblingPosition::Before(s.clone()),
        (3, _, Some(s)) => SiblingPosition::After(s.clone()),
        _ => SiblingPosition::Last,
    }
}

#[test]
fn random_edits_keep_every_list_whole_and_every_count_right() {
    let mut runner = TestRunner::new(Config {
        cases: 64,
        rng_seed: RngSeed::Fixed(0x0071_1e5e),
        failure_persistence: None,
        ..Config::default()
    });
    runner
        .run(&proptest::collection::vec(edit(), 1..24), |edits| {
            let mut doc = outline_pdf();
            for e in edits {
                let mut tree = OutlineTree::read(&doc).unwrap();
                let items = tree.items();
                let pick = |i: usize| (!items.is_empty()).then(|| items[i % items.len()].clone());
                match e {
                    Edit::Insert { parent, at } => {
                        let parent = parent.and_then(pick);
                        let at = position(&tree, parent.as_ref(), at);
                        let r = item(&mut doc, "new");
                        tree.insert(r, parent.as_ref(), &at).unwrap();
                    }
                    Edit::Move { item, parent, at } => {
                        let (Some(item), parent) = (pick(item), parent.and_then(pick)) else {
                            continue;
                        };
                        let at = position(&tree, parent.as_ref(), at);
                        match tree.move_item(&item, parent.as_ref(), &at) {
                            Ok(()) | Err(OutlineTreeError::IntoItself(_)) => {}
                            Err(e) => panic!("{e:?}"),
                        }
                    }
                    Edit::Remove { item, promote } => {
                        let Some(item) = pick(item) else { continue };
                        tree.remove(&item, promote).unwrap();
                    }
                    Edit::Open { item, open } => {
                        let Some(item) = pick(item) else { continue };
                        tree.set_open(&item, open).unwrap();
                    }
                }
                tree.write(&mut doc.cos_mut().edit()).unwrap();
                let reached = check_links(&doc);
                prop_assert_eq!(reached, OutlineTree::read(&doc).unwrap().items().len());
                prop_assert!(!OutlineTree::read(&doc).unwrap().repaired());
            }
            Ok(())
        })
        .unwrap();
}
