//! The size budgets: at most [`MODULE_BUDGET`] lines of non-test code per module, and a count of
//! `#[expect]` (or `#[allow]`) on clippy's size lints that may not grow. Today's modules over the
//! budget and today's count are recorded in [`BASELINE`], which only shrinks: the check fails when
//! a recorded module grows, and also when it shrinks or drops under the budget until its entry is
//! lowered or removed, so the improvement can't be undone unnoticed. Against the merge base with
//! `main`, no entry may be added or raised.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::{BASELINE, Base, Problem, Tree, problem, warning};

/// Lines of non-test code a module may hold (maintainability guideline, "Size").
pub(super) const MODULE_BUDGET: usize = 800;

/// Clippy's lints on the size of a function or its signature: silencing one is how code grows
/// past a budget, so their `#[expect]`s are counted.
const SIZE_LINTS: [&str; 4] = [
    "too_many_lines",
    "cognitive_complexity",
    "too_many_arguments",
    "excessive_nesting",
];

/// The contents of [`BASELINE`].
#[derive(Debug, Default, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(super) struct Baseline {
    /// `#[expect]` and `#[allow]` attributes naming a size lint, across every file the check reads.
    #[serde(default)]
    size_expects: usize,
    /// Path → lines of non-test code, for each module over the budget.
    #[serde(default)]
    modules: BTreeMap<String, usize>,
}

/// Whether the module budget applies to `path`: library and binary sources (`src/` of a crate
/// or tool, `src-tauri/src/` of an app), not tests, benches, examples or generated files.
fn in_budget(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    let source = matches!(
        parts.as_slice(),
        ["crates" | "tools", _, "src", ..] | ["apps", _, "src-tauri", "src", ..]
    );
    source && !is_generated(path)
}

/// Whether `path` is generated (`generated/` or `*_generated.rs`, code-organization §9): exempt
/// from both budgets.
fn is_generated(path: &str) -> bool {
    path.split('/').any(|part| part == "generated") || path.ends_with("_generated.rs")
}

/// Each module's lines of non-test code against [`MODULE_BUDGET`] and the baseline.
pub(super) fn module_budget(tree: &Tree) -> Vec<Problem> {
    const RULE: &str = "module-budget";
    let baseline = &tree.baseline;
    let test_files = tree.out_of_line_test_files();
    let mut problems = Vec::new();
    let mut counted = BTreeMap::new();
    for (path, source) in &tree.sources {
        if !in_budget(path) || test_files.contains(path) {
            continue;
        }
        let lines = source.non_test_lines();
        counted.insert(path.as_str(), lines);
        let message = match baseline.modules.get(path) {
            None if lines > MODULE_BUDGET => format!(
                "{lines} lines of non-test code, over the {MODULE_BUDGET}-line budget: split the module \
                 (tests in #[cfg(test)] items, comments and blank lines don't count)"
            ),
            Some(&recorded) if lines > recorded => format!(
                "{lines} lines of non-test code, {} more than its baseline of {recorded}: a module over the \
                 {MODULE_BUDGET}-line budget may not grow; move code out of it",
                lines - recorded
            ),
            Some(_) if lines <= MODULE_BUDGET => format!(
                "{lines} lines of non-test code, within the {MODULE_BUDGET}-line budget: remove its entry from {BASELINE}"
            ),
            Some(&recorded) if lines < recorded => format!(
                "{lines} lines of non-test code, down from its baseline of {recorded}: lower its entry in \
                 {BASELINE} to {lines} so the baseline only shrinks"
            ),
            _ => continue,
        };
        problems.push(problem(path, 0, RULE, message));
    }
    for path in baseline.modules.keys() {
        if !counted.contains_key(path.as_str()) {
            problems.push(problem(
                BASELINE,
                0,
                RULE,
                format!("{path} is in the baseline but is no longer a module the budget reads: remove its entry"),
            ));
        }
    }
    problems
}

/// The number of size lints that `#[expect]`, `#[allow]` (or a `cfg_attr` holding one) attributes
/// name in each file the check reads (tests and examples included, generated files not), against
/// the baseline's `size-expects`.
pub(super) fn size_expects(tree: &Tree) -> Vec<Problem> {
    const RULE: &str = "size-expect";
    let mut per_file = BTreeMap::new();
    for (path, source) in tree.sources.iter().filter(|(path, _)| !is_generated(path)) {
        let count: usize = source
            .attributes()
            .iter()
            .map(|attribute| size_lints_silenced(attribute.content))
            .sum();
        if count > 0 {
            per_file.insert(path.as_str(), count);
        }
    }
    let total: usize = per_file.values().sum();
    let recorded = tree.baseline.size_expects;
    let files: Vec<String> = per_file
        .iter()
        .map(|(path, count)| format!("{path} ({count})"))
        .collect();
    let message = match total.cmp(&recorded) {
        std::cmp::Ordering::Greater => format!(
            "{total} #[expect]/#[allow] on size lints, more than the baseline's {recorded}: split the function \
             instead of silencing {}; they are in {}",
            SIZE_LINTS.join(", "),
            files.join(", ")
        ),
        std::cmp::Ordering::Less => {
            format!("{total} #[expect]/#[allow] on size lints, down from {recorded}: set size-expects to {total}")
        }
        std::cmp::Ordering::Equal => return Vec::new(),
    };
    vec![problem(BASELINE, 0, RULE, message)]
}

/// The baseline against its version at the merge base with `main`: no module joins it, no
/// count goes up. (A module that grows past its count fails [`module_budget`] already; this
/// catches the edit that raises the count with it.)
pub(super) fn baseline_shrinks(tree: &Tree) -> Vec<Problem> {
    const RULE: &str = "baseline-shrinks";
    let base = match &tree.base {
        Base::Records {
            baseline: Some(base), ..
        } => base,
        // A new baseline file: nothing to compare with yet.
        Base::Records { baseline: None, .. } => return Vec::new(),
        Base::Unknown => {
            let message = "no merge base with main (a shallow clone needs fetch-depth: 0), so the record files \
                           weren't compared with main's";
            return vec![warning(BASELINE, RULE, message)];
        }
    };
    let mut problems = Vec::new();
    for (path, &count) in &tree.baseline.modules {
        let message = match base.modules.get(path) {
            None => format!("{path} joins the baseline at {count}: a module may not join it; split the module instead"),
            Some(&was) if count > was => format!(
                "{path}'s entry goes up from {was} to {count}: the baseline only shrinks; move code out instead"
            ),
            Some(_) => continue,
        };
        problems.push(problem(BASELINE, 0, RULE, message));
    }
    if tree.baseline.size_expects > base.size_expects {
        problems.push(problem(
            BASELINE,
            0,
            RULE,
            format!(
                "size-expects goes up from {} to {}: the baseline only shrinks; split the function instead",
                base.size_expects, tree.baseline.size_expects
            ),
        ));
    }
    problems
}

/// How many size lints an attribute's content silences.
fn size_lints_silenced(content: &str) -> usize {
    let compact: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    if !["expect(", "allow(", "cfg_attr("]
        .iter()
        .any(|start| compact.starts_with(start))
    {
        return 0;
    }
    SIZE_LINTS
        .iter()
        .map(|lint| {
            let name = format!("clippy::{lint}");
            compact
                .match_indices(&name)
                .filter(|(at, _)| {
                    let next = compact.as_bytes().get(at + name.len());
                    !next.is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
                })
                .count()
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    /// A module of `lines` lines of code, with a test module and comments that don't count.
    fn module(lines: usize) -> String {
        let mut text = String::from("//! A module.\n\n");
        for index in 0..lines {
            writeln!(text, "const C{index}: u32 = {index};").unwrap();
        }
        text.push_str("\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n");
        text
    }

    fn baseline(entries: &str) -> String {
        format!("[modules]\n{entries}\n")
    }

    fn budget_messages(files: &[(&str, &str)]) -> Vec<String> {
        let tree = Tree::from_files(files);
        module_budget(&tree)
            .into_iter()
            .map(|p| format!("{}: {}", p.path, p.finding.message))
            .collect()
    }

    #[test]
    fn a_module_at_the_budget_should_pass() {
        assert_eq!(
            budget_messages(&[("crates/a/src/lib.rs", &module(800))]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_new_module_over_the_budget_should_fail() {
        let messages = budget_messages(&[("crates/a/src/big.rs", &module(801))]);
        assert!(messages.len() == 1 && messages[0].contains("801 lines of non-test code, over the 800-line budget"));
    }

    #[test]
    fn a_baselined_module_should_pass_at_its_count() {
        let baseline = baseline("\"crates/a/src/big.rs\" = 900");
        assert_eq!(
            budget_messages(&[("crates/a/src/big.rs", &module(900)), (BASELINE, &baseline)]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn one_more_line_in_a_baselined_module_should_fail() {
        let baseline = baseline("\"crates/a/src/big.rs\" = 900");
        let messages = budget_messages(&[("crates/a/src/big.rs", &module(901)), (BASELINE, &baseline)]);
        assert!(messages.len() == 1 && messages[0].contains("1 more than its baseline of 900"));
    }

    #[test]
    fn a_shrunk_module_should_ask_for_a_lower_baseline() {
        let baseline = baseline("\"crates/a/src/big.rs\" = 900");
        let messages = budget_messages(&[("crates/a/src/big.rs", &module(850)), (BASELINE, &baseline)]);
        assert!(messages.len() == 1 && messages[0].contains("lower its entry"));
    }

    #[test]
    fn a_module_back_under_the_budget_should_leave_the_baseline() {
        let baseline = baseline("\"crates/a/src/big.rs\" = 900");
        let messages = budget_messages(&[("crates/a/src/big.rs", &module(700)), (BASELINE, &baseline)]);
        assert!(messages.len() == 1 && messages[0].contains("remove its entry"));
    }

    #[test]
    fn a_removed_module_should_leave_the_baseline() {
        let baseline = baseline("\"crates/a/src/gone.rs\" = 900");
        let messages = budget_messages(&[(BASELINE, &baseline)]);
        assert!(messages.len() == 1 && messages[0].starts_with(BASELINE));
    }

    #[test]
    fn tests_examples_and_generated_files_should_have_no_budget() {
        let big = module(900);
        let files = [
            "crates/a/tests/big.rs",
            "crates/a/examples/big.rs",
            "crates/a/benches/big.rs",
            "crates/a/src/generated/codes.rs",
            "crates/a/src/std14_generated.rs",
        ]
        .map(|path| (path, big.as_str()));
        assert_eq!(budget_messages(&files), Vec::<String>::new());
    }

    #[test]
    fn an_out_of_line_test_module_should_have_no_budget() {
        let big = module(900);
        let parent = "#[cfg(test)]\nmod tests;\n";
        let files = [
            ("crates/a/src/crypt/mod.rs", parent),
            ("crates/a/src/crypt/tests.rs", big.as_str()),
        ];
        assert_eq!(budget_messages(&files), Vec::<String>::new());
    }

    #[test]
    fn tools_and_apps_should_have_the_budget() {
        let big = module(801);
        let messages = budget_messages(&[
            ("tools/xtask/src/big.rs", &big),
            ("apps/desktop/src-tauri/src/big.rs", &big),
        ]);
        assert_eq!(messages.len(), 2);
    }

    #[test]
    fn a_test_module_named_by_a_path_attribute_should_have_no_budget() {
        let big = module(900);
        let parent = "#[cfg(test)]\n#[path = \"checks.rs\"]\nmod tests;\n";
        let files = [
            ("crates/a/src/lib.rs", parent),
            ("crates/a/src/checks.rs", big.as_str()),
        ];
        assert_eq!(budget_messages(&files), Vec::<String>::new());
    }

    fn shrink_messages(baseline: &str, base: &str) -> Vec<String> {
        let tree = Tree::from_files(&[(BASELINE, baseline)]).with_base(base, "");
        baseline_shrinks(&tree).into_iter().map(|p| p.finding.message).collect()
    }

    #[test]
    fn a_raised_baseline_entry_should_fail_against_the_merge_base() {
        let messages = shrink_messages("[modules]\n\"a.rs\" = 901\n", "[modules]\n\"a.rs\" = 900\n");
        assert!(messages.len() == 1 && messages[0].contains("goes up from 900 to 901"));
    }

    #[test]
    fn a_new_baseline_entry_should_fail_against_the_merge_base() {
        let messages = shrink_messages("[modules]\n\"a.rs\" = 900\n", "");
        assert!(messages.len() == 1 && messages[0].contains("joins the baseline"));
    }

    #[test]
    fn a_raised_size_expect_count_should_fail_against_the_merge_base() {
        assert_eq!(shrink_messages("size-expects = 2\n", "size-expects = 1\n").len(), 1);
    }

    #[test]
    fn a_lowered_baseline_should_pass_against_the_merge_base() {
        let messages = shrink_messages(
            "size-expects = 1\n[modules]\n\"a.rs\" = 850\n",
            "size-expects = 2\n[modules]\n\"a.rs\" = 900\n\"b.rs\" = 900\n",
        );
        assert_eq!(messages, Vec::<String>::new());
    }

    fn expect_messages(files: &[(&str, &str)]) -> Vec<String> {
        size_expects(&Tree::from_files(files))
            .into_iter()
            .map(|p| p.finding.message)
            .collect()
    }

    const ONE_EXPECT: &str = "#[expect(clippy::too_many_lines, reason = \"a table\")]\nfn f() {}\n";

    #[test]
    fn size_expects_at_the_recorded_count_should_pass() {
        assert_eq!(
            expect_messages(&[("crates/a/src/lib.rs", ONE_EXPECT), (BASELINE, "size-expects = 1\n")]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_new_size_expect_should_fail() {
        let messages = expect_messages(&[("crates/a/src/lib.rs", ONE_EXPECT)]);
        assert!(messages.len() == 1 && messages[0].contains("crates/a/src/lib.rs (1)"));
    }

    #[test]
    fn a_removed_size_expect_should_lower_the_count() {
        let messages = expect_messages(&[("crates/a/src/lib.rs", "fn f() {}\n"), (BASELINE, "size-expects = 1\n")]);
        assert!(messages.len() == 1 && messages[0].contains("set size-expects to 0"));
    }

    #[test]
    fn size_lints_should_be_counted_in_every_silencing_attribute() {
        let content = [
            "expect(clippy::too_many_lines, clippy::too_many_arguments, reason = \"x\")",
            "allow(clippy::cognitive_complexity)",
            "cfg_attr(test, expect(clippy::excessive_nesting))",
            "expect(clippy::too_many_lines_x)",
            "doc = \"clippy::too_many_lines\"",
        ]
        .map(size_lints_silenced);
        assert_eq!(content, [2, 1, 1, 0, 0]);
    }

    #[test]
    fn a_size_expect_in_a_generated_file_should_not_count() {
        assert_eq!(
            expect_messages(&[("crates/a/src/generated/codes.rs", ONE_EXPECT)]),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_size_lint_in_a_comment_or_string_should_not_count() {
        let text = "// #[expect(clippy::too_many_lines)]\nconst S: &str = \"#[expect(clippy::too_many_lines)]\";\n";
        assert_eq!(expect_messages(&[("crates/a/src/lib.rs", text)]), Vec::<String>::new());
    }
}
