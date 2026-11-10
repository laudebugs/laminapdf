//! ADR 0010 rule 1, capabilities not platforms: no package may import `@tauri-apps/*`, call
//! `isTauri()`, read Tauri's globals or read the user agent. UI code branches on the `Platform`'s
//! capability flags; only an app's entry may ask which shell it runs in. Run by
//! `cargo xtask check ts` over every source file in `packages/*/src`, whole-line comments left out,
//! and over `apps/desktop/src`, where only the entry (`main.tsx`) may call `isTauri` (ADR 0008).

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::workspace::Workspace;

/// What a package must not use, and why each is refused.
const FORBIDDEN: &[(&str, &str)] = &[
    ("@tauri-apps/", "imports Tauri"),
    ("isTauri", "asks whether it runs in Tauri"),
    ("__TAURI", "reads Tauri's globals"),
    ("userAgent", "reads the user agent"),
];

const EXTENSIONS: &[&str] = &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"];

pub(crate) fn check(ws: &Workspace) -> Result<()> {
    crate::step("platform boundary: packages branch on capabilities, never on the platform (ADR 0010)");
    let mut found = Vec::new();
    for source in sources(&ws.root.join("packages"))? {
        let text = std::fs::read_to_string(&source).with_context(|| format!("reading {}", source.display()))?;
        let name = source.strip_prefix(&ws.root).unwrap_or(&source).display().to_string();
        found.extend(
            violations(&text)
                .into_iter()
                .map(|(line, why)| format!("  {name}:{line}: {why}")),
        );
    }
    found.extend(desktop_entry_only(ws)?);
    if !found.is_empty() {
        bail!(
            "packages must branch on Platform capabilities, not on the platform (ADR 0010 rule 1):\n{}",
            found.join("\n")
        );
    }
    Ok(())
}

/// The desktop app asks whether it runs in Tauri once, in its entry: every other module takes the
/// Platform it is given (ADR 0008, ADR 0010).
fn desktop_entry_only(ws: &Workspace) -> Result<Vec<String>> {
    let src = ws.root.join("apps/desktop/src");
    let mut found = Vec::new();
    if !src.is_dir() {
        return Ok(found);
    }
    let mut files = Vec::new();
    walk(&src, &mut files)?;
    files.sort();
    for source in files.iter().filter(|f| f.file_name().is_none_or(|n| n != "main.tsx")) {
        let text = std::fs::read_to_string(source).with_context(|| format!("reading {}", source.display()))?;
        let name = source.strip_prefix(&ws.root).unwrap_or(source).display().to_string();
        found.extend(
            code_lines(&text)
                .filter(|(_, line)| contains_word_start(line, "isTauri"))
                .map(|(index, _)| {
                    format!(
                        "  {name}:{}: asks whether it runs in Tauri outside the desktop entry (main.tsx)",
                        index + 1
                    )
                }),
        );
    }
    Ok(found)
}

/// Every script file under each `packages/<name>/src`, generated code included.
fn sources(packages: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for package in std::fs::read_dir(packages).context("listing packages/")? {
        let src = package?.path().join("src");
        if src.is_dir() {
            walk(&src, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn walk(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in std::fs::read_dir(dir).with_context(|| format!("listing {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name != "node_modules") {
                walk(&path, files)?;
            }
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EXTENSIONS.contains(&ext))
        {
            files.push(path);
        }
    }
    Ok(())
}

/// The 1-based line and reason of each forbidden use in `source`, outside whole-line comments.
fn violations(source: &str) -> Vec<(usize, &'static str)> {
    let mut found = Vec::new();
    for (index, line) in code_lines(source) {
        for (needle, why) in FORBIDDEN {
            if contains_word_start(line, needle) {
                found.push((index + 1, *why));
            }
        }
    }
    found
}

/// `needle` in `line` where it doesn't continue an identifier: `isTauri` but not `notIsTauri`.
fn contains_word_start(line: &str, needle: &str) -> bool {
    line.match_indices(needle).any(|(at, _)| {
        !line[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '$')
    })
}

/// The 0-based index and text of each line that isn't a whole-line comment: `//` lines and
/// `/* … */` blocks that start a line are left out. Telling a comment from a string, a regex or JSX
/// text anywhere else would take a parser, so a comment after code counts as code: write the user
/// agent, not its property name, there.
fn code_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut in_block = false;
    source.lines().enumerate().filter(move |(_, line)| {
        let text = line.trim_start();
        if in_block {
            in_block = !line.contains("*/");
            return false;
        }
        if let Some(rest) = text.strip_prefix("/*") {
            in_block = !rest.contains("*/");
            return false;
        }
        !text.starts_with("//")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn violations_should_find_each_way_of_asking_for_the_platform() {
        let source = "import { invoke } from \"@tauri-apps/api/core\";\n\
                      if (isTauri()) {}\n\
                      const ua = navigator.userAgent;\n\
                      const inside = \"__TAURI_INTERNALS__\" in window;\n\
                      const brands = navigator.userAgentData?.brands;\n";
        assert_eq!(
            violations(source),
            vec![
                (1, "imports Tauri"),
                (2, "asks whether it runs in Tauri"),
                (3, "reads the user agent"),
                (4, "reads Tauri's globals"),
                (5, "reads the user agent"),
            ]
        );
    }

    #[test]
    fn violations_should_skip_whole_line_comments_and_keep_line_numbers() {
        let source = "// never read navigator.userAgent\n\
                      /* isTauri() is for\n   the shell's entry */\n\
                      /** One line of TSDoc: isTauri */\n\
                      const a = isTauri;\n";
        assert_eq!(violations(source), vec![(5, "asks whether it runs in Tauri")]);
    }

    #[test]
    fn violations_should_not_be_misled_by_quotes_in_jsx_text_or_slashes_in_regexes() {
        let source = "const p = <p>Don't</p>;\n\
                      const re = /\\/*x/;\n\
                      const ua = navigator.userAgent;\n";
        assert_eq!(violations(source), vec![(3, "reads the user agent")]);
    }

    #[test]
    fn violations_should_count_a_comment_after_code() {
        assert_eq!(
            violations("run(); // isTauri\n"),
            vec![(1, "asks whether it runs in Tauri")]
        );
    }

    #[test]
    fn violations_should_ignore_names_that_only_end_with_a_forbidden_one() {
        assert_eq!(violations("const notIsTauri = 1; const myuserAgent = 2;\n"), vec![]);
    }
}
