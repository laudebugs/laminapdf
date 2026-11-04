//! `laminapdf check`: a clean file, a repaired one, and strict mode.

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
