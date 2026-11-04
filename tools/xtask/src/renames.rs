//! `docs/renames.json` (`laminapdf.renames/1`, L5a): what old names became, shipped in the package
//! docs so an agent holding an old name finds the new one. `cargo xtask check docs` validates it
//! against its schema, requires an entry for every deprecated Rust or TypeScript item, and checks
//! that every new name still appears in the sources.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::codegen;
use crate::tracked::Tracked;

const RENAMES: &str = "docs/renames.json";
const SCHEMA: &str = "docs/renames.schema.json";

/// Runs the check over the tracked sources under `root`.
pub(crate) fn check(root: &Path) -> Result<()> {
    let read = |path: &str| -> Result<Value> {
        let text = std::fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"))?;
        serde_json::from_str(&text).with_context(|| format!("{path} is not JSON"))
    };
    let renames = read(RENAMES)?;
    codegen::validate(&read(SCHEMA)?, &renames).with_context(|| format!("{RENAMES} does not match {SCHEMA}"))?;
    let tracked = Tracked::from_git(root)?;
    let mut sources = Vec::new();
    for path in tracked.files().filter(|p| is_source(p)) {
        let text = std::fs::read_to_string(root.join(path)).with_context(|| format!("reading {}", path.display()))?;
        sources.push((path.to_path_buf(), text));
    }
    let problems = problems(&renames, &sources);
    if !problems.is_empty() {
        bail!("{RENAMES}:\n  {}", problems.join("\n  "));
    }
    Ok(())
}

/// Shipped Rust and TypeScript sources: `crates/*/src/**.rs` and `packages/*/src/**.ts`.
fn is_source(path: &Path) -> bool {
    let parts: Vec<_> = path.iter().filter_map(|p| p.to_str()).collect();
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or_default();
    match parts.as_slice() {
        ["crates", _, "src", ..] => extension == "rs",
        ["packages", _, "src", .., file] => extension == "ts" && !file.to_ascii_lowercase().ends_with(".d.ts"),
        _ => false,
    }
}

/// Every problem with `renames` against `sources` (path, text), one line each.
fn problems(renames: &Value, sources: &[(PathBuf, String)]) -> Vec<String> {
    let entries = renames
        .get("renames")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let field = |entry: &Value, key: &str| entry.get(key).and_then(Value::as_str).unwrap_or_default().to_owned();
    let mut problems = Vec::new();
    for (path, text) in sources {
        for name in deprecated_items(path, text) {
            let covered = entries.iter().any(|e| last_name(&field(e, "from")) == name);
            if !covered {
                problems.push(format!(
                    "{} deprecates `{name}`, but no entry's `from` names it",
                    path.display()
                ));
            }
        }
    }
    for entry in &entries {
        let to = field(entry, "to");
        let name = last_name(&to);
        let found = sources.iter().any(|(_, text)| contains_word(text, name));
        if !found {
            problems.push(format!("`{to}`: `{name}` appears in no shipped source"));
        }
    }
    problems
}

/// The last identifier of a path such as `laminapdf::Error::error_code` or `Document.open`.
fn last_name(path: &str) -> &str {
    path.rsplit(|c: char| !(c.is_alphanumeric() || c == '_'))
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

/// Whether `text` holds `word` with no identifier character on either side.
fn contains_word(text: &str, word: &str) -> bool {
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    !word.is_empty()
        && text.match_indices(word).any(|(at, _)| {
            let before = text[..at].chars().next_back();
            let after = text[at + word.len()..].chars().next();
            !before.is_some_and(ident) && !after.is_some_and(ident)
        })
}

/// The names of the items marked deprecated in one file: the item after each `#[deprecated`
/// (Rust) or after the comment holding `@deprecated` (TypeScript).
fn deprecated_items(path: &Path, text: &str) -> Vec<String> {
    let rust = path.extension().is_some_and(|e| e == "rs");
    let marker = if rust { "#[deprecated" } else { "@deprecated" };
    let lines: Vec<&str> = text.lines().collect();
    let mut names = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let deprecates = if rust {
            line.trim_start().starts_with(marker)
        } else {
            line.contains(marker)
        };
        if !deprecates {
            continue;
        }
        let name = lines.iter().skip(i + 1).find_map(|l| item_name(l, rust));
        if let Some(name) = name {
            names.push(name);
        }
    }
    names
}

/// The name an item line declares, skipping attributes, doc comments and comment lines.
fn item_name(line: &str, rust: bool) -> Option<String> {
    let line = line.trim();
    if line.is_empty()
        || line.starts_with("#[")
        || line.starts_with("//")
        || line.starts_with('*')
        || line.starts_with("/*")
    {
        return None;
    }
    let keywords: &[&str] = if rust {
        &[
            "fn", "struct", "enum", "trait", "type", "const", "static", "mod", "union",
        ]
    } else {
        &["function", "class", "interface", "type", "const", "let", "enum"]
    };
    let words: Vec<&str> = line
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|w| !w.is_empty())
        .collect();
    let after_keyword = words
        .windows(2)
        .find(|w| w.first().is_some_and(|k| keywords.contains(k)))
        .and_then(|w| w.get(1));
    if rust {
        // A multi-line `#[deprecated(…)]`'s arguments name no item; only a declaration counts.
        return after_keyword.map(|w| (*w).to_owned());
    }
    // A TypeScript method or getter (`get pages()`, `save(…)`): its first name.
    let fallback = || {
        words
            .iter()
            .find(|w| !["get", "set", "static", "readonly", "async", "export"].contains(w))
    };
    after_keyword.or_else(fallback).map(|w| (*w).to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{deprecated_items, last_name, problems};

    fn source(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    fn renames(from: &str, to: &str) -> serde_json::Value {
        json!({ "schema": "laminapdf.renames/1", "renames": [{ "from": from, "to": to }] })
    }

    #[test]
    fn last_names_are_the_final_identifier() {
        assert_eq!(last_name("laminapdf::Error::error_code"), "error_code");
        assert_eq!(last_name("Document.open"), "open");
        assert_eq!(last_name("init({ text })"), "text");
    }

    #[test]
    fn deprecated_rust_and_typescript_items_are_found() {
        let rust = "#[deprecated(note = \"use then\")]\n#[must_use]\npub fn multiply(self) {}\n";
        assert_eq!(deprecated_items(&PathBuf::from("a.rs"), rust), ["multiply"]);
        let ts = "/**\n * Old.\n * @deprecated Use `pages`.\n */\nget pageList(): number {\n";
        assert_eq!(deprecated_items(&PathBuf::from("a.ts"), ts), ["pageList"]);
        let ts = "/** @deprecated */\nexport function oldInit() {}\n";
        assert_eq!(deprecated_items(&PathBuf::from("a.ts"), ts), ["oldInit"]);
    }

    #[test]
    fn a_deprecation_without_an_entry_is_a_problem() {
        let sources = [source(
            "crates/x/src/lib.rs",
            "#[deprecated]\npub fn old() {}\npub fn new() {}\n",
        )];
        assert_eq!(problems(&renames("x::old", "x::new"), &sources), Vec::<String>::new());
        let found = problems(&renames("x::other", "x::new"), &sources);
        assert!(found.len() == 1 && found[0].contains("`old`"), "{found:?}");
    }

    #[test]
    fn a_target_missing_from_the_sources_is_a_problem() {
        let sources = [source("crates/x/src/lib.rs", "pub fn renewed() {}\n")];
        let found = problems(&renames("x::old", "x::new"), &sources);
        assert!(found.len() == 1 && found[0].contains("`new`"), "{found:?}");
    }
}
