//! Commands over several inputs: results in argument order, NDJSON lines that match their
//! schemas, the batch exit codes, `--fail-fast`, `--files-from` and `-0`.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers: a panic is a test failure"
)]

mod common;

use common::schemas::Validator;
use common::{arg, folder, laminapdf, run};
use serde_json::Value;

/// Standard output as one JSON value per line.
fn lines(stdout: &[u8]) -> Vec<Value> {
    String::from_utf8(stdout.to_vec())
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
        .collect()
}

/// 50 inputs, a large file first, so the later small ones finish first.
fn fifty() -> Vec<&'static str> {
    let files = [
        "fixtures/golden/build-text-multiscript.pdf",
        "fixtures/corpus/hello.pdf",
        "fixtures/corpus/hello-broken.pdf",
        "fixtures/corpus/outline.pdf",
        "fixtures/corpus/text-example.pdf",
    ];
    (0..50).map(|i| files[i % files.len()]).collect()
}

#[test]
fn fifty_inputs_should_come_back_in_argument_order_as_valid_ndjson() {
    let inputs = fifty();
    let mut args = vec!["check"];
    args.extend(&inputs);
    args.extend(["--jobs", "8", "--json"]);
    let run = laminapdf(&args);
    assert_eq!(run.code, 3, "hello-broken has warnings: {}", run.stderr);
    let lines = lines(&run.stdout);
    let validator = Validator::new();
    let (summary, results) = lines.split_last().unwrap();
    for (i, line) in results.iter().enumerate() {
        validator.check("result-line.schema.json", line);
        assert_eq!((line["index"].as_u64(), line["input"].as_str()), (Some(i as u64), Some(inputs[i])));
    }
    validator.check("summary-line.schema.json", summary);
    assert_eq!(
        (&summary["ok"], &summary["warnings"], &summary["failed"]),
        (&Value::from(40), &Value::from(10), &Value::from(0))
    );
}

#[test]
fn the_exit_code_should_say_how_the_batch_went() {
    let ok = "fixtures/corpus/hello.pdf";
    let warns = "fixtures/corpus/hello-broken.pdf";
    let locked = "fixtures/text/encrypted/text-shaped-aes-256.pdf";
    let code = |inputs: &[&str]| {
        let mut args = vec!["info"];
        args.extend(inputs);
        laminapdf(&args).code
    };
    assert_eq!(code(&[ok, ok]), 0);
    assert_eq!(code(&[ok, warns]), 3);
    assert_eq!(code(&[ok, "fixtures/corpus/missing.pdf", warns]), 1);
    assert_eq!(code(&[ok, locked, locked]), 5);
    assert_eq!(code(&[locked, "fixtures/corpus/missing.pdf"]), 1);
    assert_eq!(laminapdf(&["info", ok, warns, "--warnings-ok"]).code, 0);
    assert_eq!(laminapdf(&["info", "-", "-"]).code, 2);
}

#[test]
fn an_error_should_be_a_result_line_on_stdout() {
    let run = laminapdf(&["info", "fixtures/corpus/missing.pdf", "fixtures/corpus/hello.pdf", "--json"]);
    let lines = lines(&run.stdout);
    assert_eq!(lines[0]["status"], "error");
    assert_eq!(lines[0]["error"]["code"], "input-unreadable");
    assert_eq!(lines[1]["status"], "ok");
    assert_eq!((run.code, run.stderr.as_str()), (1, ""));
}

#[test]
fn fail_fast_should_skip_what_comes_after_a_failure() {
    let mut args = vec!["info", "fixtures/corpus/hello.pdf", "fixtures/corpus/missing.pdf"];
    args.extend(["fixtures/corpus/hello.pdf"; 10]);
    args.extend(["--fail-fast", "--jobs", "1", "--json"]);
    let lines = lines(&laminapdf(&args).stdout);
    let summary = lines.last().unwrap();
    assert_eq!((lines.len(), &summary["skipped"], &summary["failed"]), (3, &Value::from(10), &Value::from(1)));
}

#[test]
fn files_from_should_add_inputs_one_per_line_or_nul_separated() {
    let list = folder("batch").join("list.txt");
    std::fs::write(&list, "fixtures/corpus/hello.pdf\n\nfixtures/corpus/outline.pdf\n").unwrap();
    let from_file = laminapdf(&["info", "fixtures/corpus/raw.pdf", "--files-from", arg(&list), "--json"]);
    let inputs: Vec<Value> = lines(&from_file.stdout).iter().map(|l| l["input"].clone()).collect();
    assert_eq!(
        inputs,
        ["fixtures/corpus/raw.pdf", "fixtures/corpus/hello.pdf", "fixtures/corpus/outline.pdf"]
            .map(Value::from)
            .into_iter()
            .chain([Value::Null])
            .collect::<Vec<_>>()
    );
    let nul = run(
        &["check", "--files-from", "-", "-0", "--json"],
        Some(b"fixtures/corpus/hello.pdf\0fixtures/corpus/outline.pdf\0"),
        &[],
    );
    assert_eq!(lines(&nul.stdout).last().unwrap()["ok"], 2);
}

#[test]
fn one_input_should_print_as_before() {
    let one = laminapdf(&["info", "fixtures/corpus/hello.pdf", "--json"]).json();
    assert_eq!((one.get("type"), one["schema"].as_str()), (None, Some("laminapdf.info/1")));
    let text = laminapdf(&["check", "fixtures/corpus/hello.pdf", "fixtures/corpus/outline.pdf"]);
    assert!(text.stdout_text().starts_with("==> fixtures/corpus/hello.pdf <==\n"), "{}", text.stdout_text());
    assert!(text.stderr.contains("2 ok, 0 with warnings, 0 failed, 0 skipped"), "{}", text.stderr);
}
