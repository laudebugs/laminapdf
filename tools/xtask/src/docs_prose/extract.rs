//! Turns a docs page into what the prose rules read: its frontmatter and a list of blocks
//! (paragraphs, list items, headings, `<Snippet>` uses and everything else), each with its
//! starting line and its prose, in which code is a placeholder and links keep only their text.
//!
//! MDX is read as Markdown after blanking the lines Markdown can't parse (`import` and `export`
//! statements). Blanking keeps every line number, so a report points at the right line.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

/// The word that stands in for inline code: one word, never a glossary term or an ID.
pub(crate) const CODE: &str = "⟨code⟩";

/// What a page says about itself in its YAML frontmatter.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Frontmatter {
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    /// Glossary terms the page may use without linking them (`prose.allow`).
    pub(crate) allow: Vec<String>,
}

/// What kind of block a piece of prose is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A paragraph outside any list.
    Paragraph,
    /// A paragraph inside a list item.
    ListItem,
    /// A heading of this level (1–6).
    Heading(u8),
    /// A `<Snippet … />` component: tested example code.
    Snippet,
    /// A component that only wraps others (`<Tabs>`, `<TabItem>` and their closing tags).
    Wrapper,
    /// Anything else that ends the "opening paragraph": code blocks, tables, other components.
    Other,
}

/// One run of prose text, with the link it sits in (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Segment {
    pub(crate) text: String,
    pub(crate) link: Option<String>,
}

/// One block of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Block {
    pub(crate) kind: Kind,
    /// 1-based line where the block starts.
    pub(crate) line: usize,
    pub(crate) segments: Vec<Segment>,
}

impl Block {
    /// The block's prose as one string.
    pub(crate) fn text(&self) -> String {
        self.segments.iter().map(|segment| segment.text.as_str()).collect()
    }
}

/// A parsed page.
#[derive(Debug, Default)]
pub(crate) struct Page {
    /// `None` when the file has no frontmatter.
    pub(crate) frontmatter: Option<Frontmatter>,
    pub(crate) blocks: Vec<Block>,
}

/// Parses a Markdown or MDX page.
pub(crate) fn parse(source: &str) -> Page {
    let (frontmatter, body) = split_frontmatter(source);
    let body = blank_mdx_statements(&body);
    let line_starts = line_starts(&body);
    let mut walker = Walker::default();
    let parser = Parser::new_ext(&body, Options::ENABLE_TABLES).into_offset_iter();
    for (event, range) in parser {
        walker.event(event, line_of(&line_starts, range.start));
    }
    Page {
        frontmatter,
        blocks: walker.blocks,
    }
}

/// Splits off a leading `---` frontmatter block, replacing its lines with blank ones.
fn split_frontmatter(source: &str) -> (Option<Frontmatter>, String) {
    let mut lines = source.split_inclusive('\n');
    if lines.next().map(str::trim_end) != Some("---") {
        return (None, source.to_owned());
    }
    let mut yaml = Vec::new();
    let mut consumed = 1;
    for line in lines.by_ref() {
        consumed += 1;
        if line.trim_end() == "---" {
            let blank = "\n".repeat(consumed);
            let rest: String = lines.collect();
            return (Some(read_frontmatter(&yaml)), blank + &rest);
        }
        yaml.push(line.trim_end());
    }
    // No closing `---`: not frontmatter after all.
    (None, source.to_owned())
}

/// Reads the few frontmatter keys the rules need from simple YAML (`key: value` lines and a
/// `prose:` map holding an `allow:` list); the docs pages use nothing more complex.
fn read_frontmatter(lines: &[&str]) -> Frontmatter {
    let mut frontmatter = Frontmatter::default();
    let mut in_allow = false;
    for line in lines {
        let indented = line.starts_with(' ');
        let trimmed = line.trim();
        if in_allow {
            if let Some(item) = trimmed.strip_prefix("- ") {
                frontmatter.allow.push(unquote(item));
                continue;
            }
            in_allow = indented && trimmed.is_empty();
        }
        if !indented {
            if let Some(value) = trimmed.strip_prefix("title:") {
                frontmatter.title = Some(unquote(value));
            } else if let Some(value) = trimmed.strip_prefix("description:") {
                frontmatter.description = Some(unquote(value));
            }
        } else if trimmed == "allow:" {
            in_allow = true;
        }
    }
    frontmatter
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|inner| inner.strip_suffix('\'')))
        .unwrap_or(value)
        .to_owned()
}

/// Blanks MDX `import`/`export` lines (outside code fences) so Markdown doesn't read them as text.
fn blank_mdx_statements(body: &str) -> String {
    let mut in_fence = false;
    body.split_inclusive('\n')
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                in_fence = !in_fence;
            }
            let statement = trimmed.starts_with("import ") || trimmed.starts_with("export ");
            if !in_fence && statement {
                if line.ends_with('\n') { "\n" } else { "" }
            } else {
                line
            }
        })
        .collect()
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(index, _)| index + 1))
        .collect()
}

fn line_of(line_starts: &[usize], offset: usize) -> usize {
    line_starts.partition_point(|&start| start <= offset)
}

/// Walks pulldown-cmark's events and collects the blocks.
#[derive(Default)]
struct Walker {
    blocks: Vec<Block>,
    /// The paragraph or heading being read, if any.
    open: Option<Block>,
    /// Destinations of the links the walker is inside, innermost last.
    links: Vec<String>,
    list_depth: usize,
    /// Inside a table or a code block, where text isn't prose.
    skip_depth: usize,
}

impl Walker {
    fn event(&mut self, event: Event<'_>, line: usize) {
        match event {
            Event::Start(tag) => self.start(&tag, line),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.push_text(&text),
            Event::Code(_) => self.push_text(CODE),
            Event::SoftBreak | Event::HardBreak => self.push_text(" "),
            Event::Html(html) | Event::InlineHtml(html) => self.html(&html, line),
            _ => {}
        }
    }

    fn start(&mut self, tag: &Tag<'_>, line: usize) {
        // A new block (or a nested list) ends the one being read, so tight list items keep
        // their text when a nested list follows it.
        if matches!(tag, Tag::Paragraph | Tag::Heading { .. } | Tag::Item | Tag::List(_)) {
            self.close();
        }
        match tag {
            Tag::Paragraph if self.skip_depth == 0 => {
                let kind = if self.list_depth > 0 {
                    Kind::ListItem
                } else {
                    Kind::Paragraph
                };
                self.open = Some(Block {
                    kind,
                    line,
                    segments: Vec::new(),
                });
            }
            Tag::Heading { level, .. } if self.skip_depth == 0 => {
                let level = u8::try_from(*level as usize).unwrap_or(6);
                self.open = Some(Block {
                    kind: Kind::Heading(level),
                    line,
                    segments: Vec::new(),
                });
            }
            Tag::Link { dest_url, .. } => self.links.push(dest_url.to_string()),
            Tag::List(_) => self.list_depth += 1,
            Tag::Item if self.list_depth > 0 => {
                // A tight list item has text but no paragraph: open one for it.
                self.open = Some(Block {
                    kind: Kind::ListItem,
                    line,
                    segments: Vec::new(),
                });
            }
            Tag::CodeBlock(_) | Tag::Table(_) => {
                self.close();
                self.skip_depth += 1;
                self.blocks.push(Block {
                    kind: Kind::Other,
                    line,
                    segments: Vec::new(),
                });
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item => self.close(),
            TagEnd::Link => {
                self.links.pop();
            }
            TagEnd::List(_) => self.list_depth = self.list_depth.saturating_sub(1),
            TagEnd::CodeBlock | TagEnd::Table => self.skip_depth = self.skip_depth.saturating_sub(1),
            _ => {}
        }
    }

    fn push_text(&mut self, text: &str) {
        if self.skip_depth > 0 {
            return;
        }
        let link = self.links.last().cloned();
        if let Some(block) = self.open.as_mut() {
            match block.segments.last_mut() {
                Some(last) if last.link == link => last.text.push_str(text),
                _ => block.segments.push(Segment {
                    text: text.to_owned(),
                    link,
                }),
            }
        }
    }

    /// Components arrive as HTML: classify them; inline HTML inside prose is dropped.
    fn html(&mut self, html: &str, line: usize) {
        if self.open.is_some() {
            return;
        }
        let trimmed = html.trim_start();
        let kind = if trimmed.starts_with("<Snippet") {
            Kind::Snippet
        } else if ["<Tabs", "<TabItem", "</Tabs", "</TabItem"]
            .iter()
            .any(|tag| trimmed.starts_with(tag))
        {
            Kind::Wrapper
        } else if trimmed.starts_with("<!--") {
            return;
        } else {
            Kind::Other
        };
        self.blocks.push(Block {
            kind,
            line,
            segments: Vec::new(),
        });
    }

    fn close(&mut self) {
        if let Some(block) = self.open.take()
            && !block.text().trim().is_empty()
        {
            self.blocks.push(block);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(page: &Page) -> Vec<Kind> {
        page.blocks.iter().map(|block| block.kind).collect()
    }

    #[test]
    fn frontmatter_should_give_title_description_and_allowed_terms() {
        let page = parse(
            "---\ntitle: Fonts and text\ndescription: \"How fonts work.\"\nprose:\n  allow:\n    - CMap\n    - overlay\nsidebar:\n  order: 1\n---\n\nHello.\n",
        );
        let frontmatter = page.frontmatter.unwrap();
        assert_eq!(
            frontmatter,
            Frontmatter {
                title: Some("Fonts and text".into()),
                description: Some("How fonts work.".into()),
                allow: vec!["CMap".into(), "overlay".into()],
            }
        );
    }

    #[test]
    fn line_numbers_should_count_the_frontmatter_and_import_lines() {
        let page = parse("---\ntitle: T\n---\n\nimport X from \"y\";\n\nFirst paragraph.\n");
        assert_eq!(page.blocks.first().map(|block| block.line), Some(7));
    }

    #[test]
    fn inline_code_should_become_one_placeholder_word() {
        let page = parse("Call `init({ text: true })` first.\n");
        assert_eq!(page.blocks.first().unwrap().text(), format!("Call {CODE} first."));
    }

    #[test]
    fn link_text_should_keep_its_destination() {
        let page = parse("See the [overlay](/reference/glossary/#overlay).\n");
        let segments = &page.blocks.first().unwrap().segments;
        assert!(
            segments
                .iter()
                .any(|s| s.text == "overlay" && s.link.as_deref() == Some("/reference/glossary/#overlay"))
        );
    }

    #[test]
    fn tables_and_code_blocks_should_not_be_prose() {
        let page = parse("| a | b |\n| --- | --- |\n| x | y |\n\n```ts\nconst a = 1;\n```\n\nText.\n");
        assert_eq!(kinds(&page), vec![Kind::Other, Kind::Other, Kind::Paragraph]);
    }

    #[test]
    fn components_should_be_classified() {
        let page =
            parse("Intro:\n\n<Tabs>\n<TabItem label=\"TS\">\n\n<Snippet file=\"a.ts\" />\n\n</TabItem>\n</Tabs>\n");
        assert_eq!(
            kinds(&page),
            vec![
                Kind::Paragraph,
                Kind::Wrapper,
                Kind::Wrapper,
                Kind::Snippet,
                Kind::Wrapper,
                Kind::Wrapper
            ]
        );
    }

    #[test]
    fn list_items_and_headings_should_have_their_own_kinds() {
        let page = parse("## Two kinds\n\n- one item\n- two items\n");
        assert_eq!(kinds(&page), vec![Kind::Heading(2), Kind::ListItem, Kind::ListItem]);
    }
}
