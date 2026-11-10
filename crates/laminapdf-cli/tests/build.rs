//! `laminapdf build`: the golden byte for byte, the JSON report against its schema, the exit
//! codes, the font hint, and images read from the input's folder.
#![expect(clippy::unwrap_used, reason = "test helpers: a panic is a test failure")]

mod common;

use common::schemas::Validator;
use common::{arg, folder, laminapdf};

const REPORT: &str = "fixtures/markdown/report.md";

/// The fonts `examples/golden.rs` builds the golden with.
const FONTS: [&str; 8] = [
    "--font",
    "regular=fixtures/fonts/NotoSans-Regular.ttf",
    "--font",
    "bold=fixtures/fonts/NotoSans-RegularBold.ttc#1",
    "--font",
    "fallback=fixtures/fonts/NotoSansArabic-Regular.ttf",
    "--font",
    "fallback=fixtures/fonts/NotoSansDevanagari-Regular.ttf",
];

fn build(out: &str, extra: &[&str]) -> common::Run {
    let mut args = vec!["build", REPORT, "-o", out];
    args.extend(FONTS);
    args.extend(["--profile", "pdf-ua-1"]);
    args.extend(extra);
    laminapdf(&args)
}

/// A Markdown file `text` in a folder of its own.
fn markdown(name: &str, text: &str) -> std::path::PathBuf {
    let path = folder(name).join("in.md");
    std::fs::write(&path, text).unwrap();
    path
}

#[test]
fn build_should_write_the_golden_and_say_what_it_wrote() {
    let out = folder("golden").join("out.pdf");
    let run = build(arg(&out), &[]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let written = std::fs::read(&out).unwrap();
    assert_eq!(written, laminapdf_test_fixtures::fixture("golden/cli-build-report.pdf"));
    assert!(
        run.stdout_text().starts_with("built 3 pages → "),
        "{}",
        run.stdout_text()
    );
}

#[test]
fn the_json_report_should_match_its_schema() {
    let out = folder("json").join("out.pdf");
    let run = build(arg(&out), &["--json"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    let report = run.json();
    Validator::new().check("build.schema.json", &report);
    assert_eq!(
        (
            report["schema"].as_str(),
            report["pageCount"].as_u64(),
            report["tagged"].as_bool(),
            report["profiles"][0].as_str()
        ),
        (Some("laminapdf.build/1"), Some(3), Some(true), Some("PDF/UA-1"))
    );
}

#[test]
fn an_existing_output_should_need_force() {
    let out = folder("exists").join("out.pdf");
    std::fs::write(&out, b"x").unwrap();
    assert_eq!(build(arg(&out), &[]).code, 4);
    assert_eq!(build(arg(&out), &["--force"]).code, 0);
}

#[test]
fn dry_run_should_build_and_write_nothing() {
    let out = folder("dry-run").join("out.pdf");
    let run = build(arg(&out), &["--dry-run", "--json"]);
    assert_eq!(
        (run.code, out.exists(), run.json()["bytes"].as_u64()),
        (0, false, Some(0))
    );
}

#[test]
fn raw_html_should_warn_and_exit_3_unless_warnings_are_ok() {
    let input = markdown("html", "Hello <span>there</span>\n");
    let out = input.with_file_name("out.pdf");
    let run = laminapdf(&["build", arg(&input), "-o", arg(&out)]);
    assert_eq!(run.code, 3, "{}", run.stderr);
    assert!(run.stderr.contains("warning[markdown-html-ignored]"), "{}", run.stderr);
    let ok = laminapdf(&["build", arg(&input), "-o", arg(&out), "--force", "--warnings-ok"]);
    assert_eq!(ok.code, 0, "{}", ok.stderr);
}

#[test]
fn pdf_ua_without_fonts_should_fail_and_name_the_font_flag() {
    let input = markdown(
        "no-fonts",
        "---\ntitle: Notes\nlang: en\n---\n# Notes\n\nPlain Latin text.\n",
    );
    let out = input.with_file_name("out.pdf");
    let run = laminapdf(&["build", arg(&input), "-o", arg(&out), "--profile", "pdf-ua-1"]);
    // A profile's rules are usage errors (the caller asked for what the input can't give): exit 2.
    assert_eq!(run.code, 2, "{}", run.stderr);
    assert!(run.stderr.contains("error[font-not-embedded]"), "{}", run.stderr);
    assert!(run.stderr.contains("--font regular=FILE"), "{}", run.stderr);
    assert!(!out.exists());
}

#[test]
fn an_image_should_be_read_from_the_input_s_folder() {
    let input = markdown(
        "image",
        "![A photo](pictures/photo.jpg)\n\n![Remote](https://example.com/a.png)\n",
    );
    let pictures = input.with_file_name("pictures");
    std::fs::create_dir_all(&pictures).unwrap();
    std::fs::write(
        pictures.join("photo.jpg"),
        laminapdf_test_fixtures::fixture("images/photo-rgb.jpg"),
    )
    .unwrap();
    let out = input.with_file_name("out.pdf");
    let run = laminapdf(&["build", arg(&input), "-o", arg(&out), "--json", "--warnings-ok"]);
    assert_eq!(run.code, 0, "{}", run.stderr);
    // The local photo is in; the remote image is left out with a warning, never fetched.
    let warnings = run.json()["warnings"].clone();
    let codes: Vec<&str> = warnings
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w["code"].as_str())
        .collect();
    assert_eq!(codes, ["markdown-image-missing"]);
}

#[test]
fn a_missing_image_should_fail_naming_its_file() {
    let input = markdown("missing-image", "![Gone](gone.png)\n");
    let out = input.with_file_name("out.pdf");
    let run = laminapdf(&["build", arg(&input), "-o", arg(&out), "--json"]);
    assert_eq!(run.code, 1, "{}", run.stderr);
    let error = &run.stderr_json()[0]["error"];
    assert_eq!(error["code"].as_str(), Some("input-unreadable"));
    assert!(
        error["data"]["input"].as_str().unwrap().ends_with("gone.png"),
        "{error}"
    );
}

#[test]
fn a_bad_font_role_should_be_a_usage_error() {
    let input = markdown("bad-role", "# Hi\n");
    let out = input.with_file_name("out.pdf");
    for bad in ["serif=x.ttf", "bold", "bold=fixtures/fonts/NotoSans-Regular.ttf"] {
        let run = laminapdf(&["build", arg(&input), "-o", arg(&out), "--font", bad]);
        assert_eq!(run.code, 2, "{bad}: {}", run.stderr);
    }
}
