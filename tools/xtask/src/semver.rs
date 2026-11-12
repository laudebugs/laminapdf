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

/// The changesets in `dir` (none when it doesn't exist).
fn read_changesets(dir: &Path) -> Result<Vec<String>> {
    let mut texts = Vec::new();
    if !dir.is_dir() {
        return Ok(texts);
    }
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        let is_changeset =
            path.extension().is_some_and(|e| e == "md") && path.file_name().is_some_and(|n| n != "README.md");
        if is_changeset {
            texts.push(fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?);
        }
    }
    Ok(texts)
}

/// The bump this checkout's version already makes over the published `baseline`: none when they
/// are equal, else the first of major, minor and patch that differs. A pre-release step counts as
/// a patch, and the release of a pre-release's own version (`0.2.0-alpha.6` to `0.2.0`) as a
/// minor: the pre-releases promised nothing to each other, so it may carry any break they made.
fn version_step(baseline: &str, current: &str) -> Option<Bump> {
    if baseline == current {
        return None;
    }
    let numbers = |v: &str| -> Vec<u64> {
        let core = v.split(['-', '+']).next().unwrap_or(v);
        core.split('.').map(|n| n.parse().unwrap_or(0)).collect()
    };
    let (before, after) = (numbers(baseline), numbers(current));
    let leaves_pre_release = baseline.contains('-') && !current.contains('-');
    Some(if before.first() != after.first() {
        Bump::Major
    } else if before.get(1) != after.get(1) || (leaves_pre_release && before == after) {
        Bump::Minor
    } else {
        Bump::Patch
    })
}

/// The bump the next release may make: the pending changesets'. Between a version PR and the
/// release reaching npm, the checkout's version is ahead of `baseline` and the version PR has
/// consumed its changesets (moved to `.changeset/pre/` in pre mode); then that version's step and
/// the consumed changesets count too. In pre mode `.changeset/pre/` also holds earlier alphas'
/// changesets, so the check is lenient for that window, never stricter. Leaving pre mode
/// (`0.2.0`) removes `.changeset/pre/`, so there the version's step alone allows the alphas' breaks.
fn allowed_bump(root: &Path, members: &[String], baseline: &str, version: &str) -> Result<Bump> {
    let mut bump = pending_bump(&read_changesets(&root.join(".changeset"))?, members);
    if let Some(step) = version_step(baseline, version) {
        let consumed = pending_bump(&read_changesets(&root.join(".changeset/pre"))?, members);
        bump = bump.max(step).max(consumed);
    }
    Ok(bump)
}

/// The lines of an API Extractor report that state API: no blank lines, comments, imports
/// (where a type comes from is not API), the header or closing braces. Each line is prefixed with
/// the declarations it sits in (`export class PageRef › free(): void;`), and a line opening a
/// declaration is cut after its name (see [`scope_label`]). So a member removed from one class is
/// found while another class keeps the same line, and a class that gains `extends` or
/// `implements` keeps its members. A union of string literals
/// on one line (`export type ErrorCode = "a" | "b";`) becomes one line per member, so a new member
/// is an addition: new error codes are not breaking (the stability policy). Any changed line
/// counts as removed, an added optional parameter too: the check errs towards a minor bump.
fn api_lines(report: &str) -> BTreeSet<String> {
    let mut lines = BTreeSet::new();
    // One label per `{` still open, so a line that closes more braces than it opens (`}`,
    // `}): Promise<T>;`) closes exactly the blocks it ends.
    let mut scope: Vec<&str> = Vec::new();
    for line in report.lines().map(str::trim) {
        let skip = line.is_empty() || ["//", "##", ">", "```", "import "].iter().any(|p| line.starts_with(p));
        if skip {
            continue;
        }
        let (opens, closes) = (line.matches('{').count(), line.matches('}').count());
        if !matches!(line, "}" | "};") {
            let key = |text: &str| {
                let mut key = String::new();
                for label in &scope {
                    let _ = write!(key, "{label} › ");
                }
                key + text
            };
            match literal_union(line) {
                Some((head, members)) => lines.extend(members.into_iter().map(|m| key(&format!("{head} = {m}")))),
                None if opens > closes => {
                    lines.insert(key(scope_label(line)));
                }
                None => {
                    lines.insert(key(line));
                }
            }
        }
        for _ in closes..opens {
            scope.push(scope_label(line));
        }
        for _ in opens..closes {
            scope.pop();
        }
    }
    lines
}

/// A line opening a block, cut after the name it declares: `export class Builder` for
/// `export class Builder extends Handle<BaseGlue> {` (a class, interface, namespace, enum or type
/// alias), else the line itself, such as a member whose type is an object literal.
fn scope_label(line: &str) -> &str {
    let mut rest = line;
    for modifier in ["export ", "declare ", "default ", "abstract "] {
        rest = rest.strip_prefix(modifier).unwrap_or(rest);
    }
    for kind in ["class ", "interface ", "namespace ", "enum ", "type "] {
        if let Some(after) = rest.strip_prefix(kind) {
            let name = after
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                .unwrap_or(after.len());
            let end = line.len() - rest.len() + kind.len() + name;
            return &line[..end];
        }
    }
    line
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

/// One `.d.ts` an entry point declares: its subpath, the export conditions that name it (a
/// plain `types` counts as `default`), and a label: the subpath, with the conditions when the
/// subpath has several `.d.ts` files (`. (node)`).
#[derive(Debug)]
struct Entry {
    subpath: String,
    conditions: Vec<String>,
    file: PathBuf,
    label: String,
}

impl Entry {
    /// Whether `other` is the same entry point in another version of the package: the same
    /// subpath and a condition in common, so a release that gives `.` a `node` types file of its
    /// own still compares its `default` entry with the old one.
    fn matches(&self, other: &Entry) -> bool {
        self.subpath == other.subpath && self.conditions.iter().any(|c| other.conditions.contains(c))
    }
}

/// Every `.d.ts` the entry points of `package` declare (its `package.json`'s `exports`: the plain
/// `types` and each condition's, so the `node` and `workerd` entries of `.` are compared too).
fn entry_points(package: &Path) -> Result<Vec<Entry>> {
    let manifest: Value = serde_json::from_str(&fs::read_to_string(package.join("package.json"))?)?;
    let exports = manifest
        .get("exports")
        .and_then(Value::as_object)
        .context("package.json has no exports")?;
    let mut entries = Vec::new();
    for (subpath, target) in exports {
        let mut files: Vec<(&str, Vec<String>)> = Vec::new();
        let plain = target.get("types").map(|types| ("default", types));
        let conditions = target.as_object().into_iter().flatten();
        let named = conditions.filter_map(|(name, value)| Some((name.as_str(), value.get("types")?)));
        for (condition, types) in plain.into_iter().chain(named) {
            let Some(file) = types.as_str() else { continue };
            match files.iter_mut().find(|(f, _)| *f == file) {
                Some((_, known)) if !known.iter().any(|n| n == condition) => known.push(condition.to_owned()),
                Some(_) => {}
                None => files.push((file, vec![condition.to_owned()])),
            }
        }
        let several = files.len() > 1;
        for (file, conditions) in files {
            let label = if several {
                format!("{subpath} ({})", conditions.join(", "))
            } else {
                subpath.clone()
            };
            entries.push(Entry {
                subpath: subpath.clone(),
                conditions,
                file: package.join(file),
                label,
            });
        }
    }
    Ok(entries)
}

/// The report's file name for an entry point's label: `.` → `index`, `./worker/server` →
/// `worker-server`, `. (node)` → `node`.
fn report_name(label: &str) -> String {
    let name = label
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if name.is_empty() { "index".to_owned() } else { name }
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
    crate::util::copy_dir(&dir.join("node_modules").join(SDK), &copy)?;
    Ok(copy)
}

/// The newest published `@laminapdf/sdk`: npm's last listed version but the `0.0.0` placeholders
/// that reserve names.
fn newest_published(ws: &Workspace) -> Result<String> {
    let output = std::process::Command::new(program_path("npm"))
        .args(["view", SDK, "versions", "--json"])
        .current_dir(&ws.root)
        .output()
        .context("running npm view")?;
    if !output.status.success() {
        bail!(
            "`npm view {SDK} versions` failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    // npm prints a single version as a string, several as an array.
    let versions = match serde_json::from_slice(&output.stdout).context("npm view printed no JSON")? {
        Value::String(version) => vec![version],
        list => serde_json::from_value::<Vec<String>>(list).context("npm view printed no version list")?,
    };
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
    let published = install_published(work, version)?;
    let (before, after) = (entry_points(&published)?, entry_points(&local)?);
    let mut outcomes = Vec::new();
    for entry in &before {
        let name = report_name(&entry.label);
        let label = format!("{SDK} `{}`", entry.label);
        let Some(local_entry) = after.iter().find(|a| a.matches(entry)) else {
            outcomes.push(Outcome {
                name: label,
                breaking: vec![format!("the entry point {} is gone", entry.label)],
                note: String::new(),
            });
            continue;
        };
        let old = api_report(ws, &published, &entry.file, &name, &work.join("reports/published"))?;
        let new = api_report(ws, &local, &local_entry.file, &name, &work.join("reports/local"))?;
        let (removed, added) = compare(&old, &new);
        outcomes.push(Outcome {
            name: label,
            breaking: removed,
            note: format!("{} API lines added", added.len()),
        });
    }
    for entry in after.iter().filter(|a| !before.iter().any(|b| a.matches(b))) {
        outcomes.push(Outcome {
            name: format!("{SDK} `{}`", entry.label),
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
    // Checked first: the Rust check takes minutes.
    if !ws.root.join("packages/sdk/dist/index.d.ts").is_file() {
        bail!("packages/sdk/dist is missing: run `cargo xtask wasm` first");
    }
    let fixed = crate::version::fixed_group_version(&ws.root)?;
    let members: Vec<String> = fixed.members.into_iter().map(|(name, _)| name).collect();
    let bump = allowed_bump(&ws.root, &members, &baseline, &fixed.version)?;
    let breaking_allowed = bump >= Bump::Minor;
    step(&format!(
        "semver: {} against the published {baseline}; the changesets make the next release a {} one",
        fixed.version,
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
        assert_eq!(report_name(". (node)"), "node");
        assert_eq!(report_name("./worker (default, browser)"), "worker-default-browser");
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
            .map(|e| {
                let file = e.file.file_name().unwrap_or_default().to_string_lossy().into_owned();
                (e.label.as_str(), file)
            })
            .collect();
        assert_eq!(
            names,
            [
                (". (default)", "i.d.ts".to_owned()),
                (". (workerd)", "w.d.ts".to_owned()),
                ("./explain", "e.d.ts".to_owned())
            ]
        );
        // A published `.` with one types file is the new `.`'s default entry.
        let old = Entry {
            subpath: ".".to_owned(),
            conditions: vec!["default".to_owned()],
            file: PathBuf::from("i.d.ts"),
            label: ".".to_owned(),
        };
        let matched: Vec<&str> = found
            .iter()
            .filter(|e| e.matches(&old))
            .map(|e| e.label.as_str())
            .collect();
        assert_eq!(matched, [". (default)"]);
    }

    #[test]
    fn compare_should_follow_braces_when_a_literal_opens_mid_line() {
        let published = "```ts\nexport class A {\n    a(options?: {x: string;\n    }): void;\n    b(): void;\n}\nexport function c(): void;\n```\n";
        let lines = api_lines(published);
        assert!(lines.contains("export class A › b(): void;"), "{lines:?}");
        assert!(
            lines.contains("export class A › a(options?: {x: string; › }): void;"),
            "{lines:?}"
        );
        assert!(lines.contains("export function c(): void;"), "{lines:?}");
    }

    #[test]
    fn compare_should_find_a_member_removed_from_one_declaration_of_several() {
        let published = "```ts
export class A {
    free(): void;
}

export class B {
    free(): void;
}
```
";
        let local = "```ts
export class A {
    free(): void;
}

export class B {
}
```
";
        let (removed, added) = compare(published, local);
        assert_eq!(removed, ["export class B › free(): void;"]);
        assert_eq!(added, Vec::<String>::new());
    }

    #[test]
    fn compare_should_keep_a_class_s_members_when_it_gains_extends() {
        let published = "```ts\nclass Document_2 {\n    free(): void;\n}\n```\n";
        let local = "```ts\nclass Document_2 extends Handle<BaseGlue> {\n    free(): void;\n}\n```\n";
        assert_eq!(compare(published, local), (Vec::new(), Vec::new()));
        assert_eq!(
            scope_label("export declare abstract class A<T> implements B {"),
            "export declare abstract class A"
        );
        assert_eq!(
            scope_label("ensure<T>(f: () => T, options?: {"),
            "ensure<T>(f: () => T, options?: {"
        );
    }

    #[test]
    fn version_step_should_name_the_first_part_that_moved() {
        assert_eq!(version_step("0.2.0-alpha.2", "0.2.0-alpha.2"), None);
        assert_eq!(version_step("0.2.0-alpha.2", "0.2.0-alpha.3"), Some(Bump::Patch));
        assert_eq!(version_step("0.2.0", "0.2.1"), Some(Bump::Patch));
        assert_eq!(version_step("0.2.0-alpha.2", "0.3.0"), Some(Bump::Minor));
        assert_eq!(version_step("0.2.0-alpha.6", "0.2.0"), Some(Bump::Minor));
        assert_eq!(version_step("0.2.0-alpha.6", "0.2.1"), Some(Bump::Patch));
        assert_eq!(version_step("0.9.1", "1.0.0"), Some(Bump::Major));
    }

    #[test]
    fn allowed_bump_should_count_consumed_changesets_only_while_the_version_is_unpublished() {
        let minor = "---\n\"@laminapdf/sdk\": minor\n---\n\nA break.\n";
        let tree = crate::tracked::TempTree::new("semver-allowed", &[(".changeset/pre/old.md", minor)]);
        let allowed = |baseline: &str, version: &str| allowed_bump(&tree.0, &members(), baseline, version).ok();
        assert_eq!(allowed("0.2.0-alpha.2", "0.2.0-alpha.2"), Some(Bump::Patch));
        assert_eq!(allowed("0.2.0-alpha.2", "0.2.0-alpha.3"), Some(Bump::Minor));
        assert_eq!(allowed("0.2.1", "0.3.0"), Some(Bump::Minor));
    }
}
