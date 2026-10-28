//! lamina's task runner, run as `cargo xtask <task>` (alias in `.cargo/config.toml`).
//!
//! This is the single entry point for repo-wide tasks; CI runs the same commands developers run
//! (docs/guidelines/code-organization.md §10). Run `cargo xtask help` for the task list.

mod doctor;
mod licenses;
mod workspace;

use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

const USAGE: &str = "\
usage: cargo xtask <task> [args]

tasks:
  check [rust|ts]   everything a PR must pass (default: rust and ts)
                    rust: fmt --check, portability, clippy (native + wasm32), tests, rustdoc
                    ts:   biome ci, typecheck and tests of the pnpm workspace
  fmt [--check]     format Rust (cargo fmt) and TS/JSON (biome); --check only verifies
  lint              clippy native (all targets) and wasm32 (core crates), biome ci
  test              Rust tests (cargo-nextest when installed, else cargo test) + doc tests, TS tests
  portability       core crates: forbidden-API grep, dependency and unsafe rules,
                    cargo check for wasm32, iOS and Android
  wasm              build the wasm32 crates with the size-optimised `wasm-release` profile
  deny              cargo-deny (licences, advisories, bans, sources) + npm licence check
  npm-licenses      npm licence check only (pnpm licenses list)
  ci                check + wasm + deny: the whole fast lane, locally
  doctor            check that external tools are installed at their pinned versions
  help              show this message";

/// Targets every core crate must build for (they are also pinned in `rust-toolchain.toml`).
const PORTABLE_TARGETS: [&str; 3] = ["wasm32-unknown-unknown", "aarch64-apple-ios", "aarch64-linux-android"];
const WASM_TARGET: &str = "wasm32-unknown-unknown";

fn main() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    let task = args.first().map_or("help", String::as_str);
    let rest = args.get(1..).unwrap_or_default();
    let ws = Workspace::load(repo_root()?)?;

    match (task, rest) {
        ("check", []) => {
            check_rust(&ws)?;
            check_ts(&ws)
        }
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
        ("doctor", []) => doctor::run(&ws),
        ("ci", []) => {
            check_rust(&ws)?;
            check_ts(&ws)?;
            wasm(&ws)?;
            cargo_deny(&ws)?;
            licenses::check_npm(&ws)
        }
        ("help" | "-h" | "--help", _) => {
            println!("{USAGE}");
            Ok(())
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

fn check_rust(ws: &Workspace) -> Result<()> {
    fmt_rust(ws, true)?;
    portability(ws)?;
    clippy(ws)?;
    rust_tests(ws)?;
    docs(ws)?;
    step("Rust checks passed");
    Ok(())
}

fn check_ts(ws: &Workspace) -> Result<()> {
    biome_ci(ws)?;
    ws.pnpm(&["-r", "--if-present", "run", "typecheck"])?;
    ts_tests(ws)?;
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
    args.extend(ws.wasm_crates().flat_map(|name| ["-p", name]));
    args.extend(["--", "-D", "warnings"]);
    ws.cargo(&args)
}

fn rust_tests(ws: &Workspace) -> Result<()> {
    if has_nextest(ws) {
        ws.cargo(&["nextest", "run", "--locked", "--workspace", "--all-features"])?;
    } else {
        eprintln!("note: cargo-nextest not found, using cargo test (install: cargo install cargo-nextest --locked)");
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

fn has_nextest(ws: &Workspace) -> bool {
    Command::new("cargo")
        .args(["nextest", "--version"])
        .current_dir(&ws.root)
        .output()
        .is_ok_and(|out| out.status.success())
}

/// The bytes-in/bytes-out rules for core crates (code-organization.md §2), then cross-target builds.
fn portability(ws: &Workspace) -> Result<()> {
    step("portability: forbidden APIs, dependencies and unsafe in core crates");
    ws.check_core_sources()?;
    for target in PORTABLE_TARGETS {
        let mut args = vec!["check", "--locked", "--lib", "--target", target];
        args.extend(ws.core_crates().flat_map(|name| ["-p", name]));
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
    args.extend(ws.wasm_crates().flat_map(|name| ["-p", name]));
    // The core crates are rlibs, so there is no .wasm artefact yet; this only proves they build
    // with the shipping profile. When the lamina-wasm cdylib lands (Round B), this task also runs
    // wasm-bindgen and wasm-opt (binaryen pinned in doctor.rs, with explicit feature flags matching
    // rustc's wasm32 defaults), enforces a size budget, and writes the output gitignored inside the
    // consuming package, never as a separate workspace member.
    ws.cargo(&args)
}

fn cargo_deny(ws: &Workspace) -> Result<()> {
    let installed = Command::new("cargo")
        .args(["deny", "--version"])
        .output()
        .is_ok_and(|out| out.status.success());
    if !installed {
        bail!("cargo-deny is not installed: cargo install cargo-deny --locked");
    }
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
    let shown: Vec<_> = args.iter().map(|a| a.as_ref().to_string_lossy()).collect();
    step(&format!("{program} {}", shown.join(" ")));
    let status = Command::new(program)
        .args(args)
        .current_dir(dir)
        .status()
        .with_context(|| format!("failed to start `{program}`; is it installed and on PATH?"))?;
    if !status.success() {
        bail!("`{program} {}` failed with {status}", shown.join(" "));
    }
    Ok(())
}
