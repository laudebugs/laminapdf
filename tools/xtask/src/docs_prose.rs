//! `cargo xtask check docs-prose`: the rules of docs/guidelines/docs-writing.md that a tool can
//! check, over every page a reader of the docs sees: the docs site's pages, the error-code pages
//! it renders (`docs/errors/`) and the package READMEs that `TypeDoc` renders.
//!
//! The check binds while people write: a report on a file changed since the merge base with
//! `main` (the working tree and untracked drafts included) is an error, and a report on a file
//! nobody has touched is a warning, counted per rule, until the rewrite todos have cleared them.
//! `--all-strict` makes every report an error (Q1l turns it on in `check docs`).

mod extract;
mod rules;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::changes::default_base;
use crate::tracked::{Tracked, git};
use rules::{Config, Finding, Scope};

/// How the check runs.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Options {
    /// Every report is an error, not only reports on changed files.
    pub(crate) all_strict: bool,
    /// Print the reports as JSON instead of text.
    pub(crate) json: bool,
    /// Print warnings one by one instead of counting them.
    pub(crate) warnings: bool,
}

/// One finding in one file, and whether it fails the check.
#[derive(Debug)]
struct Report {
    path: String,
    finding: Finding,
    error: bool,
}

/// `cargo xtask check docs-prose [--all-strict] [--warnings] [--json]`.
pub(crate) fn run(root: &Path, options: Options) -> Result<()> {
    let config = load_config(root)?;
    let files = files(root)?;
    let changed = changed_files(root);
    let mut reports = Vec::new();
    for (path, scope) in &files {
        let source = std::fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"))?;
        let page = extract::parse(&source);
        let error = options.all_strict || changed.as_ref().is_some_and(|changed| changed.contains(path));
        reports.extend(rules::check(&page, *scope, &config).into_iter().map(|finding| Report {
            path: path.clone(),
            finding,
            error,
        }));
    }
    if options.json {
        println!("{}", json(&reports));
    } else {
        print_text(&reports, files.len(), changed.as_ref(), options.warnings);
    }
    let errors = reports.iter().filter(|report| report.error).count();
    if errors > 0 {
        bail!("docs-prose: {errors} report(s) on changed files; see docs/guidelines/docs-writing.md");
    }
    Ok(())
}

fn load_config(root: &Path) -> Result<Config> {
    let path = root.join("tools/xtask/docs-prose.toml");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// The files the check reads, with the rules that apply to each, sorted by path. Untracked files
/// count too, so a new page is checked while it is drafted.
fn files(root: &Path) -> Result<BTreeMap<String, Scope>> {
    let tracked = Tracked::from_git(root)?;
    let untracked = git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
    let paths = tracked
        .files()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .chain(untracked.split('\0').filter(|path| !path.is_empty()).map(str::to_owned));
    Ok(paths
        .filter_map(|path| scope(&path).map(|scope| (path, scope)))
        .collect())
}

/// Which rules apply to a path, or `None` when the check doesn't read it.
fn scope(path: &str) -> Option<Scope> {
    if let Some(page) = path.strip_prefix("apps/docs/src/content/docs/") {
        if !has_extension(page, &["md", "mdx"]) {
            return None;
        }
        let guide = page.starts_with("guides/") || page.starts_with("recipes/");
        return Some(Scope::SitePage { guide });
    }
    let error_page = path.starts_with("docs/errors/") && has_extension(path, &["md"]) && !path.ends_with("/README.md");
    let package_readme =
        path.starts_with("packages/") && path.ends_with("/README.md") && path.matches('/').count() == 2;
    (error_page || package_readme).then_some(Scope::Rendered)
}

/// True when `path`'s extension is one of `extensions`, ignoring case.
fn has_extension(path: &str, extensions: &[&str]) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.iter().any(|wanted| extension.eq_ignore_ascii_case(wanted)))
}

/// Paths changed since the merge base with `main`, the working tree and untracked files included;
/// `None` (and a note) when there is no merge base, so nothing is strict.
fn changed_files(root: &Path) -> Option<BTreeSet<String>> {
    let base = default_base(root);
    let changed = git(root, &["merge-base", base, "HEAD"]).and_then(|merge_base| {
        let diff = git(root, &["diff", "--name-only", "--no-renames", "-z", merge_base.trim()])?;
        let untracked = git(root, &["ls-files", "--others", "--exclude-standard", "-z"])?;
        Ok(diff
            .split('\0')
            .chain(untracked.split('\0'))
            .filter(|path| !path.is_empty())
            .map(str::to_owned)
            .collect())
    });
    match changed {
        Ok(changed) => Some(changed),
        Err(error) => {
            eprintln!("docs-prose: no merge base with {base} ({error:#}); every report is a warning");
            None
        }
    }
}

fn print_text(reports: &[Report], file_count: usize, changed: Option<&BTreeSet<String>>, show_warnings: bool) {
    let mut warnings_by_rule: BTreeMap<&str, usize> = BTreeMap::new();
    for report in reports {
        if report.error || show_warnings {
            let level = if report.error { "error" } else { "warning" };
            let Finding { line, rule, message } = &report.finding;
            println!("{level}: {}:{line}: {rule}: {message}", report.path);
        }
        if !report.error {
            *warnings_by_rule.entry(report.finding.rule).or_default() += 1;
        }
    }
    let errors = reports.iter().filter(|report| report.error).count();
    let changed_count = changed.map_or(0, BTreeSet::len);
    println!(
        "docs-prose: {file_count} files, {errors} error(s) on changed files ({changed_count} paths changed), {} warning(s) elsewhere",
        reports.len() - errors
    );
    if !warnings_by_rule.is_empty() && !show_warnings {
        let counts: Vec<String> = warnings_by_rule
            .iter()
            .map(|(rule, count)| format!("{rule} {count}"))
            .collect();
        println!("  warnings by rule: {} (list them with --warnings)", counts.join(", "));
    }
}

fn json(reports: &[Report]) -> String {
    let items: Vec<serde_json::Value> = reports
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_pages_should_be_in_scope_and_guides_marked() {
        assert_eq!(
            scope("apps/docs/src/content/docs/install.mdx"),
            Some(Scope::SitePage { guide: false })
        );
        assert_eq!(
            scope("apps/docs/src/content/docs/guides/limits.mdx"),
            Some(Scope::SitePage { guide: true })
        );
        assert_eq!(
            scope("apps/docs/src/content/docs/recipes/invoice.mdx"),
            Some(Scope::SitePage { guide: true })
        );
    }

    #[test]
    fn rendered_pages_should_be_in_scope() {
        assert_eq!(scope("docs/errors/disposed.md"), Some(Scope::Rendered));
        assert_eq!(scope("packages/sdk/README.md"), Some(Scope::Rendered));
    }

    #[test]
    fn other_files_should_be_out_of_scope() {
        assert_eq!(scope("docs/errors/README.md"), None);
        assert_eq!(scope("packages/sdk/examples/README.md"), None);
        assert_eq!(scope("docs/plans/quality.md"), None);
        assert_eq!(scope("apps/docs/src/content/docs/data.json"), None);
    }

    #[test]
    fn the_repository_config_should_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(load_config(&root).is_ok_and(|config| !config.banned_words.is_empty()));
    }
}
