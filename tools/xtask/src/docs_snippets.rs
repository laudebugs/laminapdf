//! `cargo xtask check docs-snippets`: the rules of docs/guidelines/examples.md that a tool can check,
//! over the examples (`packages/sdk/examples/**`, `crates/lamina/examples/**`) and the files the
//! docs site's `Snippet` component may show (its `import.meta.glob`). Strict on changed files and
//! counted warnings elsewhere, like `docs-prose` ([`crate::reports`]).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::reports::{Finding, Options, Ratchet, finding};
use crate::tracked::Tracked;

const SNIPPET: &str = "apps/docs/src/components/Snippet.astro";
const PAGES: &str = "apps/docs/src/content/docs/";
const EXAMPLE_DIRS: [&str; 2] = ["packages/sdk/examples/", "crates/lamina/examples/"];
const MAX_LINES: usize = 100;

/// `tools/xtask/docs-snippets.toml`: the TypeScript and Rust examples that show the same task.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    #[serde(default)]
    pair: Vec<Pair>,
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
        let mut findings = file_findings(path, &text);
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
        headers.insert(path.clone(), header(&text).first_sentence());
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

/// The examples the rules apply to: everything under the example folders, and every file the
/// `Snippet` glob matches under an `examples/` folder.
fn examples(paths: &BTreeSet<String>, globs: &[Glob]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| !path.ends_with("/UNUSED.md") && !path.ends_with("/README.md"))
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
    let first_line = source.get(..start).map_or(1, |before| before.matches('\n').count() + 1);
    let Some(rest) = source.get(start..) else {
        return Vec::new();
    };
    let end = rest.find(']').unwrap_or(rest.len());
    rest.get(..end)
        .unwrap_or_default()
        .lines()
        .enumerate()
        .filter_map(|(offset, line)| {
            let quoted = line.trim().trim_end_matches(',');
            let pattern = quoted.strip_prefix('"')?.strip_suffix('"')?;
            let pattern = pattern.trim_start_matches("../");
            Some(Glob {
                pattern: pattern.to_owned(),
                line: first_line + offset,
            })
        })
        .collect()
}

/// `{a,b}` → both patterns (one level, as Vite's globs use it).
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
        .map(|choice| format!("{before}{choice}{after}"))
        .collect()
}

fn match_parts(parts: &[&str], names: &[&str]) -> bool {
    match (parts.split_first(), names.split_first()) {
        (None, None) => true,
        (Some((&"**", rest)), _) => {
            match_parts(rest, names) || names.split_first().is_some_and(|(_, more)| match_parts(parts, more))
        }
        (Some((part, rest)), Some((name, more))) => match_name(part, name) && match_parts(rest, more),
        _ => false,
    }
}

/// `*` matches any run of characters inside one name.
fn match_name(pattern: &str, name: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == name,
        Some((prefix, suffix)) => {
            name.len() >= prefix.len() + suffix.len() && name.starts_with(prefix) && name.ends_with(suffix)
        }
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
                    "pages show examples, not tests: move `{}` into an examples/ folder and drop it from the glob",
                    glob.pattern
                ),
            )
        })
        .collect()
}

/// The `file` of every `<Snippet>` on a page.
fn snippet_uses(page: &str) -> Vec<String> {
    page.match_indices("<Snippet")
        .filter_map(|(start, _)| {
            let tag = page.get(start..)?;
            let tag = tag.get(..tag.find('>')?)?;
            let value = tag.split_once("file=\"")?.1;
            Some(value.get(..value.find('"')?)?.to_owned())
        })
        .collect()
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

/// The comment lines at the top of a file (`//`, `//!`, `/* … */` and ` * ` lines, `#` in TOML).
fn header(text: &str) -> Header {
    let mut lines = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        let comment = ["//!", "///", "//", "/**", "/*", "*/", "*", "#"]
            .iter()
            .find_map(|marker| trimmed.strip_prefix(marker));
        match comment {
            Some(rest) => lines.push((index + 1, rest.trim_end_matches("*/").trim().to_owned())),
            None if trimmed.is_empty() && lines.is_empty() => {}
            None => break,
        }
    }
    lines.retain(|(_, line)| !line.is_empty());
    Header { lines }
}

/// The rules that read one file: the header, imports, scaffolding and length.
fn file_findings(path: &str, text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let head = header(text);
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
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let code = line.trim_start();
        if rust && code.starts_with("use lamina_") {
            findings.push(finding(
                line_number,
                "public-imports",
                "import from `lamina::…`, the crate readers depend on, not from `lamina_<crate>`",
            ));
        }
        if rust && code.contains("include_bytes!(\"../../..") {
            findings.push(finding(
                line_number,
                "public-imports",
                "read the input as a reader's app would (a parameter or a file next to the example), not from the repository",
            ));
        }
        for marker in ["lamina-ex-", "D:20"] {
            if line.contains(marker) {
                findings.push(finding(
                    line_number,
                    "test-scaffolding",
                    format!("`{marker}…` is a fixed test value: make it a parameter and let the tests pass it in"),
                ));
            }
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
                    format!("the pair table names `{side}`, which doesn't exist"),
                ));
            }
        }
        if stem(&pair.ts) != stem(&pair.rust) {
            findings.push(finding(
                0,
                "pair",
                format!(
                    "`{}` and `{}` show the same task under different names: rename one",
                    pair.ts, pair.rust
                ),
            ));
        }
        if let (Some(ts), Some(rust)) = (headers.get(&pair.ts), headers.get(&pair.rust))
            && ts != rust
        {
            findings.push(finding(
                1,
                "pair",
                format!(
                    "the header's first sentence differs from `{}`'s: use the same one in both",
                    pair.rust
                ),
            ));
        }
        out.push((pair.ts.clone(), findings));
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
                out.push(((*ts).clone(), vec![finding(0, "pair", message)]));
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
        file_findings(path, text)
            .into_iter()
            .map(|finding| finding.rule)
            .collect()
    }

    #[test]
    fn globs_should_match_folders_names_and_choices() {
        let examples = glob("packages/sdk/examples/**/*.{ts,toml}");
        assert!(examples.matches("packages/sdk/examples/hello.ts"));
        assert!(examples.matches("packages/sdk/examples/worker/src/index.ts"));
        assert!(examples.matches("packages/sdk/examples/worker/wrangler.toml"));
        assert!(!examples.matches("packages/sdk/examples/README.md"));
        assert!(!glob("crates/lamina/examples/*.rs").matches("crates/lamina/examples/differential/text-facts.rs"));
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
    fn snippet_uses_should_be_read_from_pages() {
        let page = "Text.\n\n<Snippet file=\"packages/sdk/examples/hello.ts\" />\n<Snippet\n  title=\"x\"\n  file=\"crates/lamina/examples/hello.rs\"\n/>\n";
        assert_eq!(
            snippet_uses(page),
            vec!["packages/sdk/examples/hello.ts", "crates/lamina/examples/hello.rs"]
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
        let text = "//! Shows text.\nuse lamina_build::Builder;\nconst F: &[u8] = include_bytes!(\"../../../fixtures/x.ttf\");\n";
        assert_eq!(rules("a.rs", text), vec!["public-imports", "public-imports"]);
    }

    #[test]
    fn test_scaffolding_should_be_reported() {
        let text = "// Shows metadata.\nconst id = \"lamina-ex-hello\";\nconst date = \"D:20260101\";\n";
        assert_eq!(rules("a.ts", text), vec!["test-scaffolding", "test-scaffolding"]);
    }

    #[test]
    fn a_long_file_should_be_reported() {
        let text = format!("// Shows many lines.\n{}", "x;\n".repeat(MAX_LINES));
        assert_eq!(rules("a.ts", &text), vec!["file-length"]);
    }

    #[test]
    fn the_first_sentence_should_join_comment_lines() {
        let head = header("//! Builds a page with\n//! a heading. Then saves it.\nfn main() {}\n");
        assert_eq!(head.first_sentence(), "Builds a page with a heading.");
    }

    #[test]
    fn pairs_should_share_names_and_first_sentences() {
        let config = Config {
            pair: vec![Pair {
                ts: "packages/sdk/examples/images-and-vectors.ts".into(),
                rust: "crates/lamina/examples/vectors.rs".into(),
            }],
        };
        let examples = vec![
            "packages/sdk/examples/images-and-vectors.ts".to_owned(),
            "crates/lamina/examples/vectors.rs".to_owned(),
            "packages/sdk/examples/hello.ts".to_owned(),
            "crates/lamina/examples/hello.rs".to_owned(),
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
        // A different name and a different sentence, then hello's pair missing from the table.
        assert_eq!(rules, vec!["pair", "pair", "pair"]);
    }

    #[test]
    fn the_repository_config_should_parse() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(load_config(&root).is_ok_and(|config| !config.pair.is_empty()));
    }
}
