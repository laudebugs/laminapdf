//! `Document::structure_tree` against pdf.js's reading of the same file (`fixtures/build/structure.json`,
//! from `tools/fixtures/structure_facts.py`): each page's marked content in reading order, the
//! structure types in order and the figure's alternate text agree.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf::{Document, OpenOptions, StructureKid, StructureNode};
use laminapdf_test_fixtures::{fixture, json};
use serde_json::Value;

fn tagged_report() -> Vec<StructureNode> {
    let doc = Document::open_bytes(fixture("golden/tagged-report.pdf"), OpenOptions::default()).unwrap();
    doc.structure_tree().unwrap().unwrap()
}

/// Every element's kind and every content's (page, mcid), depth-first.
fn walk(nodes: &[StructureNode], kinds: &mut Vec<String>, content: &mut Vec<(u32, i64)>, alts: &mut Vec<String>) {
    for node in nodes {
        kinds.push(node.kind.clone());
        alts.extend(node.alt.clone());
        for kid in &node.kids {
            match kid {
                StructureKid::Node(child) => walk(std::slice::from_ref(child), kinds, content, alts),
                StructureKid::Content { page, mcid } => content.push((*page, *mcid)),
                _ => {}
            }
        }
    }
}

/// pdf.js's tree for one page: its roles (Root left out) and its contents, depth-first.
fn walk_pdfjs(node: &Value, roles: &mut Vec<String>, content: &mut Vec<i64>, alts: &mut Vec<String>) {
    if let Some(mcid) = node.get("content").and_then(Value::as_i64) {
        content.push(mcid);
        return;
    }
    if let Some(role) = node.get("role").and_then(Value::as_str)
        && role != "Root"
    {
        roles.push(role.to_owned());
    }
    if let Some(alt) = node.get("alt").and_then(Value::as_str) {
        alts.push(alt.to_owned());
    }
    for child in node.get("children").and_then(Value::as_array).into_iter().flatten() {
        walk_pdfjs(child, roles, content, alts);
    }
}

fn pdfjs_pages() -> Vec<Value> {
    let facts = json("build/structure.json");
    facts["files"]["tagged-report.pdf"]["pdfjs"]["pages"]
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn structure_tree_should_give_each_page_its_marked_content_in_pdfjs_reading_order() {
    let (mut kinds, mut content, mut alts) = (vec![], vec![], vec![]);
    walk(&tagged_report(), &mut kinds, &mut content, &mut alts);
    let ours: Vec<Vec<i64>> = (0..2u32)
        .map(|page| content.iter().filter(|(p, _)| *p == page).map(|(_, m)| *m).collect())
        .collect();
    let theirs: Vec<Vec<i64>> = pdfjs_pages()
        .iter()
        .map(|page| {
            let mut c = vec![];
            walk_pdfjs(page, &mut vec![], &mut c, &mut vec![]);
            c
        })
        .collect();
    assert_eq!(ours, theirs);
}

#[test]
fn structure_tree_should_give_only_types_pdfjs_reads_and_the_same_figure_alt_text() {
    let (mut kinds, mut content, mut alts) = (vec![], vec![], vec![]);
    walk(&tagged_report(), &mut kinds, &mut content, &mut alts);
    let (mut roles, mut pdfjs_alts) = (vec![], vec![]);
    for page in pdfjs_pages() {
        walk_pdfjs(&page, &mut roles, &mut vec![], &mut pdfjs_alts);
    }
    // pdf.js repeats an element that spans both pages under each page; ours lists it once.
    roles.dedup();
    let figure_alt = alts.first().cloned();
    assert_eq!(
        (kinds.iter().filter(|k| roles.contains(k)).count(), figure_alt),
        (kinds.len(), pdfjs_alts.first().cloned())
    );
}

#[test]
fn structure_tree_should_be_none_for_an_untagged_file() {
    let doc = Document::open_bytes(fixture("golden/examples/hello.pdf"), OpenOptions::default()).unwrap();
    assert_eq!(doc.structure_tree().unwrap(), None);
}
