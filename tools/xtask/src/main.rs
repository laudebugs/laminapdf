//! lamina's task runner, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
//!
//! This is the single entry point for repo-wide tasks; CI runs the same commands developers run
//! (docs/guidelines/code-organization.md §10). Run `cargo xtask help` for the task list.

mod annots_parity;
mod benchmarks;
mod capabilities;
mod changes;
mod changesets;
mod cli_docs;
mod codegen;
mod corpus;
mod crash;
mod design_screens;
mod desktop_release;
mod desktop_sidecar;
mod docs_links;
mod docs_previews;
mod docs_prose;
mod docs_snippets;
mod doctor;
mod examples;
mod examples_lints;
mod extract_bench;
mod fixtures;
mod gallery;
mod harness;
mod indexnow;
mod licenses;
mod lines;
mod live;
mod machine;
mod maintainability;
mod models;
mod nightly;
mod notices;
mod package_docs;
mod package_metadata;
mod pdf_runtimes;
mod pdfium;
mod plain_version;
mod platform_boundary;
mod portable_paths;
mod progress;
mod proof;
mod publish;
mod release;
mod renames;
mod render;
mod render_diff;
mod reports;
mod roundtrip;
mod runner;
mod sdk_doc_comments;
mod semver;
mod since;
mod todos;
mod tracked;
mod ts_structure;
mod usage;
mod util;
mod verapdf;
mod verify_install;
mod version;
mod viewer_perf;
mod wasm;
mod wasm_glue;
mod wasm_module;
mod web;
mod web_release;
#[cfg(test)]
mod workflows;
mod workspace;

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::usage::USAGE;
use crate::workspace::Workspace;

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
        // Needs only git and `cargo metadata --no-deps`.
        ("desktop-key", []) => repo_root().and_then(|root| changes::desktop::print_key(&root)),
        // `version` rewrites Cargo.toml and Cargo.lock, so it can't wait for `cargo metadata
        // --locked` (which fails on a manifest bumped by hand, the case `--check` reports).
        ("version", _) => version::parse_args(rest).and_then(|args| version::run(&repo_root()?, &args)),
        // Needs only the report and the symbols (Wrangler fetches them): no `cargo metadata`.
        ("crash", _) => repo_root().and_then(|root| crash::run(&root, rest)),
        // Needs only curl and the files it names: no `cargo metadata`.
        ("indexnow", _) => indexnow::parse_args(rest).and_then(|task| indexnow::run(&task)),
        // Needs only gh: no `cargo metadata`.
        ("release", _) => repo_root().and_then(|root| release::prepare::run(&root, rest)),
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
        ("web", args) => web::run(&ws, args),
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
        ("docs-links", args) if args.iter().any(|a| a == "--external") => {
            docs_links::external::run(&ws.root, &docs_links::cache::parse_args(args)?)
        }
        ("todos", []) => todos::run(&ws.root, false),
        ("todos", [flag]) if flag == "--check" => todos::run(&ws.root, true),
        ("progress", args) => progress::run(&ws.root, progress::offline(args, "progress")?),
        ("capabilities", []) => capabilities::run(&ws.root, false),
        ("capabilities", [flag]) if flag == "--check" => capabilities::run(&ws.root, true),
        ("golden", []) => harness::golden(&ws, false),
        ("golden", [flag]) if flag == "--update" => harness::golden(&ws, true),
        ("examples", []) => examples::run(&ws),
        ("fetch-pdfium", []) => pdfium::fetch(&ws).map(|_| ()),
        ("models", [cmd]) if cmd == "check" => models::check(&ws.root),
        ("models", [cmd, name]) if cmd == "fetch" => models::fetch(&ws, name).map(|_| ()),
        ("cli-docs", []) => cli_docs::run(&ws),
        ("cli-dist", []) => cli_docs::dist_files(&ws),
        ("desktop-notices", args) => notices::desktop(&ws, args),
        ("desktop-release", args) => desktop_release::run(&ws, args),
        ("desktop-sidecar", args) => desktop_sidecar::run(&ws, args),
        ("desktop-frontend", []) => web::desktop_frontend(&ws),
        ("fixtures", [what, rest @ ..]) if what == "large-pdf" => fixtures::large_pdf(&ws, rest),
        ("extract-bench", args) => extract_bench::run(&ws, &extract_bench::parse_args(args)?),
        ("benchmarks", args) => benchmarks::run(&ws, &benchmarks::parse_args(args)?),
        ("verapdf", args) => verapdf::run(&ws, verapdf::dir_arg(args)?),
        ("docs-previews", []) => docs_previews::run(&ws),
        ("render-golden", []) => render::run(&ws, false),
        ("render-golden", [flag]) if flag == "--update" => render::run(&ws, true),
        ("render-diff", args) => render_diff::run(&ws, &render_diff::parse_args(args)?),
        ("annots-parity", args) => annots_parity::run(&ws, &annots_parity::parse_args(args)?),
        ("corpus", [what]) if what == "facts" || what == "scripts" => harness::corpus_report(&ws, what),
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
        ("proof", [sub, rest @ ..]) if sub == "snapshot" => {
            proof::snapshot::run(&ws.root, progress::offline(rest, "proof snapshot")?)
        }
        ("proof", args) => proof::run(&ws, &proof::parse_args(args)?),
        ("roundtrip", args) => roundtrip::run_suite(&ws, &roundtrip::parse_args(args)?),
        ("semver", args) => semver::run(&ws, args),
        ("gallery", args) => gallery::run(&ws, gallery::parse_args(args)?),
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
        [scope] if scope == "since" => since::check(ws),
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
    step("todos --check: the todo graph and the Depends on links; no committed board or Blocks rows");
    todos::run(&ws.root, true)?;
    step("portable paths: every tracked path checks out on Windows, macOS and Linux");
    portable_paths::check(&ws.root)?;
    step("design screens: every screen in the Paper inventory has its area heading and design file");
    design_screens::check(&ws.root)?;
    step("capabilities: manifest vs 06 §3, tags, and no README claim beyond the tagged tests");
    capabilities::check_static(&ws.root)?;
    step("renames: docs/renames.json against its schema, every deprecation listed, every new name defined");
    renames::check(&ws.root)?;
    step(
        "package metadata: no ADR number, todo ID or \"private\" in a published package's description or README opening",
    );
    package_metadata::check(ws)?;
    step("sdk doc comments: no ADR number or todo ID in @laminapdf/sdk's API reference");
    sdk_doc_comments::check(&ws.root)?;
    step("docs previews: the docs' example pictures match the example goldens");
    docs_previews::check(&ws.root)?;
    models::check(&ws.root)?;
    codegen::codes(ws, true)?;
    codegen::boundary::generate(ws, true)?;
    version::run(&ws.root, &version::Command::Sync { check: true })?;
    step("Docs checks passed");
    Ok(())
}

fn check_rust(ws: &Workspace) -> Result<()> {
    fmt_rust(ws, true)?;
    separate_lockfiles(ws)?;
    maintainability::run(&ws.root)?;
    portability(ws)?;
    clippy(ws)?;
    step("clippy-examples: lamina's examples, functions of at most 40 lines (strict on every example)");
    examples_lints::clippy(ws, reports::Options::strict())?;
    rust_tests(ws)?;
    docs(ws)?;
    // rustdoc's JSON is the same on every host, so CI checks it on one leg, Linux x64, rather
    // than build it on all six; a local run always checks it.
    if env::var_os("CI").is_none() || cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        since::check(ws)?;
    }
    cargo_deny(ws)?;
    step("Rust checks passed");
    Ok(())
}

/// The workspaces outside the root one that build Lamina PDF's crates (the render tool and the
/// fuzz targets) keep lockfiles of their own: a crate the root workspace adds must reach them
/// too, or the nightly lane's `--locked` builds of them fail. Checked here, on every PR, since
/// those builds run only nightly or when their own files change.
fn separate_lockfiles(ws: &Workspace) -> Result<()> {
    step("separate lockfiles: tools/render and fuzz match the crates they build");
    for manifest in [render::MANIFEST, "fuzz/Cargo.toml"] {
        // `cargo metadata` prints the whole graph as JSON: only its verdict matters here.
        let out = Command::new("cargo")
            .args(["metadata", "--locked", "--format-version", "1", "--manifest-path"])
            .arg(ws.root.join(manifest))
            .current_dir(&ws.root)
            .output()
            .with_context(|| format!("running cargo metadata for {manifest}"))?;
        if !out.status.success() {
            bail!(
                "{manifest}'s Cargo.lock is stale: run `cargo metadata --manifest-path {manifest}` and commit it\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
    Ok(())
}

/// Every export of every package entry has a doc comment (`TypeDoc`'s `notDocumented`, warnings
/// as errors; maintainability guideline, "API"): `apps/docs/scripts/check-api-docs.mjs`.
fn api_docs(ws: &Workspace) -> Result<()> {
    step("every package export is documented (TypeDoc)");
    ws.pnpm(&["--filter", "@laminapdf/docs", "run", "check:api-docs"])
}

fn check_ts(ws: &Workspace) -> Result<()> {
    platform_boundary::check(ws)?;
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
        &["-p", "laminapdf-wasm", "--features", "extract,reflow"],
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
    step(&shown_command(program, args, envs));
    let status = Command::new(program_path(program))
        .args(args)
        .envs(envs.iter().copied())
        .current_dir(dir)
        .status()
        .with_context(|| format!("failed to start `{program}`; is it installed and on PATH?"))?;
    if !status.success() {
        bail!("`{}` failed with {status}", shown_command(program, args, &[]));
    }
    Ok(())
}

/// `program args…` as the log shows it, after the extra environment variables as `KEY=value`.
pub(crate) fn shown_command<S: AsRef<OsStr>>(program: &str, args: &[S], envs: &[(&str, &str)]) -> String {
    let mut shown = String::new();
    for (key, value) in envs {
        for part in [key, "=", value, " "] {
            shown.push_str(part);
        }
    }
    shown.push_str(program);
    for arg in args {
        shown.push(' ');
        shown.push_str(&arg.as_ref().to_string_lossy());
    }
    shown
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
