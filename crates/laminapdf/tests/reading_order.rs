//! Reading order (todo U2b): `fixtures/reading-order/` holds generated files with their blocks
//! in reading order (`crates/laminapdf-build/tests/common/reading_order.rs`). This scores what
//! `Document::page_texts` returns against them and fails below the threshold in
//! `fixtures/reading-order/threshold.json`, a ratchet: it also fails when the score has risen
//! past the threshold, so the threshold is raised with the change that raised the score.
//!
//! The metric, per file (all lines of all pages, in the order extraction returns them):
//!
//! * text is tokenized once per file, into words when any expected block has a space, else
//!   into characters (Chinese); each line is split into the longest stretches of its tokens
//!   that occur in one expected block as whole tokens, and each stretch is assigned to that
//!   block (the block the previous stretch went to when it contains the stretch, else the
//!   first such block not yet started, else the first such block);
//! * **order** = 1 − d(S, E) / max(|S|, |E|), where S is the sequence of blocks the stretches
//!   went to, consecutive repeats merged, E is the blocks in reading order (0, 1, …, n−1) and
//!   d is the Levenshtein distance over block indices;
//! * **text** = the mean over expected blocks of 1 − d(got, want) / max(|got|, |want|) over
//!   characters, `got` being the block's tokens joined as the block is written (with spaces,
//!   or none for text without spaces; two empty texts score 1);
//! * **score** = (order + text) / 2. The corpus score is the mean over files.
#![cfg(feature = "text")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    reason = "test code and helpers: a panic is a test failure; counts are small; scores of exact inputs are exact"
)]

use std::fmt::Write as _;
use std::path::PathBuf;

use laminapdf::extract::TextOptions;
use laminapdf::{Document, OpenOptions};
use serde_json::Value;

fn corpus_dir() -> PathBuf {
    laminapdf_test_fixtures::fixtures_dir().join("reading-order")
}

/// The Levenshtein distance between two sequences.
fn levenshtein<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut current = vec![i + 1; b.len() + 1];
        for (j, y) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(x != y);
            current[j + 1] = substitute.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        previous = current;
    }
    previous[b.len()]
}

/// 1 − d / max length; 1 for two empty sequences.
fn similarity<T: PartialEq>(a: &[T], b: &[T]) -> f64 {
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 1.0;
    }
    1.0 - levenshtein(a, b) as f64 / longest as f64
}

/// A file's scores.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Score {
    order: f64,
    text: f64,
    /// Stretches of extracted text found in no expected block.
    unassigned: usize,
}

impl Score {
    fn total(self) -> f64 {
        f64::midpoint(self.order, self.text)
    }
}

/// `text` as tokens: words when `spaced`, else characters.
fn tokens(text: &str, spaced: bool) -> Vec<String> {
    if spaced {
        text.split_whitespace().map(str::to_owned).collect()
    } else {
        text.chars().filter(|c| !c.is_whitespace()).map(String::from).collect()
    }
}

/// Whether `needle` occurs in `haystack` as a run of whole tokens.
fn contains_run(haystack: &[String], needle: &[String]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// Scores `lines` (extracted, in order) against `blocks` (expected, in reading order). Text
/// is tokenized once per file: by words when any expected block has a space, else by
/// characters (Chinese), so an extraction error is scored the same on every line; stretches
/// match whole tokens only, so a broken word earns nothing.
fn score(blocks: &[String], lines: &[String]) -> Score {
    let spaced = blocks.iter().any(|b| b.contains(' '));
    let joiner = if spaced { " " } else { "" };
    let block_tokens: Vec<Vec<String>> = blocks.iter().map(|b| tokens(b, spaced)).collect();
    let mut assigned: Vec<Vec<String>> = vec![Vec::new(); blocks.len()];
    let mut sequence: Vec<usize> = Vec::new();
    let mut unassigned = 0;
    let mut current: Option<usize> = None;
    for line in lines {
        let line = tokens(line, spaced);
        let mut at = 0;
        while at < line.len() {
            // The longest stretch from `at` that some block contains as whole tokens.
            let mut best: Option<(usize, usize)> = None; // (end, block)
            for end in (at + 1..=line.len()).rev() {
                let stretch = &line[at..end];
                let candidates: Vec<usize> = (0..blocks.len())
                    .filter(|&i| contains_run(&block_tokens[i], stretch))
                    .collect();
                if candidates.is_empty() {
                    continue;
                }
                let pick = current
                    .filter(|c| candidates.contains(c))
                    .or_else(|| candidates.iter().copied().find(|&i| assigned[i].is_empty()))
                    .unwrap_or(candidates[0]);
                best = Some((end, pick));
                break;
            }
            if let Some((end, block)) = best {
                assigned[block].extend(line[at..end].iter().cloned());
                if sequence.last() != Some(&block) {
                    sequence.push(block);
                }
                current = Some(block);
                at = end;
            } else {
                unassigned += 1;
                at += 1;
            }
        }
    }
    let expected: Vec<usize> = (0..blocks.len()).collect();
    let order = similarity(&sequence, &expected);
    let text = if blocks.is_empty() {
        1.0
    } else {
        block_tokens
            .iter()
            .zip(&assigned)
            .map(|(want, got)| {
                let got: Vec<char> = got.join(joiner).chars().collect();
                let want: Vec<char> = want.join(joiner).chars().collect();
                similarity(&got, &want)
            })
            .sum::<f64>()
            / blocks.len() as f64
    };
    Score {
        order,
        text,
        unassigned,
    }
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn identical_extraction_scores_one() {
    let blocks = strings(&["alpha beta gamma", "delta epsilon", "zeta eta theta"]);
    let lines = strings(&["alpha beta", "gamma", "delta epsilon", "zeta eta theta"]);
    let s = score(&blocks, &lines);
    assert_eq!((s.order, s.text, s.unassigned), (1.0, 1.0, 0));
}

#[test]
fn reversed_blocks_score_a_third_for_order_and_keep_their_text() {
    let blocks = strings(&["alpha beta", "gamma delta", "epsilon zeta"]);
    let lines = strings(&["epsilon zeta", "gamma delta", "alpha beta"]);
    let s = score(&blocks, &lines);
    // S = [2, 1, 0] against E = [0, 1, 2]: two substitutions.
    assert!((s.order - 1.0 / 3.0).abs() < 1e-12 && s.text == 1.0, "{s:?}");
}

#[test]
fn a_missing_block_costs_order_and_text() {
    let blocks = strings(&["alpha beta", "gamma delta"]);
    let s = score(&blocks, &strings(&["alpha beta"]));
    assert_eq!((s.order, s.text), (0.5, 0.5));
}

#[test]
fn a_line_across_blocks_is_split_between_them() {
    // Two table cells on one baseline come out as one line.
    let blocks = strings(&["alpha beta", "gamma delta"]);
    let s = score(&blocks, &strings(&["alpha beta gamma delta"]));
    assert_eq!((s.order, s.text, s.unassigned), (1.0, 1.0, 0));
}

#[test]
fn text_without_spaces_is_split_by_character() {
    let blocks = strings(&["你好世界", "中文字体"]);
    let s = score(&blocks, &strings(&["你好世界中文字体"]));
    assert_eq!((s.order, s.text), (1.0, 1.0));
}

#[test]
fn text_without_spaces_over_several_lines_keeps_its_full_score() {
    let blocks = strings(&["你好世界中文字体"]);
    let s = score(&blocks, &strings(&["你好世界", "中文字体"]));
    assert_eq!((s.order, s.text), (1.0, 1.0));
}

#[test]
fn a_broken_word_earns_nothing() {
    let blocks = strings(&["signal pepper canyon"]);
    let s = score(&blocks, &strings(&["ignal pepper canyon"]));
    assert_eq!(s.unassigned, 1, "{s:?}");
}

#[test]
fn words_glued_together_are_scored_the_same_on_any_line() {
    // Two cells drawn without a space between them: the glued token is lost whether or not
    // the rest of its line has spaces.
    let blocks = strings(&["red copper.", "Silver lining"]);
    let alone = score(&blocks, &strings(&["copper.Silver"]));
    let with_others = score(&blocks, &strings(&["red copper.Silver lining"]));
    assert_eq!((alone.unassigned, with_others.unassigned), (1, 1));
}

#[test]
fn extra_text_is_counted_but_not_scored() {
    let blocks = strings(&["alpha beta"]);
    let s = score(&blocks, &strings(&["alpha beta", "page 7"]));
    assert_eq!((s.order, s.text, s.unassigned), (1.0, 1.0, 2));
}

/// One corpus file: its name, the expected blocks in reading order, and its scores.
fn run_corpus() -> Vec<(String, Score)> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(corpus_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "json") && p.file_name().is_some_and(|n| n != "threshold.json"))
        .collect();
    files.sort();
    assert!(files.len() >= 20, "the corpus has {} files", files.len());
    files
        .iter()
        .map(|json| {
            let expected: Value = serde_json::from_slice(&std::fs::read(json).unwrap()).unwrap();
            let blocks: Vec<String> = expected["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b["text"].as_str().unwrap().to_owned())
                .collect();
            let pdf = json.with_extension("pdf");
            let doc = Document::open_bytes(std::fs::read(&pdf).unwrap(), OpenOptions::default()).unwrap();
            let lines: Vec<String> = doc
                .page_texts(&TextOptions::new())
                .unwrap()
                .iter()
                .flat_map(laminapdf::extract::PageText::lines)
                .collect();
            let name = json.file_stem().unwrap().to_string_lossy().into_owned();
            (name, score(&blocks, &lines))
        })
        .collect()
}

/// The score table, as Markdown.
fn table(scores: &[(String, Score)], mean: f64, threshold: f64) -> String {
    let mut out = String::from(
        "## Reading order (todo U2b)\n\n| File | Order | Text | Score | Unassigned |\n| --- | ---: | ---: | ---: | ---: |\n",
    );
    for (name, s) in scores {
        let _ = writeln!(
            out,
            "| `{name}` | {:.3} | {:.3} | {:.3} | {} |",
            s.order,
            s.text,
            s.total(),
            s.unassigned
        );
    }
    let _ = writeln!(out, "\nCorpus mean **{mean:.3}** (threshold {threshold:.3}).");
    out
}

/// How far above the threshold the mean may rise before the threshold must be raised.
const RATCHET_SLACK: f64 = 0.005;

#[test]
fn the_corpus_meets_the_ratchet_threshold() {
    let scores = run_corpus();
    let mean = scores.iter().map(|(_, s)| s.total()).sum::<f64>() / scores.len() as f64;
    let file: Value = serde_json::from_slice(&std::fs::read(corpus_dir().join("threshold.json")).unwrap()).unwrap();
    let threshold = file["mean"].as_f64().unwrap();
    let report = table(&scores, mean, threshold);
    println!("{report}");
    // Into the job summary once per run of the fast lane's `rust` job (not every job that
    // runs the tests).
    if std::env::var("GITHUB_JOB").as_deref() == Ok("rust")
        && let Some(summary) = std::env::var_os("GITHUB_STEP_SUMMARY")
    {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(summary)
            .unwrap();
        f.write_all(report.as_bytes()).unwrap();
    }
    assert!(
        mean >= threshold - 1e-9,
        "reading-order score {mean:.4} fell below the threshold {threshold:.4}\n{report}"
    );
    assert!(
        mean <= threshold + RATCHET_SLACK,
        "reading-order score {mean:.4} rose past the threshold {threshold:.4}: raise `mean` in \
         fixtures/reading-order/threshold.json to {:.4} (the ratchet only goes up)\n{report}",
        (mean * 10_000.0).floor() / 10_000.0
    );
}
