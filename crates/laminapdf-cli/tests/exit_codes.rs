//! Each exit code of ADR 0031 §5, from a real run of the binary.

mod common;

use common::{folder, laminapdf, run};

const HELLO: &str = "fixtures/corpus/hello.pdf";
const REPAIRED: &str = "fixtures/corpus/hello-broken.pdf";
const ENCRYPTED: &str = "fixtures/text/encrypted/text-shaped-aes-256.pdf";

#[test]
fn success_should_exit_0() {
    assert_eq!(laminapdf(&["info", HELLO]).code, 0);
}

#[test]
fn a_file_that_isnt_a_pdf_should_exit_1() {
    let run = run(&["info", "-"], Some(b"not a pdf at all"), &[]);
    assert_eq!(run.code, 1, "{}", run.stderr);
}

#[test]
fn a_bad_range_or_flag_should_exit_2() {
    assert_eq!(laminapdf(&["extract", HELLO, "--pages", "1-"]).code, 2);
    assert_eq!(laminapdf(&["extract", HELLO, "--pages", "2"]).code, 2);
    assert_eq!(laminapdf(&["info", "--no-such-flag", HELLO]).code, 2);
    assert_eq!(laminapdf(&[]).code, 2);
}

#[test]
fn warnings_only_should_exit_3_unless_warnings_ok() {
    assert_eq!(laminapdf(&["check", REPAIRED]).code, 3);
    assert_eq!(laminapdf(&["check", REPAIRED, "--warnings-ok"]).code, 0);
}

#[test]
fn a_repair_in_strict_mode_should_exit_1() {
    let run = laminapdf(&["check", REPAIRED, "--strict", "--json"]);
    assert_eq!(run.code, 1);
    assert_eq!(run.stderr_json()[0]["error"]["code"], "strict");
}

#[test]
fn replacing_a_file_without_force_should_exit_4() {
    let out = folder("exit-4").join("out.pdf");
    std::fs::write(&out, b"keep me").unwrap();
    let run = laminapdf(&["pages", "rotate", HELLO, "90", "-o", out.to_str().unwrap()]);
    assert_eq!(run.code, 4, "{}", run.stderr);
    assert_eq!(std::fs::read(&out).unwrap(), b"keep me");
}

#[test]
fn a_missing_or_wrong_password_should_exit_5() {
    assert_eq!(laminapdf(&["info", ENCRYPTED]).code, 5);
    assert_eq!(
        run(&["info", ENCRYPTED], None, &[("LAMINA_PDF_PASSWORD", "wrong")]).code,
        5
    );
    assert_eq!(
        run(&["info", ENCRYPTED], None, &[("LAMINA_PDF_PASSWORD", "user")]).code,
        0
    );
}

#[test]
fn a_password_file_should_open_the_document() {
    let file = folder("password-file").join("password.txt");
    std::fs::write(&file, "user\n").unwrap();
    let run = laminapdf(&["info", ENCRYPTED, "--password-file", file.to_str().unwrap(), "--json"]);
    assert_eq!((run.code, run.json()["encrypted"].as_bool()), (0, Some(true)));
}

#[test]
fn the_exit_code_table_should_list_0_to_5() {
    let run = laminapdf(&["help", "exit-codes"]);
    let numbers: Vec<char> = run
        .stdout_text()
        .lines()
        .skip(1)
        .filter_map(|l| l.trim().chars().next())
        .collect();
    assert_eq!((run.code, numbers), (0, vec!['0', '1', '2', '3', '4', '5']));
}
