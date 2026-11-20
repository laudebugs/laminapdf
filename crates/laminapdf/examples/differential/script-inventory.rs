//! How many of the scripts in a set of PDFs Lamina PDF handles ([ADR 0021] §3's evidence): each
//! file's [`Document::form_scripts`], counted per source as handled (a form helper call done
//! natively), not handled, or XFA. The nightly lane runs it over the external corpus and the
//! test forms (`cargo xtask corpus scripts`) and puts the table in its summary.
//!
//! [ADR 0021]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0021-pdf-javascript-policy.md
//!
//! It is not one of the façade's golden examples (`examples/*.rs`), so it lives in a subfolder
//! with its own `path` in `Cargo.toml`.
//!
//! ```sh
//! cargo run --release -p laminapdf --example script-inventory -- <out-dir> <source>=<dir>…
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "a developer tool: failing loudly is the right behaviour"
)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use laminapdf::{Document, OpenOptions, ScriptState};
use serde_json::{Value, json};

/// One source's counts.
#[derive(Default)]
struct Counts {
    files: u64,
    failed: u64,
    with_scripts: u64,
    handled: u64,
    not_handled: u64,
    xfa: u64,
}

fn pdfs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            pdfs(&path, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("pdf")) {
            out.push(path);
        }
    }
}

fn count(path: &Path, counts: &mut Counts) {
    counts.files += 1;
    let Ok(bytes) = std::fs::read(path) else {
        counts.failed += 1;
        return;
    };
    let scripts = catch_unwind(AssertUnwindSafe(|| {
        Document::open_bytes(bytes, OpenOptions::default())
            .ok()
            .and_then(|doc| doc.form_scripts().ok())
    }));
    let Ok(Some(scripts)) = scripts else {
        counts.failed += 1;
        return;
    };
    if !scripts.is_empty() {
        counts.with_scripts += 1;
    }
    for s in scripts {
        match s.state {
            ScriptState::Handled => counts.handled += 1,
            ScriptState::Xfa => counts.xfa += 1,
            _ => counts.not_handled += 1,
        }
    }
}

fn share(handled: u64, all: u64) -> String {
    if all == 0 {
        return "–".to_owned();
    }
    #[expect(clippy::cast_precision_loss, reason = "counts of scripts")]
    let percent = 100.0 * handled as f64 / all as f64;
    format!("{percent:.1} %")
}

fn main() {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().expect("usage: script-inventory <out-dir> <source>=<dir>…"));
    let mut by_source: BTreeMap<String, Counts> = BTreeMap::new();
    for arg in args {
        let (name, dir) = arg.split_once('=').expect("each source as <name>=<dir>");
        let mut files = Vec::new();
        pdfs(Path::new(dir), &mut files);
        let counts = by_source.entry(name.to_owned()).or_default();
        for file in files {
            count(&file, counts);
        }
    }
    let mut md = String::from(
        "### Form scripts (ADR 0021 §3)\n\n| Source | Files | With scripts | Scripts | Handled | Not handled | XFA | Handled share |\n| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    let mut json = serde_json::Map::new();
    for (name, c) in &by_source {
        let all = c.handled + c.not_handled + c.xfa;
        let _ = writeln!(
            md,
            "| {name} | {} | {} | {all} | {} | {} | {} | {} |",
            c.files,
            c.with_scripts,
            c.handled,
            c.not_handled,
            c.xfa,
            share(c.handled, all)
        );
        json.insert(
            name.clone(),
            json!({"files": c.files, "failed": c.failed, "with_scripts": c.with_scripts, "scripts": all,
                   "handled": c.handled, "not_handled": c.not_handled, "xfa": c.xfa}),
        );
    }
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("report.md"), &md).unwrap();
    std::fs::write(
        out.join("report.json"),
        serde_json::to_string_pretty(&Value::Object(json)).unwrap() + "\n",
    )
    .unwrap();
    print!("{md}");
}
