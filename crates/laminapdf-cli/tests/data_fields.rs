//! Each CLI code's `data` has exactly the keys its `docs/errors/` page lists under "Data fields",
//! from a real run of the binary (the façade's `tests/data_fields.rs` leaves these codes to this
//! file).

mod common;

use std::collections::BTreeSet;

use common::{folder, laminapdf, root};

/// The keys `docs/errors/<code>.md` lists: its "Data fields" bullets, `` - `key`: ``.
fn documented(code: &str) -> BTreeSet<String> {
    let page = std::fs::read_to_string(root().join(format!("docs/errors/{code}.md"))).unwrap_or_default();
    page.split("## Data fields")
        .nth(1)
        .unwrap_or_default()
        .lines()
        .take_while(|line| !line.starts_with("## "))
        .filter_map(|line| line.strip_prefix("- `")?.split_once('`').map(|(key, _)| key.to_owned()))
        .collect()
}

#[test]
fn every_cli_code_should_carry_the_data_keys_its_page_lists() {
    let existing = folder("data-fields").join("out.pdf");
    std::fs::write(&existing, b"x").unwrap_or_default();
    let existing = existing.to_str().unwrap_or_default();
    let runs: [(&str, Vec<&str>); 5] = [
        (
            "invalid-page-range",
            vec!["extract", "fixtures/corpus/hello.pdf", "--pages", "3"],
        ),
        ("unknown-code", vec!["explain", "nope"]),
        (
            "overwrite-refused",
            vec!["pages", "rotate", "fixtures/corpus/hello.pdf", "90", "-o", existing],
        ),
        ("input-unreadable", vec!["info", "no-such-file.pdf"]),
        (
            "output-unwritable",
            vec![
                "pages",
                "rotate",
                "fixtures/corpus/hello.pdf",
                "90",
                "-o",
                "/no/such/folder/out.pdf",
            ],
        ),
    ];
    let mut problems = Vec::new();
    for (code, mut args) in runs {
        args.push("--json");
        let lines = laminapdf(&args).stderr_json();
        let error = lines.first().map(|line| line["error"].clone()).unwrap_or_default();
        let keys: BTreeSet<String> = error["data"]
            .as_object()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default();
        if error["code"] != code || keys != documented(code) {
            problems.push(format!(
                "{code}: got {} with data keys {keys:?}, the page lists {:?}",
                error["code"],
                documented(code)
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
