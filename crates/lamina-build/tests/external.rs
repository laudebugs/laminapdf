//! What independent readers see in B5b's goldens (todo B5b, the full-lane checks):
//! `fixtures/build/facts.json` holds pypdf's and pdf.js's text, links, outline, named
//! destinations, page labels, Info and XMP for each golden, and whether `qpdf --check`
//! passes, written by `tools/fixtures/build_facts.py`. The facts must describe the goldens'
//! current bytes and equal the builder's input, except the text lines in
//! `fixtures/build/extraction-exceptions.json` (an extractor's known limitation, each with its
//! reason; an entry that no longer matches the facts fails too).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure (serde_json `Value` indexing returns `Null` when absent)"
)]

mod common;

use std::fmt::Write as _;

use common::b5b::{self, URI, golden_metadata};
use common::fixture;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const EXTRACTORS: [&str; 2] = ["pypdf", "pdfjs"];

fn facts() -> Value {
    serde_json::from_slice(&fixture("build/facts.json")).unwrap()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Each golden and the text lines each of its pages draws.
fn goldens() -> Vec<(&'static str, Vec<Vec<String>>)> {
    let owned = |pages: &[&[&str]]| -> Vec<Vec<String>> {
        pages
            .iter()
            .map(|p| p.iter().map(|l| (*l).to_owned()).collect())
            .collect()
    };
    vec![
        ("build-text-latin.pdf", b5b::text_latin_drawn().1),
        ("build-text-multiscript.pdf", b5b::text_multiscript_drawn().1),
        (
            "links-and-outlines.pdf",
            owned(&[
                &[
                    "Title page",
                    "Visit example.com",
                    "Go to chapter 1",
                    "Go to chapter 2 (named)",
                    "Contents (a linked run)",
                ],
                &["Contents"],
                &["Chapter 1"],
                &["Chapter 2"],
            ]),
        ),
        ("metadata.pdf", owned(&[&["Metadata"]])),
    ]
}

#[test]
fn facts_describe_the_current_goldens() {
    let facts = facts();
    for (name, _) in goldens() {
        let digest = hex(&Sha256::digest(fixture(&format!("golden/{name}"))));
        assert_eq!(
            facts["files"][name]["sha256"].as_str(),
            Some(digest.as_str()),
            "{name}: rerun tools/fixtures/build_facts.py"
        );
    }
}

#[test]
fn qpdf_check_passes() {
    let facts = facts();
    for (name, _) in goldens() {
        assert_eq!(facts["files"][name]["qpdf_check"].as_bool(), Some(true), "{name}");
    }
}

/// `(file, extractor, page, line) → what the extractor returns instead`.
fn exceptions() -> Vec<((String, String, usize, usize), String)> {
    let file: Value = serde_json::from_slice(&fixture("build/extraction-exceptions.json")).unwrap();
    file["exceptions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            assert!(!e["reason"].as_str().unwrap().is_empty(), "{e}");
            let n = |k: &str| usize::try_from(e[k].as_u64().unwrap()).unwrap();
            (
                (
                    e["file"].as_str().unwrap().to_owned(),
                    e["extractor"].as_str().unwrap().to_owned(),
                    n("page"),
                    n("line"),
                ),
                e["extracted"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

#[test]
fn extractors_read_the_drawn_text() {
    let facts = facts();
    let excused = exceptions();
    for (name, pages) in goldens() {
        for extractor in EXTRACTORS {
            let got = &facts["files"][name][extractor]["pages"];
            assert_eq!(got.as_array().map(Vec::len), Some(pages.len()), "{name} {extractor}");
            for (p, want) in pages.iter().enumerate() {
                let lines: Vec<&str> = got[p].as_array().unwrap().iter().map(|l| l.as_str().unwrap()).collect();
                assert_eq!(lines.len(), want.len(), "{name} {extractor} page {p}: {lines:?}");
                for (i, (got, want)) in lines.iter().zip(want).enumerate() {
                    let key = (name.to_owned(), extractor.to_owned(), p, i);
                    match excused.iter().find(|(k, _)| *k == key) {
                        Some((_, instead)) => assert_eq!(got, instead, "{key:?}: the exception is stale"),
                        None => assert_eq!(got, want, "{key:?}"),
                    }
                }
            }
        }
    }
}

#[test]
fn exceptions_are_limited_to_complex_scripts() {
    // Only the Arabic (line 1 of page 1; lines 4–5 of page 2) and Devanagari (line 2; lines 6–7)
    // of the multiscript golden: Latin, Hebrew, CJK and the standard 14 extract exactly.
    for ((file, extractor, page, line), _) in exceptions() {
        assert_eq!(file, "build-text-multiscript.pdf");
        assert!(EXTRACTORS.contains(&extractor.as_str()));
        let allowed: &[usize] = if page == 0 { &[1, 2] } else { &[4, 5, 6, 7] };
        assert!(allowed.contains(&line), "{extractor} page {page} line {line}");
    }
}

#[test]
fn links_resolve_to_the_builders_targets_in_both_readers() {
    let facts = facts();
    let want = json!([[{ "url": URI }, { "page": 2 }, { "page": 3 }, { "page": 1 }], [], [], []]);
    for extractor in EXTRACTORS {
        assert_eq!(
            facts["files"]["links-and-outlines.pdf"][extractor]["links"], want,
            "{extractor}"
        );
    }
}

#[test]
fn outlines_and_named_destinations_match_in_both_readers() {
    let facts = facts();
    let want = [
        ("Title page", 0, Some(0)),
        ("Contents", 0, Some(1)),
        ("Chapters", 0, None),
        ("Chapter 1", 1, Some(2)),
        ("Chapter 2", 1, Some(3)),
        ("Appendix", 2, Some(3)),
        ("Website", 0, None),
    ];
    for extractor in EXTRACTORS {
        let view = &facts["files"]["links-and-outlines.pdf"][extractor];
        let got: Vec<(String, u64, Option<u64>)> = view["outline"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                (
                    i["title"].as_str().unwrap().to_owned(),
                    i["depth"].as_u64().unwrap(),
                    i["page"].as_u64(),
                )
            })
            .collect();
        let want: Vec<(String, u64, Option<u64>)> = want.iter().map(|(t, d, p)| ((*t).to_owned(), *d, *p)).collect();
        assert_eq!(got, want, "{extractor}");
        assert_eq!(
            view["destinations"],
            json!({ "appendix": 3, "chapter-2": 3, "contents": 1 }),
            "{extractor}"
        );
    }
    let pdfjs = &facts["files"]["links-and-outlines.pdf"]["pdfjs"]["outline"];
    assert_eq!(
        (
            pdfjs[6]["url"].as_str(),
            pdfjs[2]["bold"].as_bool(),
            pdfjs[5]["italic"].as_bool()
        ),
        (Some(URI), Some(true), Some(true))
    );
}

#[test]
fn page_labels_read_back_in_both_readers() {
    let facts = facts();
    for extractor in EXTRACTORS {
        assert_eq!(
            facts["files"]["links-and-outlines.pdf"][extractor]["page_labels"],
            json!(b5b::LABELS),
            "{extractor}"
        );
    }
}

#[test]
fn info_and_xmp_carry_the_builders_values_in_both_readers() {
    let facts = facts();
    let meta = golden_metadata();
    let info = json!({
        "Title": meta.title, "Author": meta.author, "Subject": meta.subject, "Keywords": meta.keywords,
        "Creator": meta.creator, "Producer": meta.producer,
        "CreationDate": "D:20261029120000Z", "ModDate": "D:20261030083000+01'00'",
    });
    for extractor in EXTRACTORS {
        let view = &facts["files"]["metadata.pdf"][extractor];
        assert_eq!(view["info"], info, "{extractor}");
        let xmp = &view["xmp"];
        for (key, value) in [
            ("dc:title", &meta.title),
            ("dc:creator", &meta.author),
            ("dc:description", &meta.subject),
            ("pdf:keywords", &meta.keywords),
            ("pdf:producer", &meta.producer),
            ("xmp:creatortool", &meta.creator),
        ] {
            assert_eq!(xmp[key].as_str(), value.as_deref(), "{extractor} {key}");
        }
    }
    // pdf.js keeps the dates as written; pypdf converts them to UTC without a zone.
    let pdfjs = &facts["files"]["metadata.pdf"]["pdfjs"]["xmp"];
    let pypdf = &facts["files"]["metadata.pdf"]["pypdf"]["xmp"];
    assert_eq!(
        (pdfjs["xmp:createdate"].as_str(), pdfjs["xmp:modifydate"].as_str()),
        (Some("2026-10-29T12:00:00Z"), Some("2026-10-30T08:30:00+01:00"))
    );
    assert_eq!(
        (pypdf["xmp:createdate"].as_str(), pypdf["xmp:modifydate"].as_str()),
        (Some("2026-10-29T12:00:00"), Some("2026-10-30T07:30:00"))
    );
}

/// The version `pin` is set to in `text` (`name==1.2.3` or `NAME: &str = "1.2.3";`).
fn pinned(text: &str, pin: &str) -> String {
    text.lines()
        .find_map(|l| l.trim().strip_prefix(pin))
        .map(|rest| rest.trim_matches(|c| c == '"' || c == ';' || c == ' ').to_owned())
        .unwrap()
}

#[test]
fn facts_were_written_with_the_pinned_tools() {
    let facts = facts();
    let requirements = String::from_utf8(fixture("../tools/fixtures/requirements.txt")).unwrap();
    let script = String::from_utf8(fixture("../tools/fixtures/pdfjs_build.mjs")).unwrap();
    assert_eq!(
        (
            facts["pypdf_version"].as_str().unwrap(),
            facts["pdfjs_version"].as_str().unwrap()
        ),
        (
            pinned(&requirements, "pypdf==").as_str(),
            pinned(&script, "const PINNED =").as_str()
        )
    );
}

#[test]
fn facts_and_scripts_use_the_doctor_pins() {
    let facts = facts();
    let doctor = String::from_utf8(fixture("../tools/xtask/src/doctor.rs")).unwrap();
    let script = String::from_utf8(fixture("../tools/fixtures/pdfjs_build.mjs")).unwrap();
    assert_eq!(
        pinned(&script, "const PINNED ="),
        pinned(&doctor, "const PDFJS_DIST_VERSION: &str =")
    );
    assert_eq!(
        facts["qpdf_version"].as_str(),
        Some(pinned(&doctor, "pub(crate) const QPDF_VERSION: &str =").as_str()),
        "fixtures/build/facts.json was written with another qpdf; rerun tools/fixtures/build_facts.py"
    );
}

#[test]
fn every_exception_names_one_drawn_line() {
    let goldens = goldens();
    let mut seen = std::collections::BTreeSet::new();
    for ((file, extractor, page, line), _) in exceptions() {
        let pages = goldens
            .iter()
            .find(|(name, _)| *name == file)
            .map_or_else(|| panic!("{file} is not a B5b golden"), |(_, pages)| pages);
        assert!(
            pages.get(page).is_some_and(|lines| line < lines.len()),
            "{file} has no page {page} line {line}"
        );
        assert!(
            seen.insert((file.clone(), extractor.clone(), page, line)),
            "{file} {extractor} page {page} line {line} is listed twice"
        );
    }
}
