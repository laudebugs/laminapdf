//! ADR 0018 tier 3 for the façade's examples: each example's document, built with the fixed
//! file ID, producer and dates below, is byte-identical to its golden in
//! `fixtures/golden/examples/` and to a second run (ADR 0001 determinism). The examples take
//! these values and their input files as parameters, so the fixed values live here.
//! `cargo xtask golden --update` and `cargo xtask examples` write the outputs through
//! `write_every_example`; the other tests only compare.
#![expect(clippy::panic, reason = "test helper: a panic is a test failure")]

#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/edit_metadata.rs"]
mod edit_metadata;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/element_flow.rs"]
mod element_flow;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/extract_markdown.rs"]
mod extract_markdown;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/flowing_invoice.rs"]
mod flowing_invoice;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/flowing_report.rs"]
mod flowing_report;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/generate_then_edit.rs"]
mod generate_then_edit;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/hello.rs"]
mod hello;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/images_and_vectors.rs"]
mod images_and_vectors;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/invoice.rs"]
mod invoice;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/invoice_from_json.rs"]
mod invoice_from_json;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/links_and_outlines.rs"]
mod links_and_outlines;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/markdown_to_pdf.rs"]
mod markdown_to_pdf;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/multilingual.rs"]
mod multilingual;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/pdf_a3_invoice_with_xml.rs"]
mod pdf_a3_invoice_with_xml;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/pdf_a_archive.rs"]
mod pdf_a_archive;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/pdf_ua_report.rs"]
mod pdf_ua_report;
#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/snapshot_test.rs"]
mod snapshot_test;

mod common;

use std::error::Error;
use std::path::Path;

use common::FlowReport;
use laminapdf::{BuildOptions, Builder};
use pdf_a_archive::Level;
use sha2::{Digest, Sha256};
use snapshot_test::Outcome;

const NOTO: &str = "fonts/NotoSans-Regular.ttf";
const SCRIPT_FONTS: [&str; 5] = [
    NOTO,
    "fonts/NotoSansArabic-Regular.ttf",
    "fonts/NotoSansDevanagari-Regular.ttf",
    "fonts/NotoSansHebrew-Regular.ttf",
    "fonts/NotoSansSC-Regular.otf",
];
const PHOTO: &str = "images/photo-rgb.jpg";
/// Each example's fixed file ID (`/ID[0]`, 16 bytes).
const HELLO_ID: &[u8; 16] = b"lamina-ex-hello!";
const INVOICE_ID: &[u8; 16] = b"lamina-ex-invoic";
const MULTILINGUAL_ID: &[u8; 16] = b"lamina-ex-multil";
const IMAGES_AND_VECTORS_ID: &[u8; 16] = b"lamina-ex-vector";
const LINKS_ID: &[u8; 16] = b"lamina-ex-links!";
const EDIT_ID: &[u8; 16] = b"lamina-ex-edit!!";
const FLOW_ID: &[u8; 16] = b"lamina-ex-flow!!";
const FLOWING_ID: &[u8; 16] = b"lamina-ex-flowng";
const FLOWING_INVOICE_ID: &[u8; 16] = b"lamina-ex-flowiv";
const MARKDOWN_ID: &[u8; 16] = b"lamina-ex-mrkdwn";
const JSON_INVOICE_ID: &[u8; 16] = b"lamina-ex-jsoniv";
const PRODUCER: &str = "lamina examples";
const CREATED: &str = "D:20261030120000Z";
const EDITED: &str = "D:20261030130000Z";

/// What a reader's environment would supply, fixed so the output never changes: the file ID,
/// the producer and the creation date.
fn options(id: &[u8; 16]) -> Result<BuildOptions, Box<dyn Error>> {
    Ok(BuildOptions::new()
        .with_id(*id)
        .with_producer(PRODUCER)
        .with_creation_date(CREATED.parse()?))
}

use laminapdf_test_fixtures::{fixture, fixtures_dir};

type Build = fn() -> Result<Vec<u8>, Box<dyn Error>>;

/// Every example, by the name of its golden (`<example>-<variant>` for an example that writes
/// several).
const EXAMPLES: [(&str, Build); 17] = [
    ("hello", || hello::build(options(HELLO_ID)?)),
    ("invoice", || invoice::build(options(INVOICE_ID)?, fixture(NOTO))),
    ("multilingual", || {
        multilingual::build(options(MULTILINGUAL_ID)?, SCRIPT_FONTS.map(fixture).to_vec())
    }),
    ("images_and_vectors", || {
        images_and_vectors::build(options(IMAGES_AND_VECTORS_ID)?, fixture(PHOTO))
    }),
    ("links_and_outlines", || links_and_outlines::build(options(LINKS_ID)?)),
    ("generate_then_edit", || {
        generate_then_edit::build(options(EDIT_ID)?, Some(EDITED.parse()?))
    }),
    ("edit_metadata", || {
        edit_metadata::edit(fixture("golden/metadata.pdf"), EDITED.parse()?)
    }),
    // The same edit on a file that claims PDF/A-2u and PDF/UA-1, so `cargo xtask verapdf` (which
    // validates every example's output) checks that an edit keeps both claims.
    ("edit_metadata-pdfa", || {
        edit_metadata::edit(fixture("golden/examples/pdf_a_archive-2u.pdf"), EDITED.parse()?)
    }),
    ("pdf_ua_report", || pdf_ua_report::build(fixture(NOTO))),
    ("pdf_a_archive-2b", || {
        pdf_a_archive::build(Level::A2b, fixture(NOTO), fixture(PHOTO))
    }),
    ("pdf_a_archive-2u", || {
        pdf_a_archive::build(Level::A2u, fixture(NOTO), fixture(PHOTO))
    }),
    ("pdf_a3_invoice_with_xml", || {
        pdf_a3_invoice_with_xml::build(fixture(NOTO))
    }),
    ("element_flow", || {
        element_flow::build(options(FLOW_ID)?, fixture(PHOTO), &FlowReport::load().as_report())
    }),
    ("flowing_report", || {
        let fonts = [NOTO, "fonts/NotoSans-RegularBold.ttc", SCRIPT_FONTS[1], SCRIPT_FONTS[2]];
        flowing_report::build(options(FLOWING_ID)?, fonts.map(fixture), fixture(PHOTO))
    }),
    ("flowing_invoice", || {
        flowing_invoice::build(
            options(FLOWING_INVOICE_ID)?,
            fixture(NOTO),
            fixture("fonts/NotoSans-RegularBold.ttc"),
        )
    }),
    ("invoice_from_json", || {
        invoice_from_json::build(
            options(JSON_INVOICE_ID)?,
            &fixture("documents/invoice.json"),
            &fixture("documents/invoice-data.json"),
            fixture(NOTO),
            fixture("fonts/NotoSans-RegularBold.ttc"),
        )
    }),
    ("markdown_to_pdf", || {
        let markdown = String::from_utf8(fixture("markdown/report.md"))?;
        let fonts = [NOTO, "fonts/NotoSans-RegularBold.ttc", SCRIPT_FONTS[1], SCRIPT_FONTS[2]];
        markdown_to_pdf::build(options(MARKDOWN_ID)?, &markdown, fonts.map(fixture), fixture(PHOTO))
    }),
];

fn golden(name: &str) -> Vec<u8> {
    let path = fixtures_dir().join(format!("golden/examples/{name}.pdf"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}; run `cargo xtask golden --update`", path.display()))
}

#[test]
fn every_example_matches_its_golden() {
    let stale: Vec<&str> = EXAMPLES
        .iter()
        .filter(|(name, build)| build().unwrap() != golden(name))
        .map(|(name, _)| *name)
        .collect();
    assert!(
        stale.is_empty(),
        "examples differ from fixtures/golden/examples/: {stale:?}"
    );
}

#[test]
fn every_example_gives_the_same_bytes_twice() {
    let differ: Vec<&str> = EXAMPLES
        .iter()
        .filter(|(_, build)| build().unwrap() != build().unwrap())
        .map(|(name, _)| *name)
        .collect();
    assert!(differ.is_empty(), "nondeterministic examples: {differ:?}");
}

#[test]
fn every_example_reopens_without_warnings() {
    for (name, build) in EXAMPLES {
        let doc = laminapdf::Document::open_bytes(build().unwrap(), laminapdf::OpenOptions::new().with_strict(true))
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(doc.page_count().unwrap() > 0, "{name}");
        assert_eq!(doc.warnings().remaining(), 0, "{name}");
    }
}

#[test]
fn the_tagged_invoice_leaves_nothing_untagged() {
    let mut b = Builder::new(options(INVOICE_ID).unwrap().with_tagged(true));
    invoice::draw(&mut b, fixture(NOTO)).unwrap();
    assert_eq!(b.tagging_diagnostics(), []);
}

#[test]
fn generate_then_edit_appends_the_edit_to_the_generated_file() {
    let generated = generate_then_edit::generate(options(EDIT_ID).unwrap()).unwrap();
    let edited = generate_then_edit::build(options(EDIT_ID).unwrap(), None).unwrap();
    assert!(edited.starts_with(&generated) && edited.len() > generated.len());
}

#[test]
fn generate_then_edit_retitles_the_document() {
    let edited = generate_then_edit::build(options(EDIT_ID).unwrap(), None).unwrap();
    let doc = laminapdf::Document::open_bytes(edited, laminapdf::OpenOptions::new()).unwrap();
    assert_eq!(doc.info().unwrap().title.as_deref(), Some("Final report"));
}

/// Writes every example's document into `$LAMINA_PDF_EXAMPLES_OUT`, for `cargo xtask examples`
/// (which compares them with the goldens and runs `qpdf --check`) and `cargo xtask golden
/// --update`. Without the variable it does nothing: the nightly scaling job runs every ignored test.
#[test]
#[ignore = "run by cargo xtask examples and cargo xtask golden --update"]
fn write_every_example() {
    let Some(out) = std::env::var_os("LAMINA_PDF_EXAMPLES_OUT") else {
        return;
    };
    std::fs::create_dir_all(&out).unwrap();
    for (name, build) in EXAMPLES {
        std::fs::write(Path::new(&out).join(format!("{name}.pdf")), build().unwrap()).unwrap();
    }
}

#[test]
fn every_golden_has_an_example() {
    let dir = fixtures_dir().join("golden/examples");
    let orphans: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|f| !EXAMPLES.iter().any(|(name, _)| *f == format!("{name}.pdf")))
        .collect();
    assert!(orphans.is_empty(), "goldens without an example: {orphans:?}");
}

/// The examples are listed here (to build them in process), in `Cargo.toml` (`[[example]]`, for
/// `required-features`) and found by `cargo xtask examples` in the folder: all must agree.
#[test]
fn every_example_file_is_listed_here_and_in_the_manifest() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files: Vec<String> = std::fs::read_dir(root.join("examples"))
        .unwrap()
        .filter_map(|e| {
            let path = e.unwrap().path();
            (path.extension()? == "rs").then(|| path.file_stem().unwrap().to_string_lossy().into_owned())
        })
        .collect();
    files.sort();
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let mut in_manifest: Vec<String> = manifest
        .split("[[example]]")
        .skip(1)
        // An entry with its own `path` (examples/differential/) is a tool, not a golden example.
        .filter(|block| !block.contains("\npath = "))
        .filter_map(|block| block.split('"').nth(1).map(str::to_owned))
        .collect();
    in_manifest.sort();
    // A golden `<example>-<variant>` comes from `<example>.rs`.
    let mut here: Vec<String> = EXAMPLES
        .iter()
        .map(|(name, _)| name.split_once('-').map_or(*name, |(stem, _)| stem).to_owned())
        .collect();
    // Examples that read a PDF and write text have no PDF golden; each has its own test.
    here.extend(READERS.iter().map(|r| (*r).to_owned()));
    here.sort();
    here.dedup();
    assert_eq!((&here, &in_manifest), (&files, &files));
}

/// The examples that read a PDF instead of building one.
const READERS: [&str; 2] = ["extract_markdown", "snapshot_test"];

#[test]
fn extract_markdown_writes_the_invoices_markdown_json_and_chunks() {
    let out = std::env::temp_dir().join(format!("laminapdf-extract-example-{}", std::process::id()));
    std::fs::create_dir_all(&out).unwrap_or_else(|e| panic!("{e}"));
    let invoice = fixture("golden/examples/invoice.pdf");
    extract_markdown::write_outputs(&invoice, &out, "invoice").unwrap_or_else(|e| panic!("{e}"));
    let read = |name: &str| std::fs::read_to_string(out.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    // The whole Markdown is a golden: the docs' Extract Markdown recipe shows it as the example's
    // output. `UPDATE_GOLDEN=1` rewrites it.
    let markdown = read("invoice.md");
    let golden = fixtures_dir().join("extract/examples/invoice.md");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(golden.parent().unwrap_or_else(|| panic!("no parent")))
            .unwrap_or_else(|e| panic!("{e}"));
        std::fs::write(&golden, &markdown).unwrap_or_else(|e| panic!("{e}"));
    }
    let want = std::fs::read_to_string(&golden).unwrap_or_default();
    assert!(
        markdown == want,
        "{} is stale: run with UPDATE_GOLDEN=1",
        golden.display()
    );
    assert!(markdown.starts_with("<!-- page 1 -->\n\n# Invoice 42\n"));
    assert!(read("invoice.json").starts_with(r#"{"schema":"laminapdf.extract/1""#));
    assert!(read("invoice.chunks.jsonl").lines().count() >= 1);
    let _ = std::fs::remove_dir_all(&out);
}

/// `SNAPSHOT_HELLO_SHA256` in `packages/sdk/test/snapshot-test.test.ts`: `hello` built with the
/// TypeScript `TEST_OPTIONS`, which `test_options()` must reproduce byte for byte.
const SNAPSHOT_TEST: &str = include_str!("../../../packages/sdk/test/snapshot-test.test.ts");

fn snapshot_hello_sha256() -> &'static str {
    const PREFIX: &str = "export const SNAPSHOT_HELLO_SHA256 = \"";
    let start = SNAPSHOT_TEST
        .find(PREFIX)
        .unwrap_or_else(|| panic!("snapshot-test.test.ts no longer declares SNAPSHOT_HELLO_SHA256"))
        + PREFIX.len();
    SNAPSHOT_TEST.get(start..start + 64).unwrap_or_default()
}

#[test]
fn snapshot_test_options_give_the_typescript_bytes() {
    let bytes = hello::build(snapshot_test::test_options().unwrap()).unwrap();
    let hash = laminapdf_test_fixtures::hex(&Sha256::digest(&bytes));
    assert_eq!(hash, snapshot_hello_sha256());
}

#[test]
fn snapshot_test_creates_matches_reports_and_updates_its_reference() {
    let dir = std::env::temp_dir().join(format!("laminapdf-snapshot-example-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let reference = dir.join("hello.pdf");
    let pending = dir.join("hello.new.pdf");
    let _ = std::fs::remove_file(&reference);
    let compare = |bytes: &[u8], update| snapshot_test::compare_with_reference(bytes, &reference, update).unwrap();
    let approved = hello::build(snapshot_test::test_options().unwrap()).unwrap();
    let other = hello::build(snapshot_test::test_options().unwrap().with_producer("another app")).unwrap();

    assert_eq!(compare(&approved, false), Outcome::Created);
    assert_eq!(compare(&approved, false), Outcome::Same);
    assert_eq!(compare(&other, false), Outcome::Changed);
    assert_eq!(
        std::fs::read(&reference).unwrap(),
        approved,
        "a change leaves the reference alone"
    );
    assert_eq!(
        std::fs::read(&pending).unwrap(),
        other,
        "and writes the new PDF beside it"
    );
    assert_eq!(compare(&other, true), Outcome::Changed);
    assert_eq!(
        std::fs::read(&reference).unwrap(),
        other,
        "update replaces the reference"
    );
    assert!(!pending.exists());
    let _ = std::fs::remove_dir_all(&dir);
}
