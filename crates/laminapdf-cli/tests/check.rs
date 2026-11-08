//! `laminapdf check`: a clean file, a repaired one, strict mode, and what failed redactions leave.
#![expect(clippy::unwrap_used, reason = "test helpers: a panic is a test failure")]

mod common;

use common::laminapdf;

#[test]
fn a_clean_file_should_be_ok() {
    let run = laminapdf(&["check", "fixtures/golden/build-text-multiscript.pdf", "--json"]);
    let report = run.json();
    assert_eq!((run.code, report["status"].as_str()), (0, Some("ok")));
    assert!(report["objects"].as_u64() > Some(10));
}

#[test]
fn a_repaired_file_should_list_its_repairs() {
    let run = laminapdf(&["check", "fixtures/corpus/hello-broken.pdf"]);
    assert_eq!(run.code, 3);
    assert_eq!(
        run.stdout_text(),
        "fixtures/corpus/hello-broken.pdf: 1 warning · 1 page · 5 objects\n"
    );
}

#[test]
fn strict_mode_should_fail_on_the_first_repair() {
    let run = laminapdf(&["check", "fixtures/corpus/hello-broken.pdf", "--strict"]);
    assert_eq!(run.code, 1);
    assert!(run.stderr.starts_with("error[strict]: "), "{}", run.stderr);
}

/// A page whose sentence a black box covers, written to a folder of its own.
fn covered() -> std::path::PathBuf {
    let path = common::folder("check-covered").join("covered.pdf");
    std::fs::write(&path, common::page_with_text("Pay John Smith now.", true)).unwrap();
    path
}

#[test]
fn a_box_over_text_should_warn_covered_text_and_exit_3() {
    let path = covered();
    let run = laminapdf(&["check", common::arg(&path)]);
    assert_eq!(run.code, 3);
    assert!(run.stderr.contains("warning[covered-text]"), "{}", run.stderr);
    let report = laminapdf(&["check", common::arg(&path), "--json"]).json();
    let warning = report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["code"] == "covered-text")
        .unwrap();
    assert_eq!(
        (
            warning["location"]["pageIndex"].as_u64(),
            warning["data"]["chars"].as_u64()
        ),
        (Some(0), Some(16))
    );
}

#[test]
fn covered_text_should_obey_warnings_ok_but_not_fail_strict_mode() {
    let path = covered();
    assert_eq!(laminapdf(&["check", common::arg(&path), "--warnings-ok"]).code, 0);
    assert_eq!(laminapdf(&["check", common::arg(&path), "--strict"]).code, 3);
}

#[test]
fn text_an_update_removed_should_warn_text_in_earlier_revision() {
    let base = common::page_with_text("Account 12345678", false);
    let replaced = laminapdf_test_fixtures::stream_body("", b"BT /F1 12 Tf 72 700 Td (Account) Tj ET");
    let path = common::folder("check-revision").join("updated.pdf");
    std::fs::write(&path, common::with_update(&base, 4, &replaced)).unwrap();
    let report = laminapdf(&["check", common::arg(&path), "--json"]).json();
    let codes: Vec<&str> = report["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w["code"].as_str())
        .collect();
    assert_eq!(codes, ["text-in-earlier-revision"]);
    assert_eq!(report["warnings"][0]["data"]["words"], 1);
}
