//! `laminapdf info`: the summary a person reads, and the JSON report.

mod common;

use common::{laminapdf, run};

#[test]
fn the_summary_should_fit_in_twenty_lines() {
    let run = laminapdf(&["info", "fixtures/golden/links-and-outlines.pdf"]);
    let text = run.stdout_text();
    assert_eq!(run.code, 0);
    assert!(text.lines().count() <= 20, "{text}");
    assert!(
        text.starts_with("fixtures/golden/links-and-outlines.pdf  PDF "),
        "{text}"
    );
    assert!(text.contains(" · 4 pages · "), "{text}");
}

#[test]
fn the_report_should_describe_the_document() {
    let report = laminapdf(&["info", "fixtures/golden/tagged-report.pdf", "--json"]).json();
    assert_eq!(
        (
            report["schema"].as_str(),
            report["tagged"].as_bool(),
            report["encrypted"].as_bool()
        ),
        (Some("laminapdf.info/1"), Some(true), Some(false))
    );
    let pages = report["pages"].as_u64().unwrap();
    let sizes = report["pageSizes"].as_array().unwrap();
    assert_eq!(sizes.len() as u64, pages);
    assert_eq!(
        (sizes[0]["index"].as_u64(), sizes[0]["number"].as_u64()),
        (Some(0), Some(1))
    );
}

#[test]
fn fields_should_keep_only_the_keys_asked_for() {
    let report = laminapdf(&[
        "info",
        "fixtures/corpus/hello.pdf",
        "--json",
        "--fields",
        "pages,encrypted,warnings",
    ])
    .json();
    let mut keys: Vec<&str> = report.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["encrypted", "input", "pages", "schema", "warnings"]);
    let typo = laminapdf(&["info", "fixtures/corpus/hello.pdf", "--json", "--fields", "page"]);
    assert_eq!(
        (typo.code, typo.stderr_json()[0]["error"]["didYouMean"][0].as_str()),
        (2, Some("pages"))
    );
}

#[test]
fn warnings_should_go_to_standard_error_in_text_and_into_the_report_in_json() {
    let text = laminapdf(&["info", "fixtures/corpus/hello-broken.pdf"]);
    assert!(text.stderr.starts_with("warning[xref-repaired]: "), "{}", text.stderr);
    let json = laminapdf(&["info", "fixtures/corpus/hello-broken.pdf", "--json"]);
    assert_eq!((json.code, json.stderr.as_str()), (3, ""));
    assert_eq!(json.json()["warnings"][0]["code"], "xref-repaired");
    let quiet = run(&["info", "fixtures/corpus/hello-broken.pdf", "--quiet"], None, &[]);
    assert_eq!((quiet.code, quiet.stderr.as_str()), (3, ""));
}
