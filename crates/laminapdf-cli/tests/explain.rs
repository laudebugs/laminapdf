//! `laminapdf explain`: a code's page, and a misspelt code.

mod common;

use common::laminapdf;

#[test]
fn a_code_should_print_its_page() {
    let run = laminapdf(&["explain", "wrong-password"]);
    assert_eq!(run.code, 0);
    assert!(run.stdout_text().starts_with("# `wrong-password`\n"));
}

#[test]
fn the_report_should_carry_the_code_and_its_page() {
    let report = laminapdf(&["explain", "overwrite-refused", "--json"]).json();
    assert_eq!(
        (report["type"].as_str(), report["kind"].as_str()),
        (Some("error"), Some("state"))
    );
    assert!(
        report["docsUrl"]
            .as_str()
            .is_some_and(|u| u.ends_with("/docs/errors/overwrite-refused.md"))
    );
}

#[test]
fn a_misspelt_code_should_suggest_the_right_one() {
    let run = laminapdf(&["explain", "wrong-pasword"]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("= did you mean: wrong-password"), "{}", run.stderr);
}
