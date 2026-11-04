//! `laminapdf extract`: plain text, chosen pages, and the JSON report.

mod common;

use common::laminapdf;

#[test]
fn pages_should_be_separated_by_form_feeds() {
    let run = laminapdf(&["extract", "fixtures/golden/links-and-outlines.pdf"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.stdout_text().matches('\u{c}').count(), 3);
}

#[test]
fn a_range_should_choose_the_pages_in_its_order() {
    let report = laminapdf(&[
        "extract",
        "fixtures/golden/links-and-outlines.pdf",
        "--pages",
        "z,1",
        "--json",
    ])
    .json();
    let numbers: Vec<u64> = report["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["number"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, [4, 1]);
    assert_eq!(report["pages"][1]["index"], 0);
}

#[test]
fn a_bad_range_should_say_where_it_went_wrong() {
    let run = laminapdf(&["extract", "fixtures/corpus/hello.pdf", "--pages", "1,x", "--json"]);
    let error = &run.stderr_json()[0]["error"];
    assert_eq!((run.code, error["code"].as_str()), (2, Some("invalid-page-range")));
    assert_eq!(
        (error["data"]["range"].as_str(), error["data"]["position"].as_u64()),
        (Some("1,x"), Some(4))
    );
    assert!(
        !error["message"].as_str().unwrap().contains("1,x"),
        "the message never quotes the command line"
    );
}
