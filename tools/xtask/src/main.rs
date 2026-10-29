//! lamina's task runner, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
//!
//! This is the single entry point for repo-wide tasks; CI runs the same commands developers run
//! (docs/guidelines/code-organization.md §10). Run `cargo xtask help` for the task list.

mod changes;
mod docs_links;
mod doctor;
mod harness;
mod licenses;
mod portable_paths;
mod todos;
mod tracked;
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
                    everything a PR must pass (default: docs, rust and ts)
                    docs: docs-links, todos --check and portable paths
                    rust: fmt --check, portability, clippy (native + wasm32), tests, rustdoc, cargo-deny
                    ts:   biome ci, typecheck and tests of the pnpm workspace, npm licences
  fmt [--check]     format Rust (cargo fmt) and TS/JSON (biome); --check only verifies
  lint              clippy native (all targets) and wasm32 (core crates), biome ci
  test              Rust tests (cargo-nextest when installed, else cargo test) + doc tests, TS tests
  portability       core crates: forbidden-API grep, dependency and unsafe rules,
                    cargo check for wasm32, iOS and Android
  wasm              build the wasm32 crates with the size-optimised `wasm-release` profile
  deny              cargo-deny (licences, advisories, bans, sources) + npm licence check
  npm-licenses      npm licence check only (pnpm licenses list)
  docs-links [--json]
                    check every relative link and #anchor in the tracked Markdown files
  todos [--check]   regenerate the todo board (docs/todos/README.md) and each todo's Blocks
                    field from the Depends on links; --check only verifies
  golden [--update] regenerate every golden in fixtures/golden/ from its source and print the
                    list (--update writes them; without it, fail if they would change)
  corpus facts      rewrite fixtures/corpus/expected.json with pypdf (needs Python: see
                    tools/corpus-facts/requirements.txt; $LAMINA_PYTHON or python3)
  differential [--full] [--update]
                    lamina vs pypdf over the corpus and fixed-seed mutants, from the precomputed
                    fixtures/differential/expected.json; --update regenerates it with pypdf.
                    --full: the full lane's 36 mutants per corpus file instead (full.json),
                    run through cargo-nextest
  fuzz <target> [--time <seconds>]
                    run a fuzz target (lex, parse_object, flate_decode, open, page_tree) with
                    cargo +nightly fuzz (default 60 s); seeds come from fixtures/ and fuzz/seeds/
  fuzz --all [--time <seconds>]
                    run every fuzz target in turn (the full lane: --time 120)
  fuzz seeds [<target>]
                    only build the seed corpora in fuzz/corpus/
  fuzz cmin [<target>]
                    minimise the corpora in fuzz/corpus/ (cargo fuzz cmin)
  changes [--base <rev>]
                    list the paths changed since the merge base with <rev> (default origin/main)
                    and say whether any is code (not docs); sets `code` in $GITHUB_OUTPUT
                    (default base: origin/main, or main when there is no origin/main)
  ci                check + wasm: the whole fast lane, locally
  doctor [--install]
                    check that external tools are installed at their pinned versions;
                    --install runs the `cargo install` fixes
  help              show this message";

/// Targets every core crate must build for (they are also pinned in `rust-toolchain.toml`).
const PORTABLE_TARGETS: [&str; 3] = ["wasm32-unknown-unknown", "aarch64-apple-ios", "aarch64-linux-android"];
const WASM_TARGET: &str = "wasm32-unknown-unknown";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map_or("help", String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    // Help must work even when the workspace can't load (stale lock file, missing layer metadata).
    if matches!(task, "help" | "-h" | "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    // `changes` runs first in the full lane and needs only git: skip `cargo metadata`.
    if task == "changes" {
        return match rest {
            [] => {
                let root = repo_root()?;
                changes::run(&root, changes::default_base(&root))
            }
            [flag, base] if flag == "--base" => changes::run(&repo_root()?, base),
            _ => bail!("usage: cargo xtask changes [--base <rev>]"),
        };
    }
    let ws = Workspace::load(repo_root()?)?;

    match (task, rest) {
        ("check", []) => {
            check_docs(&ws)?;
            check_rust(&ws)?;
            check_ts(&ws)
        }
        ("check", [scope]) if scope == "docs" => check_docs(&ws),
        ("check", [scope]) if scope == "rust" => check_rust(&ws),
        ("check", [scope]) if scope == "ts" => check_ts(&ws),
        ("fmt", []) => fmt(&ws, false),
        ("fmt", [flag]) if flag == "--check" => fmt(&ws, true),
        ("lint", []) => {
            clippy(&ws)?;
            biome_ci(&ws)
        }
        ("test", []) => {
            rust_tests(&ws)?;
            ts_tests(&ws)
        }
        ("portability", []) => portability(&ws),
        ("wasm", []) => wasm(&ws),
        ("deny", []) => {
            cargo_deny(&ws)?;
            licenses::check_npm(&ws)
        }
        ("npm-licenses", []) => licenses::check_npm(&ws),
        ("doctor", []) => doctor::run(&ws, false),
        ("doctor", [flag]) if flag == "--install" => doctor::run(&ws, true),
        ("docs-links", []) => docs_links::run(&ws.root, false),
        ("docs-links", [flag]) if flag == "--json" => docs_links::run(&ws.root, true),
        ("todos", []) => todos::run(&ws.root, false),
        ("todos", [flag]) if flag == "--check" => todos::run(&ws.root, true),
        ("golden", []) => harness::golden(&ws, false),
        ("golden", [flag]) if flag == "--update" => harness::golden(&ws, true),
        ("corpus", [what]) if what == "facts" => harness::corpus_facts(&ws),
        ("differential", args) => harness::differential(&ws, &harness::parse_differential_args(args)?),
        ("fuzz", args) => harness::fuzz(&ws, &harness::parse_fuzz_args(args)?),
        ("ci", []) => {
            check_docs(&ws)?;
            check_rust(&ws)?;
            check_ts(&ws)?;
            wasm(&ws)
        }
        _ => bail!("unknown task or arguments: {}\n\n{USAGE}", args.join(" ")),
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

/// The documentation checks: fast and network-free, so they run first.
fn check_docs(ws: &Workspace) -> Result<()> {
    step("docs-links: relative links and anchors in tracked Markdown");
    docs_links::run(&ws.root, false)?;
    step("todos --check: the todo board and Blocks fields are up to date");
    todos::run(&ws.root, true)?;
    step("portable paths: every tracked path checks out on Windows, macOS and Linux");
    portable_paths::check(&ws.root)?;
    step("Docs checks passed");
    Ok(())
}

fn check_rust(ws: &Workspace) -> Result<()> {
    fmt_rust(ws, true)?;
    portability(ws)?;
    clippy(ws)?;
    rust_tests(ws)?;
    docs(ws)?;
    cargo_deny(ws)?;
    step("Rust checks passed");
    Ok(())
}

fn check_ts(ws: &Workspace) -> Result<()> {
    biome_ci(ws)?;
    ws.pnpm(&["-r", "--if-present", "run", "typecheck"])?;
    ts_tests(ws)?;
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
    ws.cargo(&args)
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
            "no {kind} crates found: every crate in crates/ declares [package.metadata.lamina] layer, \
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
    args.extend(package_args(ws.wasm_crates(), "wasm32")?);
    // The core crates are rlibs, so there is no .wasm artefact yet; this only proves they build
    // with the shipping profile. When the lamina-wasm cdylib lands (Round B), this task also runs
    // wasm-bindgen and wasm-opt (binaryen pinned in doctor.rs, with explicit feature flags matching
    // rustc's wasm32 defaults), enforces a size budget, and writes the output gitignored inside the
    // consuming package, never as a separate workspace member.
    ws.cargo(&args)
}

fn cargo_deny(ws: &Workspace) -> Result<()> {
    doctor::require(ws, &doctor::cargo_deny())?;
    ws.cargo(&["deny", "--all-features", "check"])
}

fn biome_ci(ws: &Workspace) -> Result<()> {
    ws.pnpm(&["exec", "biome", "ci", "."])
}

fn ts_tests(ws: &Workspace) -> Result<()> {
    ws.pnpm(&["-r", "--if-present", "run", "test"])
}

fn step(message: &str) {
    println!("\n==> {message}");
}

/// Runs `program args…` in `dir`, echoing the command, and fails if it exits unsuccessfully.
fn run<S: AsRef<OsStr>>(dir: &Path, program: &str, args: &[S]) -> Result<()> {
    run_env(dir, program, args, &[])
}

/// [`run`] with extra environment variables.
fn run_env<S: AsRef<OsStr>>(dir: &Path, program: &str, args: &[S], envs: &[(&str, &str)]) -> Result<()> {
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
