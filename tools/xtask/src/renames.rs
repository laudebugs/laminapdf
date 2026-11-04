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
        let rust = field(entry, "surface") != "ts";
        let names = qualified_names(&to);
        let missing: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| {
                !sources
                    .iter()
                    .any(|(path, text)| is_rust(path) == rust && defines(text, name, rust))
            })
            .collect();
        if !missing.is_empty() {
            problems.push(format!("`{to}`: no shipped source defines `{}`", missing.join("`, `")));
        }
    }
    problems
}

fn is_rust(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "rs")
}

/// The last identifier of a path such as `laminapdf::Error::error_code` or `Document.open`.
fn last_name(path: &str) -> &str {
    path.rsplit(|c: char| !is_ident(c))
        .find(|s| !s.is_empty())
        .unwrap_or(path)
}

/// The names a new path needs defined: its last identifier, and the type it sits on when the
/// segment before it is one (`Matrix` in `laminapdf_model::Matrix::then`).
fn qualified_names(path: &str) -> Vec<&str> {
    let segments: Vec<&str> = path.split(|c: char| !is_ident(c)).filter(|s| !s.is_empty()).collect();
    let mut names: Vec<&str> = segments.last().copied().into_iter().collect();
    if let [.., owner, _] = segments.as_slice()
        && owner.starts_with(|c: char| c.is_ascii_uppercase())
    {
        names.push(owner);
    }
    names
}

const fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

const RUST_ITEMS: [&str; 10] = [
    "fn", "struct", "enum", "trait", "type", "const", "static", "mod", "union", "use",
];
const TS_ITEMS: [&str; 7] = ["function", "class", "interface", "type", "const", "let", "enum"];
const MODIFIERS: [&str; 13] = [
    "pub", "crate", "super", "self", "in", "async", "unsafe", "extern", "default", "export", "static", "readonly",
    "declare",
];

/// Whether `text` declares `name`: an item (`fn name`, `struct name`, a TypeScript `class name`),
/// a re-export under that name (`use x as name`), or a TypeScript method or getter.
fn defines(text: &str, name: &str, rust: bool) -> bool {
    let items: &[&str] = if rust { &RUST_ITEMS } else { &TS_ITEMS };
    text.lines().any(|line| {
        let words = words(line);
        let declared = words
            .windows(2)
            .any(|w| matches!(w, [k, n] if (items.contains(k) || *k == "as") && *n == name));
        declared || (!rust && method_name(&words) == Some(name))
    })
}

fn words(line: &str) -> Vec<&str> {
    line.split(|c: char| !is_ident(c)).filter(|w| !w.is_empty()).collect()
}

/// A TypeScript class member's name (`get pages(): …`, `save(options) {`): its first word after
/// the modifiers, when the line declares one.
fn method_name<'a>(words: &[&'a str]) -> Option<&'a str> {
    words
        .iter()
        .copied()
        .find(|w| !MODIFIERS.contains(w) && *w != "get" && *w != "set")
}

/// The names of the items marked deprecated in one file: the item after each `#[deprecated…]`
/// attribute (Rust) or after the doc comment holding `@deprecated` (TypeScript).
fn deprecated_items(path: &Path, text: &str) -> Vec<String> {
    let rust = is_rust(path);
    let marker = if rust { "#[deprecated" } else { "@deprecated" };
    text.match_indices(marker)
        .filter_map(|(at, _)| {
            let from_marker = text.get(at..)?;
            let after = if rust {
                skip_attribute(from_marker)
            } else {
                from_marker.split_once("*/").map(|(_, r)| r)?
            };
            item_name(after, rust)
        })
        .collect()
}

/// The text after the attribute `#[…]` that `text` starts with, brackets balanced.
fn skip_attribute(text: &str) -> &str {
    let mut depth = 0usize;
    for (i, c) in text.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth == 1 => return text.get(i + 1..).unwrap_or_default(),
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    ""
}

/// The name the next declaration in `text` gives, skipping attributes and comments: an item's
/// name after its keyword (`pub const fn old` → `old`), a re-export's alias or last segment, or
/// for an enum variant or field (Rust) and a class member (TypeScript) its first word.
fn item_name(mut text: &str, rust: bool) -> Option<String> {
    loop {
        text = text.trim_start();
        if text.starts_with("#[") {
            text = skip_attribute(text);
        } else if text.starts_with("//") {
            text = text.split_once('\n').map_or("", |(_, r)| r);
        } else if text.starts_with("/*") {
            text = text.split_once("*/").map_or("", |(_, r)| r);
        } else {
            break;
        }
    }
    let line = text.lines().next()?;
    let words = words(line);
    let items: &[&str] = if rust { &RUST_ITEMS } else { &TS_ITEMS };
    let mut remaining = words
        .iter()
        .copied()
        .skip_while(|w| MODIFIERS.contains(w) || (*w == "const" && rust_fn_follows(line)));
    let first = remaining.next()?;
    if first == "use" {
        let path: Vec<&str> = remaining.collect();
        let alias = path
            .windows(2)
            .find(|w| w.first() == Some(&"as"))
            .and_then(|w| w.get(1));
        return alias.or(path.last()).map(|w| (*w).to_owned());
    }
    if items.contains(&first) {
        return remaining.next().map(str::to_owned);
    }
    if rust {
        Some(first.to_owned())
    } else {
        method_name(&words).map(str::to_owned)
    }
}

/// Whether a Rust declaration line is a function (`const fn`, `const unsafe fn`), so its `const`
/// is a modifier, not the item.
fn rust_fn_follows(line: &str) -> bool {
    words(line).contains(&"fn")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::{deprecated_items, last_name, problems, qualified_names};

    fn source(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_owned())
    }

    fn renames(from: &str, to: &str) -> serde_json::Value {
        json!({ "schema": "laminapdf.renames/1", "renames": [{ "from": from, "to": to, "surface": "rust" }] })
    }

    fn rust_items(text: &str) -> Vec<String> {
        deprecated_items(&PathBuf::from("a.rs"), text)
    }

    #[test]
    fn last_names_are_the_final_identifier() {
        assert_eq!(last_name("laminapdf::Error::error_code"), "error_code");
        assert_eq!(last_name("Document.open"), "open");
        assert_eq!(qualified_names("laminapdf_model::Matrix::then"), ["then", "Matrix"]);
        assert_eq!(
            qualified_names("laminapdf_text::glyph_name_to_unicode"),
            ["glyph_name_to_unicode"]
        );
    }

    #[test]
    fn deprecated_rust_items_are_named_by_their_declaration() {
        assert_eq!(
            rust_items("#[deprecated(note = \"use then\")]\n#[must_use]\npub fn multiply(self) {}\n"),
            ["multiply"]
        );
        assert_eq!(
            rust_items("#[deprecated(\n    note = \"x\",\n)]\npub const fn old() {}\n"),
            ["old"]
        );
        assert_eq!(rust_items("#[deprecated] pub fn old() {}\npub fn next() {}\n"), ["old"]);
        assert_eq!(
            rust_items("#[deprecated]\npub use crate::new::Thing as Old;\n"),
            ["Old"]
        );
        assert_eq!(rust_items("#[deprecated]\npub use crate::new::Thing;\n"), ["Thing"]);
        assert_eq!(
            rust_items("#[deprecated]\n/// Docs.\n    OldVariant,\n"),
            ["OldVariant"]
        );
        assert_eq!(
            rust_items("#[deprecated]\npub(crate) const LIMIT: u32 = 1;\n"),
            ["LIMIT"]
        );
    }

    #[test]
    fn deprecated_typescript_items_are_named_by_their_declaration() {
        let ts = |text: &str| deprecated_items(&PathBuf::from("a.ts"), text);
        assert_eq!(
            ts("/**\n * Old.\n * @deprecated Use `pages`.\n */\nget pageList(): number {\n"),
            ["pageList"]
        );
        assert_eq!(ts("/** @deprecated */\nexport function oldInit() {}\n"), ["oldInit"]);
        assert_eq!(ts("/** @deprecated */ export const oldName = 1;\n"), ["oldName"]);
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
    fn a_new_name_must_be_defined_not_only_mentioned() {
        let sources = [source(
            "crates/x/src/lib.rs",
            "pub fn renewed() { let new = 1; }\nimpl Matrix {}\n",
        )];
        let found = problems(&renames("x::old", "x::new"), &sources);
        assert!(found.len() == 1 && found[0].contains("`new`"), "{found:?}");
        let sources = [source(
            "crates/x/src/lib.rs",
            "pub struct Matrix;\nimpl Matrix { pub fn then(self) {} }\n",
        )];
        assert_eq!(
            problems(&renames("x::Matrix::multiply", "x::Matrix::then"), &sources),
            Vec::<String>::new()
        );
        let sources = [source("crates/x/src/lib.rs", "pub use skrifa::Tag as OpenTypeTag;\n")];
        assert_eq!(
            problems(&renames("x::Tag", "x::OpenTypeTag"), &sources),
            Vec::<String>::new()
        );
    }

    #[test]
    fn a_missing_owner_type_is_a_problem() {
        let sources = [source(
            "crates/x/src/lib.rs",
            "pub struct Other;\nimpl Other { pub fn then(self) {} }\n",
        )];
        let found = problems(&renames("x::Matrix::multiply", "x::Matrix::then"), &sources);
        assert!(found.len() == 1 && found[0].contains("`Matrix`"), "{found:?}");
    }
}
