//! The proof record: one line of `history.jsonl` on `bench-data` (`kind: "proof"`), and
//! `target/nightly/proof/proof.json`. A section is `None` (`null`) when its input is missing, so a
//! failed job never reads as a zero; `jobs` says what happened to it. The record holds counts,
//! versions and results only: no corpus file name and no text read from any file.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::harness::FuzzRun;
use crate::pdf_runtimes::RuntimeResults;
use crate::roundtrip;
use crate::verapdf;

/// The record's `kind`, which keeps it apart from the benchmark and render-diff records.
pub(crate) const KIND: &str = "proof";
/// The record format's version; a change T1b's reader can't handle bumps it.
pub(crate) const VERSION: u32 = 1;

/// One night's robustness figures. It has no `benches` key, so the benchmark comparisons
/// (`nightly::regressions`, `viewer_perf`'s) pass over it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Record {
    /// Always [`KIND`].
    pub(crate) kind: String,
    /// [`VERSION`].
    pub(crate) version: u32,
    /// The commit the night tested.
    pub(crate) commit: String,
    /// That commit's date (ISO 8601, `git show -s --format=%cI`).
    pub(crate) date: String,
    /// The day `cargo xtask proof` ran, `YYYY-MM-DD` (UTC).
    pub(crate) ran: String,
    /// The runner label (`--runner`), `local` when none was given.
    pub(crate) runner: String,
    /// `true` when the night wasn't [`Record::full`], as the other records mark short runs.
    pub(crate) quick: bool,
    /// The fixed group's version (every crate and npm package of the release).
    pub(crate) lamina: String,
    /// Whether the night ran its full budgets: an unsampled differential and render diff, 20
    /// pages per file, and every fuzz target for at least 600 s. Worked out from the reports.
    pub(crate) full: bool,
    /// Each nightly job's result (`success`, `failure`, `cancelled`, `skipped`), from `--jobs`;
    /// empty when none was given (a local run).
    pub(crate) jobs: BTreeMap<String, String>,
    /// Lamina PDF's outcome per external source (`pdfjs`, `pdfbox`, `pdfium`, `qpdf`) and for
    /// `mutants`.
    pub(crate) corpus: Option<BTreeMap<String, CorpusSource>>,
    pub(crate) differential: Option<Differential>,
    pub(crate) text: Option<Text>,
    pub(crate) render: Option<Render>,
    /// Per fuzz target that ran: a target missing here didn't run or failed before fuzzing.
    pub(crate) fuzz: Option<BTreeMap<String, FuzzRun>>,
    pub(crate) verapdf: Option<verapdf::Report>,
    pub(crate) roundtrip: Option<roundtrip::Summary>,
    pub(crate) runtimes: Option<RuntimeResults>,
}

/// How Lamina PDF read one source's files.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CorpusSource {
    pub(crate) files: u64,
    /// Opened with no repair.
    pub(crate) opened: u64,
    /// Opened after a repair (ADR 0003).
    pub(crate) repaired: u64,
    /// Refused with an error (`failed`).
    pub(crate) refused: u64,
    /// Refused while at least one other tool opened the file.
    pub(crate) refused_alone: u64,
    pub(crate) errors: Errors,
}

/// Files Lamina PDF failed to read at all: none of them is an acceptable outcome.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Errors {
    pub(crate) panic: u64,
    pub(crate) crash: u64,
    pub(crate) timeout: u64,
}

/// The structural differential (`cargo xtask differential --live`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Differential {
    /// The inputs it read: external files and mutants.
    pub(crate) inputs: u64,
    /// `--sample`, `None` when every input was read.
    pub(crate) sample: Option<u64>,
    /// Per tool.
    pub(crate) tools: BTreeMap<String, DifferentialTool>,
    /// Facts on which Lamina PDF disagreed with every tool, unexcused.
    pub(crate) failures: u64,
    /// Files Lamina PDF panicked, crashed or hung on.
    pub(crate) lamina_errors: u64,
}

/// One tool's run in the differential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct DifferentialTool {
    pub(crate) version: String,
    pub(crate) files: u64,
    pub(crate) timeouts: u64,
    pub(crate) crashes: u64,
    /// Per fact (`outcome`, `page_count`, `outline_titles`).
    pub(crate) facts: BTreeMap<String, Tally>,
}

/// How often Lamina PDF and a tool agreed on a fact.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Tally {
    pub(crate) agree: u64,
    pub(crate) disagree: u64,
    /// Disagreements an exceptions file excuses.
    pub(crate) excused: u64,
}

/// The text differential (`cargo xtask differential --live --text`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Text {
    /// Each tool's version, by name (`pdfjs`, `pdftotext`).
    pub(crate) tools: BTreeMap<String, String>,
    pub(crate) sources: BTreeMap<String, TextSource>,
}

/// Page text per source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TextSource {
    pub(crate) files: u64,
    pub(crate) pages: u64,
    /// Pages that at least one tool read, so there was something to compare.
    pub(crate) compared: u64,
    /// Compared pages where Lamina PDF matched at least one tool.
    pub(crate) agree: u64,
}

/// The render diff (`cargo xtask render-diff`, ADR 0004 §6's metric).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Render {
    pub(crate) dpi: u32,
    /// `--pages`, `None` for every page.
    pub(crate) pages_per_file: Option<u32>,
    /// `--sample`, `None` for every file.
    pub(crate) sample: Option<u64>,
    /// Per source, and `all`.
    pub(crate) sources: BTreeMap<String, RenderSource>,
}

/// One source's rendered pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RenderSource {
    pub(crate) rows: u64,
    /// Rows per class (`pass`, `minor`, `fail`, and the error classes).
    pub(crate) classes: BTreeMap<String, u64>,
    /// Pass and minor over the rows but `io-error`, in basis points (10,000 = 100 %).
    pub(crate) pass_minor_bp: u64,
}
