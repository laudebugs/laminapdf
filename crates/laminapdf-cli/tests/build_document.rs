//! `laminapdf build` with a `laminapdf.document/1` document: the golden from a file and from
//! standard input, the template errors with their pointers, fonts and images by name or from the
//! document's folder, and the flags a document refuses.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers: a panic is a test failure"
)]

mod common;

use common::{arg, folder, laminapdf, root, run};
use serde_json::{Value, json};

const INVOICE: &str = "fixtures/documents/invoice.json";
const DATA: &str = "fixtures/documents/invoice-data.json";
/// The fonts the template names, as `examples/golden.rs` gives them.
const FONTS: [&str; 4] = [
    "--font",
    "regular=fixtures/fonts/NotoSans-Regular.ttf",
    "--font",
    "bold=fixtures/fonts/NotoSans-RegularBold.ttc#1",
];

fn invoice(out: &str, data: &str, stdin: Option<&[u8]>, extra: &[&str]) -> common::Run {
    let mut args = vec!["build", INVOICE, "--data", data, "-o", out];
    args.extend(FONTS);
    args.extend(extra);
    run(&args, stdin, &[])
}

/// The one `laminapdf.error/1` line on standard error.
fn error(run: &common::Run) -> Value {
    run.stderr_json().into_iter().next().unwrap()["error"].clone()
}

/// A document `json` in a folder of its own, with `files` beside it.
fn document(name: &str, json: &Value, files: &[(&str, &[u8])]) -> std::path::PathBuf {
    let dir = folder(name);
    for (file, bytes) in files {
        std::fs::write(dir.join(file), bytes).unwrap();
    }
    let path = dir.join("doc.json");
    std::fs::write(&path, serde_json::to_vec(json).unwrap()).unwrap();
    path
}

/// A one-paragraph document whose body style uses the font `font`.
fn with_font(font: &str) -> Value {
    json!({
        "schema": "laminapdf.document/1",
        "styles": { "body": { "font": font, "size": 11 } },
        "blocks": [{ "type": "paragraph", "text": "Hello" }]
    })
}

#[test]
fn a_template_and_its_data_should_build_the_golden() {
    let out = folder("document-golden").join("out.pdf");
    let run = invoice(arg(&out), DATA, None, &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        std::fs::read(&out).unwrap(),
        laminapdf_test_fixtures::fixture("golden/cli-build-invoice.pdf")
    );
    assert!(
        run.stdout_text().starts_with("built 2 pages → "),
        "{}",
        run.stdout_text()
    );
}

#[test]
fn the_data_should_come_from_standard_input_with_a_dash() {
    let out = folder("document-stdin-data").join("out.pdf");
    let data = std::fs::read(root().join(DATA)).unwrap();
    let run = invoice(arg(&out), "-", Some(&data), &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(
        std::fs::read(&out).unwrap(),
        laminapdf_test_fixtures::fixture("golden/cli-build-invoice.pdf")
    );
}

#[test]
fn a_document_on_standard_input_should_be_read_as_one() {
    let out = folder("document-stdin").join("out.pdf");
    let run = run(
        &["build", "-", "-o", arg(&out), "--json"],
        Some(&serde_json::to_vec(&with_font("Helvetica")).unwrap()),
        &[],
    );
    assert_eq!(run.code, 0, "{}", run.stderr);
    assert_eq!(run.json()["pageCount"], 1);
}

#[test]
fn json_on_standard_input_with_another_schema_should_be_refused_not_built_as_markdown() {
    let out = folder("document-stdin-schema").join("out.pdf");
    let run = run(
        &["build", "-", "-o", arg(&out), "--json"],
        Some(br#"{ "schema": "laminapdf.document/2", "blocks": [] }"#),
        &[],
    );
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert_eq!(error(&run)["code"], "invalid-document");
    assert!(!out.exists());
}

#[test]
fn a_font_no_one_gave_should_say_how_to_give_it() {
    let path = document("document-unknown-font", &with_font("regular"), &[]);
    let run = laminapdf(&["build", arg(&path), "-o", arg(&path.with_file_name("out.pdf"))]);
    assert_eq!(run.code, 2);
    assert!(
        run.stderr.contains("= at: /styles/body/font (unknown-font)"),
        "{}",
        run.stderr
    );
    assert!(run.stderr.contains("NAME=FILE"), "{}", run.stderr);
}

#[test]
fn the_report_should_name_the_data() {
    let out = folder("document-report").join("out.pdf");
    let run = invoice(arg(&out), DATA, None, &["--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report = run.json();
    assert_eq!(
        (report["data"].as_str(), report["profiles"].clone()),
        (Some(DATA), json!(["PDF/UA-1"]))
    );
}

#[test]
fn a_misspelt_key_should_exit_2_with_its_pointer_and_the_key_meant() {
    let mut template: Value = serde_json::from_slice(&std::fs::read(root().join(INVOICE)).unwrap()).unwrap();
    template["blocks"][0]["spaceAftr"] = json!(12);
    let path = document("document-misspelt", &template, &[]);
    let out = path.with_file_name("out.pdf");
    let mut args = vec!["build", arg(&path), "--data", DATA, "-o", arg(&out), "--json"];
    args.extend(FONTS);
    let run = laminapdf(&args);
    assert_eq!(run.code, 2, "{}", run.stderr);
    let error = error(&run);
    assert_eq!(
        (
            error["code"].as_str(),
            error["data"]["pointer"].as_str(),
            error["didYouMean"][0].as_str()
        ),
        (
            Some("invalid-document"),
            Some("/blocks/0/spaceAftr"),
            Some("spaceAfter")
        )
    );
    assert!(!out.exists());
}

#[test]
fn data_the_template_points_at_but_lacks_should_exit_2_with_the_pointer() {
    let data = folder("document-missing").join("data.json");
    std::fs::write(&data, br#"{ "invoice": {}, "items": [] }"#).unwrap();
    let out = data.with_file_name("out.pdf");
    let run = invoice(arg(&out), arg(&data), None, &["--json"]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    let error = error(&run);
    assert_eq!(error["code"], "template-data-missing");
    assert!(
        error["data"]["pointer"]
            .as_str()
            .is_some_and(|p| p.starts_with("/invoice/")),
        "{error}"
    );
}

#[test]
fn the_text_error_should_say_where_the_fault_is() {
    let data = folder("document-missing-text").join("data.json");
    std::fs::write(&data, br#"{ "invoice": {}, "items": [] }"#).unwrap();
    let run = invoice(arg(&data.with_file_name("out.pdf")), arg(&data), None, &[]);
    assert_eq!(run.code, 2);
    assert!(run.stderr.contains("  = at: /invoice/"), "{}", run.stderr);
}

#[test]
fn a_font_the_document_names_should_be_read_from_its_folder() {
    let font = std::fs::read(root().join("fixtures/fonts/NotoSans-Regular.ttf")).unwrap();
    let path = document("document-folder-font", &with_font("body.ttf"), &[("body.ttf", &font)]);
    let out = path.with_file_name("out.pdf");
    let run = laminapdf(&[
        "build",
        arg(&path),
        "-o",
        arg(&out),
        "--profile",
        "pdf-ua-1",
        "--title",
        "Hi",
        "--lang",
        "en",
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn a_name_that_is_a_path_elsewhere_should_never_be_read() {
    // The font exists one folder up, but a document's names reach only its own folder.
    let dir = folder("document-escape");
    std::fs::write(
        dir.join("up.ttf"),
        std::fs::read(root().join("fixtures/fonts/NotoSans-Regular.ttf")).unwrap(),
    )
    .unwrap();
    let inner = dir.join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    let path = inner.join("doc.json");
    std::fs::write(&path, serde_json::to_vec(&with_font("../up.ttf")).unwrap()).unwrap();
    let run = laminapdf(&["build", arg(&path), "-o", arg(&inner.join("out.pdf")), "--json"]);
    assert_eq!(run.code, 2, "{}", run.stderr);
    let error = error(&run);
    assert_eq!(
        (error["code"].as_str(), error["data"]["reason"].as_str()),
        (Some("invalid-document"), Some("unknown-font"))
    );
}

#[test]
fn a_given_font_should_win_over_a_file_of_its_name() {
    // `body.ttf` beside the document isn't a font; --font gives the real one.
    let path = document(
        "document-given-font",
        &with_font("body.ttf"),
        &[("body.ttf", b"not a font")],
    );
    let font = root().join("fixtures/fonts/NotoSans-Regular.ttf");
    let given = format!("body.ttf={}", font.display());
    let run = laminapdf(&[
        "build",
        arg(&path),
        "-o",
        arg(&path.with_file_name("out.pdf")),
        "--font",
        &given,
    ]);
    assert_eq!(run.code, 0, "{}", run.stderr);
}

#[test]
fn flags_for_the_other_input_should_be_usage_errors() {
    let path = document("document-flags", &with_font("Helvetica"), &[]);
    let out = path.with_file_name("out.pdf");
    let page_size = laminapdf(&["build", arg(&path), "-o", arg(&out), "--page-size", "letter"]);
    let md = folder("document-flags").join("in.md");
    std::fs::write(&md, "# Hi\n").unwrap();
    let data_for_markdown = laminapdf(&["build", arg(&md), "-o", arg(&out), "--data", arg(&path)]);
    let both_stdin = run(&["build", "-", "--data", "-", "-o", arg(&out)], Some(b"{}"), &[]);
    assert_eq!(
        (page_size.code, data_for_markdown.code, both_stdin.code),
        (2, 2, 2),
        "{}\n{}\n{}",
        page_size.stderr,
        data_for_markdown.stderr,
        both_stdin.stderr
    );
    assert!(!out.exists());
}
