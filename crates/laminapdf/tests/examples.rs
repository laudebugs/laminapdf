//! ADR 0018 tier 3 for the façade's examples: each example's document, built with the fixed
//! file ID, producer and dates below, is byte-identical to its golden in
//! `fixtures/golden/examples/` and to a second run (ADR 0001 determinism). The examples take
//! these values and their input files as parameters, so the fixed values live here.
//! `cargo xtask golden --update` and `cargo xtask examples` write the outputs through
//! `write_every_example`; the other tests only compare.
#![expect(clippy::panic, reason = "test helper: a panic is a test failure")]

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
#[path = "../examples/links_and_outlines.rs"]
mod links_and_outlines;
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

use std::error::Error;
use std::path::Path;

use laminapdf::{BuildOptions, Builder};
use pdf_a_archive::Level;

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

/// A file from `fixtures/`.
fn fixture(path: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(path);
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

type Build = fn() -> Result<Vec<u8>, Box<dyn Error>>;

/// Every example, by the name of its golden (`<example>-<variant>` for an example that writes
/// several).
const EXAMPLES: [(&str, Build); 10] = [
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
];

fn golden(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../fixtures/golden/examples/{name}.pdf"));
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
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden/examples");
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
    here.sort();
    here.dedup();
    assert_eq!((&here, &in_manifest), (&files, &files));
}
