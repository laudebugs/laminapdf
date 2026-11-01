//! The error-policy check: the policies in `laminapdf_cos::error` (`tolerant`,
//! `absent_if_malformed`, `Error::must_propagate`, `Error::ends_page_walk`, …) decide which
//! errors a read swallows. A test for `LimitExceeded` or `DataPending` anywhere else in a crate's
//! library code starts a second policy, which can drift from the first.
//!
//! Two shapes count as such a test: a `matches!` that names one of the variants, and an
//! or-pattern that lists one beside other alternatives, wherever its `|` sits:
//! `Err(A | B)`, `e @ (A | B)`, `Err(A) | Err(B)` or a bare `A | B =>`. A match arm on one
//! variant alone (`Err(Error::DataPending { ranges }) => …`, which collects the ranges) handles
//! that error rather than choosing which errors propagate.

use super::source::Token;
use super::{Problem, Tree, crate_of, problem};

/// The one file whose code may test for the variants.
const HOME: &str = "crates/laminapdf-cos/src/error.rs";
/// The variants only a policy may test for.
const VARIANTS: [&str; 2] = ["LimitExceeded", "DataPending"];
/// The guidance every finding ends with.
const ADVICE: &str = "decide which errors propagate with a policy in `laminapdf_cos::error` (`tolerant`, \
                      `absent_if_malformed`, `Error::must_propagate`, `Error::ends_page_walk`), or add one there";

/// Every `matches!` and every or-pattern outside [`HOME`] and outside test code that names one
/// of [`VARIANTS`].
pub(super) fn error_policy_tests(tree: &Tree) -> Vec<Problem> {
    let test_files = tree.out_of_line_test_files();
    let mut problems = Vec::new();
    for (path, source) in &tree.sources {
        if path == HOME || crate_of(path).is_none() || test_files.contains(path) {
            continue;
        }
        let tokens = source.tokens();
        let mut report = |at: usize, what: &str, variant: &str| {
            let offset = tokens.get(at).map_or(0, |t| t.offset());
            if !source.is_test(offset) {
                let message = format!("{what} on `{variant}`: {ADVICE}");
                problems.push(problem(path, source.line_of(offset), "error-policy", message));
            }
        };
        let mut inside_matches = 0..0;
        for (at, token) in tokens.iter().enumerate() {
            if let Some(end) = matches_call(&tokens, at) {
                if let Some(variant) = tokens.get(at..end).unwrap_or_default().iter().find_map(variant_of) {
                    report(at, "`matches!`", variant);
                }
                inside_matches = at..end;
            } else if let Some(variant) = variant_of(token)
                && !inside_matches.contains(&at)
                && in_or_pattern(&tokens, at)
            {
                report(at, "an or-pattern", variant);
            }
        }
    }
    problems
}

/// The one of [`VARIANTS`] that `token` is, if any.
fn variant_of(token: &Token<'_>) -> Option<&'static str> {
    VARIANTS.into_iter().find(|v| token.is(v))
}

/// The index just past the `)` of a `matches!(…)` that starts at `tokens[at]`.
fn matches_call(tokens: &[Token<'_>], at: usize) -> Option<usize> {
    let is_punct = |offset: usize, byte: u8| tokens.get(at + offset).is_some_and(|t| t.is_punct(byte));
    let opens = tokens.get(at).is_some_and(|t| t.is("matches")) && is_punct(1, b'!') && is_punct(2, b'(');
    opens.then(|| closing(tokens, at + 2) + 1)
}

/// The index of the bracket that closes the one at `tokens[open]` (the last index when the file
/// ends first). Parentheses, brackets and braces all count.
fn closing(tokens: &[Token<'_>], open: usize) -> usize {
    let mut depth = 0_usize;
    for (at, token) in tokens.iter().enumerate().skip(open) {
        if b"([{".iter().any(|&b| token.is_punct(b)) {
            depth += 1;
        } else if b")]}".iter().any(|&b| token.is_punct(b)) {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return at;
            }
        }
    }
    tokens.len().saturating_sub(1)
}

/// Whether the variant at `tokens[at]` is one alternative of an or-pattern over a result's
/// errors: it sits inside an `Err(…)`, and a lone `|` follows its fields and any closing `)`, or
/// one comes before its path (`Err(`, `name @ (` and `Error::` included) right after the end of
/// another alternative.
fn in_or_pattern(tokens: &[Token<'_>], at: usize) -> bool {
    let is = |index: usize, byte: u8| tokens.get(index).is_some_and(|t| t.is_punct(byte));
    let lone_bar =
        |index: usize| is(index, b'|') && !is(index + 1, b'|') && !index.checked_sub(1).is_some_and(|i| is(i, b'|'));
    if !inside_err(tokens, at) {
        return false;
    }
    let mut after = at + 1;
    if is(after, b'{') || is(after, b'(') {
        after = closing(tokens, after) + 1;
    }
    while is(after, b')') {
        after += 1;
    }
    if lone_bar(after) {
        return true;
    }
    let path = |t: &Token<'_>| matches!(t, Token::Word(..)) || t.is_punct(b':') || t.is_punct(b'(') || t.is_punct(b'@');
    let Some(before) = tokens.get(..at).and_then(|head| head.iter().rposition(|t| !path(t))) else {
        return false;
    };
    let ends_alternative = before.checked_sub(1).and_then(|i| tokens.get(i)).is_some_and(|t| {
        t.is_punct(b'}')
            || t.is_punct(b')')
            || matches!(t, Token::Word(..)) && before.checked_sub(2).is_some_and(|i| is(i, b':'))
    });
    lone_bar(before) && ends_alternative
}

/// Whether `tokens[at]` sits inside the parentheses of an `Err(…)`, directly or through
/// `name @ (…)` groups. A `match` on an error itself (`Error::A | Error::B => …`, as in
/// `kind()` or `location()`) maps each error to data rather than choosing which ones propagate.
fn inside_err(tokens: &[Token<'_>], at: usize) -> bool {
    let mut depth = 0_usize;
    for index in (0..at).rev() {
        let Some(token) = tokens.get(index) else { return false };
        if token.is_punct(b')') || token.is_punct(b'}') || token.is_punct(b']') {
            depth += 1;
        } else if token.is_punct(b'(') || token.is_punct(b'{') || token.is_punct(b'[') {
            if depth > 0 {
                depth -= 1;
                continue;
            }
            let opener = index.checked_sub(1).and_then(|i| tokens.get(i));
            if !token.is_punct(b'(') {
                return false;
            }
            if opener.is_some_and(|t| t.is("Err")) {
                return true;
            }
            if !opener.is_some_and(|t| t.is_punct(b'@')) {
                return false;
            }
        } else if depth == 0 && (token.is_punct(b';') || token.is_punct(b'=') || token.is_punct(b',')) {
            return false;
        }
    }
    false
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
    fn a_variant_after_the_bar_should_count() {
        let text = "fn f(r: R) -> R { match r { Err(e @ (Error::Strict { .. } | Error::LimitExceeded { .. })) => r, _ => r } }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:1"]);
    }

    #[test]
    fn an_or_pattern_in_err_should_be_reported() {
        let text = "fn f(r: R) -> bool { match r { Err(Error::LimitExceeded { .. } | Error::Strict { .. }) => true, \
                    _ => false } }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:1"]);
    }

    #[test]
    fn alternatives_of_err_should_be_reported() {
        let text = "fn f(r: R) -> R {\n    match r {\n        Err(Error::Source(_)) | Err(Error::DataPending { .. }) => r,\n        \
                    _ => r,\n    }\n}\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:3"]);
    }

    #[test]
    fn a_match_on_the_error_itself_should_be_allowed() {
        let text = "fn location(&self) -> L { match self {\n    Self::LimitExceeded { .. } | Self::Source(_) => L::NONE,\n} }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
    }

    #[test]
    fn a_unit_variant_before_the_bar_should_count() {
        let text = "fn f(r: R) { if let Err(Error::WrongPassword | Error::DataPending { .. }) = r {} }\n";
        assert_eq!(found("crates/a/src/lib.rs", text), ["crates/a/src/lib.rs:1"]);
    }

    #[test]
    fn closures_and_constructors_should_be_allowed() {
        let text = "fn f(v: Vec<R>) -> Result<(), Error> {\n    let e = v.iter().map(|r| Error::DataPending { ranges: r.0 });\n    \
                    let ok = a || b;\n    Err(Error::LimitExceeded { limit: l, at: x.map(|p| p | 1) })\n}\n";
        assert_eq!(found("crates/a/src/lib.rs", text), Vec::<String>::new());
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
