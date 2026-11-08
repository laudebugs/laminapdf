//! `cargo xtask benchmarks [--no-run]`: writes `apps/docs/src/data/benchmarks.json`, the
//! numbers the docs site's benchmarks page shows, from the reports the measuring tasks write:
//!
//! - `generation`: `cargo xtask pdf-runtimes`' timings per runtime
//!   (`target/xtask/pdf-runtimes/measurements.json`), each runtime's version, and criterion's
//!   estimates for the Rust twins (`target/nightly/bench/results.json`, from `cargo xtask bench`);
//! - `sizes`: `cargo xtask size`'s gzip and brotli bytes per module and for the JS
//!   (`target/size-report.json`);
//! - `extraction`: the newest `cargo xtask extract-bench` run of each candidate
//!   (`target/bench/extract/<dataset>/<candidate>-results.json`), which this command never runs
//!   (it takes about an hour and downloads the dataset).
//!
//! Without `--no-run` it first runs `size` (after `wasm`), `pdf-runtimes` in every runtime and
//! `bench`. Each section carries its own run stamp (version, commit, date, machine). A report
//! that is missing keeps the committed section, with a warning, so one refresh never erases
//! numbers it couldn't measure. Timings are reported, never asserted (ADR 0018 tier 11).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::step;
use crate::workspace::Workspace;

/// Where the page reads its numbers.
pub(crate) const DATA: &str = "apps/docs/src/data/benchmarks.json";
/// The data file's format: the docs site's reader refuses any other.
const VERSION: u64 = 1;
/// The extraction candidates, in the order the page shows them.
const CANDIDATES: [&str; 2] = ["laminapdf", "laminapdf-stream"];
/// The dataset `extract-bench` scores (`tools/bench/extract-datasets.toml`).
const DATASET: &str = "olmocr-bench";
/// The Rust twins of the runtimes' cases: criterion's name, and the case it matches.
const RUST_BENCHES: [(&str, &str); 2] = [
    ("generate/invoice_one_page", "invoice"),
    ("generate/report_100_pages", "report"),
];

/// What `cargo xtask benchmarks` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    no_run: bool,
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    match args {
        [] => Ok(Args::default()),
        [flag] if flag == "--no-run" => Ok(Args { no_run: true }),
        _ => bail!("usage: cargo xtask benchmarks [--no-run]"),
    }
}

pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    // The machine and its load average as the run starts, before the measuring adds to it.
    let stamp = crate::machine::stamp(&ws.root);
    if !args.no_run {
        crate::wasm::size(ws, false)?;
        crate::pdf_runtimes::run(ws, &crate::pdf_runtimes::Args::default())?;
        crate::nightly::bench(ws, &crate::nightly::parse_bench_args(&[])?)?;
    }
    let path = ws.root.join(DATA);
    let committed: Value = match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).with_context(|| format!("parsing {DATA}"))?,
        Err(_) => json!({}),
    };
    let report = |rel: &str| -> Option<Value> {
        let text = fs::read_to_string(ws.root.join(rel)).ok()?;
        serde_json::from_str(&text).ok()
    };
    let generation = report("target/xtask/pdf-runtimes/measurements.json").map(|measurements| {
        let rust = report("target/nightly/bench/results.json");
        generation(
            &measurements,
            rust.as_ref(),
            &runtime_versions(&ws.root),
            &rust_toolchain(&ws.root),
            &stamp,
        )
    });
    let sizes = report("target/size-report.json").map(|sizes_report| sizes(&sizes_report, &stamp));
    let mut extraction = Map::new();
    for candidate in CANDIDATES {
        let rel = format!("target/bench/extract/{DATASET}/{candidate}-results.json");
        match report(&rel) {
            Some(results) => {
                extraction.insert(candidate.to_owned(), results);
            }
            None => {
                if let Some(kept) = committed.pointer(&format!("/extraction/{candidate}")) {
                    eprintln!("::warning::benchmarks: no {rel}; keeping the committed {candidate} extraction section");
                    extraction.insert(candidate.to_owned(), kept.clone());
                }
            }
        }
    }
    let data = json!({
        "version": VERSION,
        "generation": keep(generation, &committed, "generation"),
        "sizes": keep(sizes, &committed, "sizes"),
        "extraction": Value::Object(extraction),
    });
    fs::create_dir_all(path.parent().context("the data file has a folder")?)?;
    fs::write(&path, serde_json::to_string_pretty(&data)? + "\n")?;
    step(&format!("benchmarks: wrote {DATA}"));
    Ok(())
}

/// `fresh`, else the committed file's `key` (with a warning), else `null`.
fn keep(fresh: Option<Value>, committed: &Value, key: &str) -> Value {
    fresh.unwrap_or_else(|| {
        eprintln!("::warning::benchmarks: nothing measured for `{key}`; keeping the committed section");
        committed.get(key).cloned().unwrap_or(Value::Null)
    })
}

/// The `generation` section: one row per runtime with its cases in milliseconds (memory in
/// bytes), and the Rust twins in milliseconds.
fn generation(
    measurements: &Value,
    rust: Option<&Value>,
    versions: &BTreeMap<String, String>,
    toolchain: &str,
    stamp: &Value,
) -> Value {
    let mut by_runtime: BTreeMap<String, Map<String, Value>> = BTreeMap::new();
    for m in measurements.as_array().into_iter().flatten() {
        let (Some(runtime), Some(case), Some(value)) = (
            m.get("runtime").and_then(Value::as_str),
            m.get("case").and_then(Value::as_str),
            m.get("value").and_then(Value::as_f64),
        ) else {
            continue;
        };
        by_runtime
            .entry(runtime.to_owned())
            .or_default()
            .insert(case.to_owned(), json!(value));
    }
    let runtimes: Vec<Value> = by_runtime
        .into_iter()
        .map(|(runtime, cases)| {
            json!({
                "runtime": runtime,
                "version": versions.get(runtime_family(&runtime)),
                "cases": cases,
            })
        })
        .collect();
    let mut rust_cases = Map::new();
    for (bench, case) in RUST_BENCHES {
        if let Some(ns) = rust
            .and_then(|r| r.pointer(&format!("/benches/{}", bench.replace('/', "~1"))))
            .and_then(Value::as_f64)
        {
            rust_cases.insert(case.to_owned(), json!(ns / 1e6));
        }
    }
    json!({
        "run": stamp,
        "runtimes": runtimes,
        "rust": { "toolchain": toolchain, "cases": rust_cases },
    })
}

/// The tool a runtime label runs on: `node-24` and `node` → `node`; the browsers → `playwright`;
/// `workerd` → `wrangler`, which runs it.
fn runtime_family(runtime: &str) -> &str {
    match runtime {
        "chromium" | "firefox" | "webkit" => "playwright",
        "workerd" => "wrangler",
        r if r.starts_with("node") => "node",
        r => r,
    }
}

/// The versions of the tools the runtimes run on: Node on `PATH`, and Deno, Bun, Playwright and
/// wrangler from the pnpm catalog (`pnpm-workspace.yaml`), which pins what the suite installs.
fn runtime_versions(root: &Path) -> BTreeMap<String, String> {
    let catalog = fs::read_to_string(root.join("pnpm-workspace.yaml")).unwrap_or_default();
    let mut versions = catalog_versions(&catalog, &["deno", "bun", "playwright", "wrangler"]);
    if let Ok(out) = std::process::Command::new("node").arg("--version").output()
        && out.status.success()
    {
        let v = String::from_utf8_lossy(&out.stdout)
            .trim()
            .trim_start_matches('v')
            .to_owned();
        versions.insert("node".to_owned(), v);
    }
    versions
}

/// The versions `names` have in the default `catalog:` of a `pnpm-workspace.yaml`.
fn catalog_versions(yaml: &str, names: &[&str]) -> BTreeMap<String, String> {
    let mut in_catalog = false;
    let mut out = BTreeMap::new();
    for line in yaml.lines() {
        if !line.starts_with(' ') && !line.starts_with('#') && !line.is_empty() {
            in_catalog = line.trim_end() == "catalog:";
            continue;
        }
        let Some((key, value)) = line.trim().split_once(':') else {
            continue;
        };
        let key = key.trim().trim_matches('"');
        if in_catalog && names.contains(&key) {
            out.insert(key.to_owned(), value.trim().trim_matches('"').to_owned());
        }
    }
    out
}

/// The Rust toolchain `rust-toolchain.toml` pins (`channel`).
fn rust_toolchain(root: &Path) -> String {
    let text = fs::read_to_string(root.join("rust-toolchain.toml")).unwrap_or_default();
    toml::from_str::<toml::Value>(&text)
        .ok()
        .and_then(|t| t.get("toolchain")?.get("channel")?.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// The `sizes` section from `cargo xtask size`'s report: the WebAssembly modules (raw, gzip,
/// brotli) and the JS bundles (minified, gzip), with their budgets.
fn sizes(report: &Value, stamp: &Value) -> Value {
    let mut modules = Vec::new();
    let mut js = Vec::new();
    for (name, entry) in report.as_object().into_iter().flatten() {
        if let Some(wasm) = entry.get("wasm") {
            modules.push(json!({
                "name": name,
                "raw": wasm.get("raw"),
                "gzip": wasm.get("gzip"),
                "brotli": wasm.get("brotli"),
                "budgetGzip": entry.pointer("/budget/wasmGzip"),
            }));
        } else if let Some(min) = entry.get("jsMinified") {
            js.push(json!({
                "name": name,
                "minified": min.get("raw"),
                "gzip": min.get("gzip"),
                "budgetGzip": entry.pointer("/budget/jsGzip"),
            }));
        }
    }
    json!({ "run": stamp, "modules": modules, "js": js })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stamp() -> Value {
        json!({ "version": "0.2.0", "commit": "abc", "date": "2026-11-08", "machine": {} })
    }

    #[test]
    fn parse_args_should_take_no_run_alone() {
        assert_eq!(parse_args(&[]).unwrap(), Args::default());
        assert!(parse_args(&["--no-run".into()]).unwrap().no_run);
        assert!(parse_args(&["--quick".into()]).is_err());
    }

    #[test]
    fn generation_should_group_the_cases_by_runtime_with_its_tools_version() {
        let measurements = json!([
            { "runtime": "deno", "case": "invoice", "value": 2.5 },
            { "runtime": "deno", "case": "report", "value": 120.0 },
            { "runtime": "chromium", "case": "invoice", "value": 3.0 },
        ]);
        let versions = BTreeMap::from([("deno".into(), "2.9.6".into()), ("playwright".into(), "1.64.1".into())]);
        let g = generation(&measurements, None, &versions, "1.99.1", &stamp());
        assert_eq!(
            g["runtimes"],
            json!([
                { "runtime": "chromium", "version": "1.64.1", "cases": { "invoice": 3.0 } },
                { "runtime": "deno", "version": "2.9.6", "cases": { "invoice": 2.5, "report": 120.0 } },
            ])
        );
    }

    #[test]
    fn generation_should_turn_criterions_nanoseconds_into_milliseconds() {
        let rust =
            json!({ "benches": { "generate/report_100_pages": 45_000_000, "generate/invoice_one_page": 1_500_000 } });
        let g = generation(&json!([]), Some(&rust), &BTreeMap::new(), "1.99.1", &stamp());
        assert_eq!(
            g["rust"],
            json!({ "toolchain": "1.99.1", "cases": { "invoice": 1.5, "report": 45.0 } })
        );
    }

    #[test]
    fn runtime_family_should_map_labels_to_the_tool_that_runs_them() {
        let got: Vec<&str> = ["node-24", "node", "webkit", "workerd", "bun"]
            .map(runtime_family)
            .to_vec();
        assert_eq!(got, ["node", "node", "playwright", "wrangler", "bun"]);
    }

    #[test]
    fn catalog_versions_should_read_only_the_default_catalog() {
        let yaml = "packages:\n  - packages/*\ncatalog:\n  # a comment\n  deno: 2.9.6\n  \"bun\": 1.4.4\n  vite: 8.3.5\n\
                    catalogs:\n  old:\n    deno: 1.0.0\n";
        assert_eq!(
            catalog_versions(yaml, &["deno", "bun"]),
            BTreeMap::from([("bun".into(), "1.4.4".into()), ("deno".into(), "2.9.6".into())])
        );
    }

    #[test]
    fn sizes_should_split_the_modules_from_the_js_bundles() {
        let report = json!({
            "base": { "wasm": { "raw": 10, "gzip": 4, "brotli": 3 }, "budget": { "wasmGzip": 5, "glueGzip": 1 } },
            "@laminapdf/sdk": { "jsMinified": { "raw": 8, "gzip": 2 }, "budget": { "jsGzip": 3 } },
        });
        let s = sizes(&report, &stamp());
        assert_eq!(
            s["modules"],
            json!([{ "name": "base", "raw": 10, "gzip": 4, "brotli": 3, "budgetGzip": 5 }])
        );
        assert_eq!(
            s["js"],
            json!([{ "name": "@laminapdf/sdk", "minified": 8, "gzip": 2, "budgetGzip": 3 }])
        );
    }

    #[test]
    fn keep_should_fall_back_to_the_committed_section() {
        let committed = json!({ "sizes": { "modules": [] } });
        assert_eq!(keep(None, &committed, "sizes"), json!({ "modules": [] }));
        assert_eq!(keep(Some(json!(1)), &committed, "sizes"), json!(1));
    }
}
