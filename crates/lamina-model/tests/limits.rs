//! ADR 0003 §3/§4: `max_tree_depth` is a hard limit that every model view hits inside a
//! tolerant walk, so the view skips the subtree with its warning. Paired tests: at the limit
//! everything is read; one past, the deepest item is skipped with a warning (and strict mode
//! turns that warning into `Error::Strict`).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{classic, outline_titles};
use lamina_model::{Document, Error, Limits, OpenOptions, Warning};

const DEPTH: usize = 3;

fn open_with_depth(bytes: Vec<u8>, strict: bool) -> Document {
    let options = OpenOptions::default()
        .with_limits(Limits::default().with_max_tree_depth(DEPTH))
        .with_strict(strict);
    Document::open(bytes, options).unwrap()
}

fn codes(doc: &Document) -> Vec<&'static str> {
    doc.warnings().iter().map(Warning::code).collect()
}

/// An outline of `levels` items, each the only child of the one before (objects 5..).
fn nested_outline(levels: usize) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
        "<< /First 5 0 R >>".to_owned(),
    ];
    for level in 0..levels {
        let num = 5 + level;
        let child = if level + 1 < levels {
            format!("/First {} 0 R", num + 1)
        } else {
            String::new()
        };
        objects.push(format!("<< /Title (level {}) {child} >>", level + 1));
    }
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    classic(&refs, "")
}

/// A name tree whose only entry sits `depth` kids below the root (objects 4..).
fn nested_name_tree(depth: usize) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Names << /Dests 4 0 R >> >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
    ];
    for level in 0..depth {
        objects.push(format!("<< /Kids [{} 0 R] >>", 5 + level));
    }
    objects.push("<< /Names [(deep) 42] >>".to_owned());
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    classic(&refs, "")
}

/// Page labels whose only range sits `depth` kids below the number tree's root.
fn nested_labels(depth: usize) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /PageLabels 4 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
    ];
    for level in 0..depth {
        objects.push(format!("<< /Kids [{} 0 R] >>", 5 + level));
    }
    objects.push("<< /Nums [0 << /S /R >>] >>".to_owned());
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    classic(&refs, "")
}

mod outline {
    use super::*;

    #[test]
    fn nesting_at_the_limit_reads_every_item() {
        let doc = open_with_depth(nested_outline(DEPTH), false);
        assert_eq!((outline_titles(&doc).unwrap().len(), codes(&doc)), (DEPTH, vec![]));
    }

    #[test]
    fn nesting_past_the_limit_skips_the_deepest_item_with_a_warning() {
        let doc = open_with_depth(nested_outline(DEPTH + 1), false);
        assert_eq!(
            (outline_titles(&doc).unwrap().len(), codes(&doc)),
            (DEPTH, vec!["outline-item-skipped"])
        );
    }

    #[test]
    fn nesting_past_the_limit_is_an_error_in_strict_mode() {
        let doc = open_with_depth(nested_outline(DEPTH + 1), true);
        assert!(matches!(doc.outline(), Err(Error::Strict { .. })));
    }
}

mod name_tree {
    use super::*;

    fn deep(doc: &Document) -> Option<i64> {
        doc.name_tree("Dests")
            .unwrap()
            .unwrap()
            .get(b"deep")
            .unwrap()
            .and_then(|v| v.as_int())
    }

    #[test]
    fn a_leaf_at_the_limit_is_found() {
        let doc = open_with_depth(nested_name_tree(DEPTH), false);
        assert_eq!((deep(&doc), codes(&doc)), (Some(42), vec![]));
    }

    #[test]
    fn a_leaf_past_the_limit_is_skipped_with_a_warning() {
        let doc = open_with_depth(nested_name_tree(DEPTH + 1), false);
        assert_eq!((deep(&doc), codes(&doc)), (None, vec!["name-tree-node-skipped"]));
    }

    #[test]
    fn a_leaf_past_the_limit_is_an_error_in_strict_mode() {
        let doc = open_with_depth(nested_name_tree(DEPTH + 1), true);
        let tree = doc.name_tree("Dests").unwrap().unwrap();
        assert!(matches!(tree.entries(), Err(Error::Strict { .. })));
    }
}

mod page_labels {
    use super::*;

    fn label(doc: &Document) -> Option<String> {
        doc.pages().unwrap().get(0).unwrap().label().unwrap()
    }

    #[test]
    fn a_range_at_the_limit_is_used() {
        let doc = open_with_depth(nested_labels(DEPTH), false);
        assert_eq!((label(&doc), codes(&doc)), (Some("I".into()), vec![]));
    }

    #[test]
    fn a_range_past_the_limit_is_skipped_with_a_warning() {
        let doc = open_with_depth(nested_labels(DEPTH + 1), false);
        assert_eq!((label(&doc), codes(&doc)), (None, vec!["name-tree-node-skipped"]));
    }
}

#[test]
fn a_long_outline_chain_is_read_iteratively() {
    // 10 000 siblings: the walk has no recursion, so this only costs time.
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R >>".to_owned(),
        "<< /First 5 0 R >>".to_owned(),
    ];
    let n = 10_000;
    for i in 0..n {
        let next = if i + 1 < n {
            format!("/Next {} 0 R", 6 + i)
        } else {
            String::new()
        };
        objects.push(format!("<< /Title (i) {next} >>"));
    }
    let refs: Vec<&str> = objects.iter().map(String::as_str).collect();
    let doc = Document::open(classic(&refs, ""), OpenOptions::default()).unwrap();
    assert_eq!(doc.outline().unwrap().len(), n);
}
