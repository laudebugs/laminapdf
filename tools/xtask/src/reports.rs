//! The reports of a ratcheting check (`docs-prose`, `docs-snippets`, `clippy-examples`): a report
//! on a file changed since the merge base with `main` (the working tree and untracked drafts
//! included) is an error, and a report on a file nobody has touched is a warning, counted per rule,
//! until the cleanup todos have cleared them. `--all-strict` makes every report an error.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Result, bail};

use crate::changes::default_base;
use crate::tracked::git;

/// How a ratcheting check runs: its command-line flags.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Options {
    /// Every report is an error, not only reports on changed files.
    pub(crate) all_strict: bool,
    /// Print the reports as JSON instead of text.
    pub(crate) json: bool,
    /// Print warnings one by one instead of counting them.
    pub(crate) warnings: bool,
}

impl Options {
    /// Parses `[--all-strict] [--warnings] [--json]`; `name` is the check, for the usage line.
    pub(crate) fn parse(name: &str, flags: &[String]) -> Result<Self> {
        let mut options = Self::default();
        for flag in flags {
            match flag.as_str() {
                "--all-strict" => options.all_strict = true,
                "--warnings" => options.warnings = true,
                "--json" => options.json = true,
                _ => bail!("usage: cargo xtask check {name} [--all-strict] [--warnings] [--json]"),
            }
        }
        if options.json && options.warnings {
            bail!("{name}: --json already lists every report; drop --warnings");
        }
        Ok(options)
    }
}

/// One finding in one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// 1-based line, or 0 when the finding is about the whole file.
    pub(crate) line: usize,
    /// The rule's name, as the report prints it (`sentence-length`).
    pub(crate) rule: &'static str,
    /// What is wrong and what to do.
    pub(crate) message: String,
}

/// A finding at `line` for `rule`.
pub(crate) fn finding(line: usize, rule: &'static str, message: impl Into<String>) -> Finding {
    Finding {
        line,
        rule,
        message: message.into(),
    }
}

/// One finding in one file, and whether it fails the check.
#[derive(Debug)]
pub(crate) struct Report {
    path: String,
    finding: Finding,
    error: bool,
}

/// The files that make a report an error, and the reports collected so far.
#[derive(Debug)]
pub(crate) struct Ratchet {
    name: &'static str,
    options: Options,
    changed: Option<BTreeSet<String>>,
    reports: Vec<Report>,
}

impl Ratchet {
    /// Starts check `name`, reading the changed files from git; `untracked` is
    /// [`Ratchet::untracked`]'s list, which the check usually needs too.
    pub(crate) fn new(root: &Path, name: &'static str, options: Options, untracked: &[String]) -> Self {
        Self {
            name,
            options,
            changed: changed_files(root, untracked, name),
            reports: Vec::new(),
        }
    }

    /// Untracked files git doesn't ignore, so a check can read drafts too.
    pub(crate) fn untracked(root: &Path) -> Result<Vec<String>> {
        untracked_files(root)
    }

    /// Whether a report on `path` is an error.
    pub(crate) fn is_strict(&self, path: &str) -> bool {
        self.options.all_strict || self.changed.as_ref().is_some_and(|changed| changed.contains(path))
    }

    /// Records `findings` on `path`.
    pub(crate) fn add(&mut self, path: &str, findings: impl IntoIterator<Item = Finding>) {
        let error = self.is_strict(path);
        self.reports.extend(findings.into_iter().map(|finding| Report {
            path: path.to_owned(),
            finding,
            error,
        }));
    }

    /// Prints the reports and fails when any is an error; `file_count` is how many files were read.
    pub(crate) fn finish(mut self, file_count: usize, guideline: &str) -> Result<()> {
        self.reports
            .sort_by(|a, b| (&a.path, a.finding.line).cmp(&(&b.path, b.finding.line)));
        if self.options.json {
            println!("{}", self.json());
        } else {
            self.print_text(file_count);
        }
        let errors = self.reports.iter().filter(|report| report.error).count();
        if errors > 0 {
            bail!("{}: {errors} report(s) on changed files; see {guideline}", self.name);
        }
        Ok(())
    }

    fn print_text(&self, file_count: usize) {
        let mut warnings_by_rule: BTreeMap<&str, usize> = BTreeMap::new();
        for report in &self.reports {
            if report.error || self.options.warnings {
                let level = if report.error { "error" } else { "warning" };
                let Finding { line, rule, message } = &report.finding;
                println!("{level}: {}:{line}: {rule}: {message}", report.path);
            }
            if !report.error {
                *warnings_by_rule.entry(report.finding.rule).or_default() += 1;
            }
        }
        let errors = self.reports.iter().filter(|report| report.error).count();
        let changed_count = self.changed.as_ref().map_or(0, BTreeSet::len);
        println!(
            "{}: {file_count} files, {errors} error(s) on changed files ({changed_count} paths changed), {} warning(s) elsewhere",
            self.name,
            self.reports.len() - errors
        );
        if !warnings_by_rule.is_empty() && !self.options.warnings {
            let counts: Vec<String> = warnings_by_rule
                .iter()
                .map(|(rule, count)| format!("{rule} {count}"))
                .collect();
            println!("  warnings by rule: {} (list them with --warnings)", counts.join(", "));
        }
    }

    fn json(&self) -> String {
        let items: Vec<serde_json::Value> = self
            .reports
            .iter()
            .map(|report| {
                serde_json::json!({
                    "path": report.path,
                    "line": report.finding.line,
                    "rule": report.finding.rule,
                    "message": report.finding.message,
                    "level": if report.error { "error" } else { "warning" },
                })
            })
            .collect();
        serde_json::Value::Array(items).to_string()
    }
}

/// Paths changed since the merge base with `main`, the working tree and untracked files included;
/// `None` (and a note) when there is no merge base, so nothing is strict.
fn changed_files(root: &Path, untracked: &[String], name: &str) -> Option<BTreeSet<String>> {
    let base = default_base(root);
    let changed = git(root, &["merge-base", base, "HEAD"]).and_then(|merge_base| {
        let diff = git(root, &["diff", "--name-only", "--no-renames", "-z", merge_base.trim()])?;
        Ok(diff
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .chain(untracked.iter().cloned())
            .collect())
    });
    match changed {
        Ok(changed) => Some(changed),
        Err(error) => {
            // A shallow CI checkout has no merge base: say so where CI shows it, not only in the log.
            let note = format!(
                "no merge base with {base}, so every report is a warning (a shallow clone needs fetch-depth: 0)"
            );
            if std::env::var_os("GITHUB_ACTIONS").is_some() {
                println!("::warning title={name}::{note}");
            }
            eprintln!("{name}: {note} ({error:#})");
            None
        }
    }
}

/// Untracked files that git doesn't ignore.
fn untracked_files(root: &Path) -> Result<Vec<String>> {
    let listing = git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    Ok(listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(list: &[&str]) -> Vec<String> {
        list.iter().map(|flag| (*flag).to_owned()).collect()
    }

    #[test]
    fn flags_should_parse() {
        let options = Options::parse("x", &flags(&["--all-strict", "--warnings"]));
        assert!(options.is_ok_and(|options| options.all_strict && options.warnings && !options.json));
    }

    #[test]
    fn json_with_warnings_should_be_refused() {
        assert!(Options::parse("x", &flags(&["--json", "--warnings"])).is_err());
    }

    #[test]
    fn an_unknown_flag_should_be_refused() {
        assert!(Options::parse("x", &flags(&["--strict"])).is_err());
    }
}
