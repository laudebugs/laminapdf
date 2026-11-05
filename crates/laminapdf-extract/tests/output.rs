//! The serialisers: Markdown goldens for the reading-order corpus, JSON that validates against
//! the committed schema (and a schema that matches the Rust types), chunks that slice the
//! Markdown exactly, and hidden text in none of the text outputs.
//!
//! `UPDATE_GOLDEN=1 cargo test -p laminapdf-extract --all-features --test output` rewrites
//! `fixtures/extract/markdown/` and `schemas/extract.schema.json`.
#![cfg(feature = "schema")]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use std::path::PathBuf;

use laminapdf_build::{BuildOptions, Builder, PageSize, Standard14, TextRenderMode};
use laminapdf_extract::{
    ChunkOptions, ExtractOptions, Extraction, JsonOptions, MarkdownOptions, SplitOn, extract, json_schema, to_json,
    tokens_approx,
};
use laminapdf_model::{Document, OpenOptions, PointPt, Pt};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::CancelToken;
use laminapdf_test_fixtures::fixtures_dir;

fn update() -> bool {
    std::env::var_os("UPDATE_GOLDEN").is_some()
}

fn run(doc: &Document) -> Extraction {
    extract(doc, &ExtractOptions::new(), &CancelToken::new()).unwrap()
}

/// The reading-order corpus's files, by stem.
fn corpus() -> Vec<(String, Document)> {
    let dir = fixtures_dir().join("reading-order");
    let mut names: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "pdf"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|p| {
            let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
            (
                stem,
                Document::open(std::fs::read(&p).unwrap(), OpenOptions::default()).unwrap(),
            )
        })
        .collect()
}

/// A page with visible text, a hidden line (text rendering mode 3) and a heading-sized line.
fn with_hidden_text() -> Document {
    let mut b = Builder::new(BuildOptions::default());
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.text(
        "Visible words on the page.",
        &font,
        Pt::new(11.0),
        PointPt::from_points(72.0, 100.0),
    )
    .unwrap();
    s.text(
        "SECRET hidden line",
        &font,
        Pt::new(11.0),
        PointPt::from_points(72.0, 300.0),
    )
    .unwrap()
    .render_mode(TextRenderMode::Invisible);
    s.text(
        "More visible text below.",
        &font,
        Pt::new(11.0),
        PointPt::from_points(72.0, 500.0),
    )
    .unwrap();
    page.finish().unwrap();
    let mut doc = b.finish().unwrap();
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new()).unwrap();
    Document::open(bytes, OpenOptions::default()).unwrap()
}

#[test]
fn markdown_for_each_corpus_file_equals_its_golden() {
    let dir = fixtures_dir().join("extract/markdown");
    let mut mismatched = Vec::new();
    for (stem, doc) in corpus() {
        let markdown = run(&doc)
            .to_markdown(&MarkdownOptions::new().with_page_markers(true))
            .text;
        let path = dir.join(format!("{stem}.md"));
        if update() {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(&path, &markdown).unwrap();
        } else if std::fs::read_to_string(&path).ok().as_deref() != Some(markdown.as_str()) {
            mismatched.push(stem);
        }
    }
    assert!(
        mismatched.is_empty(),
        "Markdown differs from the golden for {mismatched:?}"
    );
}

#[test]
fn the_committed_schema_matches_the_rust_types() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/extract.schema.json");
    let generated = serde_json::to_string_pretty(&json_schema()).unwrap() + "\n";
    if update() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &generated).unwrap();
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        generated,
        "schemas/extract.schema.json is stale: run with UPDATE_GOLDEN=1"
    );
}

#[test]
fn json_output_validates_against_the_committed_schema() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas/extract.schema.json");
    let schema: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource("urn:extract", schema).unwrap();
    let index = compiler.compile("urn:extract", &mut schemas).unwrap();
    let mut docs = corpus();
    docs.push(("hidden".to_owned(), with_hidden_text()));
    for (stem, doc) in docs {
        let out = run(&doc);
        let markdown = out.to_markdown(&MarkdownOptions::new());
        let chunks = out.chunks(&markdown, &ChunkOptions::new());
        let json = to_json(&out, &markdown, &JsonOptions::new().with_chunks(chunks));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        if let Err(e) = schemas.validate(&value, index) {
            panic!("{stem}: {e}");
        }
    }
}

#[test]
fn every_blocks_and_chunks_text_range_slices_the_markdown_to_its_text() {
    for (stem, doc) in corpus() {
        let out = run(&doc);
        let markdown = out.to_markdown(&MarkdownOptions::new().with_page_markers(true));
        for (i, range) in markdown.ranges.iter().enumerate() {
            assert_eq!(markdown.slice(*range), markdown.block(i), "{stem} block {i}");
        }
        for options in [
            ChunkOptions::new().with_max_tokens(40),
            ChunkOptions::new()
                .with_max_tokens(40)
                .with_overlap(15)
                .with_split_on(SplitOn::Blocks),
            ChunkOptions::new().with_split_on(SplitOn::Pages),
        ] {
            for chunk in out.chunks(&markdown, &options) {
                assert_eq!(markdown.slice(chunk.text_range), chunk.text, "{stem} {}", chunk.id);
                assert!(
                    !chunk.pages.is_empty() && !chunk.regions.is_empty(),
                    "{stem} {}",
                    chunk.id
                );
                assert_ne!(chunk.block_ids.len(), 0);
            }
        }
    }
}

#[test]
fn chunks_cover_every_block_in_order_and_overlap_only_when_asked() {
    let (_, doc) = corpus().into_iter().find(|(s, _)| s == "one-column-ordered").unwrap();
    let out = run(&doc);
    let markdown = out.to_markdown(&MarkdownOptions::new());
    let ids: Vec<&str> = out.blocks.iter().map(|b| b.id.as_str()).collect();
    // A budget of about two blocks, and an overlap that fits any one block.
    let largest = (0..ids.len()).map(|i| tokens_approx(markdown.block(i))).max().unwrap();
    let budget = 2 * largest + 2;
    let plain = out.chunks(
        &markdown,
        &ChunkOptions::new()
            .with_max_tokens(budget)
            .with_split_on(SplitOn::Blocks),
    );
    let joined: Vec<&str> = plain
        .iter()
        .flat_map(|c| c.block_ids.iter().map(String::as_str))
        .collect();
    assert_eq!(joined, ids, "without overlap every block is in exactly one chunk");
    assert!(plain.len() > 1);
    let overlapping = out.chunks(
        &markdown,
        &ChunkOptions::new()
            .with_max_tokens(budget)
            .with_overlap(largest)
            .with_split_on(SplitOn::Blocks),
    );
    let total: usize = overlapping.iter().map(|c| c.block_ids.len()).sum();
    assert!(total > ids.len(), "with overlap some blocks repeat");
}

#[test]
fn hidden_text_is_in_no_text_output_and_is_listed_in_dropped() {
    let out = run(&with_hidden_text());
    let markdown = out.to_markdown(&MarkdownOptions::new());
    let text = out.to_text();
    let chunks = out.chunks(&markdown, &ChunkOptions::new());
    assert!(!markdown.text.contains("SECRET") && !text.text.contains("SECRET"));
    assert!(chunks.iter().all(|c| !c.text.contains("SECRET")));
    assert!(markdown.text.contains("Visible words"));
    let json: serde_json::Value = serde_json::from_str(&to_json(&out, &markdown, &JsonOptions::new())).unwrap();
    assert_eq!(json["dropped"][0]["reason"], "invisible");
    assert!(
        json["dropped"][0].get("spans").is_none(),
        "dropped text stays out unless asked"
    );
    assert!(!json["blocks"].to_string().contains("SECRET"));
}

#[test]
fn the_same_input_gives_the_same_json_bytes() {
    let a = run(&with_hidden_text());
    let b = run(&with_hidden_text());
    let (ma, mb) = (
        a.to_markdown(&MarkdownOptions::new()),
        b.to_markdown(&MarkdownOptions::new()),
    );
    assert_eq!(
        to_json(&a, &ma, &JsonOptions::new()),
        to_json(&b, &mb, &JsonOptions::new())
    );
}
