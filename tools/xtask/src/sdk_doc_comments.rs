//! `check docs`' rule for `@laminapdf/sdk`'s doc comments: `TypeDoc` turns every `/** … */` comment
//! in `packages/sdk/src` into the public API reference on the docs site, which people and coding
//! agents read, so none of them may name an ADR or a todo (the project's internal record, which a
//! reader of the reference can't open). Plain `//` comments never reach the reference and may cite
//! both. `src/generated/` is checked too, since its comments reach the reference as well: they are
//! fixed where `cargo xtask gen` takes them from (its templates, `codes.json`, the JSON schemas and
//! the Rust doc comments those come from).

use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::docs_prose::{is_adr_number, is_todo_id};
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

/// The ADR numbers and todo IDs in `source`'s `/** … */` comments, each with its 1-based line.
fn problems(source: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    let mut rest = source;
    let mut offset = 0;
    while let Some(start) = rest.find("/**") {
        let body = rest.get(start..).unwrap_or_default();
        let len = body.find("*/").map_or(body.len(), |end| end + 2);
        let comment = body.get(..len).unwrap_or_default();
        let first_line = source.get(..offset + start).unwrap_or_default().matches('\n').count() + 1;
        for (index, line) in comment.lines().enumerate() {
            found.extend(internal_words(line).into_iter().map(|word| (first_line + index, word)));
        }
        offset += start + len;
        rest = body.get(len..).unwrap_or_default();
    }
    found
}

/// The ADR numbers (`ADR 0026`, `ADR-0026`) and todo IDs (`B5a`) in one line of a comment.
fn internal_words(line: &str) -> Vec<String> {
    let words: Vec<&str> = line
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .filter(|w| !w.is_empty())
        .collect();
    let mut found = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let next = words.get(index + 1).copied();
        if is_adr_number(word, next) {
            let number = word.strip_prefix("ADR-").or(next).unwrap_or_default();
            found.push(format!("ADR {number}"));
        } else if is_todo_id(word) {
            found.push(format!("the todo ID {word}"));
        }
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
}
