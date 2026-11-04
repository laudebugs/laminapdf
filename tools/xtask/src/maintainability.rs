//! `cargo xtask check maintainability`: the rules of docs/guidelines/maintainability.md that a
//! tool can check in Rust code, without building anything.
//!
//! Each check is a name and a function in [`CHECKS`]. A check reads the [`Tree`] (the Rust files
//! of `crates/`, `tools/` and `apps/`, scanned once, and the two record files, parsed once) and
//! returns its problems. What exists today is recorded in [`BASELINE`] and [`DUPLICATES`]; a
//! check fails when the code and a record disagree in either direction, and when the baseline
//! grows against the merge base with `main`; it warns when the allow-list grows.

mod budget;
mod helpers;
mod lexers;
mod names;
mod policy;
mod source;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

use crate::reports::{Finding, Ratchet};
use crate::tracked::{Tracked, git};
use crate::util::has_extension;
use budget::Baseline;
use names::Allowed;
use source::{Source, TestModule};

/// The check's name, as its summary line prints it.
const NAME: &str = "maintainability";
/// The guideline every finding points to.
const GUIDELINE: &str = "docs/guidelines/maintainability.md";
/// The modules over the size budget and the count of size lints named by `#[expect]`/`#[allow]`
/// ([`budget`]).
const BASELINE: &str = "tools/xtask/maintainability-baseline.toml";
/// Today's duplicate public type names and façade collisions, each with its reason ([`names`]).
const DUPLICATES: &str = "tools/xtask/maintainability-duplicates.toml";
/// The top-level folders whose Rust files the checks read.
const ROOTS: [&str; 3] = ["crates/", "tools/", "apps/"];

/// One check: its name, as reports print it, and the function that runs it.
struct Check {
    name: &'static str,
    run: fn(&Tree) -> Vec<Problem>,
}

/// Every check, in the order they run. A new check is one entry here.
const CHECKS: [Check; 9] = [
    Check {
        name: "module-budget",
        run: budget::module_budget,
    },
    Check {
        name: "size-expect",
        run: budget::size_expects,
    },
    Check {
        name: "baseline-shrinks",
        run: budget::baseline_shrinks,
    },
    Check {
        name: "duplicate-type",
        run: names::duplicate_types,
    },
    Check {
        name: "facade-collision",
        run: names::facade_collisions,
    },
    Check {
        name: "allow-list-shrinks",
        run: names::allow_list_shrinks,
    },
    Check {
        name: "error-policy",
        run: policy::error_policy_tests,
    },
    Check {
        name: "test-helpers",
        run: helpers::test_helper_copies,
    },
    Check {
        name: "one-lexer",
        run: lexers::lexer_copies,
    },
];

/// One finding of a check in one file: an error, or a warning that only asks a reader to look.
#[derive(Debug)]
struct Problem {
    path: String,
    finding: Finding,
    warning: bool,
}

/// An error of `rule` in `path` at `line` (0: about the whole file).
fn problem(path: &str, line: usize, rule: &'static str, message: impl Into<String>) -> Problem {
    Problem {
        path: path.to_owned(),
        finding: crate::reports::finding(line, rule, message),
        warning: false,
    }
}

/// A warning of `rule` in `path`: printed, never a failure.
fn warning(path: &str, rule: &'static str, message: impl Into<String>) -> Problem {
    Problem {
        warning: true,
        ..problem(path, 0, rule, message)
    }
}

/// The two record files as the merge base with `main` has them, to check that they only shrink.
#[derive(Debug, Default)]
enum Base {
    /// Each file at the merge base; `None` when it didn't exist there yet.
    Records {
        baseline: Option<Baseline>,
        allowed: Option<Allowed>,
    },
    /// No merge base (a shallow clone), or a record file there that no longer parses: the records
    /// can't be compared, and a warning says so.
    #[default]
    Unknown,
}

/// What the checks read: the Rust files, scanned, and the record files, parsed.
#[derive(Debug, Default)]
struct Tree {
    sources: BTreeMap<String, Source>,
    baseline: Baseline,
    allowed: Allowed,
    base: Base,
}

impl Tree {
    /// The tracked and untracked (not ignored) Rust files under [`ROOTS`], the record files and
    /// their versions at the merge base with `main`.
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
        let read = |path: &str| std::fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"));
        Ok(Self {
            sources,
            baseline: parse(BASELINE, &read(BASELINE)?)?,
            allowed: parse(DUPLICATES, &read(DUPLICATES)?)?,
            base: load_base(root),
        })
    }

    /// The files of the modules declared by `#[cfg(test)] mod name;` across the tree.
    fn out_of_line_test_files(&self) -> Vec<String> {
        self.sources
            .iter()
            .flat_map(|(path, source)| {
                source
                    .test_modules
                    .iter()
                    .flat_map(move |module| test_module_files(path, module))
            })
            .collect()
    }
}

/// Parses record file `path`.
fn parse<T: DeserializeOwned>(path: &str, text: &str) -> Result<T> {
    toml::from_str(text).with_context(|| format!("parsing {path}"))
}

/// The record files at the merge base with `main` (`changes::default_base`).
fn load_base(root: &Path) -> Base {
    let base = crate::changes::default_base(root);
    let Ok(merge_base) = git(root, &["merge-base", base, "HEAD"]) else {
        return Base::Unknown;
    };
    let merge_base = merge_base.trim();
    let at_base = |path: &str| crate::changes::at_revision(root, merge_base, path).ok();
    // A base file that doesn't parse (a later schema change) can't be compared, like a missing
    // merge base: a warning, not a failure that only `--admin` could get past.
    let (Ok(baseline), Ok(allowed)) = (
        at_base(BASELINE).map(|text| parse(BASELINE, &text)).transpose(),
        at_base(DUPLICATES).map(|text| parse(DUPLICATES, &text)).transpose(),
    ) else {
        return Base::Unknown;
    };
    // A module moved since the merge base keeps its entry: compare it under its new path.
    let Ok(moved) = crate::changes::renames(root, merge_base) else {
        return Base::Unknown;
    };
    let baseline = baseline.map(|baseline: Baseline| baseline.follow_renames(&moved));
    Base::Records { baseline, allowed }
}

#[cfg(test)]
impl Tree {
    /// A tree of in-memory files: `.rs` files are scanned, [`BASELINE`] and [`DUPLICATES`]
    /// parsed (a missing one is empty). The merge base has the same records.
    fn from_files(files: &[(&str, &str)]) -> Self {
        let mut tree = Self::default();
        for (path, text) in files {
            match *path {
                BASELINE => tree.baseline = parse(path, text).unwrap(),
                DUPLICATES => tree.allowed = parse(path, text).unwrap(),
                _ => {
                    tree.sources.insert((*path).to_owned(), Source::new(text));
                }
            }
        }
        tree.base = Base::Records {
            baseline: Some(tree.baseline.clone()),
            allowed: Some(tree.allowed.clone()),
        };
        tree
    }

    /// The same tree with these record files at the merge base instead.
    fn with_base(mut self, baseline: &str, allowed: &str) -> Self {
        self.base = Base::Records {
            baseline: Some(parse(BASELINE, baseline).unwrap()),
            allowed: Some(parse(DUPLICATES, allowed).unwrap()),
        };
        self
    }
}

/// The crate a library or binary source belongs to: `crates/<crate>/src/…`.
fn crate_of(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("crates/")?;
    let (krate, inside) = rest.split_once('/')?;
    inside.starts_with("src/").then_some(krate)
}

/// Whether the checks read `path`: a `.rs` file under [`ROOTS`].
fn is_rust_in_scope(path: &str) -> bool {
    has_extension(path, &["rs"]) && ROOTS.iter().any(|root| path.starts_with(root))
}

/// The files a test module declared out of line in `parent` can live in: its `#[path]`,
/// relative to the parent's folder, or the two places Cargo looks for `name`.
fn test_module_files(parent: &str, module: &TestModule) -> Vec<String> {
    match &module.path {
        Some(path) => {
            let dir = parent.rsplit_once('/').map_or("", |(dir, _)| dir);
            vec![format!("{dir}/{path}")]
        }
        None => child_module_files(parent, &module.name).to_vec(),
    }
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

/// Runs every check and fails if any finds an error.
pub(crate) fn run(root: &Path) -> Result<()> {
    let names: Vec<&str> = CHECKS.iter().map(|check| check.name).collect();
    crate::step(&format!("maintainability: {}", names.join(", ")));
    let tree = Tree::load(root)?;
    let mut ratchet = Ratchet::everywhere(NAME);
    for Problem { path, finding, warning } in run_checks(&tree) {
        if warning {
            ratchet.warn(&path, [finding]);
        } else {
            ratchet.add(&path, [finding]);
        }
    }
    ratchet.finish(tree.sources.len(), GUIDELINE)
}

/// Every check's problems.
fn run_checks(tree: &Tree) -> Vec<Problem> {
    CHECKS.iter().flat_map(|check| (check.run)(tree)).collect()
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
        assert!(run_checks(&tree).is_empty());
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

    #[test]
    fn a_path_attribute_should_name_the_test_file_beside_its_parent() {
        let module = TestModule {
            name: "tests".to_owned(),
            path: Some("checks/tests.rs".to_owned()),
        };
        assert_eq!(
            test_module_files("crates/a/src/crypt.rs", &module),
            ["crates/a/src/checks/tests.rs"]
        );
    }
}
