//! The todo-ID check: a comment in `src/` says why the code is the way it is, and a todo's ID
//! says only when it was written (maintainability.md: names and comments). The history belongs
//! in commit messages and plans, where `git log` and the board keep it.
//!
//! The IDs are read from the todo board's file names (`docs/todos/<state>/<ID>-<title>.md`), so a
//! new todo is banned as soon as it exists. Only IDs of docs-prose's shape count
//! ([`is_todo_id`]: a letter suffix, as in `B5b` and `Q2t`): the plain ones (`A1`, `H2`, `P1`,
//! `X1`) are also paper sizes, heading levels and the Unicode bidi algorithm's rule names, which
//! code may name. A suffixed ID that is also a name code needs (UAX #9's rule `X5a`, should a
//! todo `X5a` ever exist) goes in `id_exceptions` in `tools/xtask/docs-prose.toml`, which
//! docs-prose skips too.

use crate::docs_prose::is_todo_id;
use crate::lines;

use super::{Problem, Tree, problem};

const RULE: &str = "todo-id";
/// The folder whose file names give the todo IDs.
const TODOS: &str = "docs/todos/";
/// The top-level folders whose packages' `src/` the check reads.
const ROOTS: [&str; 2] = ["crates/", "packages/"];
/// The source files the check reads.
const EXTENSIONS: [&str; 9] = ["rs", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

/// The todo ID that file `path` of the board is named for: `B5b` for
/// `docs/todos/done/B5b-builder-text.md`; `None` for any other file, or a plain ID.
pub(super) fn todo_id(path: &str) -> Option<&str> {
    let name = path.strip_prefix(TODOS)?.rsplit('/').next()?.strip_suffix(".md")?;
    let (id, _title) = name.split_once('-')?;
    is_todo_id(id).then_some(id)
}

/// Whether the check reads `path`: a source file in the `src/` of a crate or package, outside
/// generated code.
pub(super) fn is_in_scope(path: &str) -> bool {
    let in_src = ROOTS.iter().any(|root| {
        path.strip_prefix(root)
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(_, inside)| inside.starts_with("src/"))
    });
    in_src && !path.contains("/generated/") && crate::util::has_extension(path, &EXTENSIONS)
}

/// Every todo ID, as a whole word, in the files [`is_in_scope`] accepts, but for
/// `id_exceptions`.
pub(super) fn todo_ids(tree: &Tree) -> Vec<Problem> {
    let mut problems = Vec::new();
    for (path, text) in &tree.src_texts {
        let mut starts = None;
        for (offset, word) in words(text) {
            if tree.todo_ids.contains(word) && !tree.id_exceptions.iter().any(|exception| exception == word) {
                let line = lines::line_of(starts.get_or_insert_with(|| lines::starts(text)), offset);
                let message = format!(
                    "`{word}` is a todo ID: say why the code is so in words; the todo's history belongs in commit messages and plans"
                );
                problems.push(problem(path, line, RULE, message));
            }
        }
    }
    problems
}

/// The words of `text` with their byte offsets: runs of ASCII letters, digits and `_`, so
/// `(B5b)` and `B5b's` hold the word `B5b`, and `review_b5b` and `0xB5b` don't.
fn words(text: &str) -> Vec<(usize, &str)> {
    let mut words = Vec::new();
    let mut start = None;
    // A trailing separator ends the last word.
    for (at, c) in text.char_indices().chain([(text.len(), ' ')]) {
        let in_word = c.is_ascii_alphanumeric() || c == '_';
        match (start, in_word) {
            (None, true) => start = Some(at),
            (Some(from), false) => {
                words.extend(text.get(from..at).map(|word| (from, word)));
                start = None;
            }
            _ => {}
        }
    }
    words
}

#[cfg(test)]
mod tests {
    use super::super::Tree;
    use super::*;

    fn found(files: &[(&str, &str)]) -> Vec<(String, usize)> {
        todo_ids(&Tree::from_files(files))
            .into_iter()
            .map(|problem| (problem.path, problem.finding.line))
            .collect()
    }

    const BOARD: [(&str, &str); 3] = [
        ("docs/todos/done/B5b-builder-text.md", ""),
        ("docs/todos/ongoing/Q2t-history-out-of-names.md", ""),
        ("docs/todos/done/P1-release.md", ""),
    ];

    #[test]
    fn todo_ids_should_come_from_suffixed_board_file_names() {
        let ids = [
            "docs/todos/done/B5b-builder-text.md",
            "docs/todos/ready/E10j-forms.md",
            "docs/todos/done/P1-release.md",
            "docs/todos/README.md",
            "docs/plans/B5b-notes.md",
            "docs/todos/done/b5b-lower.md",
            "docs/todos/done/Bb-letters.md",
        ]
        .map(todo_id);
        assert_eq!(ids, [Some("B5b"), Some("E10j"), None, None, None, None, None]);
    }

    #[test]
    fn an_id_exception_should_pass() {
        let mut files = BOARD.to_vec();
        files.push(("crates/a/src/bidi.rs", "// UAX #9 rule B5b.\n"));
        let mut tree = Tree::from_files(&files);
        assert_eq!(todo_ids(&tree).len(), 1);
        tree.id_exceptions = vec!["B5b".to_owned()];
        assert!(todo_ids(&tree).is_empty());
    }

    #[test]
    fn a_todo_id_in_a_src_comment_should_fail_on_its_line() {
        let mut files = BOARD.to_vec();
        files.push(("crates/a/src/lib.rs", "//! Fonts.\n// Shaped (B5b), see Q2t's list.\n"));
        files.push(("packages/sdk/src/init.ts", "/** Loads B5b's module. */\n"));
        assert_eq!(
            found(&files),
            [
                ("crates/a/src/lib.rs".to_owned(), 2),
                ("crates/a/src/lib.rs".to_owned(), 2),
                ("packages/sdk/src/init.ts".to_owned(), 1),
            ]
        );
    }

    #[test]
    fn plain_ids_parts_of_words_and_files_out_of_scope_should_pass() {
        let mut files = BOARD.to_vec();
        let text = "// UAX #9 rule P1; review_b5b, 0xB5b and B5bx are not IDs.\n";
        files.extend([
            ("crates/a/src/lib.rs", text),
            ("crates/a/tests/text.rs", "// B5b\n"),
            ("crates/a/src/generated/codes.rs", "// B5b\n"),
            ("packages/sdk/test/init.test.ts", "// B5b\n"),
            ("tools/xtask/src/main.rs", "// B5b\n"),
        ]);
        assert_eq!(found(&files), []);
    }

    #[test]
    fn the_scope_should_be_the_src_of_crates_and_packages() {
        let scoped = [
            "crates/a/src/lib.rs",
            "packages/sdk/src/worker/client.ts",
            "packages/sdk/src/worker/entry.mts",
            "crates/a/src/generated/codes.rs",
            "crates/a/tests/text.rs",
            "packages/sdk/src/README.md",
            "apps/docs/src/index.ts",
        ]
        .map(is_in_scope);
        assert_eq!(scoped, [true, true, true, false, false, false, false]);
    }

    #[test]
    fn words_should_split_on_anything_but_letters_digits_and_underscores() {
        assert_eq!(
            words("(B5b) B5b's review_b5b\n0xB5b"),
            [(1, "B5b"), (6, "B5b"), (10, "s"), (12, "review_b5b"), (23, "0xB5b")]
        );
    }
}
