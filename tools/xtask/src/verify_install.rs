//! `cargo xtask verify-install`: does the library install and run outside this repository?
//! (ADR 0017, todo B7b; ADR 0018 §2's clean-install smoke tests of the release lane.)
//!
//! It packs `@laminapdf/wasm` and `@laminapdf/sdk` with `pnpm pack`, then builds temporary projects in
//! the system temp directory, outside the repository, whose only lamina dependencies are those
//! tarballs or the `laminapdf` crate, and runs the quick start (`packages/sdk/examples/hello.ts`,
//! `crates/laminapdf/examples/hello.rs`, copied in, called with the golden's file ID, producer and
//! creation date) in each runtime:
//!
//! - `node`: every `--node <binary>` (default: `node` on `PATH`; Node ≥ 22.18 runs `.ts` files);
//! - `deno`, `bun`: the repository's pinned binaries (dev dependencies of `@laminapdf/sdk`);
//! - `chromium`: a Vite app built for production and opened in Playwright's Chromium;
//! - `workerd`: a Worker run by `wrangler dev --local`;
//! - `rust`: a binary with a `path =` dependency on `crates/laminapdf`, and one with a `git =`
//!   dependency pinned to `--git-ref` (a tag such as `v0.1.0-alpha.0`, or a commit SHA: default
//!   `HEAD`) at `--git-url` (default: this clone, as a `file://` URL).
//!
//! Every runtime's output must be byte-identical to `fixtures/golden/examples/hello.pdf`, and the
//! JavaScript ones reopen it and count its pages. The vite, Playwright and wrangler tools are the
//! repository's (pinned); the projects' own dependencies are only what they verify.
//!
//! With `--from-registry` (todo P3) the projects install the published packages instead, as the
//! install page tells users to: `@laminapdf/sdk` from npm (which brings `@laminapdf/wasm`) and
//! `laminapdf` from crates.io, at the fixed group's version (`--version` for another). One Rust
//! binary depends on the crate from crates.io in place of the path and git ones. `--wait
//! <minutes>` first polls both registries until the version is there: an npm version goes public
//! only once the owner approves it (ADR 0017 §7).

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::util::{git_stdout, sha256_hex, summary};
use crate::workspace::Workspace;
use crate::{program_path, step};

/// Every runtime, in the order a full run takes them.
const RUNTIMES: [&str; 6] = ["node", "deno", "bun", "chromium", "workerd", "rust"];
/// The golden every runtime's quick start must reproduce byte for byte.
const GOLDEN: &str = "fixtures/golden/examples/hello.pdf";
/// The quick starts copied into the projects.
const HELLO_TS: &str = "packages/sdk/examples/hello.ts";
/// The golden's file ID, producer and creation date, which the TypeScript quick start takes as options.
const HELLO_OPTIONS_TS: &str = "packages/sdk/test/suite/hello-options.ts";
const HELLO_RS: &str = "crates/laminapdf/examples/hello.rs";
/// The façade crate the Rust projects depend on (its `[package] name`, which a test checks).
const FACADE_CRATE: &str = "laminapdf";
/// The package whose dev dependencies provide deno, bun, vite, Playwright and wrangler.
const TOOLS_PACKAGE: &str = "packages/sdk";
/// The line a JavaScript check prints (`report.ts`).
const PREFIX: &str = "LAMINA_PDF_VERIFY ";
const VERSION_PREFIX: &str = "LAMINA_PDF_VERIFY_VERSION ";

const REPORT_TS: &str = include_str!("verify_install/report.ts");
const MAIN_TS: &str = include_str!("verify_install/main.ts");
/// How a Node that can't parse `using` builds the project first (the Node quick start's advice):
/// `tsc` with these options, on the command line so no tsconfig.json sits above the Vite and
/// Worker projects. `ESNext.Disposable` types `using`; tsc lowers it to ES2022.
const NODE_TSC_ARGS: [&str; 14] = [
    "main.ts",
    "--outDir",
    "dist",
    "--target",
    "ES2022",
    "--module",
    "NodeNext",
    "--moduleResolution",
    "NodeNext",
    "--rewriteRelativeImportExtensions",
    "--strict",
    "--skipLibCheck",
    "--lib",
    "ES2023,ESNext.Disposable,DOM",
];
const VITE_INDEX: &str = include_str!("verify_install/vite/index.html");
const VITE_MAIN: &str = include_str!("verify_install/vite/main.ts");
const FREE_PORT: &str = include_str!("verify_install/free-port.mjs");
const CHROMIUM_RUNNER: &str = include_str!("verify_install/vite/chromium.mjs");
const WORKER_INDEX: &str = include_str!("verify_install/worker/index.ts");
const WRANGLER_TOML: &str = include_str!("verify_install/worker/wrangler.toml");
const WORKERD_RUNNER: &str = include_str!("verify_install/worker/workerd.mjs");
/// The Rust project's `main`: the quick start's `build` with the golden's options.
const RUST_MAIN: &str = include_str!("verify_install/rust/main.rs.in");

const USAGE: &str = "usage: cargo xtask verify-install [node|deno|bun|chromium|workerd|rust]… \
                     [--node <binary>]… [--git-ref <tag-or-sha>] [--git-url <url>] [--keep] \
                     [--from-registry [--version <version>] [--wait <minutes>]]";

/// What `cargo xtask verify-install` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    runtimes: Vec<&'static str>,
    nodes: Vec<String>,
    git_ref: Option<String>,
    git_url: Option<String>,
    keep: bool,
    from_registry: bool,
    version: Option<String>,
    wait_minutes: u64,
}

pub(crate) fn parse_args(args: &[String]) -> Result<Args> {
    let mut parsed = Args::default();
    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--node" => parsed.nodes.push(rest.next().context(USAGE)?.clone()),
            "--git-ref" => parsed.git_ref = Some(rest.next().context(USAGE)?.clone()),
            "--git-url" => parsed.git_url = Some(rest.next().context(USAGE)?.clone()),
            "--keep" => parsed.keep = true,
            "--from-registry" => parsed.from_registry = true,
            "--version" => parsed.version = Some(rest.next().context(USAGE)?.clone()),
            "--wait" => {
                let minutes = rest.next().context(USAGE)?;
                parsed.wait_minutes = minutes
                    .parse()
                    .with_context(|| format!("--wait takes whole minutes, not {minutes:?}; {USAGE}"))?;
            }
            name => {
                let Some(&runtime) = RUNTIMES.iter().find(|r| **r == name) else {
                    bail!("unknown runtime or argument {name:?}; {USAGE}");
                };
                if !parsed.runtimes.contains(&runtime) {
                    parsed.runtimes.push(runtime);
                }
            }
        }
    }
    if !parsed.from_registry && (parsed.version.is_some() || parsed.wait_minutes > 0) {
        bail!("--version and --wait go with --from-registry; {USAGE}");
    }
    if parsed.from_registry && (parsed.git_ref.is_some() || parsed.git_url.is_some()) {
        bail!("--from-registry builds the Rust binary from crates.io: --git-ref and --git-url don't apply; {USAGE}");
    }
    if parsed.runtimes.is_empty() {
        parsed.runtimes = RUNTIMES.to_vec();
    }
    if parsed.nodes.is_empty() {
        parsed.nodes.push("node".to_owned());
    }
    Ok(parsed)
}

/// How the git dependency names `git_ref`, resolved in the clone at `root`: `tag = <ref>` when
/// it is an existing tag, otherwise `rev = <full SHA>` of the commit it names (a branch, a short
/// SHA, `HEAD`…), so Cargo never takes a branch for a tag.
fn git_spec(root: &Path, git_ref: &str) -> Result<(&'static str, String)> {
    let is_tag = git_stdout(
        root,
        &["rev-parse", "--verify", "--quiet", &format!("refs/tags/{git_ref}")],
    )
    .is_some();
    if is_tag {
        return Ok(("tag", git_ref.to_owned()));
    }
    let sha = git_stdout(
        root,
        &["rev-parse", "--verify", "--quiet", &format!("{git_ref}^{{commit}}")],
    )
    .with_context(|| format!("--git-ref {git_ref:?} names no tag or commit in this clone"))?;
    Ok(("rev", sha))
}

/// Where the projects get Lamina PDF from.
enum Source {
    /// The `pnpm pack` tarballs, and the crate by path and by git.
    Checkout,
    /// npm and crates.io, at this version.
    Registry(String),
}

impl Source {
    fn heading(&self) -> String {
        match self {
            Self::Checkout => "clean projects outside the repository".to_owned(),
            Self::Registry(version) => format!("the published {version} from npm and crates.io"),
        }
    }
}

/// The packages the registry check needs on npm (`@laminapdf/sdk` and the module package it
/// depends on) and the crate on crates.io.
const NPM_PACKAGES: [&str; 2] = ["@laminapdf/sdk", "@laminapdf/wasm"];

/// Which of the packages aren't published at `version` yet, as readable lines (none: all are).
fn unpublished(ws: &Workspace, version: &str) -> Result<Vec<String>> {
    let mut missing = Vec::new();
    for name in NPM_PACKAGES {
        if !crate::publish::on_npm(ws, name, version)? {
            missing.push(format!(
                "{name}@{version} is not on npm (staged versions wait for approval on npmjs.com)"
            ));
        }
    }
    if !crate::publish::on_crates_io(FACADE_CRATE, version)? {
        missing.push(format!("{FACADE_CRATE} {version} is not on crates.io"));
    }
    Ok(missing)
}

/// Polls the registries every minute, for up to `minutes`, until `version` is on both. A
/// registry that fails to answer (a 429, a 5xx, no network) costs one more minute, not the wait.
fn wait_for_registries(ws: &Workspace, version: &str, minutes: u64) -> Result<()> {
    for attempt in 0..=minutes {
        let missing = match unpublished(ws, version) {
            Ok(missing) => missing,
            Err(error) if attempt < minutes => vec![format!("{error:#}")],
            Err(error) => return Err(error),
        };
        if missing.is_empty() {
            return Ok(());
        }
        if attempt == minutes {
            bail!("after {minutes} minutes of waiting: {}", missing.join("; "));
        }
        step(&format!("verify-install: waiting a minute: {}", missing.join("; ")));
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
    Ok(())
}

/// One runtime's result: its label (with the version found) and what went wrong, if anything.
struct Outcome {
    label: String,
    result: Result<String>,
}

/// A fresh folder for the projects, in the system temp directory and outside the repository.
fn work_dir(ws: &Workspace) -> Result<PathBuf> {
    let work = std::env::temp_dir().join(format!("lamina-verify-install-{}", std::process::id()));
    let root = ws.root.canonicalize().unwrap_or_else(|_| ws.root.clone());
    let parent = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir());
    if parent.starts_with(&root) {
        bail!(
            "the temp directory {} is inside the repository: set TMPDIR elsewhere",
            parent.display()
        );
    }
    if work.exists() {
        fs::remove_dir_all(&work).with_context(|| format!("removing {}", work.display()))?;
    }
    fs::create_dir_all(&work)?;
    Ok(work)
}

/// Where the projects get Lamina PDF from: the checkout, or the registries once `--wait` sees the
/// version on both.
fn source(ws: &Workspace, args: &Args) -> Result<Source> {
    if !args.from_registry {
        return Ok(Source::Checkout);
    }
    let version = match &args.version {
        Some(version) => version.clone(),
        None => crate::version::fixed_group_version(&ws.root)?.version,
    };
    wait_for_registries(ws, &version, args.wait_minutes)?;
    Ok(Source::Registry(version))
}

/// `cargo xtask verify-install …`: every runtime runs even when an earlier one failed; the
/// failures are listed at the end, with a table on stdout and in the job summary.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let golden = sha256_hex(&fs::read(ws.root.join(GOLDEN)).with_context(|| format!("reading {GOLDEN}"))?);
    // Before the work folder, so a failed wait leaves no folder behind.
    let source = source(ws, args)?;
    let work = work_dir(ws)?;
    step(&format!("verify-install: projects in {}", work.display()));

    let js = args.runtimes.iter().any(|r| *r != "rust");
    let mut outcomes = Vec::new();
    let installed = if js { Some(install_js(ws, &work, &source)) } else { None };
    for &runtime in &args.runtimes {
        match (runtime, &installed) {
            ("rust", _) => match &source {
                Source::Checkout => outcomes.extend(rust(ws, &work, args, &golden)),
                Source::Registry(version) => outcomes.push(Outcome {
                    label: format!("Rust binary, `{FACADE_CRATE} = \"={version}\"` from crates.io"),
                    result: rust_project(ws, &work, "rust-registry", &format!("\"={version}\""), &golden, false),
                }),
            },
            (_, Some(Err(error))) => outcomes.push(Outcome {
                label: runtime.to_owned(),
                result: Err(anyhow::anyhow!("installing the packages failed: {error:#}")),
            }),
            ("node", _) => {
                let mut compiled = None;
                for node in &args.nodes {
                    let version = version_of(node, &["--version"]);
                    let js = work.join("js");
                    let (label, result) = if parses_using(&version) {
                        (format!("Node {version}"), js_line(&js, node, &["main.ts"], &[]))
                    } else {
                        // Node before 24.2 can't parse `using`: build with tsc first, as the Node
                        // quick start tells its users to. Once, for every such Node.
                        let built = compiled.get_or_insert_with(|| compile_js(ws, &js)).as_ref();
                        let result = match built {
                            Ok(()) => js_line(&js, node, &["dist/main.js"], &[]),
                            Err(error) => Err(anyhow::anyhow!("tsc failed: {error:#}")),
                        };
                        (format!("Node {version} (built with tsc)"), result)
                    };
                    outcomes.push(Outcome {
                        label,
                        result: result.and_then(|out| check_line(&out, &golden)),
                    });
                }
            }
            ("deno", _) => {
                let deno = tool(ws, "deno");
                outcomes.push(Outcome {
                    label: format!("Deno {}", version_of(&deno, &["--version"])),
                    result: js_line(&work.join("js"), &deno, &["run", "--allow-read", "main.ts"], &[])
                        .and_then(|out| check_line(&out, &golden)),
                });
            }
            ("bun", _) => {
                let bun = tool(ws, "bun");
                outcomes.push(Outcome {
                    label: format!("Bun {}", version_of(&bun, &["--version"])),
                    result: js_line(&work.join("js"), &bun, &["run", "main.ts"], &[])
                        .and_then(|out| check_line(&out, &golden)),
                });
            }
            ("chromium", _) => outcomes.push(chromium(ws, &work, &golden)),
            ("workerd", _) => outcomes.push(workerd(ws, &work, &golden)),
            (other, None) => bail!("no installed packages for {other}"),
            (other, _) => bail!("no check for runtime {other:?}"),
        }
    }

    let table = table(&source.heading(), &outcomes);
    println!("\n{table}");
    summary(&table)?;
    if args.keep {
        println!("kept {}", work.display());
    } else {
        fs::remove_dir_all(&work).with_context(|| format!("removing {}", work.display()))?;
    }
    let failed: Vec<&str> = outcomes
        .iter()
        .filter(|o| o.result.is_err())
        .map(|o| o.label.as_str())
        .collect();
    if failed.is_empty() {
        step(&format!("verify-install: {} passed", outcomes.len()));
        Ok(())
    } else {
        for outcome in &outcomes {
            if let Err(error) = &outcome.result {
                eprintln!("verify-install: {} failed: {error:#}", outcome.label);
            }
        }
        bail!("verify-install: {} failed", failed.join(", "))
    }
}

/// The Markdown table of `outcomes`, under a heading that says what was installed.
fn table(heading: &str, outcomes: &[Outcome]) -> String {
    let mut out = format!("### verify-install ({heading})\n\n| Runtime | Result |\n| --- | --- |\n");
    for outcome in outcomes {
        let result = match &outcome.result {
            Ok(detail) => format!("pass: {detail}"),
            Err(error) => format!("**fail**: {}", format!("{error:#}").lines().next().unwrap_or_default()),
        };
        let _ = writeln!(
            out,
            "| {} | {} |",
            crate::util::md_cell(&outcome.label),
            crate::util::md_cell(&result)
        );
    }
    out
}

/// The repository's pinned `name` (a dev dependency of `@laminapdf/sdk`), else `name` on `PATH`.
fn tool(ws: &Workspace, name: &str) -> String {
    let pinned = ws.root.join(TOOLS_PACKAGE).join("node_modules/.bin").join(name);
    if pinned.exists() {
        pinned.to_string_lossy().into_owned()
    } else {
        name.to_owned()
    }
}

/// Whether Node `version` (`v24.2.0`) parses `using` declarations: 24.2 and newer. An unreadable
/// version is taken as new, so the run shows the error instead of hiding it behind tsc.
fn parses_using(version: &str) -> bool {
    let mut parts = version.trim_start_matches('v').split('.').map(str::parse::<u32>);
    match (parts.next(), parts.next()) {
        (Some(Ok(major)), Some(Ok(minor))) => (major, minor) >= (24, 2),
        _ => true,
    }
}

/// Builds the JavaScript project into `dist/` with the repository's pinned TypeScript.
fn compile_js(ws: &Workspace, js: &Path) -> Result<()> {
    crate::run(js, &tool(ws, "tsc"), &NODE_TSC_ARGS)
}

/// The first line `program args…` prints, or `?`.
fn version_of(program: &str, args: &[&str]) -> String {
    Command::new(program_path(program))
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .map(|l| l.trim().to_owned())
        })
        .unwrap_or_else(|| "?".to_owned())
}

/// Runs `program args…` in `dir` and returns its stdout (stderr goes to ours); fails on a non-zero exit.
fn js_line(dir: &Path, program: &str, args: &[&str], envs: &[(&str, &str)]) -> Result<String> {
    step(&format!("{program} {} (in {})", args.join(" "), dir.display()));
    let output = Command::new(program_path(program))
        .args(args)
        .envs(envs.iter().copied())
        .current_dir(dir)
        .stderr(std::process::Stdio::inherit())
        .output()
        .with_context(|| format!("failed to start {program}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    print!("{stdout}");
    if !output.status.success() {
        bail!("`{program} {}` failed with {}", args.join(" "), output.status);
    }
    Ok(stdout)
}

/// Checks the `LAMINA_PDF_VERIFY {json}` line in `output` against the golden's SHA-256 (one page).
fn check_line(output: &str, golden: &str) -> Result<String> {
    let json = output
        .lines()
        .find_map(|line| line.trim().strip_prefix(PREFIX))
        .context("no LAMINA_PDF_VERIFY line in the output")?;
    let value: Value = serde_json::from_str(json).with_context(|| format!("unreadable check line {json:?}"))?;
    let sha = value.get("sha256").and_then(Value::as_str).unwrap_or_default();
    let pages = value.get("pages").and_then(Value::as_u64);
    let length = value.get("length").and_then(Value::as_u64).unwrap_or_default();
    if sha != golden {
        bail!("the quick start's {length} bytes have SHA-256 {sha}, not the golden's {golden} ({GOLDEN})");
    }
    if pages != Some(1) {
        bail!("the quick start's document reopened with {pages:?} pages, not 1");
    }
    Ok(format!("{length} bytes, identical to {GOLDEN}; reopened, 1 page"))
}

/// The version a `LAMINA_PDF_VERIFY_VERSION <runtime> <version>` line names, if any.
fn reported_version(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(VERSION_PREFIX))
        .map(str::to_owned)
}

/// Installs Lamina PDF with npm into `<work>/js` from `source` (the packed tarballs, or npm),
/// with the quick start and the checks: `js/` (Node, Deno, Bun), `js/vite/` and `js/worker/`.
fn install_js(ws: &Workspace, work: &Path, source: &Source) -> Result<()> {
    let dependencies = match source {
        Source::Checkout => pack(ws, work)?,
        Source::Registry(version) => serde_json::json!({ "@laminapdf/sdk": version }),
    };
    let js = work.join("js");
    let package = serde_json::json!({
        "name": "lamina-verify-install",
        "private": true,
        "type": "module",
        "dependencies": dependencies,
    });
    write(
        &js.join("package.json"),
        &(serde_json::to_string_pretty(&package)? + "\n"),
    )?;
    write_projects(ws, &js)?;
    // npm, not pnpm: a plain project, as a user would set one up. The tarballs need nothing from
    // a registry (@laminapdf/sdk's one dependency is the @laminapdf/wasm tarball beside it).
    crate::run(
        &js,
        "npm",
        &[
            "install",
            "--no-audit",
            "--no-fund",
            "--ignore-scripts",
            "--no-package-lock",
        ],
    )?;
    match source {
        Source::Checkout => Ok(()),
        Source::Registry(version) => check_installed(&js, version),
    }
}

/// Every npm package of the registry check installed at `version`: npm's resolution, not a
/// stale tag, gave the projects what they test.
fn check_installed(js: &Path, version: &str) -> Result<()> {
    for name in NPM_PACKAGES {
        let path = js.join("node_modules").join(name).join("package.json");
        let text = fs::read_to_string(&path).with_context(|| format!("{name} wasn't installed"))?;
        let package: Value = serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let installed = package.get("version").and_then(Value::as_str).unwrap_or("?");
        if installed != version {
            bail!("npm installed {name} {installed}, not {version}");
        }
    }
    Ok(())
}

/// Packs `@laminapdf/wasm` and `@laminapdf/sdk` into `<work>/tarballs`; returns the project's
/// dependencies on the two tarballs.
fn pack(ws: &Workspace, work: &Path) -> Result<Value> {
    for dist in ["packages/wasm/dist/lamina_pdf_text.wasm", "packages/sdk/dist/index.js"] {
        if !ws.root.join(dist).is_file() {
            bail!("{dist} is missing: run `cargo xtask wasm` first");
        }
    }
    let tarballs = work.join("tarballs");
    fs::create_dir_all(&tarballs)?;
    let destination = tarballs.to_string_lossy().into_owned();
    for package in ["packages/wasm", "packages/sdk"] {
        crate::run(
            &ws.root.join(package),
            "pnpm",
            &["pack", "--pack-destination", &destination],
        )?;
    }
    let wasm = find_tarball(&tarballs, "laminapdf-wasm-")?;
    let pdf = find_tarball(&tarballs, "laminapdf-sdk-")?;
    check_packed_dependency(&pdf)?;
    Ok(serde_json::json!({
        "@laminapdf/sdk": format!("file:{}", pdf.display()),
        "@laminapdf/wasm": format!("file:{}", wasm.display()),
    }))
}

/// The quick start, its options and the checks, copied into each project of `js`.
fn write_projects(ws: &Workspace, js: &Path) -> Result<()> {
    let hello = fs::read_to_string(ws.root.join(HELLO_TS)).with_context(|| format!("reading {HELLO_TS}"))?;
    let hello_options =
        fs::read_to_string(ws.root.join(HELLO_OPTIONS_TS)).with_context(|| format!("reading {HELLO_OPTIONS_TS}"))?;
    for dir in [js.to_path_buf(), js.join("vite"), js.join("worker/src")] {
        write(&dir.join("hello.ts"), &hello)?;
        write(&dir.join("hello-options.ts"), &hello_options)?;
        write(&dir.join("report.ts"), REPORT_TS)?;
    }
    write(&js.join("main.ts"), MAIN_TS)?;
    write(&js.join("vite/index.html"), VITE_INDEX)?;
    write(&js.join("vite/main.ts"), VITE_MAIN)?;
    write(&js.join("vite/chromium.mjs"), CHROMIUM_RUNNER)?;
    for dir in ["vite", "worker"] {
        write(&js.join(dir).join("free-port.mjs"), FREE_PORT)?;
    }
    write(&js.join("worker/src/index.ts"), WORKER_INDEX)?;
    write(&js.join("worker/wrangler.toml"), WRANGLER_TOML)?;
    write(&js.join("worker/workerd.mjs"), WORKERD_RUNNER)
}

fn write(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text).with_context(|| format!("writing {}", path.display()))
}

fn find_tarball(dir: &Path, prefix: &str) -> Result<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with(prefix) && Path::new(n).extension().is_some_and(|e| e.eq_ignore_ascii_case("tgz"))
            })
        })
        .collect();
    match (found.pop(), found.is_empty()) {
        (Some(path), true) => Ok(path),
        (None, _) => bail!("pnpm pack wrote no {prefix}*.tgz"),
        (Some(_), false) => bail!("more than one {prefix}*.tgz in {}", dir.display()),
    }
}

/// The packed `@laminapdf/sdk` must depend on `@laminapdf/wasm` at its own version (`pnpm pack`
/// replaces `workspace:*`): a user installing the tarballs would otherwise need the workspace.
fn check_packed_dependency(pdf: &Path) -> Result<()> {
    let output = Command::new("tar")
        .args(["-xzOf"])
        .arg(pdf)
        .arg("package/package.json")
        .output()
        .context("failed to run tar")?;
    if !output.status.success() {
        bail!("reading package.json from {} failed", pdf.display());
    }
    let package: Value = serde_json::from_slice(&output.stdout).context("the packed package.json is not JSON")?;
    let version = package.get("version").and_then(Value::as_str);
    let dependency = package
        .pointer("/dependencies/@laminapdf~1wasm")
        .and_then(Value::as_str);
    if version.is_none() || dependency != version {
        bail!("the packed @laminapdf/sdk depends on @laminapdf/wasm {dependency:?}, not its own version {version:?}");
    }
    Ok(())
}

/// The Vite app, built and opened in Chromium (installs Playwright's Chromium first, a no-op
/// when it is there).
fn chromium(ws: &Workspace, work: &Path, golden: &str) -> Outcome {
    let result = (|| {
        crate::run(
            &ws.root,
            "pnpm",
            &[
                "--filter",
                "@laminapdf/sdk",
                "exec",
                "playwright",
                "install",
                "chromium",
            ],
        )?;
        let from = ws.root.join(TOOLS_PACKAGE).join("package.json");
        let from = from.to_string_lossy();
        let output = js_line(
            &work.join("js/vite"),
            "node",
            &["chromium.mjs"],
            &[("LAMINA_PDF_TOOLS_FROM", &from)],
        )?;
        Ok((reported_version(&output), check_line(&output, golden)?))
    })();
    split("Chromium (Vite app)", result)
}

/// The Worker under `wrangler dev --local`.
fn workerd(ws: &Workspace, work: &Path, golden: &str) -> Outcome {
    let wrangler = tool(ws, "wrangler");
    let version = version_of(&wrangler, &["--version"]);
    let result = js_line(
        &work.join("js/worker"),
        "node",
        &["workerd.mjs"],
        &[("LAMINA_PDF_WRANGLER", &wrangler)],
    )
    .and_then(|out| check_line(&out, golden));
    Outcome {
        label: format!("workerd (wrangler {version} dev --local)"),
        result,
    }
}

/// `label`, with the version the runner reported when it did.
fn split(label: &str, result: Result<(Option<String>, String)>) -> Outcome {
    match result {
        Ok((version, detail)) => Outcome {
            label: version.map_or_else(|| label.to_owned(), |v| format!("{label}: {v}")),
            result: Ok(detail),
        },
        Err(error) => Outcome {
            label: label.to_owned(),
            result: Err(error),
        },
    }
}

/// The two Rust binaries: `laminapdf` by path and by git, built with the pinned toolchain into one
/// shared target directory (their third-party crates build once).
fn rust(ws: &Workspace, work: &Path, args: &Args, golden: &str) -> Vec<Outcome> {
    let git_ref = args.git_ref.clone().unwrap_or_else(|| "HEAD".to_owned());
    let git_url = args.git_url.clone().or_else(|| {
        git_stdout(&ws.root, &["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .map(|dir| format!("file://{dir}"))
    });
    let path_dep = format!(
        "{{ path = {:?} }}",
        ws.root.join("crates").join(FACADE_CRATE).to_string_lossy()
    );
    let spec = git_spec(&ws.root, &git_ref);
    let (kind, value) = spec.as_ref().map_or(("rev", git_ref.clone()), |(k, v)| (*k, v.clone()));
    let mut outcomes = vec![Outcome {
        label: format!("Rust binary, `{FACADE_CRATE} = {{ path = … }}`"),
        result: rust_project(ws, work, "rust-path", &path_dep, golden, true),
    }];
    outcomes.push(Outcome {
        label: format!(
            "Rust binary, `{FACADE_CRATE} = {{ git = {}, {kind} = \"{value}\" }}`",
            git_url.as_deref().unwrap_or("?")
        ),
        result: match (&git_url, spec) {
            (Some(url), Ok(_)) => rust_project(
                ws,
                work,
                "rust-git",
                &format!("{{ git = {url:?}, {kind} = {value:?} }}"),
                golden,
                true,
            ),
            (None, _) => Err(anyhow::anyhow!("no git URL: pass --git-url")),
            (_, Err(error)) => Err(error),
        },
    });
    outcomes
}

/// Builds and runs the quick start in `<work>/<name>` with `dependency` on Lamina PDF. `locked`
/// copies the repository's `Cargo.lock`, so the third-party crates are the versions CI tests
/// (Cargo drops the entries this project doesn't use); without it Cargo resolves afresh, as a
/// user's project does.
fn rust_project(
    ws: &Workspace,
    work: &Path,
    name: &str,
    dependency: &str,
    golden: &str,
    locked: bool,
) -> Result<String> {
    let dir = work.join(name);
    write(&dir.join("Cargo.toml"), &rust_manifest(name, dependency))?;
    write(
        &dir.join("src/hello.rs"),
        &fs::read_to_string(ws.root.join(HELLO_RS)).with_context(|| format!("reading {HELLO_RS}"))?,
    )?;
    write(&dir.join("src/main.rs"), RUST_MAIN)?;
    if locked {
        fs::copy(ws.root.join("Cargo.lock"), dir.join("Cargo.lock")).context("copying Cargo.lock")?;
    }
    let toolchain = crate::doctor::toolchain_channel(ws)?;
    let target = work.join("rust-target").to_string_lossy().into_owned();
    let out = dir.join("out");
    let out_arg = out.to_string_lossy().into_owned();
    crate::run_env(
        &dir,
        "cargo",
        &["run", "--quiet", "--", &out_arg],
        &[
            ("RUSTUP_TOOLCHAIN", &toolchain),
            ("CARGO_TARGET_DIR", &target),
            // The system git: it has the user's credentials (SSH keys, credential helpers).
            ("CARGO_NET_GIT_FETCH_WITH_CLI", "true"),
            ("CARGO_PROFILE_DEV_DEBUG", "0"),
        ],
    )?;
    let bytes = fs::read(out.join("hello.pdf")).context("the binary wrote no hello.pdf")?;
    let sha = sha256_hex(&bytes);
    if sha != golden {
        bail!(
            "hello.pdf ({} bytes) has SHA-256 {sha}, not the golden's {golden} ({GOLDEN})",
            bytes.len()
        );
    }
    Ok(format!("{} bytes, identical to {GOLDEN}", bytes.len()))
}

/// The `Cargo.toml` of a Rust project: the façade crate under its own name, as a user writes it.
fn rust_manifest(name: &str, dependency: &str) -> String {
    format!(
        "# cargo xtask verify-install: the Rust quick start ({HELLO_RS}) outside the\n\
         # repository. An empty [workspace]: this project is its own workspace.\n\
         [package]\nname = \"lamina-verify-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
         [dependencies]\n{FACADE_CRATE} = {dependency}\n\n[workspace]\n"
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Args, FACADE_CRATE, check_line, git_spec, parse_args, rust_manifest};
    use crate::util::git_stdout;

    #[test]
    fn rust_main_should_use_the_options_the_hello_golden_was_written_with() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let read = |path: &str| std::fs::read_to_string(root.join(path)).unwrap();
        let examples = read("crates/laminapdf/tests/examples.rs");
        let options_ts = read(super::HELLO_OPTIONS_TS);
        for value in ["lamina-ex-hello!", "lamina examples", "D:20261030120000Z"] {
            let quoted = format!("\"{value}\"");
            assert!(super::RUST_MAIN.contains(&quoted), "main.rs.in lacks {quoted}");
            assert!(examples.contains(&quoted), "tests/examples.rs lacks {quoted}");
            assert!(options_ts.contains(&quoted), "hello-options.ts lacks {quoted}");
        }
    }

    #[test]
    fn parses_using_should_start_at_node_24_2() {
        for (version, expected) in [
            ("v22.23.4", false),
            ("v24.1.0", false),
            ("v24.2.0", true),
            ("v26.0.0", true),
            ("?", true),
        ] {
            assert_eq!(super::parses_using(version), expected, "{version}");
        }
    }

    #[test]
    fn hello_options_should_import_nothing_relative() {
        // verify-install copies the file alone into each project, so a relative import can't resolve.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source = std::fs::read_to_string(root.join(super::HELLO_OPTIONS_TS)).unwrap();
        assert!(
            !source.contains("from \"./") && !source.contains("from \"../"),
            "{source}"
        );
    }

    #[test]
    fn rust_manifest_should_depend_on_the_facade_by_its_package_name() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let crate_manifest =
            std::fs::read_to_string(root.join("crates").join(FACADE_CRATE).join("Cargo.toml")).unwrap();
        let package: toml::Table = toml::from_str(&crate_manifest).unwrap();
        assert_eq!(package["package"]["name"].as_str(), Some(FACADE_CRATE));

        let manifest: toml::Table = toml::from_str(&rust_manifest("rust-path", "{ path = \"x\" }")).unwrap();
        let deps = manifest["dependencies"].as_table().unwrap();
        assert_eq!(deps.keys().collect::<Vec<_>>(), vec![FACADE_CRATE]);
    }

    fn args(list: &[&str]) -> Args {
        parse_args(&list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()).unwrap()
    }

    #[test]
    fn parse_args_should_default_to_every_runtime_and_node_on_path() {
        let parsed = args(&[]);
        assert_eq!((parsed.runtimes.len(), parsed.nodes), (6, vec!["node".to_owned()]));
    }

    #[test]
    fn parse_args_should_read_runtimes_and_flags() {
        let parsed = args(&[
            "rust",
            "node",
            "--node",
            "/n22",
            "--node",
            "/n24",
            "--git-ref",
            "v0.1.0-alpha.0",
            "--keep",
        ]);
        assert_eq!(
            (parsed.runtimes, parsed.nodes, parsed.git_ref.as_deref(), parsed.keep),
            (
                vec!["rust", "node"],
                vec!["/n22".to_owned(), "/n24".to_owned()],
                Some("v0.1.0-alpha.0"),
                true
            )
        );
    }

    #[test]
    fn parse_args_should_reject_unknown_runtimes() {
        assert!(parse_args(&["firefox".to_owned()]).is_err());
    }

    #[test]
    fn parse_args_should_read_the_registry_flags() {
        let parsed = args(&["node", "--from-registry", "--version", "0.2.0-alpha.2", "--wait", "45"]);
        assert!(parsed.from_registry);
        assert_eq!(parsed.version.as_deref(), Some("0.2.0-alpha.2"));
        assert_eq!(parsed.wait_minutes, 45);
    }

    #[test]
    fn parse_args_should_keep_the_registry_flags_together() {
        let fails = |list: &[&str]| parse_args(&list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>()).is_err();
        assert!(
            fails(&["--version", "0.2.0-alpha.2"]),
            "--version without --from-registry"
        );
        assert!(fails(&["--wait", "5"]), "--wait without --from-registry");
        assert!(fails(&["--from-registry", "--wait", "soon"]), "--wait takes minutes");
        assert!(
            fails(&["--from-registry", "--git-ref", "v0.2.0"]),
            "no git dependency from the registry"
        );
    }

    #[test]
    fn a_registry_manifest_should_pin_the_published_version() {
        let manifest: toml::Table = toml::from_str(&rust_manifest("rust-registry", "\"=0.2.0-alpha.2\"")).unwrap();
        assert_eq!(manifest["dependencies"][FACADE_CRATE].as_str(), Some("=0.2.0-alpha.2"));
    }

    /// A scratch repository with two commits, a branch and a tag on the first commit.
    fn scratch_repo(name: &str) -> (crate::tracked::TempTree, String) {
        let tree = crate::tracked::TempTree::new(name, &[("a.txt", "a")]);
        let git = |args: &[&str]| {
            let ok = std::process::Command::new("git")
                .args([
                    "-c",
                    "user.name=t",
                    "-c",
                    "user.email=t@example.invalid",
                    "-c",
                    "commit.gpgsign=false",
                ])
                .args(args)
                .current_dir(&tree.0)
                .status()
                .unwrap()
                .success();
            assert!(ok, "git {args:?}");
        };
        git(&["init", "-q"]);
        git(&["add", "a.txt"]);
        git(&["commit", "-q", "-m", "first"]);
        git(&["tag", "v0.0.0-test.1"]);
        git(&["branch", "feature"]);
        git(&["commit", "-q", "--allow-empty", "-m", "second"]);
        let first = git_stdout(&tree.0, &["rev-parse", "HEAD~1"]).unwrap();
        (tree, first)
    }

    #[test]
    fn git_spec_should_pin_a_tag_by_tag() {
        let (tree, _) = scratch_repo("spec-tag");
        assert_eq!(
            git_spec(&tree.0, "v0.0.0-test.1").unwrap(),
            ("tag", "v0.0.0-test.1".to_owned())
        );
    }

    #[test]
    fn git_spec_should_pin_a_branch_by_its_full_sha() {
        let (tree, first) = scratch_repo("spec-branch");
        assert_eq!(git_spec(&tree.0, "feature").unwrap(), ("rev", first));
    }

    #[test]
    fn git_spec_should_expand_a_short_sha() {
        let (tree, first) = scratch_repo("spec-short");
        assert_eq!(git_spec(&tree.0, first.get(..8).unwrap()).unwrap(), ("rev", first));
    }

    #[test]
    fn git_spec_should_reject_an_unknown_ref() {
        let (tree, _) = scratch_repo("spec-unknown");
        assert!(git_spec(Path::new(&tree.0), "no-such-ref").is_err());
    }

    #[test]
    fn check_line_should_accept_the_golden_and_reject_other_bytes() {
        let line = |sha: &str| format!("noise\nLAMINA_PDF_VERIFY {{\"sha256\":\"{sha}\",\"length\":10,\"pages\":1}}\n");
        assert_eq!(
            [
                check_line(&line("ab"), "ab").is_ok(),
                check_line(&line("cd"), "ab").is_ok(),
                check_line("", "ab").is_ok()
            ],
            [true, false, false]
        );
    }
}
