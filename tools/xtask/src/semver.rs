//! `cargo xtask semver`: does the next release's version say what changed since the published
//! one? (Todo P3; `docs/stability-policy.md`.) Two checks against the latest published release:
//!
//! - **Rust:** `cargo semver-checks check-release --workspace` compares every published crate
//!   with its newest version on crates.io (the tool is pinned in `doctor.rs`).
//! - **TypeScript:** API Extractor's report of each `@laminapdf/sdk` entry point, from this
//!   checkout's build (`cargo xtask wasm` first) and from the published package installed from
//!   npm. A line of the published report missing from this one is a breaking change: an export,
//!   member or signature removed or changed.
//!
//! The stability policy puts breaking changes in a minor release while the version is `0.x`, so a
//! breaking change passes while a pending changeset gives the fixed group a minor (or major) bump,
//! and fails otherwise: add one with `pnpm changeset`. Whether each PR declares its own break is
//! a review matter; this check keeps the release's bump honest.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::util::summary;
use crate::workspace::Workspace;
use crate::{program_path, step};

/// The package whose public API the TypeScript check compares.
const SDK: &str = "@laminapdf/sdk";

/// A changeset's bump, least to most.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Bump {
    Patch,
    Minor,
    Major,
}

impl Bump {
    fn release_type(self) -> &'static str {
        match self {
            Self::Patch => "patch",
            Self::Minor => "minor",
            Self::Major => "major",
        }
    }
}

/// The largest bump the pending changesets give any of `members` (patch when none does). A
/// changeset is a `.changeset/*.md` file other than the README, whose front matter lists
/// `"<package>": <bump>` lines.
fn pending_bump(changesets: &[String], members: &[String]) -> Bump {
    let mut bump = Bump::Patch;
    for text in changesets {
        let mut lines = text.lines();
        if lines.next().map(str::trim) != Some("---") {
            continue;
        }
        for line in lines.take_while(|l| l.trim() != "---") {
            let Some((name, kind)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim().trim_matches(|c| c == '"' || c == '\'');
            if !members.iter().any(|m| m == name) {
                continue;
            }
            let this = match kind.trim() {
                "major" => Bump::Major,
                "minor" => Bump::Minor,
                _ => Bump::Patch,
            };
            bump = bump.max(this);
        }
    }
    bump
}

fn read_changesets(root: &Path) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    for entry in fs::read_dir(root.join(".changeset")).context("reading .changeset/")? {
        let path = entry?.path();
        let is_changeset =
            path.extension().is_some_and(|e| e == "md") && path.file_name().is_some_and(|n| n != "README.md");
        if is_changeset {
            texts.push(fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?);
        }
    }
    Ok(texts)
}

/// The lines of an API Extractor report that state API: no blank lines, comments, imports
/// (where a type comes from is not API) or the header. A union of string literals on one line
/// (`export type ErrorCode = "a" | "b";`) becomes one line per member, so a new member is an
/// addition: new error codes are not breaking (the stability policy).
fn api_lines(report: &str) -> BTreeSet<String> {
    let mut lines = BTreeSet::new();
    for line in report.lines().map(str::trim_end) {
        let t = line.trim_start();
        let skip = t.is_empty() || ["//", "##", ">", "```", "import "].iter().any(|p| t.starts_with(p));
        if skip {
            continue;
        }
        match literal_union(t) {
            Some((head, members)) => lines.extend(members.into_iter().map(|m| format!("{head} = {m}"))),
            None => {
                lines.insert(line.to_owned());
            }
        }
    }
    lines
}

/// `export type Name = "a" | "b";` as (`export type Name`, its members); None for any other line.
fn literal_union(line: &str) -> Option<(&str, Vec<&str>)> {
    let (head, body) = line.split_once("= ")?;
    let members: Vec<&str> = body.strip_suffix(';')?.split(" | ").collect();
    let literals = members
        .iter()
        .all(|m| m.len() >= 2 && m.starts_with('"') && m.ends_with('"'));
    (head.starts_with("export type ") && literals).then(|| (head.trim_end(), members))
}

/// `(removed, added)`: the published report's API lines missing from this checkout's, and the
/// other way round.
fn compare(published: &str, local: &str) -> (Vec<String>, Vec<String>) {
    let (before, after) = (api_lines(published), api_lines(local));
    let removed = before.difference(&after).cloned().collect();
    let added = after.difference(&before).cloned().collect();
    (removed, added)
}

/// The `.d.ts` of each entry point in `package` (its `package.json`'s `exports`, the plain
/// `types` or the `default` condition's), by subpath.
fn entry_points(package: &Path) -> Result<Vec<(String, PathBuf)>> {
    let manifest: Value = serde_json::from_str(&fs::read_to_string(package.join("package.json"))?)?;
    let exports = manifest
        .get("exports")
        .and_then(Value::as_object)
        .context("package.json has no exports")?;
    let mut entries = Vec::new();
    for (subpath, target) in exports {
        let types = target
            .get("types")
            .or_else(|| target.pointer("/default/types"))
            .and_then(Value::as_str);
        if let Some(types) = types {
            entries.push((subpath.clone(), package.join(types)));
        }
    }
    Ok(entries)
}

/// The report's file name for entry point `subpath`: `.` → `index`, `./worker/server` →
/// `worker-server`.
fn report_name(subpath: &str) -> String {
    let name = subpath.trim_start_matches("./").replace('/', "-");
    if name.is_empty() || name == "." {
        "index".to_owned()
    } else {
        name
    }
}

/// Runs API Extractor on `entry` of `package`, writing `<out>/<name>.api.md`. The config goes
/// inside the package: API Extractor finds the package's `package.json` from the config's folder.
fn api_report(ws: &Workspace, package: &Path, entry: &Path, name: &str, out: &Path) -> Result<String> {
    let config_dir = package.join("node_modules/.cache/lamina-api-report");
    fs::create_dir_all(&config_dir)?;
    let config = json!({
        "projectFolder": package,
        "mainEntryPointFilePath": entry,
        "apiReport": { "enabled": true, "reportFolder": out, "reportTempFolder": out.join("tmp"), "reportFileName": name },
        "docModel": { "enabled": false },
        "dtsRollup": { "enabled": false },
        "tsdocMetadata": { "enabled": false },
        "compiler": { "overrideTsconfig": { "compilerOptions": {
            "target": "ES2022", "module": "NodeNext", "moduleResolution": "NodeNext",
            "lib": ["ES2022", "DOM", "ESNext.Disposable"], "types": [], "skipLibCheck": true, "strict": true,
        } } },
        "messages": {
            "compilerMessageReporting": { "default": { "logLevel": "warning" } },
            "extractorMessageReporting": { "default": { "logLevel": "none" } },
            "tsdocMessageReporting": { "default": { "logLevel": "none" } },
        },
    });
    let path = config_dir.join(format!("{name}.json"));
    fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    let path = path.to_string_lossy().into_owned();
    ws.pnpm(&[
        "--filter",
        SDK,
        "exec",
        "api-extractor",
        "run",
        "--local",
        "--config",
        &path,
    ])?;
    let report = out.join(format!("{name}.api.md"));
    fs::read_to_string(&report).with_context(|| format!("API Extractor wrote no {}", report.display()))
}

/// Installs `@laminapdf/sdk@<version>` from npm into `<work>/published`, and returns a copy of
/// it beside that folder's `node_modules`: API Extractor reads a package under `node_modules` as
/// an external one, listing its re-exports instead of their declarations. From the copy, the
/// package's own imports (`@laminapdf/wasm`) still resolve to the installed packages.
fn install_published(work: &Path, version: &str) -> Result<PathBuf> {
    let dir = work.join("published");
    fs::create_dir_all(&dir)?;
    fs::write(
        dir.join("package.json"),
        "{\"name\":\"lamina-semver\",\"private\":true,\"type\":\"module\"}\n",
    )?;
    let spec = format!("{SDK}@{version}");
    crate::run(
        &dir,
        "npm",
        &[
            "install",
            "--no-audit",
            "--no-fund",
            "--ignore-scripts",
            "--no-package-lock",
            &spec,
        ],
    )?;
    let copy = dir.join("sdk");
    copy_dir(&dir.join("node_modules").join(SDK), &copy)?;
    Ok(copy)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from).with_context(|| format!("reading {}", from.display()))? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).with_context(|| format!("copying to {}", target.display()))?;
        }
    }
    Ok(())
}

/// The newest published `@laminapdf/sdk`: npm's last listed version but the `0.0.0` placeholders
/// that reserve names.
fn newest_published(ws: &Workspace) -> Result<String> {
    let output = std::process::Command::new(program_path("npm"))
        .args(["view", SDK, "versions", "--json"])
        .current_dir(&ws.root)
        .output()
        .context("running npm view")?;
    let versions: Vec<String> = serde_json::from_slice(&output.stdout).context("npm view printed no version list")?;
    versions
        .into_iter()
        .rfind(|v| !v.starts_with("0.0.0"))
        .context("npm lists no published version of @laminapdf/sdk")
}

/// One check's result for the summary: the name, the breaking changes found, and the rest.
struct Outcome {
    name: String,
    breaking: Vec<String>,
    note: String,
}

/// The TypeScript check: every entry point's report, published against this checkout.
fn typescript(ws: &Workspace, work: &Path, version: &str) -> Result<Vec<Outcome>> {
    let local = ws.root.join("packages/sdk");
    if !local.join("dist/index.d.ts").is_file() {
        bail!("packages/sdk/dist is missing: run `cargo xtask wasm` first");
    }
    let published = install_published(work, version)?;
    let (before, after) = (entry_points(&published)?, entry_points(&local)?);
    let mut outcomes = Vec::new();
    for (subpath, entry) in &before {
        let name = report_name(subpath);
        let label = format!("{SDK} `{subpath}`");
        let Some((_, local_entry)) = after.iter().find(|(s, _)| s == subpath) else {
            outcomes.push(Outcome {
                name: label,
                breaking: vec![format!("the entry point {subpath} is gone")],
                note: String::new(),
            });
            continue;
        };
        let old = api_report(ws, &published, entry, &name, &work.join("reports/published"))?;
        let new = api_report(ws, &local, local_entry, &name, &work.join("reports/local"))?;
        let (removed, added) = compare(&old, &new);
        outcomes.push(Outcome {
            name: label,
            breaking: removed,
            note: format!("{} API lines added", added.len()),
        });
    }
    for (subpath, _) in after.iter().filter(|(s, _)| !before.iter().any(|(b, _)| b == s)) {
        outcomes.push(Outcome {
            name: format!("{SDK} `{subpath}`"),
            breaking: Vec::new(),
            note: "a new entry point".to_owned(),
        });
    }
    Ok(outcomes)
}

/// cargo-semver-checks' exit code when it finds a breaking change; others are its own errors.
const SEMVER_CHECKS_BREAKING: i32 = 100;

/// The Rust check: `cargo semver-checks` over the publishable crates that crates.io has at
/// `baseline`, as a release that may add but not break (`--release-type minor`), so it reports
/// every breaking change; whether one is allowed is the caller's decision. A crate added since
/// has no baseline: it is listed, not checked. The tool's own output names each change.
fn rust(ws: &Workspace, baseline: &str) -> Result<Outcome> {
    crate::doctor::require(ws, &crate::doctor::cargo_semver_checks())?;
    let mut new = Vec::new();
    for name in ws.publishable_crates() {
        if !crate::publish::on_crates_io(name, baseline)? {
            new.push(name.to_owned());
        }
    }
    let mut args = vec![
        "semver-checks",
        "check-release",
        "--workspace",
        "--baseline-version",
        baseline,
    ];
    args.extend(["--release-type", "minor"]);
    for name in &new {
        args.extend(["--exclude", name.as_str()]);
    }
    step(&format!("cargo {}", args.join(" ")));
    let status = std::process::Command::new("cargo")
        .args(&args)
        .current_dir(&ws.root)
        .status()
        .context("running cargo semver-checks")?;
    let breaking = match status.code() {
        Some(0) => Vec::new(),
        Some(SEMVER_CHECKS_BREAKING) => {
            vec!["cargo-semver-checks found breaking changes: its output above lists them".to_owned()]
        }
        _ => bail!("cargo semver-checks failed with {status}"),
    };
    let mut note = String::from("breaking changes against the published crates");
    if !new.is_empty() {
        let _ = write!(note, "; new since {baseline}, so unchecked: {}", new.join(", "));
    }
    Ok(Outcome {
        name: "Rust crates (cargo-semver-checks)".to_owned(),
        breaking,
        note,
    })
}

/// `cargo xtask semver [--baseline <version>]`.
pub(crate) fn run(ws: &Workspace, args: &[String]) -> Result<()> {
    let baseline = match args {
        [] => newest_published(ws)?,
        [flag, version] if flag == "--baseline" => version.clone(),
        _ => bail!("usage: cargo xtask semver [--baseline <version>]"),
    };
    let members: Vec<String> = crate::version::fixed_group_version(&ws.root)?
        .members
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let bump = pending_bump(&read_changesets(&ws.root)?, &members);
    let breaking_allowed = bump >= Bump::Minor;
    step(&format!(
        "semver: against the published {baseline}; the pending changesets make the next release a {} one",
        bump.release_type()
    ));
    let work = ws.root.join("target/semver");
    if work.exists() {
        fs::remove_dir_all(&work)?;
    }
    fs::create_dir_all(&work)?;
    let mut outcomes = vec![rust(ws, &baseline)?];
    outcomes.extend(typescript(ws, &work, &baseline)?);
    let mut table = format!(
        "### Semver against {baseline} (next release: {})\n\n| Check | Breaking changes | Notes |\n| --- | --- | --- |\n",
        bump.release_type()
    );
    for o in &outcomes {
        let _ = writeln!(
            table,
            "| {} | {} | {} |",
            o.name,
            o.breaking.len(),
            crate::util::md_cell(&o.note)
        );
    }
    for o in outcomes.iter().filter(|o| !o.breaking.is_empty()) {
        let _ = writeln!(table, "\n{}:\n\n```text\n{}\n```", o.name, o.breaking.join("\n"));
    }
    println!("{table}");
    summary(&table)?;
    let broken: Vec<&str> = outcomes
        .iter()
        .filter(|o| !o.breaking.is_empty())
        .map(|o| o.name.as_str())
        .collect();
    if broken.is_empty() || breaking_allowed {
        return Ok(());
    }
    bail!(
        "breaking changes since {baseline} in {}, and no pending changeset bumps the minor version: \
         add one with `pnpm changeset` (minor; the stability policy lists breaking changes under \
         Minor Changes while the version is 0.x)",
        broken.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members() -> Vec<String> {
        vec!["@laminapdf/sdk".to_owned(), "@laminapdf/wasm".to_owned()]
    }

    #[test]
    fn pending_bump_should_take_the_largest_bump_of_a_member() {
        let changesets = vec![
            "---\n\"@laminapdf/sdk\": patch\n---\n\nA fix.\n".to_owned(),
            "---\n\"@laminapdf/wasm\": minor\n\"@laminapdf/docs\": major\n---\n\nNew.\n".to_owned(),
        ];
        assert_eq!(pending_bump(&changesets, &members()), Bump::Minor);
    }

    #[test]
    fn pending_bump_should_be_patch_without_a_members_changeset() {
        let changesets = vec![
            "---\n\"@laminapdf/docs\": major\n---\n".to_owned(),
            "No front matter.\n".to_owned(),
        ];
        assert_eq!(pending_bump(&changesets, &members()), Bump::Patch);
        assert_eq!(pending_bump(&[], &members()), Bump::Patch);
    }

    #[test]
    fn compare_should_report_removed_lines_as_breaking_and_ignore_comments() {
        let published = "## API Report File for \"@laminapdf/sdk\"\n\n```ts\n// @public\nexport function a(x: number): void;\n\n// @public\nexport function b(): void;\n```\n";
        let local = "## API Report File for \"@laminapdf/sdk\"\n\n```ts\n// @beta\nexport function a(x: number, y?: string): void;\n\n// @public\nexport function b(): void;\n\nexport function c(): void;\n```\n";
        let (removed, added) = compare(published, local);
        assert_eq!(removed, ["export function a(x: number): void;"]);
        assert_eq!(
            added,
            [
                "export function a(x: number, y?: string): void;",
                "export function c(): void;"
            ]
        );
    }

    #[test]
    fn compare_should_treat_a_new_string_literal_member_as_an_addition_and_ignore_imports() {
        let published = "```ts\nimport { A } from './a.js';\nexport type Code = \"x\" | \"y\";\n```\n";
        let local = "```ts\nimport { A } from './b.js';\nexport type Code = \"x\" | \"y\" | \"z\";\n```\n";
        let (removed, added) = compare(published, local);
        assert_eq!(removed, Vec::<String>::new());
        assert_eq!(added, ["export type Code = \"z\""]);
        let (removed, _) = compare(local, published);
        assert_eq!(removed, ["export type Code = \"z\""]);
    }

    #[test]
    fn report_name_should_name_each_entry_point() {
        assert_eq!(report_name("."), "index");
        assert_eq!(report_name("./explain"), "explain");
        assert_eq!(report_name("./worker/server"), "worker-server");
    }

    #[test]
    fn entry_points_should_read_the_types_of_each_export() {
        let tree = crate::tracked::TempTree::new(
            "semver-entries",
            &[(
                "package.json",
                r#"{"exports": {".": {"workerd": {"types": "./w.d.ts"}, "default": {"types": "./i.d.ts", "default": "./i.js"}}, "./explain": {"types": "./e.d.ts"}, "./schema.json": "./s.json"}}"#,
            )],
        );
        let found = entry_points(&tree.0).unwrap_or_default();
        let names: Vec<(&str, String)> = found
            .iter()
            .map(|(s, p)| {
                (
                    s.as_str(),
                    p.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                )
            })
            .collect();
        assert_eq!(names, [(".", "i.d.ts".to_owned()), ("./explain", "e.d.ts".to_owned())]);
    }
}
