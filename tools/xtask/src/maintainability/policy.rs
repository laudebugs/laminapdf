//! The error-policy check (todo Q2e): which errors a read swallows is decided by the policies in
//! `laminapdf_cos::error` (`tolerant`, `absent_if_malformed`, `Error::must_propagate`,
//! `Error::ends_page_walk`, …), so a `matches!` on `LimitExceeded` or `DataPending` anywhere else
//! in a crate's library code is a second policy in the making.

use super::source::Token;
use super::{Problem, Tree, names, problem};

/// The one file whose `matches!` may name the variants.
const HOME: &str = "crates/laminapdf-cos/src/error.rs";
/// The variants only a policy may test for.
const VARIANTS: [&str; 2] = ["LimitExceeded", "DataPending"];

/// Every `matches!` outside [`HOME`] and outside test code that names one of [`VARIANTS`].
pub(super) fn error_policy_matches(tree: &Tree) -> Vec<Problem> {
    let test_files = tree.out_of_line_test_files();
    let mut problems = Vec::new();
    for (path, source) in &tree.sources {
        if path == HOME || names::crate_of(path).is_none() || test_files.contains(path) {
            continue;
        }
        let tokens = source.tokens();
        for (at, token) in tokens.iter().enumerate() {
            let opens = token.is("matches")
                && tokens.get(at + 1).is_some_and(|t| t.is_punct(b'!'))
                && tokens.get(at + 2).is_some_and(|t| t.is_punct(b'('));
            if !opens || source.is_test(token.offset()) {
                continue;
            }
            if let Some(variant) = named_variant(tokens.get(at + 2..).unwrap_or_default()) {
                problems.push(problem(
                    path,
                    source.line_of(token.offset()),
                    "error-policy",
                    format!(
                        "`matches!` on `{variant}`: decide which errors propagate with a policy in \
                         `laminapdf_cos::error` (`tolerant`, `absent_if_malformed`, \
                         `Error::must_propagate`, `Error::ends_page_walk`), or add one there"
                    ),
                ));
            }
        }
    }
    problems
}

/// The first of [`VARIANTS`] inside the parentheses that `tokens` opens with.
fn named_variant(tokens: &[Token<'_>]) -> Option<&'static str> {
    let mut depth = 0_usize;
    for token in tokens {
        if token.is_punct(b'(') {
            depth += 1;
        } else if token.is_punct(b')') {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return None;
            }
        } else if let Some(variant) = VARIANTS.into_iter().find(|v| token.is(v)) {
            return Some(variant);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(path: &str, text: &str) -> Vec<String> {
        let tree = Tree::from_files(&[(path, text)]);
        error_policy_matches(&tree)
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
    fn the_policies_home_should_be_allowed() {
        let text = "fn f(e: &Error) -> bool { matches!(e, Error::LimitExceeded { .. }) }\n";
        assert_eq!(found(HOME, text), Vec::<String>::new());
    }

    #[test]
    fn a_variant_after_the_matches_should_not_count() {
        let text = "fn f(e: &Error) -> bool { matches!(e, Error::Strict { .. }) }\nfn g() -> Error { Error::DataPending { ranges: vec![] } }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
    }

    #[test]
    fn test_code_tests_and_other_folders_should_be_skipped() {
        let check = "matches!(r, Err(Error::LimitExceeded { .. }))";
        let in_test = format!("#[cfg(test)]\nmod tests {{\n    fn t() {{ assert!({check}); }}\n}}\n");
        let elsewhere = format!("fn t() {{ assert!({check}); }}\n");
        let found = [
            found("crates/a/src/lib.rs", &in_test),
            found("crates/a/tests/limits.rs", &elsewhere),
            found("tools/xtask/src/main.rs", &elsewhere),
        ];
        assert_eq!(found, [Vec::<String>::new(), Vec::new(), Vec::new()]);
    }

    #[test]
    fn comments_and_strings_should_not_count() {
        let text = "// matches!(e, Error::LimitExceeded { .. })\nconst S: &str = \"matches!(e, DataPending)\";\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
    }
}
