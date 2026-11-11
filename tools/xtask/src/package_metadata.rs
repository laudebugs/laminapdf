//! `check docs`' package-metadata rule: npm and crates.io show a package's description in their
//! search results and the first paragraph of its README at the top of its page, so both must
//! speak to the people choosing a package. Every published package needs a description, and
//! neither it nor the README's opening may name an ADR or a todo (the project's internal record,
//! which a reader of the registry can't use) or call the package private. The rest of a README is
//! reference for contributors and may cite both.
//!
//! The published packages are the crates `cargo publish --workspace` uploads, the npm fixed group
//! (`.changeset/config.json`) and the CLI's npm wrapper, `@laminapdf/cli`, which its own release
//! publishes.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::docs_prose::{is_adr_number, is_todo_id};
use crate::workspace::Workspace;

/// The CLI's npm wrapper: private in the workspace, published by `cli-release.yml`.
const CLI_PACKAGE: &str = "packages/cli";

/// Fails naming each published package whose description or README opening breaks the rule.
pub(crate) fn check(ws: &Workspace) -> Result<()> {
    let mut found = Vec::new();
    for (name, dir, description) in ws.publishable_crate_details() {
        found.extend(problems(name, description, &read(&dir.join("README.md"))?));
    }
    let mut npm: Vec<PathBuf> = crate::version::fixed_group_version(&ws.root)?
        .members
        .into_iter()
        .map(|(_, dir)| dir)
        .collect();
    npm.push(ws.root.join(CLI_PACKAGE));
    for dir in npm {
        let package = crate::version::read_json(&dir.join("package.json"))?;
        let name = package.get("name").and_then(Value::as_str).unwrap_or_default();
        let description = package.get("description").and_then(Value::as_str).unwrap_or_default();
        found.extend(problems(name, description, &read(&dir.join("README.md"))?));
    }
    if !found.is_empty() {
        bail!(
            "published packages describe themselves badly:\n  {}\n\
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

/// What is wrong with `package`'s description and the first paragraph of its README.
fn problems(package: &str, description: &str, readme: &str) -> Vec<String> {
    let mut found = Vec::new();
    if description.trim().is_empty() {
        found.push(format!("{package}: it has no description"));
    }
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

/// The README's first paragraph: the first run of non-blank prose lines, after the headings,
/// HTML comments and blocks, images and badges, and fenced code that may come before it, with link
/// destinations left out (a link's path may name a todo file that its text doesn't).
fn first_paragraph(readme: &str) -> String {
    let mut paragraph = Vec::new();
    let (mut in_comment, mut in_fence) = (false, false);
    for line in readme.lines() {
        let trimmed = line.trim();
        if in_comment {
            in_comment = !trimmed.contains("-->");
            continue;
        }
        if in_fence {
            in_fence = !trimmed.starts_with("```");
            continue;
        }
        if paragraph.is_empty() {
            if trimmed.starts_with("<!--") {
                in_comment = !trimmed.contains("-->");
                continue;
            }
            if trimmed.starts_with("```") {
                in_fence = true;
                continue;
            }
            let not_prose = ["#", "<", "![", "[!["].iter().any(|start| trimmed.starts_with(start));
            if trimmed.is_empty() || not_prose {
                continue;
            }
        } else if trimmed.is_empty() {
            break;
        }
        paragraph.push(trimmed);
    }
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

/// The ADR numbers and todo IDs in `text`, and any claim that the package is private ("is
/// private", "private package"); other uses of the word, such as files that stay private, pass.
fn internal_words(text: &str) -> Vec<String> {
    let words: Vec<&str> = text
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .flat_map(|word| {
            if word.starts_with("ADR-") {
                vec![word]
            } else {
                word.split('-').collect()
            }
        })
        .filter(|w| !w.is_empty())
        .collect();
    let mut found = Vec::new();
    for (index, word) in words.iter().enumerate() {
        let next = words.get(index + 1).copied();
        let previous = index.checked_sub(1).and_then(|i| words.get(i)).copied();
        if is_adr_number(word, next) {
            found.push(format!(
                "ADR {}",
                word.strip_prefix("ADR-").or(next).unwrap_or_default()
            ));
        } else if is_todo_id(word) {
            found.push(format!("the todo ID {word}"));
        } else if word.eq_ignore_ascii_case("private")
            && (matches!(previous, Some("is" | "It" | "it")) || next.is_some_and(|n| n.eq_ignore_ascii_case("package")))
        {
            found.push("\"private\" for the package".to_owned());
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
            ["@laminapdf/wasm: its README's first paragraph names \"private\" for the package"]
        );
    }

    #[test]
    fn problems_should_report_a_missing_description() {
        let found = problems("@laminapdf/viewer", " ", "# viewer\n\nShows PDFs.\n");
        assert_eq!(found, ["@laminapdf/viewer: it has no description"]);
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
    fn first_paragraph_should_skip_multi_line_comments_html_and_code_before_the_prose() {
        let readme = "# x\n\n<!--\nGenerated: B5a\n-->\n<p align=\"center\"><img src=\"logo.svg\"></p>\n![logo](logo.png)\n```sh\nnpm i x\n```\n\nThe prose.\nSecond line.\n\nLater.\n";
        assert_eq!(first_paragraph(readme), "The prose. Second line.");
    }

    #[test]
    fn internal_words_should_report_a_joined_adr() {
        assert_eq!(internal_words("See ADR-0004."), ["ADR 0004"]);
    }

    #[test]
    fn internal_words_should_leave_paper_sizes_years_and_privacy_alone() {
        let text = "An A4 page, a 2026 ADR-free note, version R5; your files stay private.";
        assert_eq!(internal_words(text), Vec::<String>::new());
    }
}
