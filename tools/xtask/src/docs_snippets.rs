//! `cargo xtask check docs-snippets`: the rules of docs/guidelines/examples.md that a tool can check,
//! over the examples (`packages/sdk/examples/**`, `crates/laminapdf/examples/**`) and the files the
//! docs site's `Snippet` component may show (its `import.meta.glob`). Run on its own, strict on
//! changed files and counted warnings elsewhere, like `docs-prose` ([`crate::reports`]);
//! `cargo xtask check docs` runs it strict on every file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::reports::{Finding, Options, Ratchet, finding};
use crate::tracked::Tracked;

const SNIPPET: &str = "apps/docs/src/components/Snippet.astro";
const PAGES: &str = "apps/docs/src/content/docs/";
/// The TypeScript examples' folder (`@laminapdf/sdk`'s).
pub(crate) const TS_EXAMPLES: &str = "packages/sdk/examples/";
/// The Rust examples' folder (the `laminapdf` façade's).
pub(crate) const RUST_EXAMPLES: &str = "crates/laminapdf/examples/";
const EXAMPLE_DIRS: [&str; 2] = [TS_EXAMPLES, RUST_EXAMPLES];
const MAX_LINES: usize = 100;

/// Folders under [`EXAMPLE_DIRS`] that hold developer tools rather than examples for readers:
/// the text differential's `text-facts`. The examples checks skip them, as `cargo xtask examples`
/// does.
const TOOL_DIRS: [&str; 1] = ["crates/laminapdf/examples/differential/"];

/// Whether `path` is in one of the [`TOOL_DIRS`].
pub(crate) fn is_tool(path: &str) -> bool {
    TOOL_DIRS.iter().any(|dir| path.starts_with(dir))
}

/// `tools/xtask/docs-snippets.toml`: the TypeScript and Rust examples that show the same task,
/// and the examples whose subject is testing a reader's PDFs.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    pair: Vec<Pair>,
    #[serde(default, rename = "test-example")]
    test_example: Vec<TestExample>,
}

/// An example that shows a reader how to test their own PDFs, so it fixes the file ID and dates on
/// purpose: the `test-scaffolding` rule skips it.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TestExample {
    path: String,
    /// Why the file may fix them; required, so the list stays short and each entry explained.
    #[expect(dead_code, reason = "read by people, required by the format")]
    reason: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pair {
    ts: String,
    rust: String,
}

/// `cargo xtask check docs-snippets [--all-strict] [--warnings] [--json]`.
pub(crate) fn run(root: &Path, options: Options) -> Result<()> {
    let config = load_config(root)?;
    let untracked = Ratchet::untracked(root)?;
    let tracked = Tracked::from_git(root)?;
    let paths: BTreeSet<String> = tracked
        .files()
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .chain(untracked.iter().cloned())
        .filter(|path| root.join(path).is_file())
        .collect();
    let read = |path: &str| std::fs::read_to_string(root.join(path)).with_context(|| format!("reading {path}"));

    let snippet = read(SNIPPET)?;
    let globs = snippet_globs(&snippet);
    if globs.is_empty() {
        anyhow::bail!(
            "docs-snippets: found no patterns in {SNIPPET}'s import.meta.glob([...]) (one double-quoted pattern per line)"
        );
    }
    let mut used = BTreeSet::new();
    for page in paths
        .iter()
        .filter(|path| path.starts_with(PAGES) && crate::util::has_extension(path, &["mdx"]))
    {
        used.extend(snippet_uses(&read(page)?));
    }
    let unused_lists: BTreeMap<&str, String> = EXAMPLE_DIRS
        .iter()
        .filter_map(|dir| read(&format!("{dir}UNUSED.md")).ok().map(|text| (*dir, text)))
        .collect();

    let mut ratchet = Ratchet::new(root, "docs-snippets", options, &untracked);
    ratchet.add(SNIPPET, glob_findings(&globs));
    let examples = examples(&paths, &globs);
    let mut headers = BTreeMap::new();
    for path in &examples {
        let text = read(path)?;
        let test_example = config.test_example.iter().any(|t| &t.path == path);
        let mut findings = file_findings(path, &text, test_example);
        let listed = example_dir(path).is_some_and(|(dir, rest)| {
            unused_lists
                .get(dir)
                .is_some_and(|list| list.contains(&format!("`{rest}`")))
        });
        if !used.contains(path.as_str()) && !listed {
            findings.push(finding(
                0,
                "unused",
                "no page shows this example: add a <Snippet file> for it, or list it in examples/UNUSED.md with the reason",
            ));
        }
        headers.insert(path.clone(), header(path, &text).first_sentence());
        ratchet.add(path, findings);
    }
    for (path, findings) in pair_findings(&config, &examples, &headers) {
        ratchet.add(&path, findings);
    }
    ratchet.finish(examples.len(), "docs/guidelines/examples.md")
}

fn load_config(root: &Path) -> Result<Config> {
    let path = root.join("tools/xtask/docs-snippets.toml");
    let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

/// The examples among `paths` (as [`examples`] finds them), with the `Snippet` globs read from
/// the repository at `root`: what `biome-examples` lints too.
pub(crate) fn example_paths(root: &Path, paths: &BTreeSet<String>) -> Result<Vec<String>> {
    let snippet = std::fs::read_to_string(root.join(SNIPPET)).with_context(|| format!("reading {SNIPPET}"))?;
    Ok(examples(paths, &snippet_globs(&snippet)))
}

/// The examples the rules apply to: everything under the example folders, and every file the
/// `Snippet` glob matches under an `examples/` folder.
fn examples(paths: &BTreeSet<String>, globs: &[Glob]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| crate::util::has_extension(path, &["ts", "rs", "toml"]) && !is_tool(path))
        .filter(|path| {
            EXAMPLE_DIRS.iter().any(|dir| path.starts_with(dir))
                || (path.contains("/examples/") && globs.iter().any(|glob| glob.matches(path)))
        })
        .cloned()
        .collect()
}

/// `(the example folder, the path inside it)`.
fn example_dir(path: &str) -> Option<(&'static str, &str)> {
    EXAMPLE_DIRS
        .iter()
        .find_map(|dir| path.strip_prefix(dir).map(|rest| (*dir, rest)))
}

/// One pattern of `Snippet`'s `import.meta.glob`, from the repository root, and its line.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Glob {
    pattern: String,
    line: usize,
}

impl Glob {
    /// Whether `path` matches: `**` is any number of folders, `*` any part of one name, and
    /// `{a,b}` either alternative.
    fn matches(&self, path: &str) -> bool {
        expand_braces(&self.pattern).iter().any(|pattern| {
            let parts: Vec<&str> = pattern.split('/').collect();
            let names: Vec<&str> = path.split('/').collect();
            match_parts(&parts, &names)
        })
    }
}

/// The patterns between `import.meta.glob(` and `]`, with their `../` prefix removed.
fn snippet_globs(source: &str) -> Vec<Glob> {
    let Some(start) = source.find("import.meta.glob(") else {
        return Vec::new();
    };
    let Some(rest) = source.get(start..) else {
        return Vec::new();
    };
    let list = rest.get(..rest.find(']').unwrap_or(rest.len())).unwrap_or_default();
    let first_line = source.get(..start).map_or(1, |before| before.matches('\n').count() + 1);
    // Every double-quoted string in the list, however the formatter wrapped it, outside comments.
    // (The list ends at its first `]`, so a `]` in a comment there would end it early.)
    let mut globs = Vec::new();
    for (offset, line) in list.lines().enumerate() {
        let code = line.split_once("//").map_or(line, |(code, _)| code);
        let mut quotes = code.match_indices('"').map(|(index, _)| index);
        while let (Some(open), Some(close)) = (quotes.next(), quotes.next()) {
            if let Some(pattern) = code.get(open + 1..close) {
                globs.push(Glob {
                    pattern: pattern.trim_start_matches("../").to_owned(),
                    line: first_line + offset,
                });
            }
        }
    }
    globs
}

/// `{a,b}` → both patterns, every group in turn (no nesting, as Vite's globs here use it).
fn expand_braces(pattern: &str) -> Vec<String> {
    let (Some(open), Some(close)) = (pattern.find('{'), pattern.find('}')) else {
        return vec![pattern.to_owned()];
    };
    let (Some(before), Some(inside), Some(after)) = (
        pattern.get(..open),
        pattern.get(open + 1..close),
        pattern.get(close + 1..),
    ) else {
        return vec![pattern.to_owned()];
    };
    inside
        .split(',')
        .flat_map(|choice| expand_braces(&format!("{before}{choice}{after}")))
        .collect()
}

fn match_parts(parts: &[&str], names: &[&str]) -> bool {
    match (parts.split_first(), names.split_first()) {
        (None, None) => true,
        (Some((&"**", rest)), _) => {
            match_parts(rest, names)
                || names
                    .split_first()
                    .is_some_and(|(name, more)| !name.starts_with('.') && match_parts(parts, more))
        }
        (Some((part, rest)), Some((name, more))) => match_name(part, name) && match_parts(rest, more),
        _ => false,
    }
}

/// `*` matches any run of characters inside one name, but not a leading `.` (as picomatch,
/// Vite's matcher, treats dotfiles).
fn match_name(pattern: &str, name: &str) -> bool {
    if name.starts_with('.') && !pattern.starts_with('.') {
        return false;
    }
    match_wildcards(pattern, name)
}

fn match_wildcards(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((prefix, rest)) => name.strip_prefix(prefix).is_some_and(|name| {
            name.char_indices()
                .map(|(index, _)| index)
                .chain(std::iter::once(name.len()))
                .any(|index| name.get(index..).is_some_and(|tail| match_wildcards(rest, tail)))
        }),
    }
}

/// Pages show examples, never tests: a glob into a `test/` folder is reported.
fn glob_findings(globs: &[Glob]) -> Vec<Finding> {
    globs
        .iter()
        .filter(|glob| glob.pattern.split('/').any(|part| part == "test"))
        .map(|glob| {
            finding(
                glob.line,
                "glob-test-path",
                format!(
                    "pages show examples, not tests: move the files `{}` matches into an examples/ folder, then drop the pattern from Snippet's glob",
                    glob.pattern
                ),
            )
        })
        .collect()
}

/// The files a page shows: the `file` of every `<Snippet>`, and the `ts` and `rust` of every
/// `<Example>` (which shows its code through `Snippet`).
fn snippet_uses(page: &str) -> Vec<String> {
    let mut uses = Vec::new();
    for (start, _) in page.match_indices('<') {
        let Some(tag) = page.get(start..).and_then(|t| t.find('>').and_then(|end| t.get(..end))) else {
            continue;
        };
        let attributes: &[&str] = if tag.starts_with("<Snippet") {
            &["file"]
        } else if tag.starts_with("<Example ") || tag.starts_with("<Example\n") {
            &["ts", "rust"]
        } else {
            continue;
        };
        for name in attributes {
            if let Some(value) = attribute(tag, name) {
                uses.push(value.to_owned());
            }
        }
    }
    uses
}

/// The double-quoted value of attribute `name` in `tag`, matched as a whole word.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("{name}=\"");
    let at = tag
        .match_indices(&needle)
        .find(|(at, _)| {
            tag.get(..*at)
                .is_some_and(|before| before.ends_with(char::is_whitespace))
        })?
        .0;
    let value = tag.get(at + needle.len()..)?;
    value.get(..value.find('"')?)
}

/// The comment an example opens with.
#[derive(Debug, Default)]
struct Header {
    /// 1-based line of each comment line, and its text without the comment marker.
    lines: Vec<(usize, String)>,
}

impl Header {
    /// The header's first sentence, which a TypeScript and Rust pair share.
    fn first_sentence(&self) -> String {
        let text: Vec<&str> = self.lines.iter().map(|(_, line)| line.as_str()).collect();
        let text = text.join(" ");
        let end = text
            .match_indices(". ")
            .map(|(index, _)| index + 1)
            .next()
            .unwrap_or(text.len());
        text.get(..end).unwrap_or(&text).trim().to_owned()
    }
}

/// The comment lines at the top of a file: `//`, `//!`, `/* … */` and ` * ` lines, or `#` lines
/// in TOML (in Rust, `#` starts an attribute).
fn header(path: &str, text: &str) -> Header {
    let markers: &[&str] = if crate::util::has_extension(path, &["toml"]) {
        &["#"]
    } else {
        &["//!", "///", "//", "/**", "/*", "*/", "*"]
    };
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let comment = markers.iter().find_map(|marker| trimmed.strip_prefix(marker));
        match comment {
            Some(rest) => lines.push((index + 1, rest.trim_end_matches("*/").trim().to_owned())),
            None if trimmed.is_empty() && lines.is_empty() => {}
            None => break,
        }
    }
    lines.retain(|(_, line)| !line.is_empty());
    Header { lines }
}

/// The rules that read one file: the header, imports, scaffolding (unless `test_example`: the
/// file is listed under `[[test-example]]`) and length.
fn file_findings(path: &str, text: &str, test_example: bool) -> Vec<Finding> {
    let mut findings = Vec::new();
    let head = header(path, text);
    if head.lines.is_empty() {
        findings.push(finding(
            1,
            "header",
            "open the file with a comment that says what it shows",
        ));
    }
    for (line, comment) in &head.lines {
        if let Some(what) = header_problem(comment) {
            findings.push(finding(
                *line,
                "header",
                format!("remove {what} from the header comment: it is for readers, so say what the example shows"),
            ));
        }
    }
    let rust = crate::util::has_extension(path, &["rs"]);
    let toml = crate::util::has_extension(path, &["toml"]);
    let mut in_block_comment = false;
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let Some(code) = code_part(line, toml, &mut in_block_comment) else {
            continue;
        };
        if rust && names_internal_crate(code) {
            findings.push(finding(
                line_number,
                "public-imports",
                "import from `laminapdf::…`, the crate readers depend on, not from `laminapdf_<crate>`",
            ));
        }
        if rust && code.contains("include_bytes!(\"../../..") {
            findings.push(finding(
                line_number,
                "public-imports",
                "read the input as a reader's app would (a parameter or a file next to the example), not from the repository",
            ));
        }
        let scaffolding = [
            ("a `lamina-ex-…` ID", !test_example && code.contains("lamina-ex-")),
            ("a quoted PDF date (`\"D:…\"`)", !test_example && has_fixed_date(code)),
        ];
        for (marker, _) in scaffolding.iter().filter(|(_, found)| *found) {
            findings.push(finding(
                line_number,
                "test-scaffolding",
                format!("{marker} is a fixed test value: make it a parameter and let the tests pass it in"),
            ));
        }
    }
    let count = text.lines().count();
    if count > MAX_LINES {
        findings.push(finding(
            MAX_LINES + 1,
            "file-length",
            format!("the file has {count} lines; split it into examples of {MAX_LINES} or fewer"),
        ));
    }
    findings
}

/// The code on a line without its comments, or `None` when the line is all comment. Block
/// comments (`/* … */`) are followed across lines; a trailing `//` (or `#` in TOML) is cut.
fn code_part<'a>(line: &'a str, toml: bool, in_block_comment: &mut bool) -> Option<&'a str> {
    let code = line.trim_start();
    if toml {
        return (!code.starts_with('#')).then_some(code);
    }
    if *in_block_comment {
        *in_block_comment = !code.contains("*/");
        return None;
    }
    if code.starts_with("/*") {
        *in_block_comment = !code.contains("*/");
        return None;
    }
    let code = code.split_once(" //").map_or(code, |(code, _)| code);
    (!code.starts_with("//")).then_some(code)
}

/// The crates besides the façade that readers depend on directly: `laminapdf-render`, which draws
/// pages (the façade never renders), as `@laminapdf/render` is in TypeScript.
const PUBLIC_CRATES: [&str; 1] = ["render"];

/// Whether code names a crate other than the façade and [`PUBLIC_CRATES`]: `laminapdf_build::…`,
/// `use laminapdf_text`.
fn names_internal_crate(code: &str) -> bool {
    code.match_indices("laminapdf_").any(|(index, _)| {
        let before = code.get(..index).and_then(|before| before.chars().next_back());
        let starts_word = before.is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let rest = code.get(index + "laminapdf_".len()..).unwrap_or_default();
        let name_len = rest.chars().take_while(|c| c.is_ascii_lowercase() || *c == '_').count();
        let after = rest.get(name_len..).unwrap_or_default();
        // `laminapdf_x::…` anywhere; `use laminapdf_x;` and `use laminapdf_x as y;` only in a use line.
        let use_line = ["use ", "pub use ", "extern crate "]
            .iter()
            .any(|start| code.starts_with(start));
        let public = rest.get(..name_len).is_some_and(|name| PUBLIC_CRATES.contains(&name));
        starts_word
            && name_len > 0
            && !public
            && (after.starts_with("::") || (use_line && (after.starts_with(';') || after.starts_with(" as "))))
    })
}

/// A PDF date literal fixed in the code (`"D:20260101…"`), which only a test needs.
fn has_fixed_date(line: &str) -> bool {
    line.match_indices("D:").any(|(index, _)| {
        let quoted = index
            .checked_sub(1)
            .and_then(|before| line.get(before..index))
            .is_some_and(|quote| matches!(quote, "\"" | "'" | "`" | "("));
        let digits = line
            .get(index + 2..)
            .unwrap_or_default()
            .chars()
            .take_while(char::is_ascii_digit)
            .count();
        quoted && digits >= 4
    })
}

/// What a header line mentions that it shouldn't: an ADR or todo ID, a repository path, or test
/// vocabulary.
fn header_problem(comment: &str) -> Option<String> {
    let words: Vec<&str> = comment
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | ',' | ';' | ':' | '.' | '`'))
        .filter(|word| !word.is_empty())
        .collect();
    for (index, word) in words.iter().enumerate() {
        let next_is_number = words
            .get(index + 1)
            .is_some_and(|next| next.len() == 4 && next.chars().all(|c| c.is_ascii_digit()));
        if (*word == "ADR" && next_is_number) || word.starts_with("ADR-") {
            return Some("an ADR number".to_owned());
        }
        if crate::docs_prose::is_todo_id(word) {
            return Some(format!("the todo ID `{word}`"));
        }
        let lower = word.to_lowercase();
        if matches!(lower.as_str(), "golden" | "goldens" | "twin" | "twins") {
            return Some(format!("\"{word}\""));
        }
    }
    ["fixtures/", "crates/", "packages/", "test/", "tools/"]
        .iter()
        .find(|prefix| comment.contains(*prefix))
        .map(|prefix| format!("the repository path `{prefix}…`"))
}

/// A TypeScript and Rust example of the same task share a name (`links-and-outlines.ts`,
/// `links_and_outlines.rs`) and their header's first sentence. The table in `docs-snippets.toml`
/// lists the pairs; an unlisted file whose name matches one on the other side is reported too.
fn pair_findings(
    config: &Config,
    examples: &[String],
    headers: &BTreeMap<String, String>,
) -> Vec<(String, Vec<Finding>)> {
    let mut out = Vec::new();
    let stem = |path: &str| {
        let name = path.rsplit('/').next().unwrap_or(path);
        let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
        stem.replace('-', "_")
    };
    let listed: BTreeSet<&str> = config
        .pair
        .iter()
        .flat_map(|pair| [pair.ts.as_str(), pair.rust.as_str()])
        .collect();
    for pair in &config.pair {
        let mut findings = Vec::new();
        for side in [&pair.ts, &pair.rust] {
            if !examples.contains(side) {
                findings.push(finding(
                    0,
                    "pair",
                    format!(
                        "`{side}` doesn't exist: fix its path in tools/xtask/docs-snippets.toml or remove the pair"
                    ),
                ));
            }
        }
        if stem(&pair.ts) != stem(&pair.rust) {
            findings.push(finding(
                0,
                "pair",
                format!(
                    "`{}` and `{}` show the same task under different names: give both the same name (kebab-case for .ts, snake_case for .rs) and update the pair table",
                    pair.ts, pair.rust
                ),
            ));
        }
        let sentences_differ = matches!(
            (headers.get(&pair.ts), headers.get(&pair.rust)),
            (Some(ts), Some(rust)) if ts != rust
        );
        // On both files, so a PR that changes either side is held to the pair; each names the other.
        for (path, other) in [(&pair.ts, &pair.rust), (&pair.rust, &pair.ts)] {
            let mut side = findings.clone();
            if sentences_differ {
                side.push(finding(
                    1,
                    "pair",
                    format!("the header's first sentence differs from `{other}`'s: use the same one in both"),
                ));
            }
            out.push((path.clone(), side));
        }
    }
    let ts: Vec<&String> = examples
        .iter()
        .filter(|path| crate::util::has_extension(path, &["ts"]))
        .collect();
    for rust in examples.iter().filter(|path| crate::util::has_extension(path, &["rs"])) {
        for ts in ts.iter().filter(|ts| stem(ts) == stem(rust)) {
            if !listed.contains(ts.as_str()) || !listed.contains(rust.as_str()) {
                let message = format!(
                    "`{ts}` and `{rust}` share a name: add them to the pair table in tools/xtask/docs-snippets.toml"
                );
                out.push(((*ts).clone(), vec![finding(0, "pair", message.clone())]));
                out.push((rust.clone(), vec![finding(0, "pair", message)]));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glob(pattern: &str) -> Glob {
        Glob {
            pattern: pattern.to_owned(),
            line: 1,
        }
    }

    fn rules(path: &str, text: &str) -> Vec<&'static str> {
        file_findings(path, text, false)
            .into_iter()
            .map(|finding| finding.rule)
            .collect()
    }

    #[test]
    fn a_developer_tool_under_the_examples_should_not_be_checked_as_an_example() {
        let paths: BTreeSet<String> = [
            "crates/laminapdf/examples/hello.rs",
            "crates/laminapdf/examples/differential/text-facts.rs",
        ]
        .map(String::from)
        .into();
        assert_eq!(examples(&paths, &[]), ["crates/laminapdf/examples/hello.rs"]);
    }

    #[test]
    fn globs_should_match_folders_names_and_choices() {
        let examples = glob("packages/sdk/examples/**/*.{ts,toml}");
        assert!(examples.matches("packages/sdk/examples/hello.ts"));
        assert!(examples.matches("packages/sdk/examples/worker/src/index.ts"));
        assert!(examples.matches("packages/sdk/examples/worker/wrangler.toml"));
        assert!(!examples.matches("packages/sdk/examples/README.md"));
        assert!(
            !glob("crates/laminapdf/examples/*.rs").matches("crates/laminapdf/examples/differential/text-facts.rs")
        );
    }

    #[test]
    fn the_snippet_glob_should_be_read_with_its_lines() {
        let source = "x\nconst FILES = import.meta.glob(\n  [\n    \"../../../../a/test/b.ts\",\n    \"../../../../c/*.rs\",\n  ],\n);\n";
        let globs = snippet_globs(source);
        assert_eq!(
            globs,
            vec![
                Glob {
                    pattern: "a/test/b.ts".into(),
                    line: 4
                },
                Glob {
                    pattern: "c/*.rs".into(),
                    line: 5
                },
            ]
        );
        assert_eq!(glob_findings(&globs).len(), 1);
    }

    #[test]
    fn a_glob_on_one_line_should_be_read() {
        let source = "x\nconst F = import.meta.glob(\n  [\"../../a/*.ts\", \"../../b/*.rs\"],\n  {},\n);\n";
        let patterns: Vec<(String, usize)> = snippet_globs(source).into_iter().map(|g| (g.pattern, g.line)).collect();
        assert_eq!(patterns, vec![("a/*.ts".to_owned(), 3), ("b/*.rs".to_owned(), 3)]);
    }

    #[test]
    fn quotes_in_glob_comments_should_be_ignored() {
        let source = "const F = import.meta.glob(\n  [\n    // see \"x\"\n    \"../../a/*.ts\",\n  ],\n);\n";
        let patterns: Vec<String> = snippet_globs(source).into_iter().map(|glob| glob.pattern).collect();
        assert_eq!(patterns, vec!["a/*.ts"]);
    }

    #[test]
    fn snippet_uses_should_be_read_from_pages() {
        let page = "Text.\n\n<Snippet file=\"packages/sdk/examples/hello.ts\" />\n<Snippet\n  title=\"x\"\n  file=\"crates/laminapdf/examples/hello.rs\"\n/>\n";
        assert_eq!(
            snippet_uses(page),
            vec!["packages/sdk/examples/hello.ts", "crates/laminapdf/examples/hello.rs"]
        );
    }

    #[test]
    fn an_examples_code_should_count_as_shown() {
        let page = "<Example ts=\"packages/sdk/examples/hello.ts\" rust=\"crates/laminapdf/examples/hello.rs\" result=\"hello\" />\n<Example\n  ts=\"packages/sdk/examples/invoice.ts\"\n/>\n<ExampleCode ts=\"x.ts\" />";
        assert_eq!(
            snippet_uses(page),
            vec![
                "packages/sdk/examples/hello.ts",
                "crates/laminapdf/examples/hello.rs",
                "packages/sdk/examples/invoice.ts"
            ]
        );
    }

    #[test]
    fn a_clean_example_should_pass() {
        let text = "// Builds a one-page PDF with a line of text.\nimport { init } from \"@laminapdf/sdk\";\n";
        assert_eq!(rules("packages/sdk/examples/hello.ts", text), Vec::<&str>::new());
    }

    #[test]
    fn a_missing_or_internal_header_should_be_reported() {
        assert_eq!(rules("a.ts", "import x from \"y\";\n"), vec!["header"]);
        assert_eq!(
            rules("a.ts", "// The twin of hello.rs (B5b), see ADR 0016.\n"),
            vec!["header",]
        );
        assert_eq!(
            rules("a.rs", "//! Writes fixtures/golden/examples/hello.pdf.\n"),
            vec!["header"]
        );
    }

    #[test]
    fn rust_examples_should_use_the_facade_and_no_repository_files() {
        let text = "//! Shows text.\nuse laminapdf_build::Builder;\nconst F: &[u8] = include_bytes!(\"../../../fixtures/x.ttf\");\n";
        assert_eq!(rules("a.rs", text), vec!["public-imports", "public-imports"]);
    }

    #[test]
    fn test_scaffolding_should_be_reported() {
        let text = "// Shows metadata.\nconst id = \"lamina-ex-hello\";\nconst date = \"D:20260101\";\n";
        assert_eq!(rules("a.ts", text), vec!["test-scaffolding", "test-scaffolding"]);
    }

    #[test]
    fn a_test_example_may_fix_the_file_id_and_dates() {
        let text = "// Checks a PDF against its reference.\nconst date = \"D:20260101\";\n";
        assert_eq!(file_findings("a.ts", text, true), Vec::<Finding>::new());
    }

    #[test]
    fn a_test_example_entry_should_need_a_reason() {
        let parse = |text: &str| toml::from_str::<Config>(text).map(|config| config.test_example.len());
        assert_eq!(
            parse("[[test-example]]\npath = \"a.ts\"\nreason = \"tests a reader's PDFs\"\n").ok(),
            Some(1)
        );
        assert!(parse("[[test-example]]\npath = \"a.ts\"\n").is_err());
    }

    #[test]
    fn a_long_file_should_be_reported() {
        let text = format!("// Shows many lines.\n{}", "x;\n".repeat(MAX_LINES));
        assert_eq!(rules("a.ts", &text), vec!["file-length"]);
    }

    #[test]
    fn the_first_sentence_should_join_comment_lines() {
        let head = header(
            "a.rs",
            "//! Builds a page with\n//! a heading. Then saves it.\nfn main() {}\n",
        );
        assert_eq!(head.first_sentence(), "Builds a page with a heading.");
    }

    #[test]
    fn pairs_should_share_names_and_first_sentences() {
        let config = Config {
            pair: vec![Pair {
                ts: "packages/sdk/examples/images-and-vectors.ts".into(),
                rust: "crates/laminapdf/examples/vectors.rs".into(),
            }],
            test_example: Vec::new(),
        };
        let examples = vec![
            "packages/sdk/examples/images-and-vectors.ts".to_owned(),
            "crates/laminapdf/examples/vectors.rs".to_owned(),
            "packages/sdk/examples/hello.ts".to_owned(),
            "crates/laminapdf/examples/hello.rs".to_owned(),
        ];
        let headers: BTreeMap<String, String> = examples
            .iter()
            .map(|path| {
                (
                    path.clone(),
                    if path.contains("vectors.rs") { "B." } else { "A." }.to_owned(),
                )
            })
            .collect();
        let rules: Vec<&str> = pair_findings(&config, &examples, &headers)
            .into_iter()
            .flat_map(|(_, findings)| findings.into_iter().map(|finding| finding.rule))
            .collect();
        // A different name and a different sentence (on both files), then hello's pair missing
        // from the table (on both files).
        assert_eq!(rules, vec!["pair"; 6]);
    }

    #[test]
    fn a_rust_attribute_should_not_count_as_a_header() {
        assert_eq!(rules("a.rs", "#![allow(clippy::x)]\nfn main() {}\n"), vec!["header"]);
        assert_eq!(
            rules("a.toml", "# Deploys the example.\nname = \"x\"\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn only_quoted_pdf_dates_should_be_scaffolding() {
        let text = "// Shows IDs.\nconst uuid = \"UUID:2024\"; // a D:2026 date\n";
        assert_eq!(rules("a.ts", text), Vec::<&str>::new());
        assert_eq!(
            rules("a.ts", "// Shows a date.\nconst d = \"D:20260101\";\n"),
            vec!["test-scaffolding"]
        );
    }

    #[test]
    fn internal_crates_should_be_reported_wherever_they_are_named() {
        let text = "//! Shows text.\npub use laminapdf_text::Font;\nlet b = laminapdf_build::Builder::new();\nuse laminapdf::Document;\n";
        assert_eq!(rules("a.rs", text), vec!["public-imports", "public-imports"]);
    }

    #[test]
    fn the_render_crate_should_be_a_public_import_but_not_its_pdfium_adapter() {
        let text = "//! Shows a page.\nuse laminapdf_render::{HayroBackend, RenderBackend};\n";
        assert_eq!(rules("a.rs", text), Vec::<&str>::new());
        let text = "//! Shows a page.\nuse laminapdf_render_pdfium::PdfiumBackend;\n";
        assert_eq!(rules("a.rs", text), vec!["public-imports"]);
    }

    #[test]
    fn globs_should_handle_several_stars_groups_and_dotfiles() {
        assert!(glob("a/x*y*z.ts").matches("a/x1y2z.ts"));
        assert!(!glob("a/x*y*z.ts").matches("a/x1z.ts"));
        assert!(glob("a/{b,c}/*.{ts,rs}").matches("a/c/d.rs"));
        assert!(!glob("a/*.ts").matches("a/.hidden.ts"));
    }

    #[test]
    fn identifiers_that_start_with_laminapdf_should_not_be_crates() {
        let text = "//! Shows lengths.\nlet n = laminapdf_len as usize;\nreturn laminapdf_bytes;\n";
        assert_eq!(rules("a.rs", text), Vec::<&str>::new());
        assert_eq!(
            rules("a.rs", "//! Shows text.\nuse laminapdf_text as text;\n"),
            vec!["public-imports"]
        );
    }

    #[test]
    fn comments_should_not_be_read_as_code() {
        let text = "// Shows a date.\n/* \"D:20260101\"\n * lamina-ex-x\n */\nlet x = 1; // \"D:20260101\"\n";
        assert_eq!(rules("a.ts", text), Vec::<&str>::new());
        assert_eq!(
            rules("a.toml", "# Deploys it.\n# \"D:20260101\"\nname = \"x\"\n"),
            Vec::<&str>::new()
        );
    }

    #[test]
    fn double_stars_should_skip_dot_folders() {
        assert!(!glob("a/**/*.ts").matches("a/.cache/x.ts"));
        assert!(glob("a/**/*.ts").matches("a/b/x.ts"));
    }

    #[test]
    fn the_repository_config_should_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(load_config(&root).is_ok_and(|config| !config.pair.is_empty()));
    }
}
