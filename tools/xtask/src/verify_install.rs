//! `cargo xtask verify-install`: does the library install and run outside this repository?
//! (ADR 0017, todo B7b; ADR 0018 §2's clean-install smoke tests of the release lane.)
//!
//! It packs `@laminapdf/wasm` and `@laminapdf/sdk` with `pnpm pack`, then builds temporary projects in
//! the system temp directory, outside the repository, whose only lamina dependencies are those
//! tarballs or the `laminapdf` crate, and runs the quick start (`packages/sdk/examples/hello.ts`,
//! `crates/laminapdf/examples/hello.rs`, copied in) in each runtime:
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
const HELLO_RS: &str = "crates/laminapdf/examples/hello.rs";
/// The package whose dev dependencies provide deno, bun, vite, Playwright and wrangler.
const TOOLS_PACKAGE: &str = "packages/sdk";
/// The line a JavaScript check prints (`report.ts`).
const PREFIX: &str = "LAMINA_VERIFY ";
const VERSION_PREFIX: &str = "LAMINA_VERIFY_VERSION ";

const REPORT_TS: &str = include_str!("verify_install/report.ts");
const MAIN_TS: &str = include_str!("verify_install/main.ts");
const VITE_INDEX: &str = include_str!("verify_install/vite/index.html");
const VITE_MAIN: &str = include_str!("verify_install/vite/main.ts");
const FREE_PORT: &str = include_str!("verify_install/free-port.mjs");
const CHROMIUM_RUNNER: &str = include_str!("verify_install/vite/chromium.mjs");
const WORKER_INDEX: &str = include_str!("verify_install/worker/index.ts");
const WRANGLER_TOML: &str = include_str!("verify_install/worker/wrangler.toml");
const WORKERD_RUNNER: &str = include_str!("verify_install/worker/workerd.mjs");

const USAGE: &str = "usage: cargo xtask verify-install [node|deno|bun|chromium|workerd|rust]… \
                     [--node <binary>]… [--git-ref <tag-or-sha>] [--git-url <url>] [--keep]";

/// What `cargo xtask verify-install` was asked to do.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Args {
    runtimes: Vec<&'static str>,
    nodes: Vec<String>,
    git_ref: Option<String>,
    git_url: Option<String>,
    keep: bool,
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

/// One runtime's result: its label (with the version found) and what went wrong, if anything.
struct Outcome {
    label: String,
    result: Result<String>,
}

/// `cargo xtask verify-install …`: every runtime runs even when an earlier one failed; the
/// failures are listed at the end, with a table on stdout and in the job summary.
pub(crate) fn run(ws: &Workspace, args: &Args) -> Result<()> {
    let golden = sha256_hex(&fs::read(ws.root.join(GOLDEN)).with_context(|| format!("reading {GOLDEN}"))?);
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
    step(&format!("verify-install: projects in {}", work.display()));

    let js = args.runtimes.iter().any(|r| *r != "rust");
    let mut outcomes = Vec::new();
    let installed = if js { Some(install_js(ws, &work)) } else { None };
    for &runtime in &args.runtimes {
        match (runtime, &installed) {
            ("rust", _) => outcomes.extend(rust(ws, &work, args, &golden)),
            (_, Some(Err(error))) => outcomes.push(Outcome {
                label: runtime.to_owned(),
                result: Err(anyhow::anyhow!("packing or installing the tarballs failed: {error:#}")),
            }),
            ("node", _) => {
                for node in &args.nodes {
                    let version = version_of(node, &["--version"]);
                    outcomes.push(Outcome {
                        label: format!("Node {version}"),
                        result: js_line(&work.join("js"), node, &["main.ts"], &[])
                            .and_then(|out| check_line(&out, &golden)),
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

    let table = table(&outcomes);
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

/// The Markdown table of `outcomes`.
fn table(outcomes: &[Outcome]) -> String {
    let mut out = String::from(
        "### verify-install (clean projects outside the repository)\n\n| Runtime | Result |\n| --- | --- |\n",
    );
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

/// Checks the `LAMINA_VERIFY {json}` line in `output` against the golden's SHA-256 (one page).
fn check_line(output: &str, golden: &str) -> Result<String> {
    let json = output
        .lines()
        .find_map(|line| line.trim().strip_prefix(PREFIX))
        .context("no LAMINA_VERIFY line in the output")?;
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

/// The version a `LAMINA_VERIFY_VERSION <runtime> <version>` line names, if any.
fn reported_version(output: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(VERSION_PREFIX))
        .map(str::to_owned)
}

/// Packs both packages into `<work>/tarballs` and installs them with npm into `<work>/js`, with
/// the quick start and the checks: `js/` (Node, Deno, Bun), `js/vite/` and `js/worker/`.
fn install_js(ws: &Workspace, work: &Path) -> Result<()> {
    for dist in ["packages/wasm/dist/lamina_text.wasm", "packages/sdk/dist/index.js"] {
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

    let js = work.join("js");
    let package = serde_json::json!({
        "name": "lamina-verify-install",
        "private": true,
        "type": "module",
        "dependencies": {
            "@laminapdf/sdk": format!("file:{}", pdf.display()),
            "@laminapdf/wasm": format!("file:{}", wasm.display()),
        },
    });
    write(
        &js.join("package.json"),
        &(serde_json::to_string_pretty(&package)? + "\n"),
    )?;
    let hello = fs::read_to_string(ws.root.join(HELLO_TS)).with_context(|| format!("reading {HELLO_TS}"))?;
    for dir in [js.clone(), js.join("vite"), js.join("worker/src")] {
        write(&dir.join("hello.ts"), &hello)?;
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
    write(&js.join("worker/workerd.mjs"), WORKERD_RUNNER)?;
    // npm, not pnpm: a plain project, as a user would set one up; the tarballs need nothing from
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
    )
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
            &[("LAMINA_TOOLS_FROM", &from)],
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
        &[("LAMINA_WRANGLER", &wrangler)],
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
    let path_dep = format!("{{ path = {:?} }}", ws.root.join("crates/laminapdf").to_string_lossy());
    let spec = git_spec(&ws.root, &git_ref);
    let (kind, value) = spec.as_ref().map_or(("rev", git_ref.clone()), |(k, v)| (*k, v.clone()));
    let mut outcomes = vec![Outcome {
        label: "Rust binary, `laminapdf = { path = … }`".to_owned(),
        result: rust_project(ws, work, "rust-path", &path_dep, golden),
    }];
    outcomes.push(Outcome {
        label: format!(
            "Rust binary, `laminapdf = {{ git = {}, {kind} = \"{value}\" }}`",
            git_url.as_deref().unwrap_or("?")
        ),
        result: match (&git_url, spec) {
            (Some(url), Ok(_)) => rust_project(
                ws,
                work,
                "rust-git",
                &format!("{{ git = {url:?}, {kind} = {value:?} }}"),
                golden,
            ),
            (None, _) => Err(anyhow::anyhow!("no git URL: pass --git-url")),
            (_, Err(error)) => Err(error),
        },
    });
    outcomes
}

fn rust_project(ws: &Workspace, work: &Path, name: &str, dependency: &str, golden: &str) -> Result<String> {
    let dir = work.join(name);
    let manifest = format!(
        "# cargo xtask verify-install: the Rust quick start (crates/laminapdf/examples/hello.rs) outside the\n\
         # repository. An empty [workspace]: this project is its own workspace.\n\
         [package]\nname = \"lamina-verify-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\npublish = false\n\n\
         [dependencies]\nlamina = {dependency}\n\n[workspace]\n"
    );
    write(&dir.join("Cargo.toml"), &manifest)?;
    write(
        &dir.join("src/main.rs"),
        &fs::read_to_string(ws.root.join(HELLO_RS)).with_context(|| format!("reading {HELLO_RS}"))?,
    )?;
    // The repository's lockfile, so the third-party crates are the versions CI tests (Cargo
    // drops the entries this project doesn't use).
    fs::copy(ws.root.join("Cargo.lock"), dir.join("Cargo.lock")).context("copying Cargo.lock")?;
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Args, check_line, git_spec, parse_args};
    use crate::util::git_stdout;

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
        let line = |sha: &str| format!("noise\nLAMINA_VERIFY {{\"sha256\":\"{sha}\",\"length\":10,\"pages\":1}}\n");
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
