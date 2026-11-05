//! lamina's task runner, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
//!
//! This is the single entry point for repo-wide tasks; CI runs the same commands developers run
//! (docs/guidelines/code-organization.md §10). Run `cargo xtask help` for the task list.

mod capabilities;
mod changes;
mod cli_docs;
mod codegen;
mod corpus;
mod docs_links;
mod docs_prose;
mod docs_snippets;
mod doctor;
mod examples;
mod examples_lints;
mod extract_bench;
mod harness;
mod licenses;
mod lines;
mod live;
mod maintainability;
mod nightly;
mod package_docs;
mod pdf_runtimes;
mod pdfium;
mod portable_paths;
mod progress;
mod publish;
mod release;
mod renames;
mod render;
mod render_diff;
mod reports;
mod roundtrip;
mod runner;
mod todos;
mod tracked;
mod ts_structure;
mod util;
mod verapdf;
mod verify_install;
mod version;
mod viewer_perf;
mod wasm;
mod wasm_glue;
mod wasm_module;
#[cfg(test)]
mod workflows;
mod workspace;

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

const USAGE: &str = "\
usage: cargo xtask <task> [args]

tasks:
  check [rust|ts|docs]
                    everything a PR must pass (default: docs, rust and ts); runs docs-prose,
                    docs-snippets, biome-examples and clippy-examples with --all-strict
                    docs: docs-links, docs-prose, docs-snippets, todos --check, portable paths, the capability
                    matrix's static checks (capabilities --check runs the tests),
                    gen codes --check, gen boundary --check and version sync --check
                    rust: fmt --check, maintainability, portability, clippy (native + wasm32), clippy-examples,
                          tests, rustdoc, cargo-deny
                    ts:   biome ci, biome-examples, typecheck and tests of the pnpm
                          workspace, the docs site's and the website's builds (apps/docs,
                          apps/site), npm licences
  check maintainability
                    docs/guidelines/maintainability.md's Rust rules, without a build: modules of at most 800
                    lines of non-test code and no more size-lint #[expect]s than
                    tools/xtask/maintainability-baseline.toml records, no public type name in two crates
                    and no name the lamina façade re-exports twice beyond
                    tools/xtask/maintainability-duplicates.toml (both files only shrink)
  check docs-prose [--all-strict] [--warnings] [--json]
                    reader-facing text (docs site, docs/errors, package READMEs) against
                    docs/guidelines/docs-writing.md: on its own, errors on files changed since the
                    merge base with main (drafts included), counted warnings elsewhere;
                    --all-strict fails on every report, --warnings lists them
                    (--json lists every report, so it takes no --warnings)
  check docs-snippets [--all-strict] [--warnings] [--json]
                    the examples and Snippet's glob against docs/guidelines/examples.md: on its own,
                    strict on changed examples, warnings elsewhere; TS/Rust pairs in
                    tools/xtask/docs-snippets.toml; skips examples/differential/ (a developer tool)
  biome-examples [--all-strict]
                    Biome with --error-on-warnings on the changed TS examples (all with --all-strict)
  clippy-examples [--all-strict] [--warnings] [--json]
                    clippy on lamina's examples (functions of at most 40 lines, no discarded
                    results): on its own, strict on changed examples, warnings elsewhere
  gen codes [--check|--md]
                    validate fixtures/codes.json against fixtures/codes.schema.json and write
                    the error contract from it: ErrorCode/WarningCode
                    (crates/laminapdf-syntax/src/generated/), the façade's explain pages
                    (crates/laminapdf/src/generated/), @laminapdf/sdk's unions, tables and explain
                    pages (packages/sdk/src/generated/), the laminapdf.error/1 JSON Schema
                    (packages/sdk/schemas/, crates/laminapdf/schemas/), @laminapdf/render's code
                    tables (packages/render/src/generated/) and docs/errors/README.md's tables;
                    --check fails naming each stale file; --md prints the tables
  gen boundary [--check]
                    the TypeScript side of the lists that cross the WASM boundary, read from the
                    Rust source: @laminapdf/sdk's builder enums (laminapdf-wasm's name tables,
                    TagKind::as_str) and @laminapdf/render-hayro's HAYRO_VERSION (the hayro pin);
                    --check fails naming each stale file
  fmt [--check]     format Rust (cargo fmt) and TS/JSON (biome); --check only verifies
  lint              clippy native (all targets) and wasm32 (core crates), biome ci
  test              Rust tests (cargo-nextest when installed, else cargo test) + doc tests, TS tests
  portability       core crates: forbidden-API grep, dependency and unsafe rules,
                    cargo check for wasm32, iOS and Android
  wasm              build the wasm32 crates with the size-optimised `wasm-release` profile, then
                    the @laminapdf/wasm modules (lamina_pdf_base, lamina_pdf_text) into packages/wasm/dist/
                    (wasm-bindgen, wasm-opt) and their tests in Node
  size [--update-baseline]
                    measure packages/wasm/dist/ (raw, gzip, brotli; glue min+gzip), write
                    target/size-report.json, fail above packages/wasm/size-budgets.json
  deny              cargo-deny (licences, advisories, bans, sources) + npm licence check
  npm-licenses      npm licence check only (pnpm licenses list)
  docs-links [--external] [--json]
                    check every relative link and #anchor in the tracked Markdown files;
                    --external checks the http(s) URLs over the network instead (nightly;
                    allowlist in .config/link-check.toml, report in target/nightly/links/)
  todos [--check]   regenerate the todo board (docs/todos/README.md) and each todo's Blocks
                    field from the Depends on links; --check only verifies
  progress [--offline]
                    write apps/site/src/data/progress.json, the website's roadmap and dashboard:
                    the board's counts, the roadmap's milestones, merged and open pull requests
                    (gh); --offline keeps the last snapshot's in-progress list
  capabilities [--check]
                    run every test tagged `/// capability: <ID> <surface> <status>` and
                    rewrite docs/capabilities/README.md from what they prove; --check only
                    verifies (and fails if a tagged test fails)
  golden [--update] regenerate every golden in fixtures/golden/ from its source and print the
                    list (--update writes them, the lamina examples' in fixtures/golden/examples/,
                    the reading-order corpus in fixtures/reading-order/, and the shaping goldens
                    in fixtures/shaping/ when hb-shape is installed;
                    without it, fail if they would change)
  examples          run the lamina crate's examples into target/examples-out/, compare them
                    with fixtures/golden/examples/ and run qpdf --check on each (needs qpdf)
  render-golden [--update]
                    render every page of fixtures/golden/examples/ with PDFium (pypdfium2:
                    tools/render/requirements.txt, $LAMINA_PDF_PYTHON) and hayro (tools/render, its
                    own workspace) at 72 dpi, compare with fixtures/render-golden/ (thresholds.toml;
                    diff images in target/xtask/render-golden/), and hayro vs PDFium at 150 dpi;
                    --update rewrites the PNGs (and still fails on a parity disagreement)
                    then laminapdf-render's PDFium and hayro adapters on fixtures/corpus/ at 96 dpi
                    against fixtures/golden/render/ (fetches PDFium first; laminapdf-render-pdfium's
                    ignored tests, or its `goldens` example with --update)
  render-diff [--corpus external] [--source <name>]… [--pages N] [--sample N] [--jobs N] [--top N]
              [--history <dir> [--runner <label>] [--record] [--dry-run]]
                    nightly: every page of the external corpus by laminapdf-render's hayro and PDFium
                    adapters at 96 dpi, compared with the hayro-parity spike's metric (tools/render's
                    render-diff, per-file processes with time limits); report.json, summary.html
                    with the worst diffs in target/nightly/render-diff/; with --history, a pass-rate
                    drop of more than 2 points writes regression.md (--dry-run prints the issue)
  corpus facts      rewrite fixtures/corpus/expected.json with pypdf (needs Python: see
                    tools/corpus-facts/requirements.txt; $LAMINA_PDF_PYTHON or python3)
  corpus fetch [--source <name>]… [--update-manifest]
                    check out the external corpora pinned in fixtures/corpus-manifest.toml into
                    target/corpus/ and verify every file's SHA-256 (--update-manifest rewrites the lists)
  differential [--full | --text] [--update]
                    lamina vs pypdf over the corpus and fixed-seed mutants, from the precomputed
                    fixtures/differential/expected.json; --update regenerates it with pypdf.
                    --full: the full lane's 36 mutants per corpus file instead (full.json),
                    run through cargo-nextest
                    --text: lamina's page text vs pdf.js and pdftotext (whitespace and bidi
                    controls removed) from fixtures/differential/text.json; --update regenerates
                    it (tools/differential/text.mjs; pdftotext at the pinned poppler)
  differential --live [--corpus external|mutants|all] [--source <name>]… [--tools a,b] [--sample N]
                    nightly: lamina vs live pdf.js, PDFium, qpdf and pypdf over the external corpus
                    and 250 mutants per corpus file; report in target/nightly/differential/
  differential --live --text [--source <name>]… [--sample N]
                    nightly: lamina's page text vs live pdf.js and pdftotext over the external
                    corpus (whitespace and bidi controls removed; a page agrees when lamina
                    matches one tool); report in target/nightly/differential-text/
  fetch-pdfium      download the pinned native PDFium (pdfium-binaries chromium/7881) for this host,
                    check its SHA-256, unpack the library and its notices, print LAMINA_PDF_PDFIUM_DIR
                    (cache: $LAMINA_PDF_PDFIUM_CACHE, $RUNNER_TOOL_CACHE/lamina-pdfium or target/pdfium)
  cli-docs          the laminapdf CLI's man pages and shell completions into target/cli-docs/
  extract-bench [--subset <name>]… [--sample N] [--tables lattice-and-stream] [--update-lock]
                    score lamina's extraction on the pinned public benchmark of
                    tools/bench/extract-datasets.toml: fetch into target/bench/datasets/ (SHA-256 per
                    file against its lock), write Markdown with the extract-bench example, score it with
                    the benchmark's own tests (Python with tools/bench/requirements.txt; $LAMINA_PDF_PYTHON
                    or python3); report in target/bench/extract/<dataset>/
  verapdf [--dir <dir>]
                    run the lamina examples into target/examples-out/ (or take the PDFs in <dir>)
                    and validate each with veraPDF for the profiles its XMP claims (PDF/A-2b, 2u,
                    3b, PDF/UA-1); any violation fails; summary in target/xtask/verapdf/summary.md.
                    veraPDF's pinned CLI (doctor.rs) is installed into $RUNNER_TOOL_CACHE/lamina-verapdf
                    or target/verapdf; Java: $LAMINA_PDF_JAVA, Homebrew's openjdk@21, or java on PATH
  quads [--update]  lamina's glyph quads vs PDFium's character boxes from the precomputed
                    fixtures/text/charboxes.json; --update regenerates it (tools/render/charbox.py,
                    pypdfium2 from tools/render/requirements.txt; $LAMINA_PDF_PYTHON or python3)
  roundtrip [--tools qpdf,pdfjs,pdfium]
                    lamina saves every fixture x proving op x mode, reopens each output and
                    checks it with qpdf --check, pdf.js and PDFium (needs qpdf, Node and
                    Python with tools/roundtrip/requirements.txt; outputs in target/xtask/roundtrip)
  fuzz <target> [--time <seconds>]
                    run a fuzz target (lex, parse_object, flate_decode, open, page_tree, content,
                    ops_json, save_roundtrip) with
                    cargo +nightly fuzz (default 60 s); seeds come from fixtures/ and fuzz/seeds/
  fuzz --all [--time <seconds>] [--jobs <n>]
                    run every fuzz target, <n> at once (default 1: in turn; at most 8); built
                    once first, each run logged to target/xtask/fuzz/<target>.log and printed
                    whole when it ends; the per-input timeout is 10 s x <n> (the full lane: 30 s
                    on PRs, 120 s on main, --jobs 4)
  fuzz seeds [<target>]
                    only build the seed corpora in fuzz/corpus/
  fuzz cmin [<target>]
                    minimise the corpora in fuzz/corpus/ (cargo fuzz cmin)
  fuzz tmin <target>
                    minimise every crash input in fuzz/artifacts/<target>/
  scaling           nightly: release-mode #[ignore]d tests (scaling ratios); one retry, reported as flaky
  bench [--quick] [--history <dir> [--runner <label>] [--record]]
                    nightly: criterion benches; with --history, flag >10% regressions (never fails)
  pdf-runtimes [node|chromium|firefox|webkit|deno|bun|workerd]… [--label <name>] [--history <dir> --runner <label> --record]
                    @laminapdf/sdk's example suite in each runtime (default: all; needs `wasm` first;
                    installs the Playwright browsers it runs): byte-identical to the goldens, with
                    the timings and WebAssembly memory against pdf-library §4 in the job summary;
                    --label writes target/xtask/pdf-runtimes/measurements-<name>.json
                    and target/xtask/pdf-runtimes/; --record appends them to <dir>/history.jsonl
  viewer-perf [chromium|firefox|webkit]… [--quick] [--history <dir> --runner <label> [--record] [--dry-run]]
                    @laminapdf/viewer's perf harness (needs `wasm` first): first page, scrolling 1,000
                    pages, memory, fling and zoom against the web plan's §4 targets, written to
                    target/viewer-perf/ and the job summary (never fails on a target); --history flags
                    >10% regressions, --record appends to <dir>/history.jsonl, --dry-run prints the issue
  nightly [--quick] the whole nightly lane locally (corpus, differential, fuzz, scaling, bench,
                    wasm + pdf-runtimes, viewer-perf, render-diff, external links)
  changes [--base <rev>]
                    list the paths changed since the merge base with <rev> (default origin/main)
                    and say whether any is code (not docs); sets `code` in $GITHUB_OUTPUT
                    (default base: origin/main, or main when there is no origin/main)
  version sync [--check]
                    copy the npm fixed group's version (`fixed` in .changeset/config.json;
                    every member must agree) into [workspace.package] version, the internal
                    [workspace.dependencies], every tracked Cargo.lock, CHANGELOG.md and
                    packages/sdk/src/version.ts (ADR 0017); --check only verifies
                    (and that every member crate has version.workspace = true)
  version check-tag <tag>
                    fail unless <tag> is v<version> (the release lane)
  verify-install [node|deno|bun|chromium|workerd|rust]… [--node <binary>]… [--git-ref <tag-or-sha>]
                 [--git-url <url>] [--keep]
                    pnpm pack both packages and run the quick start in clean projects in the
                    temp directory: Node (each --node), Deno, Bun, a Vite app in Chromium, a
                    Worker under wrangler dev --local, and Rust binaries with a path and a git
                    dependency on lamina (default ref: HEAD; default URL: this clone); each
                    output must equal fixtures/golden/examples/hello.pdf (needs `wasm` first)
  release-guard [--run <id>] [--commit <rev>]
                    fail unless the latest completed full nightly.yml run on main, or run
                    <id>, succeeded on <rev> (default HEAD) or an ancestor; full: scheduled, or
                    dispatched with every input at its default (gh; the release lane's first job)
  ci                check + wasm + size: the whole fast lane, locally
  doctor [--install]
                    check that external tools are installed at their pinned versions;
                    --install runs the `cargo install` fixes
  help              show this message";

/// Targets every core crate must build for (they are also pinned in `rust-toolchain.toml`).
const PORTABLE_TARGETS: [&str; 3] = ["wasm32-unknown-unknown", "aarch64-apple-ios", "aarch64-linux-android"];
const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// The tasks that run before `cargo metadata`, or None for every other task.
fn without_workspace(task: &str, rest: &[String]) -> Option<Result<()>> {
    let result = match (task, rest) {
        // `changes` runs first in the full lane and needs only git: skip `cargo metadata`.
        ("changes", []) => repo_root().and_then(|root| changes::run(&root, changes::default_base(&root))),
        ("changes", [flag, base]) if flag == "--base" => repo_root().and_then(|root| changes::run(&root, base)),
        ("changes", _) => Err(anyhow::anyhow!("usage: cargo xtask changes [--base <rev>]")),
        // `version` rewrites Cargo.toml and Cargo.lock, so it can't wait for `cargo metadata
        // --locked` (which fails on a manifest bumped by hand, the case `--check` reports).
        ("version", _) => version::parse_args(rest).and_then(|args| version::run(&repo_root()?, &args)),
        // Needs only gh: no `cargo metadata`.
        ("release-guard", _) => release::parse_args(rest).and_then(|args| release::guard(&repo_root()?, &args)),
        // Needs only CHANGELOG.md.
        ("release-notes", [tag, out]) => {
            repo_root().and_then(|root| release::release_notes(&root, tag, Path::new(out)))
        }
        _ => return None,
    };
    Some(result)
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map_or("help", String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    // Help must work even when the workspace can't load (stale lock file, missing layer metadata).
    if matches!(task, "help" | "-h" | "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    if let Some(result) = without_workspace(task, rest) {
        return result;
    }
    let ws = Workspace::load(repo_root()?)?;

    match (task, rest) {
        ("check", scopes) => check(&ws, scopes),
        (command @ ("biome-examples" | "clippy-examples"), flags) => ratchet(&ws, command, flags),
        ("gen", [what]) if what == "codes" => codegen::codes(&ws, false),
        ("gen", [what, flag]) if what == "codes" && flag == "--check" => codegen::codes(&ws, true),
        ("gen", [what, flag]) if what == "codes" && flag == "--md" => codegen::markdown(&ws),
        ("gen", [what]) if what == "boundary" => codegen::boundary::generate(&ws, false),
        ("gen", [what, flag]) if what == "boundary" && flag == "--check" => codegen::boundary::generate(&ws, true),
        ("fmt", []) => fmt(&ws, false),
        ("fmt", [flag]) if flag == "--check" => fmt(&ws, true),
        ("lint", []) => clippy(&ws).and_then(|()| biome_ci(&ws)),
        ("test", []) => {
            rust_tests(&ws)?;
            ts_tests(&ws)
        }
        ("portability", []) => portability(&ws),
        ("publish", args) => publish::run(&ws, publish::parse_args(args)?),
        ("sbom", [dir]) => publish::sbom(&ws, Path::new(dir)),
        ("wasm", []) => wasm(&ws),
        ("size", []) => wasm::size(&ws, false),
        ("size", [flag]) if flag == "--update-baseline" => wasm::size(&ws, true),
        ("deny", []) => {
            cargo_deny(&ws)?;
            licenses::check_npm(&ws)
        }
        ("npm-licenses", []) => licenses::check_npm(&ws),
        ("doctor", []) => doctor::run(&ws, false),
        ("doctor", [flag]) if flag == "--install" => doctor::run(&ws, true),
        ("docs-links", []) => docs_links::run(&ws.root, false),
        ("docs-links", [flag]) if flag == "--json" => docs_links::run(&ws.root, true),
        ("docs-links", [flag]) if flag == "--external" => docs_links::external::run(&ws.root, false),
        ("docs-links", [a, b]) if [a, b].iter().all(|f| *f == "--external" || *f == "--json") && a != b => {
            docs_links::external::run(&ws.root, true)
        }
        ("todos", []) => todos::run(&ws.root, false),
        ("todos", [flag]) if flag == "--check" => todos::run(&ws.root, true),
        ("progress", args) => progress::run(&ws.root, progress::offline(args)?),
        ("capabilities", []) => capabilities::run(&ws.root, false),
        ("capabilities", [flag]) if flag == "--check" => capabilities::run(&ws.root, true),
        ("golden", []) => harness::golden(&ws, false),
        ("golden", [flag]) if flag == "--update" => harness::golden(&ws, true),
        ("examples", []) => examples::run(&ws),
        ("fetch-pdfium", []) => pdfium::fetch(&ws).map(|_| ()),
        ("cli-docs", []) => cli_docs::run(&ws),
        ("extract-bench", args) => extract_bench::run(&ws, &extract_bench::parse_args(args)?),
        ("verapdf", args) => verapdf::run(&ws, verapdf::dir_arg(args)?),
        ("render-golden", []) => render::run(&ws, false),
        ("render-golden", [flag]) if flag == "--update" => render::run(&ws, true),
        ("render-diff", args) => render_diff::run(&ws, &render_diff::parse_args(args)?),
        ("corpus", [what]) if what == "facts" => harness::corpus_facts(&ws),
        ("corpus", [what, args @ ..]) if what == "fetch" => corpus::fetch(&ws, &corpus::parse_fetch_args(args)?),
        ("quads", args) => harness::quads(&ws, args),
        ("differential", [flag, args @ ..]) if flag == "--live" => live::run_live(&ws, &live::parse_args(args)?),
        ("differential", args) => harness::differential(&ws, &harness::parse_differential_args(args)?),
        ("fuzz", [cmd, target]) if cmd == "tmin" => nightly::fuzz_tmin(&ws, harness::known_fuzz_target(target)?),
        ("fuzz", args) => harness::fuzz(&ws, &harness::parse_fuzz_args(args)?),
        ("scaling", []) => nightly::scaling(&ws),
        ("bench", args) => nightly::bench(&ws, &nightly::parse_bench_args(args)?),
        ("pdf-runtimes", args) => pdf_runtimes::run(&ws, &pdf_runtimes::parse_args(args)?),
        ("viewer-perf", args) => viewer_perf::run(&ws, &viewer_perf::parse_args(args)?),
        ("nightly", args) => nightly::nightly(&ws, nightly::parse_nightly_args(args)?),
        ("roundtrip", args) => roundtrip::run_suite(&ws, &roundtrip::parse_args(args)?),
        ("verify-install", args) => verify_install::run(&ws, &verify_install::parse_args(args)?),
        ("ci", []) => {
            check_docs(&ws)?;
            check_rust(&ws)?;
            check_ts(&ws)?;
            wasm(&ws)?;
            wasm::size(&ws, false)
        }
        _ => bail!("unknown task or arguments: {}\n\n{USAGE}", args.join(" ")),
    }
}

/// `cargo xtask check [scope] [flags]`.
fn check(ws: &Workspace, scopes: &[String]) -> Result<()> {
    match scopes {
        [] => {
            check_docs(ws)?;
            check_rust(ws)?;
            check_ts(ws)
        }
        [scope] if scope == "docs" => check_docs(ws),
        [scope, flags @ ..] if is_docs_ratchet(scope) => ratchet(ws, &format!("check {scope}"), flags),
        [scope] if scope == "rust" => check_rust(ws),
        [scope] if scope == "maintainability" => maintainability::run(&ws.root),
        [scope] if scope == "ts" => check_ts(ws),
        _ => bail!("unknown check: {}\n\n{USAGE}", scopes.join(" ")),
    }
}

/// The repository root: two levels above this crate (`tools/xtask`), so tasks work from any directory.
fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .context("xtask must live at <root>/tools/xtask")
}

/// `check docs-prose` and `check docs-snippets`, the `check` scopes that [`ratchet`] runs.
fn is_docs_ratchet(scope: &str) -> bool {
    matches!(scope, "docs-prose" | "docs-snippets")
}

/// Runs a check on its own, strict on changed files and warning elsewhere unless `--all-strict`
/// ([`reports`]), as `command` (`check docs-prose`, `clippy-examples`) names it, with its flags.
/// `cargo xtask check` runs these checks with `--all-strict`.
fn ratchet(ws: &Workspace, command: &str, flags: &[String]) -> Result<()> {
    let options = reports::Options::parse(command, flags)?;
    match command {
        "check docs-prose" => docs_prose::run(&ws.root, options),
        "check docs-snippets" => docs_snippets::run(&ws.root, options),
        "biome-examples" => examples_lints::biome(ws, options),
        "clippy-examples" => examples_lints::clippy(ws, options),
        _ => bail!("{command} is not a ratcheting check"),
    }
}

/// The documentation checks: fast and network-free, so they run first.
fn check_docs(ws: &Workspace) -> Result<()> {
    step("docs-links: relative links and anchors in tracked Markdown");
    docs_links::run(&ws.root, false)?;
    step("docs-prose: reader-facing text against docs/guidelines/docs-writing.md (strict on every file)");
    docs_prose::run(&ws.root, reports::Options::strict())?;
    step("docs-snippets: the examples against docs/guidelines/examples.md (strict on every file)");
    docs_snippets::run(&ws.root, reports::Options::strict())?;
    step("todos --check: the todo board and Blocks fields are up to date");
    todos::run(&ws.root, true)?;
    step("portable paths: every tracked path checks out on Windows, macOS and Linux");
    portable_paths::check(&ws.root)?;
    step("capabilities: manifest vs 06 §3, tags, and no README claim beyond the tagged tests");
    capabilities::check_static(&ws.root)?;
    step("renames: docs/renames.json against its schema, every deprecation listed, every new name defined");
    renames::check(&ws.root)?;
    codegen::codes(ws, true)?;
    codegen::boundary::generate(ws, true)?;
    version::run(&ws.root, &version::Command::Sync { check: true })?;
    step("Docs checks passed");
    Ok(())
}

fn check_rust(ws: &Workspace) -> Result<()> {
    fmt_rust(ws, true)?;
    maintainability::run(&ws.root)?;
    portability(ws)?;
    clippy(ws)?;
    step("clippy-examples: lamina's examples, functions of at most 40 lines (strict on every example)");
    examples_lints::clippy(ws, reports::Options::strict())?;
    rust_tests(ws)?;
    docs(ws)?;
    cargo_deny(ws)?;
    step("Rust checks passed");
    Ok(())
}

/// Every export of every package entry has a doc comment (`TypeDoc`'s `notDocumented`, warnings
/// as errors; maintainability guideline, "API"): `apps/docs/scripts/check-api-docs.mjs`.
fn api_docs(ws: &Workspace) -> Result<()> {
    step("every package export is documented (TypeDoc)");
    ws.pnpm(&["--filter", "@laminapdf/docs", "run", "check:api-docs"])
}

fn check_ts(ws: &Workspace) -> Result<()> {
    biome_ci(ws)?;
    examples_lints::biome(ws, reports::Options::strict())?;
    ws.pnpm(&["-r", "--if-present", "run", "typecheck"])?;
    ts_tests(ws)?;
    api_docs(ws)?;
    ts_structure::check(ws)?;
    docs_site(ws)?;
    website(ws)?;
    licenses::check_npm(ws)?;
    step("TypeScript checks passed");
    Ok(())
}

fn fmt(ws: &Workspace, check: bool) -> Result<()> {
    fmt_rust(ws, check)?;
    if check {
        ws.pnpm(&["exec", "biome", "format", "."])
    } else {
        ws.pnpm(&["exec", "biome", "format", "--write", "."])
    }
}

fn fmt_rust(ws: &Workspace, check: bool) -> Result<()> {
    if check {
        ws.cargo(&["fmt", "--all", "--check"])
    } else {
        ws.cargo(&["fmt", "--all"])
    }
}

/// Clippy twice, because cfg-gated code is only linted when compiled (rust.md: Lints).
fn clippy(ws: &Workspace) -> Result<()> {
    ws.cargo(&[
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])?;
    let mut args = vec!["clippy", "--locked", "--target", WASM_TARGET, "--lib"];
    args.extend(package_args(ws.wasm_crates(), "wasm32")?);
    args.extend(["--", "-D", "warnings"]);
    ws.cargo(&args)?;
    // The shipping configurations, which the unified build above never compiles: the WASM
    // modules (the base: laminapdf-build without text, no SASLprep, the façade with `build`
    // only; ADR 0016 §4) and the façade without the builder. Clippy lints every workspace
    // crate a build compiles, so each of these lints its dependencies' configuration too.
    for features in [
        &["-p", "laminapdf-wasm"][..],
        &["-p", "laminapdf-wasm", "--features", "text"],
        &["-p", "laminapdf-wasm", "--features", "extract"],
        &["-p", "laminapdf", "--no-default-features"],
        // The reader configuration: extraction without the builder (B9c).
        &["-p", "laminapdf", "--no-default-features", "--features", "extract"],
    ] {
        let mut args = vec!["clippy", "--locked", "--target", WASM_TARGET, "--lib"];
        args.extend(features);
        args.extend(["--", "-D", "warnings"]);
        ws.cargo(&args)?;
    }
    Ok(())
}

fn rust_tests(ws: &Workspace) -> Result<()> {
    let nextest = doctor::cargo_nextest();
    let probe = nextest.probe(ws);
    if use_nextest(&probe) {
        ws.cargo(&["nextest", "run", "--locked", "--workspace", "--all-features"])?;
    } else {
        // nextest is optional: an absent or off-pin one is a note, not a failure (doctor reports it).
        eprintln!("note: {}; using cargo test instead", nextest.problem(&probe));
        ws.cargo(&[
            "test",
            "--locked",
            "--workspace",
            "--all-features",
            "--lib",
            "--bins",
            "--tests",
        ])?;
    }
    // nextest does not run doc tests.
    ws.cargo(&["test", "--locked", "--workspace", "--all-features", "--doc"])
}

/// Run tests with nextest only when it is installed at the pinned version.
fn use_nextest(probe: &doctor::Probe) -> bool {
    matches!(probe, doctor::Probe::Ok(_))
}

/// `-p <name>` for each crate. Fails when there are none: without `-p`, cargo would run the
/// cross-target command on the whole workspace (xtask included) instead of the crates meant.
fn package_args<'a>(names: impl Iterator<Item = &'a str>, kind: &str) -> Result<Vec<&'a str>> {
    let args: Vec<&str> = names.flat_map(|name| ["-p", name]).collect();
    if args.is_empty() {
        bail!(
            "no {kind} crates found: every crate in crates/ declares [package.metadata.laminapdf] layer, \
             and at least one must be \"core\""
        );
    }
    Ok(args)
}

/// rustdoc with warnings as errors (broken intra-doc links, bad code blocks); rust.md: Documentation.
fn docs(ws: &Workspace) -> Result<()> {
    ws.cargo(&[
        "doc",
        "--locked",
        "--workspace",
        "--no-deps",
        "--all-features",
        "--config",
        "build.rustdocflags=[\"-D\", \"warnings\"]",
    ])
}

/// The bytes-in/bytes-out rules for core crates (code-organization.md §2), then cross-target builds.
fn portability(ws: &Workspace) -> Result<()> {
    step("portability: forbidden APIs, dependencies and unsafe in core crates");
    ws.check_core_sources()?;
    harness::check_fuzz_excluded(ws)?;
    for target in PORTABLE_TARGETS {
        let mut args = vec!["check", "--locked", "--lib", "--target", target];
        args.extend(package_args(ws.core_crates(), "core")?);
        ws.cargo(&args)?;
        // The façade's structure-only build (no builder, no text) and the WASM base module's
        // (the builder without text, no SASLprep) are shipping configurations of their own
        // (todo B5c, ADR 0016 §4).
        for features in [&[][..], &["--features", "build"][..]] {
            let mut args = vec![
                "check",
                "--locked",
                "--lib",
                "--target",
                target,
                "-p",
                "laminapdf",
                "--no-default-features",
            ];
            args.extend(features);
            ws.cargo(&args)?;
        }
    }
    Ok(())
}

fn wasm(ws: &Workspace) -> Result<()> {
    let mut args = vec![
        "build",
        "--locked",
        "--lib",
        "--profile",
        "wasm-release",
        "--target",
        WASM_TARGET,
    ];
    // laminapdf-wasm is built per module by `wasm::build` (an LTO link here would be thrown away).
    args.extend(package_args(
        ws.wasm_crates().filter(|c| *c != "laminapdf-wasm"),
        "wasm32",
    )?);
    // Every wasm32 crate builds with the shipping profile; then the two modules of @laminapdf/wasm
    // (ADR 0016), written gitignored into packages/wasm/dist/, and their tests.
    ws.cargo(&args)?;
    wasm::build(ws)
}

fn cargo_deny(ws: &Workspace) -> Result<()> {
    doctor::require(ws, &doctor::cargo_deny())?;
    ws.cargo(&["deny", "--all-features", "check"])?;
    // The render tool is its own workspace with its own lockfile (hayro and its graph): the
    // same policy (the root deny.toml, cargo-deny's default from the root) applies to it.
    let manifest = ws.root.join(render::MANIFEST).to_string_lossy().into_owned();
    ws.cargo(&["deny", "--manifest-path", &manifest, "check"])
}

fn biome_ci(ws: &Workspace) -> Result<()> {
    ws.pnpm(&["exec", "biome", "ci", "."])
}

fn ts_tests(ws: &Workspace) -> Result<()> {
    ws.pnpm(&["-r", "--if-present", "run", "test"])
}

/// Builds the docs site (`apps/docs`): its `TypeDoc` reference, the snippets read from the example
/// files and the code table from `codes.json`; the build fails on any broken internal link.
fn docs_site(ws: &Workspace) -> Result<()> {
    step("docs site: astro build of apps/docs (TypeDoc, snippets, code table, link check)");
    ws.pnpm(&["--filter", "@laminapdf/docs", "run", "build"])
}

/// Builds the website (`apps/site`), whose post-build checks fail on a page that breaks the
/// website's rules (ADR 0011 §3): a file from another origin, a cookie, a missing title,
/// description or canonical URL, a broken internal link.
fn website(ws: &Workspace) -> Result<()> {
    step("website: astro build of apps/site (the website's rules, internal links)");
    ws.pnpm(&["--filter", "@laminapdf/site", "run", "build"])
}

pub(crate) fn step(message: &str) {
    println!("\n==> {message}");
}

/// Runs `program args…` in `dir`, echoing the command, and fails if it exits unsuccessfully.
fn run<S: AsRef<OsStr>>(dir: &Path, program: &str, args: &[S]) -> Result<()> {
    run_env(dir, program, args, &[])
}

/// [`run`] with extra environment variables.
pub(crate) fn run_env<S: AsRef<OsStr>>(dir: &Path, program: &str, args: &[S], envs: &[(&str, &str)]) -> Result<()> {
    let shown: Vec<_> = args.iter().map(|a| a.as_ref().to_string_lossy()).collect();
    let shown_envs: Vec<String> = envs.iter().map(|(k, v)| format!("{k}={v} ")).collect();
    step(&format!("{}{program} {}", shown_envs.concat(), shown.join(" ")));
    let status = Command::new(program_path(program))
        .args(args)
        .envs(envs.iter().copied())
        .current_dir(dir)
        .status()
        .with_context(|| format!("failed to start `{program}`; is it installed and on PATH?"))?;
    if !status.success() {
        bail!("`{program} {}` failed with {status}", shown.join(" "));
    }
    Ok(())
}

/// The program to start for `name`. On Windows, `Command` only finds `name.exe` on `PATH`,
/// but pnpm installed with npm or corepack is a `pnpm.cmd` shim: use the first of `name.exe`
/// and `name.cmd` found on `PATH`. Elsewhere, `name` itself.
pub(crate) fn program_path(name: &str) -> OsString {
    if cfg!(windows) {
        let found = env::var_os("PATH").and_then(|path| {
            env::split_paths(&path).find_map(|dir| {
                ["exe", "cmd"]
                    .into_iter()
                    .map(|ext| dir.join(format!("{name}.{ext}")))
                    .find(|candidate| candidate.is_file())
            })
        });
        if let Some(found) = found {
            return found.into_os_string();
        }
    }
    OsString::from(name)
}

#[cfg(test)]
mod tests {
    use super::{package_args, use_nextest};
    use crate::doctor::Probe;

    #[test]
    fn package_args_fails_without_crates() {
        assert!(package_args(std::iter::empty(), "core").is_err());
    }

    #[test]
    fn package_args_names_each_crate() {
        let args = package_args(["a", "b"].into_iter(), "core").unwrap();
        assert_eq!(args, ["-p", "a", "-p", "b"]);
    }

    #[test]
    fn off_pin_nextest_falls_back_to_cargo_test() {
        assert!(!use_nextest(&Probe::OffPin("cargo-nextest 0.9.151".into())));
    }

    #[test]
    fn missing_nextest_falls_back_to_cargo_test() {
        assert!(!use_nextest(&Probe::Missing));
    }
}
