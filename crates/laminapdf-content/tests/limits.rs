//! ADR 0003 §4 for the limits ADR 0023 enforces: each at its limit succeeds and one past it
//! fails with `LimitExceeded`; repairs warn, and strict mode refuses them.

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{dict, form, page_doc_with, run};
use laminapdf_content::{Error, Limit, Limits, Warning};
use laminapdf_cos::{Dict, Object};
use laminapdf_model::OpenOptions;
use laminapdf_save::Writer;

fn with(limits: Limits) -> OpenOptions {
    OpenOptions::default().with_limits(limits)
}

fn run_with(limits: Limits, content: &[u8]) -> laminapdf_content::Result<laminapdf_content::Summary> {
    let doc = page_doc_with(with(limits), |_| (content.to_vec(), Dict::new()));
    run(&doc).0
}

fn limit_of(result: &laminapdf_content::Result<laminapdf_content::Summary>) -> Option<Limit> {
    match result {
        Err(Error::LimitExceeded { limit, .. }) => Some(*limit),
        _ => None,
    }
}

#[test]
fn operators_at_the_limit_succeed() {
    let limits = Limits::default().with_max_content_operators(4);
    assert_eq!(run_with(limits, b"q Q q Q").map(|s| s.operators), Ok(4));
}

#[test]
fn operators_past_the_limit_fail() {
    let limits = Limits::default().with_max_content_operators(3);
    assert_eq!(limit_of(&run_with(limits, b"q Q q Q")), Some(Limit::ContentOperators));
}

/// Forms that each draw the next one twice: 2^depth leaves from a few bytes.
fn doubling(w: &mut Writer, depth: usize) -> Dict {
    let mut next = w.add(form(b"n", Dict::new()));
    for _ in 0..depth {
        let resources = dict([("XObject", dict([("X", Object::from(next))]).into())]);
        next = w.add(form(b"/X Do /X Do", dict([("Resources", resources.into())])));
    }
    dict([("XObject", dict([("X", Object::from(next))]).into())])
}

#[test]
fn the_operator_limit_counts_every_form_each_time_it_is_drawn() {
    // 2^20 leaves: the fuel runs out long before.
    let limits = Limits::default().with_max_content_operators(10_000);
    let doc = page_doc_with(with(limits), |w| (b"/X Do".to_vec(), doubling(w, 20)));
    assert_eq!(limit_of(&run(&doc).0), Some(Limit::ContentOperators));
}

#[test]
fn saved_states_at_the_nesting_limit_succeed() {
    let limits = Limits::default().with_max_nesting_depth(3);
    assert!(run_with(limits, b"q q q Q Q Q").is_ok());
}

#[test]
fn saved_states_past_the_nesting_limit_fail() {
    let limits = Limits::default().with_max_nesting_depth(3);
    assert_eq!(limit_of(&run_with(limits, b"q q q q")), Some(Limit::NestingDepth));
}

#[test]
fn bytes_interpreted_at_the_limit_succeed() {
    // The page content (11 bytes) plus a 6-byte form drawn twice: 23 bytes interpreted.
    let content = b"/X Do /X Do";
    let doc = page_doc_with(
        with(Limits::default().with_max_decoded_total(content.len() + 12)),
        |w| {
            let fm = w.add(form(b"0 0 m ", Dict::new()));
            (
                content.to_vec(),
                dict([("XObject", dict([("X", Object::from(fm))]).into())]),
            )
        },
    );
    assert!(run(&doc).0.is_ok());
}

#[test]
fn bytes_interpreted_past_the_limit_fail() {
    let content = b"/X Do /X Do";
    let doc = page_doc_with(
        with(Limits::default().with_max_decoded_total(content.len() + 11)),
        |w| {
            let fm = w.add(form(b"0 0 m ", Dict::new()));
            (
                content.to_vec(),
                dict([("XObject", dict([("X", Object::from(fm))]).into())]),
            )
        },
    );
    assert_eq!(limit_of(&run(&doc).0), Some(Limit::DecodedTotal));
}

#[test]
fn a_string_operand_past_the_length_limit_fails() {
    let limits = Limits::default().with_max_string_length(12);
    assert_eq!(
        limit_of(&run_with(limits, b"BT (abcdefghijklm) Tj ET")),
        Some(Limit::StringLength)
    );
}

#[test]
fn an_unbalanced_restore_is_ignored_with_a_warning() {
    let doc = page_doc_with(OpenOptions::default(), |_| (b"Q 0 0 m n".to_vec(), Dict::new()));
    let (result, rec) = run(&doc);
    assert_eq!((result.is_ok(), rec.codes()), (true, vec!["content-unbalanced-q"]));
}

#[test]
fn content_warnings_are_recorded_on_the_document() {
    let doc = page_doc_with(OpenOptions::default(), |_| (b"Q".to_vec(), Dict::new()));
    let _ = run(&doc);
    assert!(
        doc.warnings()
            .iter()
            .any(|w| matches!(w, Warning::ContentUnbalancedQ { offset: 0, .. }))
    );
}

#[test]
fn strict_mode_refuses_a_repair() {
    let doc = page_doc_with(OpenOptions::default().with_strict(true), |_| {
        (b"Q".to_vec(), Dict::new())
    });
    assert!(matches!(
        run(&doc).0,
        Err(Error::Strict {
            warning: Warning::ContentUnbalancedQ { .. }
        })
    ));
}

#[test]
fn an_unclosed_text_object_warns_at_the_end() {
    let doc = page_doc_with(OpenOptions::default(), |_| (b"BT".to_vec(), Dict::new()));
    let (_, rec) = run(&doc);
    assert_eq!(rec.codes(), ["content-unbalanced-bt"]);
}

#[test]
fn a_nested_bt_warns() {
    let doc = page_doc_with(OpenOptions::default(), |_| (b"BT BT ET".to_vec(), Dict::new()));
    let (_, rec) = run(&doc);
    assert_eq!(rec.codes(), ["content-unbalanced-bt"]);
}

#[test]
fn a_string_operand_at_the_length_limit_succeeds() {
    let limits = Limits::default().with_max_string_length(12);
    assert!(run_with(limits, b"BT (abcdefghijkl) Tj ET").is_ok());
}

/// ADR 0026 §1.1: a limit hit in content is located in the decoded content of its object.
mod positions {
    use super::*;
    use laminapdf_content::Position;

    fn at(result: &laminapdf_content::Result<laminapdf_content::Summary>) -> Option<Position> {
        match result {
            Err(Error::LimitExceeded { at, .. }) => *at,
            _ => None,
        }
    }

    #[test]
    fn a_nesting_limit_is_at_the_offset_of_the_q_in_the_content() {
        let limits = Limits::default().with_max_nesting_depth(3);
        assert!(
            matches!(
                at(&run_with(limits, b"q q q q")),
                Some(Position::InStream { offset: 6, .. })
            ),
            "{:?}",
            run_with(limits, b"q q q q")
        );
    }

    #[test]
    fn a_string_limit_is_at_an_offset_in_the_content() {
        let limits = Limits::default().with_max_string_length(12);
        let result = run_with(limits, b"BT (abcdefghijklm) Tj ET");
        assert!(
            matches!(at(&result), Some(Position::InStream { offset: 3, .. })),
            "{result:?}"
        );
    }

    #[test]
    fn a_form_depth_limit_is_at_the_form_object() {
        let limits = Limits::default().with_max_xobject_depth(2);
        let doc = page_doc_with(with(limits), |w| (b"/X Do".to_vec(), doubling(w, 4)));
        let result = run(&doc).0;
        assert!(matches!(at(&result), Some(Position::Object(_))), "{result:?}");
    }
}
