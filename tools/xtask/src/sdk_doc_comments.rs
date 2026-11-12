//! `check docs`' rule for `@laminapdf/sdk`'s doc comments: `TypeDoc` turns every `/** … */` comment
//! in `packages/sdk/src` into the public API reference on the docs site, which people and coding
//! agents read, so none of them may name an ADR or a todo (the project's internal record, which a
//! reader of the reference can't open). Plain `//` comments never reach the reference and may cite
//! both. `src/generated/` is checked too, since its comments reach the reference as well: they are
//! fixed where `cargo xtask gen` takes them from (its templates, `codes.json`, the JSON schemas and
//! the Rust doc comments those come from).

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::docs_prose::{id_words, internal_reference};
use crate::tracked::git;

/// The sources `TypeDoc` documents, as a git pathspec (`*` matches across directories).
const SOURCES: &str = "packages/sdk/src/*.ts";

/// Fails naming each doc comment in [`SOURCES`] that cites an ADR or a todo.
pub(crate) fn check(root: &Path) -> Result<()> {
    let listing = git(root, &["ls-files", "-z", "--", SOURCES])?;
    let mut found = Vec::new();
    for path in listing.split('\0').filter(|p| !p.is_empty()) {
        let text = std::fs::read_to_string(root.join(path)).with_context(|| format!("failed to read {path}"))?;
        found.extend(
            problems(&text)
                .into_iter()
                .map(|(line, word)| format!("{path}:{line}: {word}")),
        );
    }
    if !found.is_empty() {
        bail!(
            "@laminapdf/sdk's doc comments cite the project's internal record:\n  {}\n\
             say the fact in plain words: these comments are the public API reference",
            found.join("\n  ")
        );
    }
    println!("sdk doc comments: no ADR number or todo ID in the API reference's sources");
    Ok(())
}

/// The ADR numbers and todo IDs in `source`'s `/** … */` comments, each with its 1-based line. A
/// reference wrapped over two lines (`(ADR` then `0016 §7)`) counts.
fn problems(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (first_line, comment) in doc_comments(source) {
        let words: Vec<(usize, &str)> = comment
            .lines()
            .enumerate()
            .flat_map(|(index, line)| id_words(line).into_iter().map(move |word| (first_line + index, word)))
            .collect();
        for (index, (line, word)) in words.iter().enumerate() {
            let next = words.get(index + 1).map(|(_, next)| *next);
            found.extend(internal_reference(word, next).map(|reference| (*line, reference)));
        }
    }
    found
}

/// Where a scan of a TypeScript file is.
#[derive(Clone, Copy)]
enum State {
    Code,
    LineComment,
    /// A `/* … */` comment from byte `start` on line `line`; `doc` for `/** … */`.
    Block {
        doc: bool,
        start: usize,
        line: usize,
    },
    /// A string or template literal opened by this quote.
    Literal {
        quote: char,
        escaped: bool,
    },
}

/// Every `/** … */` comment in `source` with the line it starts on, skipping `//` comments and
/// string and template literals, where `/**` is text (a glob such as `"src/**/*.ts"`). An unclosed
/// comment runs to the end. Good enough for the SDK's sources: regex literals and template
/// literals nested inside `${…}` aren't told apart.
fn doc_comments(source: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut chars = source.char_indices().peekable();
    let (mut state, mut line) = (State::Code, 1);
    while let Some((at, c)) = chars.next() {
        let next = chars.peek().map(|(_, next)| *next);
        state = match state {
            State::Code => match (c, next) {
                ('/', Some('/')) => {
                    chars.next();
                    State::LineComment
                }
                ('/', Some('*')) => {
                    chars.next();
                    let after = source.get(at + 2..).unwrap_or_default();
                    let doc = after.starts_with('*') && !after.starts_with("*/");
                    State::Block { doc, start: at, line }
                }
                ('"' | '\'' | '`', _) => State::Literal {
                    quote: c,
                    escaped: false,
                },
                _ => State::Code,
            },
            State::LineComment if c == '\n' => State::Code,
            State::Block {
                doc,
                start,
                line: first,
            } if c == '*' && next == Some('/') => {
                chars.next();
                if doc {
                    found.push((first, source.get(start..at + 2).unwrap_or_default()));
                }
                State::Code
            }
            State::Literal { quote, escaped: false } if c == quote || (c == '\n' && quote != '`') => State::Code,
            State::Literal { quote, escaped } => State::Literal {
                quote,
                escaped: !escaped && c == '\\',
            },
            other => other,
        };
        if c == '\n' {
            line += 1;
        }
    }
    if let State::Block { doc: true, start, line } = state {
        found.push((line, source.get(start..).unwrap_or_default()));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_should_report_an_adr_in_a_doc_comment_with_its_line() {
        let source = "export const a = 1;\n/**\n * Opens a file (ADR 0016 §7).\n */\nexport function open() {}\n";
        assert_eq!(problems(source), [(3, "ADR 0016".to_owned())]);
    }

    #[test]
    fn problems_should_report_adr_with_a_hyphen_and_a_todo_id() {
        let source = "/** Coordinates as in ADR-0027, added by B5a. */\n";
        assert_eq!(
            problems(source),
            [(1, "ADR 0027".to_owned()), (1, "the todo ID B5a".to_owned())]
        );
    }

    #[test]
    fn problems_should_leave_plain_comments_and_code_alone() {
        let source = "// ADR 0026 §1: the object (B5a).\n/* ADR 0003 */\nconst adr = \"ADR 0004\";\n";
        assert_eq!(problems(source), []);
    }

    #[test]
    fn problems_should_pass_plain_words_and_paper_sizes() {
        let source = "/** A4 pages, revision R5, and the F12 key; see docs.laminapdf.com. */\n";
        assert_eq!(problems(source), []);
    }

    #[test]
    fn problems_should_check_an_unclosed_comment_to_the_end() {
        assert_eq!(problems("/** ADRs 0001"), [(1, "ADR 0001".to_owned())]);
    }

    #[test]
    fn problems_should_report_an_adr_wrapped_over_two_lines() {
        let source = "/**\n * Opens a file, normalised as in (ADR\n * 0016 §7).\n */\n";
        assert_eq!(problems(source), [(2, "ADR 0016".to_owned())]);
    }

    #[test]
    fn problems_should_report_todo_ids_joined_by_a_hyphen() {
        let source = "/** Added in B5a-B5c. */\n";
        assert_eq!(
            problems(source),
            [(1, "the todo ID B5a".to_owned()), (1, "the todo ID B5c".to_owned())]
        );
    }

    #[test]
    fn problems_should_skip_doc_comment_openers_in_strings_and_line_comments() {
        let source = "const glob = \"src/**/*.ts\"; // see /** ADR 0001 */\nconst t = `a/**b`;\n/** Plain. */\n";
        assert_eq!(problems(source), []);
    }

    #[test]
    fn problems_should_find_a_doc_comment_after_an_escaped_quote() {
        let source = "const q = \"say \\\"hi\\\"\";\n/** B5a */\n";
        assert_eq!(problems(source), [(2, "the todo ID B5a".to_owned())]);
    }
}
