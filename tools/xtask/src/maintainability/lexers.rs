//! The one-lexer check: PDF syntax is tokenized once, by `laminapdf-syntax`'s `Lexer`, and PNG
//! row filters are reversed once, by its `filters::unfilter_row` (maintainability.md: one home
//! per concept). A second copy drifts: `laminapdf-text`'s CMap lexer left `#xx` escapes in
//! names undecoded until it moved onto the shared lexer. A crate that needs tokens wraps the
//! `Lexer` (as `laminapdf-content` and `laminapdf-text` do) rather than writing its own.
//!
//! Two shapes outside `laminapdf-syntax`'s library code count as a copy: a `struct` or `enum`
//! whose name ends in `Lexer` or `Tokenizer`, and a `fn paeth` or `fn unfilter…`.

use super::source::Token;
use super::{Problem, Tree, crate_of, problem};

/// The crate whose library code may hold a lexer and the row filters.
const HOME: &str = "laminapdf-syntax";
const RULE: &str = "one-lexer";
/// Type-name endings that mark a lexer.
const LEXER_SUFFIXES: [&str; 2] = ["Lexer", "Tokenizer"];

/// Every lexer type and row-filter function in library code outside [`HOME`].
pub(super) fn lexer_copies(tree: &Tree) -> Vec<Problem> {
    let test_files = tree.out_of_line_test_files();
    let mut problems = Vec::new();
    for (path, source) in &tree.sources {
        if crate_of(path).is_none_or(|krate| krate == HOME) || test_files.contains(path) {
            continue;
        }
        let tokens = source.tokens();
        for pair in tokens.windows(2) {
            let [keyword, Token::Word(name, _)] = pair else {
                continue;
            };
            let message = if (keyword.is("struct") || keyword.is("enum"))
                && LEXER_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
            {
                format!("`{name}` is a second PDF lexer: wrap `laminapdf_syntax::Lexer` instead")
            } else if keyword.is("fn") && (*name == "paeth" || name.starts_with("unfilter")) {
                format!("`fn {name}` copies the PNG row filters: use `laminapdf_syntax::filters::unfilter_row`")
            } else {
                continue;
            };
            if !source.is_test(keyword.offset()) {
                problems.push(problem(path, source.line_of(keyword.offset()), RULE, message));
            }
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::super::Tree;
    use super::lexer_copies;

    fn found(files: &[(&str, &str)]) -> usize {
        lexer_copies(&Tree::from_files(files)).len()
    }

    #[test]
    fn a_lexer_or_row_filter_outside_the_syntax_crate_should_fail() {
        let text = "struct CMapLexer;\nenum Tokenizer {}\nfn paeth() {}\nfn unfilter_row() {}\n";
        assert_eq!(found(&[("crates/laminapdf-text/src/cmap.rs", text)]), 4);
    }

    #[test]
    fn the_syntax_crate_wrappers_and_tests_should_pass() {
        let wrapper = "struct Tokens { inner: laminapdf_syntax::Lexer<'static> }\nfn lex() {}\n";
        let test = "#[cfg(test)]\nmod tests {\n    fn paeth() {}\n}\n";
        assert_eq!(
            found(&[
                (
                    "crates/laminapdf-syntax/src/lexer.rs",
                    "pub struct Lexer;\npub fn paeth() {}\n"
                ),
                ("crates/laminapdf-text/src/cmap.rs", wrapper),
                ("crates/laminapdf-build/src/png.rs", test),
                ("tools/xtask/src/source.rs", "struct Lexer;\n"),
            ]),
            0
        );
    }
}
