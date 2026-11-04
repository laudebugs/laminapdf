//! Nothing waits for input: every command finishes with standard input closed (nextest's
//! timeout catches a hang), and `-` on a closed standard input is an error, not a wait.

mod common;

use common::{folder, laminapdf};

#[test]
fn every_command_should_finish_with_standard_input_closed() {
    let out = folder("no-prompt").join("out.pdf");
    let out = out.to_str().unwrap_or_default();
    let commands: [&[&str]; 7] = [
        &["info", "fixtures/corpus/hello.pdf"],
        &["check", "fixtures/corpus/hello.pdf"],
        &["extract", "fixtures/corpus/hello.pdf"],
        &["explain", "xref-repaired"],
        &["pages", "rotate", "fixtures/corpus/hello.pdf", "90", "-o", out],
        &["pages", "rotate", "fixtures/corpus/hello.pdf", "90", "-o", out],
        &["info", "fixtures/text/encrypted/text-shaped-aes-256.pdf", "--no-input"],
    ];
    let codes: Vec<i32> = commands.iter().map(|args| laminapdf(args).code).collect();
    // The second rotate finds its output there: refused, not asked about. The encrypted file
    // needs a password: refused, not asked for.
    assert_eq!(codes, [0, 0, 0, 0, 0, 4, 5]);
}

#[test]
fn standard_input_closed_should_be_an_empty_input() {
    let run = laminapdf(&["info", "-", "--json"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr_json()[0]["error"]["input"], "-");
}
