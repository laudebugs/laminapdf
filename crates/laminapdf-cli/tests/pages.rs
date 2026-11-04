//! `laminapdf pages rotate`: the output byte for byte, replacing files only with `--force`, and
//! the JSON report.

mod common;

use common::{arg, folder, laminapdf};

const HELLO: &str = "fixtures/corpus/hello.pdf";

fn golden() -> Vec<u8> {
    laminapdf_test_fixtures::fixture("golden/cli-rotate-hello.pdf")
}

#[test]
fn rotate_should_write_the_golden_as_an_incremental_update() {
    let out = folder("rotate").join("out.pdf");
    let run = laminapdf(&["pages", "rotate", HELLO, "+90", "-o", arg(&out)]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let written = std::fs::read(&out).unwrap();
    assert_eq!(written, golden());
    assert!(written.starts_with(&laminapdf_test_fixtures::fixture("corpus/hello.pdf")));
    assert_eq!(
        run.stdout_text(),
        format!("rotated 1 page → {} (incremental update)\n", arg(&out))
    );
}

#[test]
fn in_place_should_replace_the_input_only_with_force() {
    let copy = folder("in-place").join("hello.pdf");
    std::fs::write(&copy, laminapdf_test_fixtures::fixture("corpus/hello.pdf")).unwrap();
    let refused = laminapdf(&["pages", "rotate", arg(&copy), "+90", "--in-place"]);
    assert_eq!(refused.code, 4);
    assert!(
        refused.stderr.contains("error[overwrite-refused]"),
        "{}",
        refused.stderr
    );
    let done = laminapdf(&["pages", "rotate", arg(&copy), "+90", "--in-place", "--force"]);
    assert_eq!(done.code, 0, "{}", done.stderr);
    assert_eq!(std::fs::read(&copy).unwrap(), golden());
}

#[test]
fn the_report_should_name_each_page_and_its_new_rotation() {
    let out = folder("report").join("out.pdf");
    let run = laminapdf(&[
        "pages",
        "rotate",
        "fixtures/golden/links-and-outlines.pdf",
        "-90:2-z,x3",
        "-o",
        arg(&out),
        "--json",
    ]);
    let report = run.json();
    assert_eq!(
        (report["schema"].as_str(), report["saveMode"].as_str()),
        (Some("laminapdf.pages/1"), Some("incremental"))
    );
    let pages: Vec<(u64, u64, u64)> = report["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["index"].as_u64().unwrap(),
                p["number"].as_u64().unwrap(),
                p["rotation"].as_u64().unwrap(),
            )
        })
        .collect();
    assert_eq!(pages, [(1, 2, 270), (3, 4, 270)]);
}

#[test]
fn dry_run_should_write_nothing() {
    let out = folder("dry-run").join("out.pdf");
    let run = laminapdf(&["pages", "rotate", HELLO, "180", "-o", arg(&out), "--dry-run"]);
    assert_eq!((run.code, out.exists()), (0, false));
}
