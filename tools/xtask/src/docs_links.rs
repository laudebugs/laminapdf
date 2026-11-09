//! `cargo xtask docs-links`: every relative link and `#anchor` in the tracked Markdown resolves.
//!
//! Links are read with `pulldown-cmark` (inline, reference and image links; code spans, code
//! blocks and autolinks are not links). External URLs are not checked. A link's path is resolved
//! against the linking file (or the repository root for `/…`) and must be a tracked file or a
//! directory holding one, compared case-sensitively as GitHub does. A `#fragment` on a Markdown
//! target (or on the same file) must name a heading, using GitHub's slugs, or an HTML `id`/`name`.
//!
//! `cargo xtask docs-links --external` checks the `http(s)` URLs instead, over the network: the
//! nightly lane's job (see [`external`]).

pub(crate) mod cache;
pub(crate) mod external;
mod html;
mod slug;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result, bail};
use pulldown_cmark::{CowStr, Event, LinkType, Options, Parser, Tag, TagEnd};
use serde_json::json;

use self::slug::Slugger;
use crate::todos::State;
use crate::tracked::Tracked;

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
    /// A suggested replacement link (for a todo that moved to another state folder).
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
    let tracked = Tracked::from_git(root)?;
    let files = markdown_files(&tracked);
    let broken = check(root, &tracked, &files)?;
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

/// Tracked `*.md` files, without build output or installed packages.
pub(crate) fn markdown_files(tracked: &Tracked) -> Vec<PathBuf> {
    tracked
        .files()
        .filter(|path| is_markdown(path) && !is_excluded(path))
        .map(Path::to_path_buf)
        .collect()
}

fn is_markdown(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

/// Build output and installed packages are not documentation, even if something tracks them;
/// nor are Markdown test inputs (`fixtures/markdown/`, `fuzz/seeds/`), whose image links name
/// files the tests pass in.
fn is_excluded(path: &Path) -> bool {
    let parts: Vec<_> = path.iter().collect();
    parts.contains(&"node_modules".as_ref())
        || (parts.first() == Some(&"spikes".as_ref()) && parts.contains(&"target".as_ref()))
        || path.starts_with("fixtures/markdown")
        || path.starts_with("fuzz/seeds")
}

/// Checks the links in `files` (paths relative to `root`) against `tracked`; returns every
/// broken one. Each file is read and parsed once.
pub(crate) fn check(root: &Path, tracked: &Tracked, files: &[PathBuf]) -> Result<Vec<BrokenLink>> {
    let mut scans = BTreeMap::new();
    for file in files {
        let text = fs::read_to_string(root.join(file)).with_context(|| format!("reading {}", file.display()))?;
        scans.insert(file.as_path(), scan(&text));
    }
    let mut broken = Vec::new();
    for (file, doc) in &scans {
        for link in &doc.links {
            if let Some((reason, hint)) = check_link(tracked, &scans, file, &link.target) {
                broken.push(BrokenLink {
                    file: display(file),
                    line: link.line,
                    target: link.target.clone(),
                    reason,
                    hint,
                });
            }
        }
    }
    Ok(broken)
}

/// `None` when the link resolves, else why not (and a suggested link, if any).
fn check_link(
    tracked: &Tracked,
    scans: &BTreeMap<&Path, Scan>,
    from: &Path,
    target: &str,
) -> Option<(Reason, Option<String>)> {
    if is_external(target) {
        return None;
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
    let Some(resolved) = resolved.filter(|path| tracked.exists(path)) else {
        let hint = resolve(from, &percent_decode(path_part))
            .and_then(|missing| moved_todo_hint(tracked, from, &missing, path_part, fragment));
        return Some((Reason::MissingFile, hint));
    };
    let fragment = fragment.filter(|f| !f.is_empty())?;
    if !is_markdown(&resolved) || !tracked.is_file(&resolved) {
        // Fragments on source files (`#L10`) and directories are GitHub UI features, not headings.
        return None;
    }
    // A Markdown target outside the checked set (build output) has no known anchors to check.
    let anchors = &scans.get(resolved.as_path())?.anchors;
    (!anchors.contains(&percent_decode(fragment))).then_some((Reason::MissingAnchor, None))
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

/// For a missing `docs/todos/<state>/<file>`, a link from `from` to the state folder where that
/// file is tracked now, written the way the original was (`./`, `/`) and keeping its fragment.
fn moved_todo_hint(
    tracked: &Tracked,
    from: &Path,
    missing: &Path,
    written: &str,
    fragment: Option<&str>,
) -> Option<String> {
    let mut parts = missing.iter().map(|p| p.to_str());
    let (Some(Some("docs")), Some(Some("todos")), Some(Some(state)), Some(Some(file)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let moved = State::ALL
        .iter()
        .filter(|other| other.name() != state)
        .map(|other| Path::new("docs/todos").join(other.name()).join(file))
        .find(|candidate| tracked.is_file(candidate))?;
    let mut hint = if written.starts_with('/') {
        format!("/{}", display(&moved))
    } else {
        let relative = relative_path(from.parent().unwrap_or_else(|| Path::new("")), &moved);
        if written.starts_with("./") && !relative.starts_with("../") {
            format!("./{relative}")
        } else {
            relative
        }
    };
    if let Some(fragment) = fragment {
        hint.push('#');
        hint.push_str(fragment);
    }
    Some(hint)
}

/// The `/`-separated path from directory `from_dir` to `to` (both relative to the root).
fn relative_path(from_dir: &Path, to: &Path) -> String {
    let from: Vec<_> = from_dir.iter().collect();
    let to_parts: Vec<_> = to.iter().collect();
    let common = from.iter().zip(&to_parts).take_while(|(a, b)| a == b).count();
    let ups = std::iter::repeat_n("..".to_owned(), from.len().saturating_sub(common));
    let downs = to_parts.iter().skip(common).map(|p| p.to_string_lossy().into_owned());
    ups.chain(downs).collect::<Vec<_>>().join("/")
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
    crate::util::slash_path(path)
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
    /// Inline, reference and image links (not autolinks), relative or not.
    links: Vec<Link>,
    /// Every `http(s)` URL GitHub renders as a link: links, autolinks, bare URLs in text (GFM's
    /// autolink extension) and raw HTML `href`/`src`. What `--external` checks.
    external: Vec<Link>,
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
    let line_starts = crate::lines::starts(text);
    let line_of = |offset: usize| crate::lines::line_of(&line_starts, offset);

    let mut scan = Scan::default();
    let mut slugger = Slugger::default();
    let mut heading: Option<String> = None;
    // GitHub leaves image alt text out of heading anchors.
    let mut image_depth = 0usize;
    let mut html_block: Option<String> = None;
    // Bare URLs are links only outside links and code blocks. Consecutive text events are joined
    // first: the parser may split one run of text (and so one URL) at characters like `_`.
    let mut link_depth = 0usize;
    let mut code_block = false;
    let mut text_run: Option<(usize, String)> = None;
    let push_html = |scan: &mut Scan, html: &str, offset: usize| {
        for target in html::targets(html).into_iter().filter(|t| external::is_http(t)) {
            scan.external.push(Link {
                line: line_of(offset),
                target,
            });
        }
    };
    let push_link = |scan: &mut Scan, link_type: LinkType, dest: CowStr<'_>, offset: usize| {
        let line = line_of(offset);
        if external::is_http(&dest) {
            scan.external.push(Link {
                line,
                target: dest.to_string(),
            });
        }
        if !matches!(link_type, LinkType::Autolink | LinkType::Email) {
            scan.links.push(Link {
                line,
                target: dest.into_string(),
            });
        }
    };
    for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
        if let Event::Text(t) = &event
            && link_depth == 0
            && !code_block
        {
            text_run
                .get_or_insert_with(|| (range.start, String::new()))
                .1
                .push_str(t);
        } else if let Some((start, run)) = text_run.take() {
            // Line breaks are events of their own, so a run sits on one line.
            push_bare(&mut scan, line_of(start), &run);
        }
        match event {
            Event::Start(Tag::Link {
                link_type, dest_url, ..
            }) => {
                link_depth += 1;
                push_link(&mut scan, link_type, dest_url, range.start);
            }
            Event::End(TagEnd::Link) => link_depth = link_depth.saturating_sub(1),
            Event::Start(Tag::CodeBlock(_)) => code_block = true,
            Event::End(TagEnd::CodeBlock) => code_block = false,
            Event::Start(Tag::Image {
                link_type, dest_url, ..
            }) => {
                image_depth += 1;
                push_link(&mut scan, link_type, dest_url, range.start);
            }
            Event::End(TagEnd::Image) => image_depth = image_depth.saturating_sub(1),
            Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
            Event::End(TagEnd::Heading(_)) => {
                if let Some(text) = heading.take() {
                    scan.anchors.insert(slugger.next(&text));
                }
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(h) = heading.as_mut().filter(|_| image_depth == 0) {
                    h.push_str(&t);
                }
            }
            Event::Start(Tag::HtmlBlock) => html_block = Some(String::new()),
            Event::End(TagEnd::HtmlBlock) => {
                if let Some(block) = html_block.take() {
                    scan.anchors.extend(html::anchors(&block));
                }
            }
            Event::Html(part) => {
                push_html(&mut scan, &part, range.start);
                match html_block.as_mut() {
                    Some(block) => block.push_str(&part),
                    None => scan.anchors.extend(html::anchors(&part)),
                }
            }
            Event::InlineHtml(part) => {
                push_html(&mut scan, &part, range.start);
                scan.anchors.extend(html::anchors(&part));
            }
            _ => {}
        }
    }
    if let Some((start, run)) = text_run {
        push_bare(&mut scan, line_of(start), &run);
    }
    scan
}

/// Adds the bare URLs of one run of text on `line` to `scan.external`.
fn push_bare(scan: &mut Scan, line: usize, run: &str) {
    scan.external.extend(external::bare_urls(run).map(|url| Link {
        line,
        target: url.to_owned(),
    }));
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{BrokenLink, Reason, check, is_excluded, is_external, percent_decode, relative_path, resolve, scan};
    use crate::tracked::{TempTree, Tracked};

    fn anchors(text: &str) -> Vec<String> {
        scan(text).anchors.into_iter().collect()
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
    fn heading_slugs_leave_out_image_alt_text() {
        assert_eq!(anchors("## ![ci](badge.svg) Status\n"), ["-status"]);
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
    fn ids_in_multi_line_html_comments_are_not_anchors() {
        assert_eq!(anchors("<!--\n<a id=\"hidden\"></a>\n-->\n\n# Shown\n"), ["shown"]);
    }

    #[test]
    fn ids_in_multi_line_html_blocks_are_anchors() {
        assert_eq!(anchors("<div\n  ID = top>\n</div>\n"), ["top"]);
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
    fn relative_path_climbs_to_the_common_ancestor() {
        assert_eq!(
            relative_path(Path::new("docs/plans"), Path::new("docs/todos/done/X1.md")),
            "../todos/done/X1.md"
        );
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
        assert!(
            is_excluded(Path::new("spikes/pdf-basics/rust/target/doc/x.md"))
                && is_excluded(Path::new("a/node_modules/b/README.md"))
        );
    }

    #[test]
    fn markdown_test_inputs_are_excluded() {
        assert!(
            is_excluded(Path::new("fixtures/markdown/report.md"))
                && is_excluded(Path::new("fuzz/seeds/markdown/latin.md"))
                && !is_excluded(Path::new("fixtures/README.md"))
        );
    }

    #[test]
    fn docs_are_not_excluded() {
        assert!(!is_excluded(Path::new("docs/targets.md")));
    }

    /// Writes `files` to a temp tree; the first `tracked` of them are tracked, the rest are not.
    fn check_tree(name: &str, files: &[(&str, &str)], tracked: usize) -> Vec<BrokenLink> {
        let tree = TempTree::new(name, files);
        let tracked_paths: Vec<&str> = files.iter().take(tracked).map(|(path, _)| *path).collect();
        let markdown: Vec<PathBuf> = tracked_paths
            .iter()
            .filter(|path| super::is_markdown(Path::new(path)))
            .map(PathBuf::from)
            .collect();
        check(&tree.0, &Tracked::from_paths(tracked_paths), &markdown).unwrap()
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
        let files = [
            (
                "README.md",
                "# Top\n\n[guide](docs/guide.md#2-setup) [dir](docs) [self](#top)\n",
            ),
            (
                "docs/guide.md",
                "# Guide\n\n## 2. Setup\n\n[back](../README.md) [web](https://x.org)\n",
            ),
        ];
        assert_eq!(check_tree("valid", &files, 2), []);
    }

    #[test]
    fn missing_file_is_reported_with_its_line() {
        let files = [("a.md", "# A\n\nSee [b](b.md).\n")];
        assert_eq!(
            check_tree("missing-file", &files, 1),
            [broken("a.md", 3, "b.md", Reason::MissingFile, None)]
        );
    }

    #[test]
    fn wrong_case_link_is_reported() {
        let files = [("a.md", "[b](B.md)\n"), ("b.md", "# B\n")];
        assert_eq!(
            check_tree("wrong-case", &files, 2),
            [broken("a.md", 1, "B.md", Reason::MissingFile, None)]
        );
    }

    #[test]
    fn link_to_an_untracked_file_is_reported() {
        let files = [("a.md", "[draft](draft.md)\n"), ("draft.md", "# Draft\n")];
        assert_eq!(
            check_tree("untracked", &files, 1),
            [broken("a.md", 1, "draft.md", Reason::MissingFile, None)]
        );
    }

    #[test]
    fn missing_anchor_is_reported() {
        let files = [("a.md", "# A\n\n[b](b.md#nope)\n"), ("b.md", "# B\n")];
        assert_eq!(
            check_tree("missing-anchor", &files, 2),
            [broken("a.md", 3, "b.md#nope", Reason::MissingAnchor, None)]
        );
    }

    #[test]
    fn missing_same_file_anchor_is_reported() {
        let files = [("a.md", "# A\n\n[x](#b)\n")];
        assert_eq!(
            check_tree("same-file-anchor", &files, 1),
            [broken("a.md", 3, "#b", Reason::MissingAnchor, None)]
        );
    }

    #[test]
    fn moved_todo_hint_keeps_the_fragment() {
        let files = [
            ("docs/plans/p.md", "[X1](../todos/ongoing/X1-a.md#goal)\n"),
            ("docs/todos/done/X1-a.md", "# X1\n\n## Goal\n"),
        ];
        assert_eq!(
            check_tree("moved-todo", &files, 2),
            [broken(
                "docs/plans/p.md",
                1,
                "../todos/ongoing/X1-a.md#goal",
                Reason::MissingFile,
                Some("../todos/done/X1-a.md#goal")
            )]
        );
    }

    #[test]
    fn moved_todo_hint_from_the_same_folder_points_across_folders() {
        let files = [
            ("docs/todos/ready/B1-b.md", "[A1](./A1-a.md)\n"),
            ("docs/todos/done/A1-a.md", "# A1\n"),
        ];
        assert_eq!(
            check_tree("moved-todo-same-folder", &files, 2),
            [broken(
                "docs/todos/ready/B1-b.md",
                1,
                "./A1-a.md",
                Reason::MissingFile,
                Some("../done/A1-a.md")
            )]
        );
    }

    #[test]
    fn broken_link_message_names_file_line_target_and_reason() {
        let link = broken("a.md", 3, "b.md#x", Reason::MissingAnchor, None);
        assert_eq!(link.to_string(), "a.md:3: broken link b.md#x (missing anchor)");
    }
}
