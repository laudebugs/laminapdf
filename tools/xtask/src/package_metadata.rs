//! `check docs`' package-metadata rule: npm and crates.io show a package's description in their
//! search results and the first paragraph of its README at the top of its page, so both must
//! speak to the people choosing a package. Neither may name an ADR or a todo (the project's
//! internal record, which a reader of the registry can't use), or call a published package
//! private. The rest of a README is reference for contributors and may cite both.
//!
//! The published packages are the crates `cargo publish --workspace` uploads and the npm fixed
//! group (`.changeset/config.json`).

use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::docs_prose::is_todo_id;
use crate::workspace::Workspace;

/// Fails naming each published package whose description or README opening breaks the rule.
pub(crate) fn check(ws: &Workspace) -> Result<()> {
    let mut found = Vec::new();
    for (name, dir) in ws.publishable_crate_dirs() {
        let manifest = read(&dir.join("Cargo.toml"))?;
        let manifest: toml::Table =
            toml::from_str(&manifest).with_context(|| format!("{name}'s Cargo.toml is not TOML"))?;
        let description = manifest
            .get("package")
            .and_then(|package| package.get("description"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        found.extend(problems(name, description, &read(&dir.join("README.md"))?));
    }
    for (name, dir) in crate::version::fixed_group_version(&ws.root)?.members {
        let package = crate::version::read_json(&dir.join("package.json"))?;
        let description = package.get("description").and_then(Value::as_str).unwrap_or_default();
        found.extend(problems(&name, description, &read(&dir.join("README.md"))?));
    }
    if !found.is_empty() {
        bail!(
            "published packages describe themselves in the project's internal language:\n  {}\n\
             write the description and the README's first paragraph for someone choosing a package",
            found.join("\n  ")
        );
    }
    println!("package metadata: every published package's description and README opening are clean");
    Ok(())
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}

/// What `package`'s description and the first paragraph of its README name that they shouldn't.
fn problems(package: &str, description: &str, readme: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (place, text) in [
        ("description", description),
        ("README's first paragraph", &first_paragraph(readme)),
    ] {
        for word in internal_words(text) {
            found.push(format!("{package}: its {place} names {word}"));
        }
    }
    found
}

/// The README's first paragraph: the first run of non-blank lines that isn't a heading, an HTML
/// comment or a line of badges, with link destinations left out (a link's path may name a todo
/// file that its text doesn't).
fn first_paragraph(readme: &str) -> String {
    let mut lines = readme.lines().skip_while(|line| {
        let line = line.trim();
        line.is_empty() || line.starts_with('#') || line.starts_with("<!--") || line.starts_with("[![")
    });
    let paragraph: Vec<&str> = lines.by_ref().take_while(|line| !line.trim().is_empty()).collect();
    without_link_destinations(&paragraph.join(" "))
}

/// `text` with every Markdown link's `(destination)` removed: `[ADR 0004](x.md)` → `[ADR 0004]`.
fn without_link_destinations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find("](") {
        let (before, after) = rest.split_at(at + 1);
        out.push_str(before);
        rest = after
            .find(')')
            .and_then(|close| after.get(close + 1..))
            .unwrap_or_default();
    }
    out.push_str(rest);
    out
}

/// The ADR numbers, todo IDs and the word "private" in `text`, as a reader of the error sees them.
fn internal_words(text: &str) -> Vec<String> {
    let words: Vec<&str> = text
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let mut found = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let next = words.get(index + 1).copied().unwrap_or_default();
        if matches!(*word, "ADR" | "ADRs") && next.len() == 4 && next.chars().all(|c| c.is_ascii_digit()) {
            found.push(format!("ADR {next}"));
        } else if is_todo_id(word) {
            found.push(format!("the todo ID {word}"));
        } else if word.eq_ignore_ascii_case("private") {
            found.push("\"private\"".to_owned());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_should_report_an_adr_in_the_description() {
        let found = problems(
            "@laminapdf/render",
            "The web render contract of ADR 0004.",
            "# render\n\nDraws pages.\n",
        );
        assert_eq!(found, ["@laminapdf/render: its description names ADR 0004"]);
    }

    #[test]
    fn problems_should_report_a_todo_id_in_the_readme_opening() {
        let readme = "# laminapdf-build\n\nGenerating PDFs (todo B5a): a builder.\n\nMore.\n";
        let found = problems("laminapdf-build", "Generate PDFs in Rust.", readme);
        assert_eq!(
            found,
            ["laminapdf-build: its README's first paragraph names the todo ID B5a"]
        );
    }

    #[test]
    fn problems_should_report_a_published_package_called_private() {
        let found = problems(
            "@laminapdf/wasm",
            "WebAssembly modules.",
            "# wasm\n\nThe package is private.\n",
        );
        assert_eq!(
            found,
            ["@laminapdf/wasm: its README's first paragraph names \"private\""]
        );
    }

    #[test]
    fn problems_should_pass_ids_after_the_first_paragraph_and_in_link_destinations() {
        let readme = "# save\n\n[![docs](badge.svg)](docs)\n\nSave PDFs, as [the planner](../todos/done/B5a-x.md) decides.\n\nSee ADR 0007 and B5a.\n";
        assert_eq!(
            problems("laminapdf-save", "Save edited PDFs in Rust.", readme),
            Vec::<String>::new()
        );
    }

    #[test]
    fn internal_words_should_leave_paper_sizes_and_years_alone() {
        assert_eq!(
            internal_words("An A4 page, a 2026 ADR-free note, version R5."),
            Vec::<String>::new()
        );
    }
}
