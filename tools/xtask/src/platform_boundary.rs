//! ADR 0010 rule 1, capabilities not platforms: no package may import `@tauri-apps/*`, call
//! `isTauri()`, read Tauri's globals or read the user agent. UI code branches on the `Platform`'s
//! capability flags; only an app's entry may ask which shell it runs in. Run by
//! `cargo xtask check ts` over every source file in `packages/*/src`, comments left out.

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
    if !found.is_empty() {
        bail!(
            "packages must branch on Platform capabilities, not on the platform (ADR 0010 rule 1):\n{}",
            found.join("\n")
        );
    }
    Ok(())
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

/// The 1-based line and reason of each forbidden use in `source`, outside comments.
fn violations(source: &str) -> Vec<(usize, &'static str)> {
    let code = without_comments(source);
    let mut found = Vec::new();
    for (index, line) in code.lines().enumerate() {
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

/// `source` with its comments blanked out and its line breaks kept, so line numbers still match.
/// Strings and template literals are copied as they are: a URL's `//` starts no comment.
fn without_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' {
                out.extend(chars.next());
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                while chars.peek().is_some_and(|&next| next != '\n') {
                    chars.next();
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut last = ' ';
                for next in chars.by_ref() {
                    if next == '\n' {
                        out.push('\n');
                    }
                    if last == '*' && next == '/' {
                        break;
                    }
                    last = next;
                }
            }
            ('"' | '\'' | '`', _) => {
                quote = Some(c);
                out.push(c);
            }
            _ => out.push(c),
        }
    }
    out
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
    fn violations_should_skip_comments_and_keep_line_numbers() {
        let source = "// never read navigator.userAgent\n\
                      /* isTauri() is for\n   the shell's entry */\n\
                      const url = \"https://example.com/\"; // isTauri\n\
                      const a = isTauri;\n";
        assert_eq!(violations(source), vec![(5, "asks whether it runs in Tauri")]);
    }

    #[test]
    fn violations_should_read_strings_as_code_whatever_they_hold() {
        let source = "const k = \"a//b\"; const ua = navigator[\"userAgent\"];\n\
                      const t = `/* ${isTauri()} */`;\n";
        assert_eq!(
            violations(source),
            vec![(1, "reads the user agent"), (2, "asks whether it runs in Tauri")]
        );
    }

    #[test]
    fn violations_should_ignore_names_that_only_end_with_a_forbidden_one() {
        assert_eq!(violations("const notIsTauri = 1; const myuserAgent = 2;\n"), vec![]);
    }
}
