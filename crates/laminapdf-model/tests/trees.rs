//! The name- and number-tree writers (`TreeWriter`): random entry sets, edited by random
//! inserts and deletes and written again, give valid trees (sorted keys, the root without
//! `/Limits`, every other node's `/Limits` its first and last key, leaves and nodes within the
//! size bound, every leaf at one depth) that the model's readers read back with the same
//! entries; qpdf reads a written `/PageLabels` tree back the same. Seeds are fixed.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use laminapdf_cos::{Object, PdfString};
use laminapdf_model::{Document, LabelStyle, OpenOptions, PagePosition, TreeWriter};
use laminapdf_save::{FullWriter, SaveOptions};
use laminapdf_test_fixtures::fixture;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngSeed, TestRunner};

const SEED: u64 = 0x7ee5_0f7e;

fn open() -> Document {
    Document::open(fixture("pages/labels.pdf"), OpenOptions::default()).unwrap()
}

fn runner() -> TestRunner {
    TestRunner::new(Config {
        cases: 96,
        rng_seed: RngSeed::Fixed(SEED),
        failure_persistence: None,
        ..Config::default()
    })
}

/// A key as the tree stores it, comparable across both kinds.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Num(i64),
    Name(Vec<u8>),
}

fn key(obj: &Object) -> Key {
    match obj {
        Object::Integer(n) => Key::Num(*n),
        Object::String(s) => Key::Name(s.bytes.clone()),
        other => panic!("not a key: {other:?}"),
    }
}

/// Checks the tree under `root` and returns its keys in tree order.
fn check(doc: &Document, root: &Object, entries: &str, max: usize) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut leaf_depths = BTreeSet::new();
    visit(doc, root, entries, max, 0, &mut keys, &mut leaf_depths);
    assert!(leaf_depths.len() <= 1, "leaves at depths {leaf_depths:?}");
    assert!(keys.windows(2).all(|w| w[0] < w[1]), "keys sorted and unique");
    keys
}

fn visit(
    doc: &Document,
    node: &Object,
    entries: &str,
    max: usize,
    depth: usize,
    keys: &mut Vec<Key>,
    leaf_depths: &mut BTreeSet<usize>,
) {
    assert!(matches!(node, Object::Ref(_)), "every node is indirect");
    let node = doc.deref(node).unwrap();
    let dict = node.as_dict().unwrap();
    let first = keys.len();
    if let Some(items) = dict.get(entries) {
        let items = items.as_array().unwrap();
        assert!(items.len() / 2 <= max, "a leaf of {} entries", items.len() / 2);
        keys.extend(items.chunks(2).map(|pair| key(&pair[0])));
        leaf_depths.insert(depth);
    }
    if let Some(kids) = dict.get("Kids") {
        let kids = kids.as_array().unwrap();
        assert!(kids.len() <= max, "a node of {} kids", kids.len());
        for kid in kids {
            visit(doc, kid, entries, max, depth + 1, keys, leaf_depths);
        }
    }
    match dict.get("Limits") {
        None => assert_eq!(depth, 0, "only the root goes without /Limits"),
        Some(limits) => {
            assert!(depth > 0, "the root has no /Limits");
            let limits = limits.as_array().unwrap();
            let own = &keys[first..];
            assert_eq!(key(&limits[0]), own[0]);
            assert_eq!(key(&limits[1]), own[own.len() - 1]);
        }
    }
}

#[derive(Debug, Clone)]
enum Change {
    Insert(i64),
    Delete(usize),
}

fn change() -> impl Strategy<Value = Change> {
    prop_oneof![
        (-5000i64..5000).prop_map(Change::Insert),
        any::<usize>().prop_map(Change::Delete)
    ]
}

#[test]
fn number_trees_should_be_valid_and_read_back() {
    let start = proptest::collection::btree_set(-5000i64..5000, 0..700);
    let strategy = (start, proptest::collection::vec(change(), 0..60), 2usize..80);
    let result = runner().run(&strategy, |(start, changes, max)| {
        let mut doc = open();
        let writer = TreeWriter::new().with_max_kids(max);
        let mut entries: BTreeMap<i64, Object> = start.iter().map(|&k| (k, Object::Integer(k * 3))).collect();
        for round in [false, true] {
            if round {
                for c in &changes {
                    match *c {
                        Change::Insert(k) => {
                            entries.insert(k, Object::Integer(k * 3));
                        }
                        Change::Delete(i) if !entries.is_empty() => {
                            let k = *entries.keys().nth(i % entries.len()).unwrap();
                            entries.remove(&k);
                        }
                        Change::Delete(_) => {}
                    }
                }
            }
            let root = Object::Ref(writer.write_numbers(&mut doc.cos_mut().edit(), &entries).unwrap());
            let keys = check(&doc, &root, "Nums", max);
            prop_assert_eq!(keys, entries.keys().map(|&k| Key::Num(k)).collect::<Vec<_>>());
            let read: BTreeMap<i64, Object> = doc.number_tree(root.clone()).entries().unwrap().into_iter().collect();
            prop_assert_eq!(&read, &entries);
            if let Some((&k, v)) = entries.iter().nth(entries.len() / 2) {
                let found = doc.number_tree(root).get(k).unwrap();
                prop_assert_eq!(found.as_ref(), Some(v));
            }
        }
        Ok(())
    });
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn name_trees_should_be_valid_and_read_back() {
    let key = proptest::collection::vec(any::<u8>(), 0..6);
    let start = proptest::collection::btree_set(key.clone(), 0..500);
    let changes = proptest::collection::vec(prop_oneof![key.prop_map(Ok), any::<usize>().prop_map(Err)], 0..60);
    let strategy = (start, changes, 2usize..80);
    let result = runner().run(&strategy, |(start, changes, max)| {
        let mut doc = open();
        let writer = TreeWriter::new().with_max_kids(max);
        let value = |k: &[u8]| Object::String(PdfString::hex(k.iter().rev().copied().collect::<Vec<u8>>()));
        let mut entries: BTreeMap<Vec<u8>, Object> = start.iter().map(|k| (k.clone(), value(k))).collect();
        for round in [false, true] {
            if round {
                for c in &changes {
                    match c {
                        Ok(k) => {
                            entries.insert(k.clone(), value(k));
                        }
                        Err(i) if !entries.is_empty() => {
                            let k = entries.keys().nth(i % entries.len()).unwrap().clone();
                            entries.remove(&k);
                        }
                        Err(_) => {}
                    }
                }
            }
            let root = Object::Ref(writer.write_names(&mut doc.cos_mut().edit(), &entries).unwrap());
            let keys = check(&doc, &root, "Names", max);
            prop_assert_eq!(keys, entries.keys().map(|k| Key::Name(k.clone())).collect::<Vec<_>>());
            let read: BTreeMap<Vec<u8>, Object> = laminapdf_model::NameTree::new(&doc, root)
                .entries()
                .unwrap()
                .into_iter()
                .map(|(k, v)| (k, normalise(v)))
                .collect();
            let expected: BTreeMap<Vec<u8>, Object> =
                entries.iter().map(|(k, v)| (k.clone(), normalise(v.clone()))).collect();
            prop_assert_eq!(read, expected);
        }
        Ok(())
    });
    assert!(result.is_ok(), "{result:?}");
}

/// A string value by its bytes (the reader keeps how a string was written).
fn normalise(v: Object) -> Object {
    match v {
        Object::String(s) => Object::String(PdfString::literal(s.bytes)),
        other => other,
    }
}

/// A `/PageLabels` tree of 300 ranges in leaves of 4 over 300 pages, written, saved and read
/// back by qpdf (`--json`'s `pagelabels`, which lists the ranges of existing pages) with the
/// same page indices and styles.
#[test]
fn qpdf_should_read_a_written_page_label_tree_back() {
    let mut doc = open();
    let mut editor = doc.edit_page_tree().unwrap();
    let media = Object::Array(vec![0.into(), 0.into(), 200.into(), 200.into()]);
    for _ in 10..300 {
        let page = [("MediaBox", media.clone())].into_iter().collect();
        editor.insert_new(page, &PagePosition::End).unwrap();
    }
    editor.finish().unwrap();
    let styles = [b"D".as_slice(), b"r", b"A"];
    let entries: BTreeMap<i64, Object> = (0..300usize)
        .map(|i| {
            let style = Object::Name(laminapdf_cos::Name::new(styles[i % 3]));
            (
                i64::try_from(i).unwrap(),
                Object::Dict([("S", style)].into_iter().collect()),
            )
        })
        .collect();
    let mut cos = doc.cos_mut().edit();
    let root = TreeWriter::new()
        .with_max_kids(4)
        .write_numbers(&mut cos, &entries)
        .unwrap();
    let catalog = cos.trailer().get("Root").and_then(Object::as_ref).unwrap();
    let mut dict = cos.get(catalog).unwrap().unwrap().as_dict().unwrap().clone();
    dict.insert("PageLabels", Object::Ref(root));
    cos.set(catalog, Object::Dict(dict)).unwrap();
    let ranges = doc.page_labels().unwrap();
    assert_eq!(ranges.ranges().len(), 300);
    assert_eq!(ranges.ranges()[299].style, Some(LabelStyle::UpperLetters));
    let mut out = Vec::new();
    FullWriter::new(doc.cos(), SaveOptions::new()).write(&mut out).unwrap();
    let path = std::env::temp_dir().join(format!("laminapdf-model-trees-{}.pdf", std::process::id()));
    std::fs::write(&path, &out).unwrap();
    let check = Command::new("qpdf").arg("--check").arg(&path).output();
    let json = Command::new("qpdf")
        .args(["--json=2", "--json-key=pagelabels"])
        .arg(&path)
        .output();
    let _ = std::fs::remove_file(&path);
    let (Ok(check), Ok(json)) = (check, json) else {
        eprintln!("qpdf not on PATH: skipping the qpdf read-back");
        return;
    };
    assert!(check.status.success(), "{}", String::from_utf8_lossy(&check.stdout));
    let json: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let labels = json["pagelabels"].as_array().unwrap();
    let indices: Vec<i64> = labels.iter().map(|l| l["index"].as_i64().unwrap()).collect();
    assert_eq!(indices, entries.keys().copied().collect::<Vec<_>>());
    let read_styles: Vec<String> = labels
        .iter()
        .map(|l| l["label"]["/S"].as_str().unwrap().to_owned())
        .collect();
    let expected: Vec<String> = (0..300).map(|i| format!("/{}", ["D", "r", "A"][i % 3])).collect();
    assert_eq!(read_styles, expected);
}
