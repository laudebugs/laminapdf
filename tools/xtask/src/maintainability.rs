//! `cargo xtask check maintainability`: the rules of docs/guidelines/maintainability.md that a
//! tool can check in Rust code, without building anything.
//!
//! Each check is a name and a function in [`CHECKS`]. A check reads the [`Tree`] (the Rust files
//! of `crates/`, `tools/` and `apps/`, scanned once, and the check's config files) and returns its
//! findings. Every finding is an error: what exists today is recorded in a baseline or an
//! allow-list, and the check fails when the code and that record disagree in either direction.

mod budget;
mod names;
mod source;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::reports::{Finding, Ratchet};
use crate::tracked::Tracked;
use crate::util::has_extension;
use source::Source;

/// The guideline every finding points to.
const GUIDELINE: &str = "docs/guidelines/maintainability.md";
/// The modules over the size budget and the count of size-lint `#[expect]`s ([`budget`]).
const BASELINE: &str = "tools/xtask/maintainability-baseline.toml";
/// Today's duplicate public type names and façade collisions, each with its reason ([`names`]).
const DUPLICATES: &str = "tools/xtask/maintainability-duplicates.toml";
/// The top-level folders whose Rust files the checks read.
const ROOTS: [&str; 3] = ["crates/", "tools/", "apps/"];

/// One check: its name, as reports print it, and the function that runs it.
struct Check {
    name: &'static str,
    run: fn(&Tree) -> Result<Vec<Problem>>,
}

/// Every check, in the order they run. A new check is one line here.
const CHECKS: [Check; 4] = [
    Check {
        name: "module-budget",
        run: budget::module_budget,
    },
    Check {
        name: "size-expect",
        run: budget::size_expects,
    },
    Check {
        name: "duplicate-type",
        run: names::duplicate_types,
    },
    Check {
        name: "facade-collision",
        run: names::facade_collisions,
    },
];

/// One finding of a check, in one file.
#[derive(Debug)]
struct Problem {
    path: String,
    finding: Finding,
}

/// A finding of `rule` in `path` at `line` (0: about the whole file).
fn problem(path: &str, line: usize, rule: &'static str, message: impl Into<String>) -> Problem {
    Problem {
        path: path.to_owned(),
        finding: crate::reports::finding(line, rule, message),
    }
}

/// What the checks read: the Rust files, scanned, and the text of each config file.
struct Tree {
    sources: BTreeMap<String, Source>,
    configs: BTreeMap<&'static str, String>,
}

impl Tree {
    /// The tracked and untracked (not ignored) Rust files under [`ROOTS`] and the config files.
    fn load(root: &Path) -> Result<Self> {
        let untracked = Ratchet::untracked(root)?;
        let tracked = Tracked::from_git(root)?;
        let paths = tracked
            .files()
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .chain(untracked);
        let mut sources = BTreeMap::new();
        for path in paths.filter(|path| is_rust_in_scope(path)) {
            // A tracked file deleted in the working tree is gone: skip it.
            let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
                continue;
            };
            sources.insert(path, Source::new(&text));
        }
        let mut configs = BTreeMap::new();
        for config in [BASELINE, DUPLICATES] {
            let path = root.join(config);
            let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            configs.insert(config, text);
        }
        Ok(Self { sources, configs })
    }

    /// The text of config file `path` (one of the constants above).
    fn config(&self, path: &str) -> Result<&str> {
        self.configs
            .get(path)
            .map(String::as_str)
            .with_context(|| format!("{path} was not loaded"))
    }

    /// Parses config file `path` as TOML.
    fn parse_config<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        toml::from_str(self.config(path)?).with_context(|| format!("parsing {path}"))
    }

    /// The modules declared by `#[cfg(test)] mod name;` across the tree, as file paths.
    fn out_of_line_test_files(&self) -> Vec<String> {
        self.sources
            .iter()
            .flat_map(|(path, source)| {
                source
                    .test_modules
                    .iter()
                    .flat_map(move |name| child_module_files(path, name))
            })
            .collect()
    }
}

#[cfg(test)]
impl Tree {
    /// A tree of in-memory files: `.rs` files are scanned, the rest are config files (paths
    /// must be the constants above; a missing one is empty).
    fn from_files(files: &[(&str, &str)]) -> Self {
        let mut sources = BTreeMap::new();
        let mut configs: BTreeMap<&'static str, String> = [BASELINE, DUPLICATES]
            .into_iter()
            .map(|path| (path, String::new()))
            .collect();
        for (path, text) in files {
            match configs.keys().find(|config| *config == path).copied() {
                Some(config) => {
                    configs.insert(config, (*text).to_owned());
                }
                None => {
                    sources.insert((*path).to_owned(), Source::new(text));
                }
            }
        }
        Self { sources, configs }
    }
}

/// Whether the checks read `path`: a `.rs` file under [`ROOTS`].
fn is_rust_in_scope(path: &str) -> bool {
    has_extension(path, &["rs"]) && ROOTS.iter().any(|root| path.starts_with(root))
}

/// The two files a module `name` declared out of line in `parent` can live in.
fn child_module_files(parent: &str, name: &str) -> [String; 2] {
    let (dir, file) = parent.rsplit_once('/').unwrap_or(("", parent));
    let base = if matches!(file, "mod.rs" | "lib.rs" | "main.rs") {
        dir.to_owned()
    } else {
        format!("{dir}/{}", file.trim_end_matches(".rs"))
    };
    [format!("{base}/{name}.rs"), format!("{base}/{name}/mod.rs")]
}

/// Runs every check and fails if any finds a problem.
pub(crate) fn run(root: &Path) -> Result<()> {
    crate::step("maintainability: module budget, size-lint expects, duplicate type names, façade collisions");
    let tree = Tree::load(root)?;
    let problems = run_checks(&tree)?;
    for Problem { path, finding } in &problems {
        let Finding { line, rule, message } = finding;
        println!("error: {path}:{line}: {rule}: {message}");
    }
    let names: Vec<&str> = CHECKS.iter().map(|check| check.name).collect();
    println!(
        "maintainability: {} Rust files, {} check(s) ({}), {} problem(s)",
        tree.sources.len(),
        CHECKS.len(),
        names.join(", "),
        problems.len()
    );
    if !problems.is_empty() {
        bail!("maintainability: {} problem(s); see {GUIDELINE}", problems.len());
    }
    Ok(())
}

/// Every check's problems, sorted by path and line.
fn run_checks(tree: &Tree) -> Result<Vec<Problem>> {
    let mut problems = Vec::new();
    for check in &CHECKS {
        problems.extend((check.run)(tree).with_context(|| format!("check {}", check.name))?);
    }
    problems.sort_by(|a, b| (&a.path, a.finding.line).cmp(&(&b.path, b.finding.line)));
    Ok(problems)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_should_have_unique_names() {
        let mut names: Vec<&str> = CHECKS.iter().map(|check| check.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), CHECKS.len());
    }

    #[test]
    fn a_clean_tree_should_pass_every_check() {
        let tree = Tree::from_files(&[("crates/a/src/lib.rs", "pub struct A;\n")]);
        assert!(run_checks(&tree).unwrap().is_empty());
    }

    #[test]
    fn rust_files_outside_the_roots_should_be_out_of_scope() {
        let scoped = [
            "crates/a/src/lib.rs",
            "tools/xtask/src/main.rs",
            "spikes/x/src/lib.rs",
            "fuzz/src/lib.rs",
        ]
        .map(is_rust_in_scope);
        assert_eq!(scoped, [true, true, false, false]);
    }

    #[test]
    fn child_modules_of_lib_rs_should_sit_beside_it() {
        assert_eq!(
            child_module_files("crates/a/src/lib.rs", "tests"),
            ["crates/a/src/tests.rs", "crates/a/src/tests/mod.rs"]
        );
    }

    #[test]
    fn child_modules_of_a_named_file_should_sit_in_its_folder() {
        assert_eq!(
            child_module_files("crates/a/src/crypt.rs", "tests"),
            ["crates/a/src/crypt/tests.rs", "crates/a/src/crypt/tests/mod.rs"]
        );
    }
}
