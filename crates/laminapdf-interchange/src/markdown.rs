//! Comments as Markdown for a notes app (C06): a heading per page, each highlight's text as a
//! block quote, each note as a paragraph, then who wrote it and when, its status and its
//! replies.

use std::fmt::Write as _;

/// One comment as Markdown writes it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownComment {
    /// Its page, counted from 0.
    pub page: u32,
    /// The text under it, for a highlight or another text markup.
    pub quote: Option<String>,
    /// Its note (`/Contents`).
    pub note: Option<String>,
    /// Who wrote it.
    pub author: Option<String>,
    /// When, as a PDF date (`D:20261101100000Z`).
    pub date: Option<String>,
    /// Its latest review status, such as `Accepted`.
    pub status: Option<String>,
    /// Its replies, in order.
    pub replies: Vec<MarkdownReply>,
}

impl MarkdownComment {
    /// A comment on `page` with nothing in it yet.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(page: u32) -> Self {
        Self {
            page,
            ..Self::default()
        }
    }
}

/// A reply as Markdown writes it.
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownReply {
    /// Who wrote it.
    pub author: Option<String>,
    /// What it says.
    pub text: String,
}

impl MarkdownReply {
    /// A reply saying `text`, by `author`.
    ///
    /// Added in 0.7.0.
    #[must_use]
    pub fn new(author: Option<String>, text: impl Into<String>) -> Self {
        Self {
            author,
            text: text.into(),
        }
    }
}

/// What Markdown includes besides the quotes and notes (C06's Include).
///
/// Added in 0.7.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MarkdownOptions {
    /// A heading for each page.
    pub headings: bool,
    /// Who wrote each comment, and when.
    pub authors: bool,
    /// The replies.
    pub replies: bool,
}

impl Default for MarkdownOptions {
    fn default() -> Self {
        Self {
            headings: true,
            authors: true,
            replies: true,
        }
    }
}

/// `comments` as Markdown, in the order given.
///
/// ```
/// use laminapdf_interchange::{MarkdownComment, MarkdownOptions, write_markdown};
///
/// let mut c = MarkdownComment::new(2);
/// c.quote = Some("Revenue rose 12%".to_owned());
/// c.note = Some("Check the source".to_owned());
/// let md = write_markdown(&[c], &MarkdownOptions::default());
/// assert_eq!(md, "## Page 3\n\n> Revenue rose 12%\n\nCheck the source\n");
/// ```
///
/// Added in 0.7.0.
#[must_use]
pub fn write_markdown(comments: &[MarkdownComment], options: &MarkdownOptions) -> String {
    let mut out = String::new();
    let mut page = None;
    for c in comments {
        if options.headings && page != Some(c.page) {
            page = Some(c.page);
            if !out.is_empty() {
                out.push('\n');
            }
            let _ = writeln!(out, "## Page {}", u64::from(c.page) + 1);
        }
        let mut parts: Vec<String> = Vec::new();
        if let Some(q) = c.quote.as_deref().filter(|q| !q.trim().is_empty()) {
            parts.push(q.lines().map(|l| format!("> {}", escaped(l))).collect::<Vec<_>>().join("\n"));
        }
        if let Some(n) = c.note.as_deref().filter(|n| !n.trim().is_empty()) {
            parts.push(n.lines().map(escaped).collect::<Vec<_>>().join("  \n"));
        }
        let byline = byline(c, *options);
        if !byline.is_empty() {
            parts.push(byline);
        }
        if options.replies && !c.replies.is_empty() {
            let replies: Vec<String> = c
                .replies
                .iter()
                .map(|r| match &r.author {
                    Some(a) => format!("- **{}:** {}", escaped(a), escaped(&r.text)),
                    None => format!("- {}", escaped(&r.text)),
                })
                .collect();
            parts.push(replies.join("\n"));
        }
        if parts.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&parts.join("\n\n"));
        out.push('\n');
    }
    out
}

/// "*Ada · 2026-11-01 · Accepted*", with whatever of the three is known and asked for.
fn byline(c: &MarkdownComment, options: MarkdownOptions) -> String {
    let mut bits: Vec<String> = Vec::new();
    if options.authors {
        bits.extend(c.author.as_deref().map(escaped));
        bits.extend(c.date.as_deref().and_then(day));
    }
    bits.extend(c.status.clone());
    if bits.is_empty() {
        String::new()
    } else {
        format!("*{}*", bits.join(" · "))
    }
}

/// A PDF date's day, `2026-11-01`.
fn day(pdf: &str) -> Option<String> {
    let digits = pdf.strip_prefix("D:").unwrap_or(pdf);
    let (y, m, d) = (digits.get(0..4)?, digits.get(4..6)?, digits.get(6..8)?);
    [y, m, d]
        .iter()
        .all(|p| p.bytes().all(|b| b.is_ascii_digit()))
        .then(|| format!("{y}-{m}-{d}"))
}

/// `text` with the characters Markdown would read as markup escaped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if matches!(ch, '\\' | '*' | '_' | '`' | '[' | ']' | '<' | '>' | '#' | '|') {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}
