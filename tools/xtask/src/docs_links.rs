//! `cargo xtask docs-links`: every relative link and `#anchor` in the tracked Markdown resolves.
//!
//! Links are read with `pulldown-cmark` (inline, reference and image links; code spans, code
//! blocks and autolinks are not links). External URLs are not checked. A link's path is resolved
//! against the linking file (or the repository root for `/…`); a `#fragment` on a Markdown target
//! (or on the same file) must name a heading, using GitHub's slugs, or an HTML `id`/`name`.

mod slug;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use serde_json::json;

use self::slug::Slugger;

/// The todo board's state folders, used to suggest where a moved todo now lives.
const TODO_STATES: [&str; 3] = ["ready", "ongoing", "done"];

/// Why a link is broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Reason {
    MissingFile,
    MissingAnchor,
}

impl fmt::Display for Reason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::MissingFile => "missing file",
            Self::MissingAnchor => "missing anchor",
        })
    }
}

/// One broken link: where it is, what it points to, and why it does not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrokenLink {
    /// The linking file, relative to the root.
    pub(crate) file: String,
    /// 1-based line of the link.
    pub(crate) line: usize,
    pub(crate) target: String,
    pub(crate) reason: Reason,
    /// A suggested replacement path (for a todo that moved to another state folder).
    pub(crate) hint: Option<String>,
}

impl fmt::Display for BrokenLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: broken link {} ({})",
            self.file, self.line, self.target, self.reason
        )?;
        if let Some(hint) = &self.hint {
            write!(f, "; did you mean {hint}?")?;
        }
        Ok(())
    }
}

/// Runs the check on every tracked Markdown file under `root`; fails if any link is broken.
pub(crate) fn run(root: &Path, json_output: bool) -> Result<()> {
    let files = tracked_markdown(root)?;
    let broken = check_files(root, &files)?;
    if json_output {
        let items: Vec<_> = broken
            .iter()
            .map(|b| {
                json!({
                    "file": b.file,
                    "line": b.line,
                    "target": b.target,
                    "reason": b.reason.to_string(),
                    "hint": b.hint,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
    } else {
        for link in &broken {
            eprintln!("{link}");
        }
    }
    if !broken.is_empty() {
        bail!("{} broken link(s) in {} Markdown files", broken.len(), files.len());
    }
    if !json_output {
        println!("docs-links: {} Markdown files, no broken links", files.len());
    }
    Ok(())
}

/// Tracked `*.md` files, relative to `root`, without build output or installed packages.
fn tracked_markdown(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "-z", "--", "*.md"])
        .current_dir(root)
        .output()
        .context("failed to run `git ls-files`; is git installed?")?;
    if !output.status.success() {
        bail!(
            "`git ls-files` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let listing = String::from_utf8(output.stdout).context("`git ls-files` printed non-UTF-8 paths")?;
    Ok(listing
        .split('\0')
        .filter(|path| !path.is_empty() && !is_excluded(path))
        .map(PathBuf::from)
        .collect())
}

/// Build output and installed packages are not documentation, even if something tracks them.
fn is_excluded(path: &str) -> bool {
    let parts: Vec<&str> = path.split('/').collect();
    parts.contains(&"node_modules") || (parts.first() == Some(&"spikes") && parts.contains(&"target"))
}

/// Checks the links in `files` (paths relative to `root`); returns every broken one.
pub(crate) fn check_files(root: &Path, files: &[PathBuf]) -> Result<Vec<BrokenLink>> {
    let mut anchors = AnchorCache::default();
    let mut broken = Vec::new();
    for file in files {
        let text = fs::read_to_string(root.join(file)).with_context(|| format!("reading {}", file.display()))?;
        let doc = scan(&text);
        anchors.insert(file.clone(), doc.anchors);
        for link in doc.links {
            if let Some((reason, hint)) = check_link(root, file, &link.target, &mut anchors)? {
                broken.push(BrokenLink {
                    file: display(file),
                    line: link.line,
                    target: link.target,
                    reason,
                    hint,
                });
            }
        }
    }
    Ok(broken)
}

/// Anchors of each Markdown file seen so far, keyed by normalised path relative to the root.
#[derive(Default)]
struct AnchorCache(BTreeMap<PathBuf, BTreeSet<String>>);

impl AnchorCache {
    fn insert(&mut self, file: PathBuf, anchors: BTreeSet<String>) {
        self.0.insert(file, anchors);
    }

    fn get(&mut self, root: &Path, file: &Path) -> Result<&BTreeSet<String>> {
        if !self.0.contains_key(file) {
            let text = fs::read_to_string(root.join(file)).with_context(|| format!("reading {}", file.display()))?;
            self.0.insert(file.to_path_buf(), scan(&text).anchors);
        }
        self.0
            .get(file)
            .with_context(|| format!("anchors of {} were not cached", file.display()))
    }
}

/// `None` when the link resolves, else why not (and a suggested path, if any).
fn check_link(
    root: &Path,
    from: &Path,
    target: &str,
    anchors: &mut AnchorCache,
) -> Result<Option<(Reason, Option<String>)>> {
    if is_external(target) {
        return Ok(None);
    }
    let (path_part, fragment) = match target.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (target, None),
    };
    let path_part = path_part.split_once('?').map_or(path_part, |(path, _query)| path);
    let resolved = if path_part.is_empty() {
        Some(from.to_path_buf())
    } else {
        resolve(from, &percent_decode(path_part))
    };
    let Some(resolved) = resolved.filter(|path| root.join(path).exists()) else {
        return Ok(Some((Reason::MissingFile, moved_todo_hint(root, from, path_part))));
    };
    let Some(fragment) = fragment.filter(|f| !f.is_empty()) else {
        return Ok(None);
    };
    let is_markdown = resolved.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"));
    if !is_markdown || !root.join(&resolved).is_file() {
        // Fragments on source files (`#L10`) and directories are GitHub UI features, not headings.
        return Ok(None);
    }
    let fragment = percent_decode(fragment);
    let found = anchors.get(root, &resolved)?.contains(&fragment);
    Ok((!found).then_some((Reason::MissingAnchor, None)))
}

/// URLs with a scheme (`https:`, `mailto:`) or protocol-relative ones (`//host`).
fn is_external(target: &str) -> bool {
    if target.starts_with("//") {
        return true;
    }
    let scheme = target.split_once(':').map(|(scheme, _)| scheme);
    scheme.is_some_and(|s| {
        s.starts_with(|c: char| c.is_ascii_alphabetic())
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    })
}

/// `target` resolved against the linking file `from`, both relative to the root; `/…` is
/// root-relative. `None` when it climbs out of the repository.
fn resolve(from: &Path, target: &str) -> Option<PathBuf> {
    let joined = match target.strip_prefix('/') {
        Some(rooted) => PathBuf::from(rooted),
        None => from.parent().unwrap_or_else(|| Path::new("")).join(target),
    };
    let mut normal = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normal.pop() {
                    return None;
                }
            }
            Component::Normal(part) => normal.push(part),
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(normal)
}

/// For a missing `…/docs/todos/<state>/<file>.md`, the same link pointing at the state folder
/// where that file is now.
fn moved_todo_hint(root: &Path, from: &Path, path_part: &str) -> Option<String> {
    let resolved = resolve(from, path_part)?;
    let mut parts = resolved.iter().map(|p| p.to_str());
    let (Some(Some("docs")), Some(Some("todos")), Some(Some(state)), Some(Some(file)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    TODO_STATES
        .iter()
        .filter(|&&other| other != state)
        .find(|other| root.join("docs/todos").join(other).join(file).is_file())
        .map(|other| path_part.replacen(&format!("{state}/{file}"), &format!("{other}/{file}"), 1))
}

/// Decodes `%XX` escapes (GitHub links spaces as `%20`); invalid escapes are kept as written.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while let Some(&byte) = bytes.get(i) {
        let escaped = (byte == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        if let Some(decoded) = escaped {
            out.push(decoded);
            i += 3;
        } else {
            out.push(byte);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// A link found in a document.
#[derive(Debug, PartialEq, Eq)]
struct Link {
    line: usize,
    target: String,
}

/// What the checker needs from one Markdown document.
#[derive(Debug, Default)]
struct Scan {
    links: Vec<Link>,
    /// Heading slugs plus explicit HTML `id`/`name` anchors.
    anchors: BTreeSet<String>,
}

/// The parser options matching GitHub-flavoured Markdown closely enough for links and headings.
fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_GFM
}

/// The links and anchors of one Markdown document.
fn scan(text: &str) -> Scan {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(text.match_indices('\n').map(|(i, _)| i + 1))
        .collect();
    let line_of = |offset: usize| line_starts.partition_point(|&start| start <= offset);

    let mut scan = Scan::default();
    let mut slugger = Slugger::default();
    let mut heading: Option<String> = None;
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        match event {
            Event::Start(
                Tag::Link {
                    link_type, dest_url, ..
                }
                | Tag::Image {
                    link_type, dest_url, ..
                },
            ) if !matches!(link_type, LinkType::Autolink | LinkType::Email) => {
                scan.links.push(Link {
                    line: line_of(range.start),
                    target: dest_url.into_string(),
                });
            }
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => {
                if let Some(text) = heading.take() {
                    scan.anchors.insert(slugger.next(text.trim()));
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(h) = heading.as_mut() {
                    h.push_str(&t);
                }
            }
            Event::Html(html) | Event::InlineHtml(html) => scan.anchors.extend(html_anchors(&html)),
            _ => {}
        }
    }
    scan
}

/// The values of `id="…"` and `name="…"` attributes in a fragment of HTML.
fn html_anchors(html: &str) -> Vec<String> {
    let mut found = Vec::new();
    for attribute in ["id=", "name="] {
        let mut rest = html;
        while let Some(at) = rest.find(attribute) {
            let preceded_by_space = rest
                .get(..at)
                .and_then(|before| before.chars().next_back())
                .is_some_and(char::is_whitespace);
            rest = rest.get(at + attribute.len()..).unwrap_or_default();
            let Some(quote) = rest.chars().next().filter(|c| matches!(c, '"' | '\'')) else {
                continue;
            };
            let value = rest.get(1..).unwrap_or_default();
            if let (true, Some(end)) = (preceded_by_space, value.find(quote)) {
                found.push(value.get(..end).unwrap_or_default().to_owned());
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{BrokenLink, Reason, check_files, is_excluded, is_external, percent_decode, resolve, scan};

    fn anchors(text: &str) -> Vec<String> {
        let mut anchors: Vec<_> = scan(text).anchors.into_iter().collect();
        anchors.sort();
        anchors
    }

    fn targets(text: &str) -> Vec<String> {
        scan(text).links.into_iter().map(|l| l.target).collect()
    }

    #[test]
    fn inline_reference_and_image_links_are_found() {
        let text = "[a](a.md) ![i](img.png)\n\n[ref][r]\n\n[r]: ref.md#x\n";
        assert_eq!(targets(text), ["a.md", "img.png", "ref.md#x"]);
    }

    #[test]
    fn links_in_code_spans_and_blocks_are_ignored() {
        let text = "`[a](missing.md)`\n\n```md\n[b](missing.md)\n```\n\n    [c](missing.md)\n";
        assert_eq!(targets(text), Vec::<String>::new());
    }

    #[test]
    fn autolinks_are_ignored() {
        assert_eq!(
            targets("<https://example.com> <me@example.com>\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn links_in_tables_report_their_line() {
        let text = "# T\n\n| a | b |\n| --- | --- |\n| x | [y](y.md) |\n";
        let lines: Vec<_> = scan(text).links.iter().map(|l| l.line).collect();
        assert_eq!(lines, [5]);
    }

    #[test]
    fn heading_slugs_use_inline_code_and_link_text() {
        assert_eq!(
            anchors("## The `Limits` type\n\n## See [ADR 0003](x.md)\n"),
            ["see-adr-0003", "the-limits-type"]
        );
    }

    #[test]
    fn duplicate_headings_get_suffixes() {
        assert_eq!(
            anchors("# Tests\n\n## Tests\n\nTests\n---\n"),
            ["tests", "tests-1", "tests-2"]
        );
    }

    #[test]
    fn html_ids_and_names_are_anchors() {
        assert_eq!(
            anchors("<a id=\"custom\"></a>\n\nText <a name='old-name'></a>.\n"),
            ["custom", "old-name"]
        );
    }

    #[test]
    fn data_id_attributes_are_not_anchors() {
        assert_eq!(anchors("<div data-id=\"x\"></div>\n"), Vec::<String>::new());
    }

    #[test]
    fn urls_with_a_scheme_are_external() {
        assert!(
            ["https://x.org", "mailto:a@b.c", "//cdn.x.org/y"]
                .iter()
                .all(|t| is_external(t))
        );
    }

    #[test]
    fn relative_paths_are_not_external() {
        assert!(!["a.md", "../b.md#c", "#d", "dir/e:f.md"].iter().any(|t| is_external(t)));
    }

    #[test]
    fn resolve_climbs_parent_directories() {
        assert_eq!(
            resolve(Path::new("docs/todos/ready/A.md"), "../../plans/x.md"),
            Some(PathBuf::from("docs/plans/x.md"))
        );
    }

    #[test]
    fn resolve_treats_a_leading_slash_as_the_root() {
        assert_eq!(
            resolve(Path::new("docs/a.md"), "/CLAUDE.md"),
            Some(PathBuf::from("CLAUDE.md"))
        );
    }

    #[test]
    fn resolve_rejects_paths_outside_the_root() {
        assert_eq!(resolve(Path::new("a.md"), "../outside.md"), None);
    }

    #[test]
    fn percent_escapes_are_decoded() {
        assert_eq!(percent_decode("my%20file.md"), "my file.md");
    }

    #[test]
    fn invalid_percent_escapes_are_kept() {
        assert_eq!(percent_decode("100%zz"), "100%zz");
    }

    #[test]
    fn build_output_is_excluded() {
        assert!(is_excluded("spikes/pdf-basics/rust/target/doc/x.md") && is_excluded("a/node_modules/b/README.md"));
    }

    #[test]
    fn docs_are_not_excluded() {
        assert!(!is_excluded("docs/targets.md"));
    }

    /// A scratch directory of Markdown files, removed on drop.
    struct Tree(PathBuf);

    impl Tree {
        fn new(name: &str, files: &[(&str, &str)]) -> Self {
            let root = std::env::temp_dir().join(format!("lamina-xtask-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for (path, text) in files {
                let path = root.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, text).unwrap();
            }
            Self(root)
        }

        fn check(&self, files: &[&str]) -> Vec<BrokenLink> {
            let files: Vec<PathBuf> = files.iter().map(PathBuf::from).collect();
            check_files(&self.0, &files).unwrap()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn broken(file: &str, line: usize, target: &str, reason: Reason, hint: Option<&str>) -> BrokenLink {
        BrokenLink {
            file: file.to_owned(),
            line,
            target: target.to_owned(),
            reason,
            hint: hint.map(str::to_owned),
        }
    }

    #[test]
    fn valid_links_pass() {
        let tree = Tree::new(
            "valid",
            &[
                (
                    "README.md",
                    "# Top\n\n[guide](docs/guide.md#2-setup) [dir](docs) [self](#top)\n",
                ),
                (
                    "docs/guide.md",
                    "# Guide\n\n## 2. Setup\n\n[back](../README.md) [web](https://x.org)\n",
                ),
            ],
        );
        assert_eq!(tree.check(&["README.md", "docs/guide.md"]), []);
    }

    #[test]
    fn missing_file_is_reported_with_its_line() {
        let tree = Tree::new("missing-file", &[("a.md", "# A\n\nSee [b](b.md).\n")]);
        assert_eq!(
            tree.check(&["a.md"]),
            [broken("a.md", 3, "b.md", Reason::MissingFile, None)]
        );
    }

    #[test]
    fn missing_anchor_is_reported() {
        let tree = Tree::new(
            "missing-anchor",
            &[("a.md", "# A\n\n[b](b.md#nope)\n"), ("b.md", "# B\n")],
        );
        assert_eq!(
            tree.check(&["a.md"]),
            [broken("a.md", 3, "b.md#nope", Reason::MissingAnchor, None)]
        );
    }

    #[test]
    fn missing_same_file_anchor_is_reported() {
        let tree = Tree::new("same-file-anchor", &[("a.md", "# A\n\n[x](#b)\n")]);
        assert_eq!(
            tree.check(&["a.md"]),
            [broken("a.md", 3, "#b", Reason::MissingAnchor, None)]
        );
    }

    #[test]
    fn moved_todo_gets_a_hint() {
        let tree = Tree::new(
            "moved-todo",
            &[
                ("docs/plans/p.md", "[X1](../todos/ongoing/X1-a.md)\n"),
                ("docs/todos/done/X1-a.md", "# X1\n"),
            ],
        );
        assert_eq!(
            tree.check(&["docs/plans/p.md"]),
            [broken(
                "docs/plans/p.md",
                1,
                "../todos/ongoing/X1-a.md",
                Reason::MissingFile,
                Some("../todos/done/X1-a.md")
            )]
        );
    }

    #[test]
    fn broken_link_message_names_file_line_target_and_reason() {
        let link = broken("a.md", 3, "b.md#x", Reason::MissingAnchor, None);
        assert_eq!(link.to_string(), "a.md:3: broken link b.md#x (missing anchor)");
    }
}
