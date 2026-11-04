//! Pipes: `-` reads standard input and `-o -` writes standard output, so `laminapdf` composes
//! with `curl`, `rg` and itself.

mod common;

use common::run;

#[test]
fn a_pdf_should_go_through_standard_input_and_output() {
    let hello = laminapdf_test_fixtures::fixture("corpus/hello.pdf");
    let rotated = run(&["pages", "rotate", "-", "+90", "-o", "-"], Some(&hello), &[]);
    assert_eq!(rotated.code, 0, "{}", rotated.stderr);
    assert_eq!(
        rotated.stdout,
        laminapdf_test_fixtures::fixture("golden/cli-rotate-hello.pdf")
    );
    let info = run(&["info", "-", "--json"], Some(&rotated.stdout), &[]);
    assert_eq!(info.json()["pageSizes"][0]["rotation"], 90);
}

#[test]
fn extracted_text_should_be_plain_on_standard_output() {
    let hello = laminapdf_test_fixtures::fixture("corpus/hello.pdf");
    let text = run(&["extract", "-"], Some(&hello), &[]);
    assert_eq!((text.code, text.stdout_text()), (0, "Hello, World\n".to_owned()));
    assert_eq!(text.stderr, "");
}

#[test]
fn the_report_should_go_to_standard_error_when_standard_output_holds_the_pdf() {
    let hello = laminapdf_test_fixtures::fixture("corpus/hello.pdf");
    let rotated = run(&["pages", "rotate", "-", "90", "-o", "-", "--json"], Some(&hello), &[]);
    assert!(rotated.stdout.starts_with(b"%PDF-"));
    assert_eq!(rotated.stderr_json()[0]["schema"], "laminapdf.pages/1");
}
