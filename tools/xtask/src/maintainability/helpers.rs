//! The test-helpers check: the Rust test helpers several crates need live once, in
//! `laminapdf-test-fixtures` (code-organization.md §6). Two shapes outside it count as a copy:
//! a `fn fixtures_dir` or `fn classic`, and a `#[path]` that includes a file of another crate.
//! The files that still have one are listed under `test-helpers` in [`BASELINE`]; the list only
//! shrinks, and an entry whose file no longer has a copy must go.

use std::collections::BTreeSet;

use super::{BASELINE, Base, Problem, Tree, problem};

/// The crate that owns the shared helpers.
const HOME: &str = "crates/laminapdf-test-fixtures/";
/// Helper names that only [`HOME`] may define.
const HELPERS: [&str; 2] = ["fixtures_dir", "classic"];
const RULE: &str = "test-helpers";

/// Every copy outside [`HOME`] in a file the baseline doesn't list, and every baseline entry
/// that is stale or new.
pub(super) fn test_helper_copies(tree: &Tree) -> Vec<Problem> {
    let listed = tree.baseline.test_helpers.clone().unwrap_or_default();
    let mut problems = Vec::new();
    let mut with_copies = BTreeSet::new();
    for (path, source) in &tree.sources {
        if !path.starts_with("crates/") || path.starts_with(HOME) {
            continue;
        }
        let copies = copies(path, source);
        if copies.is_empty() {
            continue;
        }
        with_copies.insert(path.as_str());
        if !listed.contains(path) {
            problems.extend(
                copies
                    .into_iter()
                    .map(|(line, message)| problem(path, line, RULE, message)),
            );
        }
    }
    for path in &listed {
        if !with_copies.contains(path.as_str()) {
            let message = format!("{path} no longer copies a test helper: remove it from `test-helpers`");
            problems.push(problem(BASELINE, 0, RULE, message));
        }
    }
    // A base from before the list existed has nothing to compare with.
    if let Base::Records {
        baseline: Some(base), ..
    } = &tree.base
        && let Some(before) = &base.test_helpers
    {
        for path in listed.difference(before) {
            let message =
                format!("{path} joins `test-helpers`: the list only shrinks; use laminapdf-test-fixtures instead");
            problems.push(problem(BASELINE, 0, RULE, message));
        }
    }
    problems
}

/// The copies in `path`: `(line, message)` for each helper definition and cross-crate `#[path]`.
fn copies(path: &str, source: &super::Source) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let tokens = source.tokens();
    for pair in tokens.windows(2) {
        let [keyword, name] = pair else { continue };
        if let Some(helper) = HELPERS.iter().find(|h| keyword.is("fn") && name.is(h)) {
            let message = format!("a copy of `{helper}`: use `laminapdf_test_fixtures::{helper}`");
            found.push((source.line_of(keyword.offset()), message));
        }
    }
    let krate = crate_dir(path);
    for (offset, value) in &source.paths {
        let target = resolve(path, value);
        if crate_dir(&target) != krate {
            let message =
                format!("`#[path]` includes {target}, another crate's file: share it through laminapdf-test-fixtures");
            found.push((source.line_of(*offset), message));
        }
    }
    found
}

/// `crates/<name>` for a path inside a crate, else `None`.
fn crate_dir(path: &str) -> Option<&str> {
    let rest = path.strip_prefix("crates/")?;
    let name = rest.split('/').next()?;
    path.get(..("crates/".len() + name.len()))
}

/// `value`, relative to the folder of the file `from`, as a normalised repository path.
fn resolve(from: &str, value: &str) -> String {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for part in value.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

#[cfg(test)]
mod tests {
    use super::super::{BASELINE, Tree};
    use super::{resolve, test_helper_copies};

    fn messages(files: &[(&str, &str)]) -> Vec<String> {
        test_helper_copies(&Tree::from_files(files))
            .into_iter()
            .map(|p| p.finding.message)
            .collect()
    }

    #[test]
    fn resolve_should_follow_parent_folders() {
        assert_eq!(
            resolve("crates/a/tests/common/mod.rs", "../../../b/tests/x.rs"),
            "crates/b/tests/x.rs"
        );
    }

    #[test]
    fn a_copied_helper_should_fail() {
        let found = messages(&[("crates/a/tests/common/mod.rs", "pub fn fixtures_dir() {}\n")]);
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn the_home_crate_may_define_the_helpers() {
        let found = messages(&[("crates/laminapdf-test-fixtures/src/lib.rs", "pub fn classic() {}\n")]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_path_into_another_crate_should_fail() {
        let text = "#[path = \"../../../b/tests/common/x.rs\"]\nmod x;\n";
        let found = messages(&[("crates/a/tests/common/mod.rs", text)]);
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn a_path_inside_the_crate_should_pass() {
        let text = "#[path = \"../tests/common/mod.rs\"]\nmod common;\n";
        assert_eq!(messages(&[("crates/a/examples/golden.rs", text)]), Vec::<String>::new());
    }

    #[test]
    fn a_similar_name_should_pass() {
        let text = "fn classic_table() {}\nfn classic_file_round_trips() {}\n";
        assert_eq!(messages(&[("crates/a/src/lib.rs", text)]), Vec::<String>::new());
    }

    #[test]
    fn a_listed_file_should_pass_and_a_stale_entry_should_fail() {
        let baseline = "test-helpers = [\"crates/a/tests/common/mod.rs\", \"crates/b/tests/common/mod.rs\"]\n";
        let found = messages(&[
            (BASELINE, baseline),
            ("crates/a/tests/common/mod.rs", "pub fn classic() {}\n"),
            ("crates/b/tests/common/mod.rs", "pub fn open() {}\n"),
        ]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("crates/b/tests/common/mod.rs"), "{found:?}");
    }

    #[test]
    fn a_base_without_the_list_should_accept_todays_entries() {
        let baseline = "test-helpers = [\"crates/a/tests/common/mod.rs\"]\n";
        let tree = Tree::from_files(&[
            (BASELINE, baseline),
            ("crates/a/tests/common/mod.rs", "fn classic() {}\n"),
        ])
        .with_base("", "");
        assert!(test_helper_copies(&tree).is_empty());
    }

    #[test]
    fn a_new_entry_should_fail() {
        let baseline = "test-helpers = [\"crates/a/tests/common/mod.rs\"]\n";
        let tree = Tree::from_files(&[
            (BASELINE, baseline),
            ("crates/a/tests/common/mod.rs", "fn classic() {}\n"),
        ])
        .with_base("test-helpers = []\n", "");
        let found = test_helper_copies(&tree);
        assert_eq!(found.len(), 1, "{found:?}");
    }
}
