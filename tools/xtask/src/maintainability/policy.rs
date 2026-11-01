//! The error-policy check: the policies in `laminapdf_cos::error` (`tolerant`,
//! `absent_if_malformed`, `Error::must_propagate`, `Error::ends_page_walk`, …) decide which
//! errors a read swallows. A test for `LimitExceeded` or `DataPending` anywhere else in a crate's
//! library code starts a second policy, as text extraction's private one did when it left out
//! source errors.
//!
//! Two shapes count as such a test: a `matches!` that names one of the variants, and an
//! or-pattern that lists one beside other errors, written `Err(A | B)` or `e @ (A | B)`. A match
//! arm on one variant alone (`Err(Error::DataPending { ranges }) => …`, which collects the
//! ranges) handles that error rather than choosing which errors propagate.

use super::source::Token;
use super::{Problem, Tree, crate_of, problem};

/// The one file whose code may test for the variants.
const HOME: &str = "crates/laminapdf-cos/src/error.rs";
/// The variants only a policy may test for.
const VARIANTS: [&str; 2] = ["LimitExceeded", "DataPending"];

/// Every `matches!` and every error or-pattern outside [`HOME`] and outside test code that
/// names one of [`VARIANTS`].
pub(super) fn error_policy_tests(tree: &Tree) -> Vec<Problem> {
    let test_files = tree.out_of_line_test_files();
    let mut problems = Vec::new();
    for (path, source) in &tree.sources {
        if path == HOME || crate_of(path).is_none() || test_files.contains(path) {
            continue;
        }
        let tokens = source.tokens();
        for (at, token) in tokens.iter().enumerate() {
            let Some(shape) = shape_at(&tokens, at) else { continue };
            if source.is_test(token.offset()) {
                continue;
            }
            let group = tokens.get(at + shape.group_offset()..).unwrap_or_default();
            if let Some(variant) = named_variant(group, shape) {
                problems.push(problem(
                    path,
                    source.line_of(token.offset()),
                    "error-policy",
                    format!(
                        "{} on `{variant}`: decide which errors propagate with a policy in \
                         `laminapdf_cos::error` (`tolerant`, `absent_if_malformed`, \
                         `Error::must_propagate`, `Error::ends_page_walk`), or add one there",
                        shape.describe()
                    ),
                ));
            }
        }
    }
    problems
}

/// What opens at a token: `matches!(`, or the `(` of `Err(…)` or `name @ (…)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Matches,
    OrPattern,
}

impl Shape {
    /// How many tokens after the opening one its `(` comes.
    const fn group_offset(self) -> usize {
        match self {
            Self::Matches => 2,
            Self::OrPattern => 1,
        }
    }

    /// The shape as a finding names it.
    const fn describe(self) -> &'static str {
        match self {
            Self::Matches => "`matches!`",
            Self::OrPattern => "an or-pattern of errors",
        }
    }
}

/// The shape that opens at `tokens[at]`, if any.
fn shape_at(tokens: &[Token<'_>], at: usize) -> Option<Shape> {
    let is_punct = |offset: usize, byte: u8| tokens.get(at + offset).is_some_and(|t| t.is_punct(byte));
    let token = tokens.get(at)?;
    if token.is("matches") && is_punct(1, b'!') && is_punct(2, b'(') {
        Some(Shape::Matches)
    } else if (token.is("Err") || token.is_punct(b'@')) && is_punct(1, b'(') {
        Some(Shape::OrPattern)
    } else {
        None
    }
}

/// The first of [`VARIANTS`] inside the parentheses that `tokens` opens with. For an
/// or-pattern, only when a `|` at the group's own level lists it beside other errors.
fn named_variant(tokens: &[Token<'_>], shape: Shape) -> Option<&'static str> {
    let mut depth = 0_usize;
    let mut variant = None;
    let mut alternatives = false;
    for token in tokens {
        if token.is_punct(b'(') {
            depth += 1;
        } else if token.is_punct(b')') {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                break;
            }
        } else if token.is_punct(b'|') && depth == 1 {
            alternatives = true;
        } else if variant.is_none() {
            variant = VARIANTS.into_iter().find(|v| token.is(v));
        }
    }
    variant.filter(|_| shape == Shape::Matches || alternatives)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(path: &str, text: &str) -> Vec<String> {
        let tree = Tree::from_files(&[(path, text)]);
        error_policy_tests(&tree)
            .into_iter()
            .map(|p| format!("{}:{}", p.path, p.finding.line))
            .collect()
    }

    #[test]
    fn a_matches_on_limit_exceeded_should_be_reported() {
        let text =
            "fn f(e: &Error) -> bool {\n    matches!(\n        e,\n        Error::LimitExceeded { .. }\n    )\n}\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:2"]);
    }

    #[test]
    fn a_matches_on_data_pending_should_be_reported() {
        let text = "fn f(e: &Error) -> bool { matches!(e, Error::Strict { .. } | Error::DataPending { .. }) }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:1"]);
    }

    #[test]
    fn a_bound_or_pattern_should_be_reported() {
        let text = "fn f(r: R) -> R {\n    match r {\n        Err(e @ (Error::DataPending { .. } | Error::Source(_))) => \
                    Err(e),\n        _ => r,\n    }\n}\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:3"]);
    }

    #[test]
    fn an_or_pattern_in_err_should_be_reported() {
        let text = "fn f(r: R) -> bool { match r { Err(Error::LimitExceeded { .. } | Error::Strict { .. }) => true, \
                    _ => false } }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:1"]);
    }

    #[test]
    fn an_arm_on_one_variant_should_be_allowed() {
        let text = "fn f(r: R) { match r { Err(Error::DataPending { ranges }) => keep(ranges), _ => {} } }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
    }

    #[test]
    fn the_policies_home_should_be_allowed() {
        let text = "fn f(e: &Error) -> bool { matches!(e, Error::LimitExceeded { .. }) }\n";
        assert_eq!(found(HOME, text), Vec::<String>::new());
    }

    #[test]
    fn a_variant_after_the_matches_should_not_count() {
        let text = "fn f(e: &Error) -> bool { matches!(e, Error::Strict { .. }) }\n\
                    fn g() -> Error { Error::DataPending { ranges: vec![] } }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
    }

    #[test]
    fn test_code_tests_and_other_folders_should_be_skipped() {
        let check = "matches!(r, Err(Error::LimitExceeded { .. }))";
        let in_test = format!("#[cfg(test)]\nmod tests {{\n    fn t() {{ assert!({check}); }}\n}}\n");
        let elsewhere = format!("fn t() {{ assert!({check}); }}\n");
        let results = [
            found("crates/a/src/lib.rs", &in_test),
            found("crates/a/tests/limits.rs", &elsewhere),
            found("tools/xtask/src/main.rs", &elsewhere),
        ];
        assert_eq!(results, [Vec::<String>::new(), Vec::new(), Vec::new()]);
    }

    #[test]
    fn comments_and_strings_should_not_count() {
        let text = "// matches!(e, Error::LimitExceeded { .. })\nconst S: &str = \"matches!(e, DataPending)\";\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
    }
}
