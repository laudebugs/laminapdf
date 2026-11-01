//! Marked content (§14.6): sequences and points, property lists inline or from
//! `/Properties`, MCIDs, and repair of unbalanced sequences per content stream (ADR 0023 §3).

#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{Event, dict, form, page_doc, run, run_content};
use laminapdf_cos::{Dict, Object};

fn tags(events: &[Event]) -> Vec<String> {
    events
        .iter()
        .map(|e| match e {
            Event::Begin { tag, mcid, .. } => format!("begin {} {mcid:?}", String::from_utf8_lossy(tag)),
            Event::End => "end".to_owned(),
            Event::Point(tag) => format!("point {}", String::from_utf8_lossy(tag)),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn bmc_bdc_and_emc_nest() {
    let (_, rec) = run_content(b"/Artifact BMC /P <</MCID 3>> BDC EMC EMC");
    assert_eq!(
        tags(&rec.events),
        ["begin Artifact None", "begin P Some(3)", "end", "end"]
    );
}

#[test]
fn a_named_property_list_comes_from_properties() {
    let doc = page_doc(|_| {
        let resources = dict([("Properties", dict([("MC0", dict([("MCID", 9.into())]).into())]).into())]);
        (b"/Span /MC0 BDC EMC".to_vec(), resources)
    });
    let (_, rec) = run(&doc);
    assert_eq!(tags(&rec.events), ["begin Span Some(9)", "end"]);
}

#[test]
fn a_missing_property_list_warns_and_still_begins_the_sequence() {
    let (_, rec) = run_content(b"/OC /oc1 BDC EMC");
    assert_eq!(
        (tags(&rec.events), rec.codes()),
        (
            vec!["begin OC None".to_owned(), "end".to_owned()],
            vec!["content-resource-missing"]
        )
    );
}

#[test]
fn marked_points_are_reported() {
    let (_, rec) = run_content(b"/Tag MP /Tag2 <<>> DP");
    assert_eq!(tags(&rec.events), ["point Tag", "point Tag2"]);
}

#[test]
fn an_emc_without_a_sequence_is_ignored_with_a_warning() {
    let (_, rec) = run_content(b"EMC");
    assert_eq!((rec.events.len(), rec.codes()), (0, vec!["content-unbalanced-marked"]));
}

#[test]
fn sequences_left_open_are_closed_at_the_end_of_the_stream() {
    let (_, rec) = run_content(b"/A BMC /B BMC");
    assert_eq!(
        (tags(&rec.events), rec.codes()),
        (
            vec![
                "begin A None".to_owned(),
                "begin B None".to_owned(),
                "end".to_owned(),
                "end".to_owned()
            ],
            vec!["content-unbalanced-marked"]
        )
    );
}

#[test]
fn a_form_closes_its_own_sequences_and_cannot_close_the_page_s() {
    let doc = page_doc(|w| {
        let fm = w.add(form(b"EMC /In BMC", Dict::new()));
        let resources = dict([("XObject", dict([("X", Object::from(fm))]).into())]);
        (b"/Out BMC /X Do EMC".to_vec(), resources)
    });
    let (_, rec) = run(&doc);
    let marked: Vec<_> = tags(&rec.events)
        .into_iter()
        .filter(|t| t.starts_with("begin") || t == "end")
        .collect();
    assert_eq!(
        (marked, rec.codes()),
        (
            vec![
                "begin Out None".to_owned(),
                "begin In None".to_owned(),
                "end".to_owned(),
                "end".to_owned()
            ],
            vec!["content-unbalanced-marked", "content-unbalanced-marked"]
        )
    );
}

#[test]
fn marked_content_inside_a_form_reports_its_depth() {
    let doc = page_doc(|w| {
        let fm = w.add(form(b"/P <</MCID 1>> BDC EMC", Dict::new()));
        let resources = dict([("XObject", dict([("X", Object::from(fm))]).into())]);
        (b"/X Do".to_vec(), resources)
    });
    let (_, rec) = run(&doc);
    assert!(rec.events.iter().any(|e| matches!(e, Event::Begin { depth: 1, .. })));
}
