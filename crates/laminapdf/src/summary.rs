//! The comment summary: a new PDF/UA-1 document listing a review's comments, each page drawn
//! small beside its comments with their numbers marked on it, or the comments alone as a list.
//! It changes nothing in the document it summarises.

use laminapdf_build::flow::{
    Figure, Flow, FlowBlock, List, ListItem, ListKind, Margins, PageRegion, PageTemplate, Paragraph, ParagraphStyle,
    TextStyle,
};
use laminapdf_build::{BuildOptions, Builder, Color, FontHandle, Metadata, PageForm, Profile};
use laminapdf_model::{AnnotId, PageSize, PdfDate, PointPt};

use crate::comments::{CommentPart, PageComment};
use crate::document::Document;
use crate::error::Result;
use crate::{Font, Warning};

/// How a comment summary sets out the comments.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SummaryLayout {
    /// Each page drawn small at the left, its comments' numbers marked on it, and its comments
    /// in the column beside it.
    #[default]
    BesidePage,
    /// Each page's comments under its heading, without the page.
    List,
}

/// The paper a comment summary is printed on.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum SummaryPaper {
    /// The size of the document's first page, as it displays.
    #[default]
    SameAsDocument,
    /// This size.
    Size(PageSize),
}

/// What a comment summary shows and how: its layout, which comments, whether it lists their
/// replies and statuses and the pages without comments, its paper, title, language and fonts.
///
/// Added in 0.6.0.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct SummaryOptions {
    /// Beside each page (the default) or as a list.
    pub layout: SummaryLayout,
    /// The comments to list, by ID; every comment when `None`.
    pub comments: Option<Vec<AnnotId>>,
    /// Whether each comment's replies are listed under it (yes by default).
    pub replies: bool,
    /// Whether each comment's latest review status and check mark are listed (yes by default).
    pub status: bool,
    /// Whether pages without comments are listed too, each saying it has none (no by default).
    pub empty_pages: bool,
    /// The paper (the document's own size by default).
    pub paper: SummaryPaper,
    /// The summary's title; "Comments on" and the document's title by default, or "Comment
    /// summary" when it has none.
    pub title: Option<String>,
    /// The language of its own text, as a BCP 47 tag (`en` by default).
    pub language: String,
    /// The font its text is set in, embedded and subset.
    pub font: Font,
    /// Fonts for the characters [`SummaryOptions::font`] lacks, tried in order.
    pub fallbacks: Vec<Font>,
}

impl SummaryOptions {
    /// Every comment beside its page, with its replies and status, on the document's paper,
    /// set in `font`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn new(font: Font) -> Self {
        Self {
            layout: SummaryLayout::BesidePage,
            comments: None,
            replies: true,
            status: true,
            empty_pages: false,
            paper: SummaryPaper::SameAsDocument,
            title: None,
            language: "en".to_owned(),
            font,
            fallbacks: Vec::new(),
        }
    }

    /// Set out in `layout`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_layout(mut self, layout: SummaryLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Listing only the comments `ids`, such as those a comments panel shows.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_comments(mut self, ids: Vec<AnnotId>) -> Self {
        self.comments = Some(ids);
        self
    }

    /// With or without each comment's replies.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_replies(mut self, replies: bool) -> Self {
        self.replies = replies;
        self
    }

    /// With or without each comment's status.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_status(mut self, status: bool) -> Self {
        self.status = status;
        self
    }

    /// With or without the pages that have no comments.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_empty_pages(mut self, empty_pages: bool) -> Self {
        self.empty_pages = empty_pages;
        self
    }

    /// On `paper`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn with_paper(mut self, paper: SummaryPaper) -> Self {
        self.paper = paper;
        self
    }

    /// Titled `title`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// With its own text in `language`, a BCP 47 tag such as `fr`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// With `font` for the characters the main font lacks, after any fallbacks given before.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_fallback(mut self, font: Font) -> Self {
        self.fallbacks.push(font);
        self
    }
}

/// The share of the body's width a page beside its comments takes.
const PAGE_SHARE: f64 = 0.42;

/// The text styles a summary uses.
struct Styles {
    body: TextStyle,
    small: TextStyle,
    heading: TextStyle,
    title: TextStyle,
    mark: TextStyle,
}

impl Styles {
    fn new(builder: &mut Builder, options: &SummaryOptions) -> Result<Self> {
        let handle: FontHandle = builder.font(options.font.clone())?;
        let mut body = TextStyle::new(&handle, 10.0);
        for fallback in &options.fallbacks {
            body = body.with_fallback(&builder.font(fallback.clone())?);
        }
        let body = body.with_language(options.language.clone());
        Ok(Self {
            small: body.clone().with_size(8.5).with_color(Color::gray(0.33)),
            heading: body.clone().with_size(13.0),
            title: body.clone().with_size(18.0),
            mark: body.clone().with_size(7.0).with_color(Color::rgb(0.11, 0.37, 0.75)),
            body,
        })
    }
}

impl Document {
    /// A new document listing the comments as `options` asks, tagged and conforming to PDF/UA-1:
    /// a heading per page, then each comment with its number, kind, author, date, status, the
    /// text it marks, its note and its replies. Beside its page, the default, draws each page
    /// small at the left with the comments' numbers marked on it; the page is copied as a Form
    /// XObject, so it stays vector and its text stays text, and it is a `Figure` whose
    /// alternative text names it. Comments are numbered in document order. This document
    /// doesn't change; the summary is unsaved.
    ///
    /// A page that uses a font the file doesn't embed isn't drawn, since PDF/UA-1 needs every
    /// font embedded; its comments are listed, and the summary's warnings say so
    /// (`summary-page-not-drawn`).
    ///
    /// ```
    /// # fn font() -> Vec<u8> { include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/fonts/NotoSans-Regular.ttf")).to_vec() }
    /// use laminapdf::{BuildOptions, Builder, Font, NoteIcon, Op, PageSize, PointPt, Pt, SummaryOptions};
    ///
    /// let mut b = Builder::new(BuildOptions::new().with_id(*b"comment-summary1"));
    /// b.page(PageSize::A4).finish()?;
    /// let mut doc = b.finish()?;
    /// let note = doc.note_at(0, PointPt::new(Pt::new(56.0), Pt::new(56.0)), "Check this", NoteIcon::Comment)?;
    /// doc.apply(vec![Op::from(note)])?;
    ///
    /// let summary = doc.comment_summary(&SummaryOptions::new(Font::from_bytes(font())?))?;
    /// assert!(summary.page_count()? >= 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `missing-glyph` for text no font can draw; `limit-exceeded` for a page whose content or
    /// resources pass the document's limits; reading errors.
    ///
    /// Added in 0.6.0.
    pub fn comment_summary(&self, options: &SummaryOptions) -> Result<Self> {
        let threads: Vec<PageComment> = self
            .comment_threads(None)?
            .into_iter()
            .filter(|t| {
                options
                    .comments
                    .as_ref()
                    .is_none_or(|ids| ids.iter().any(|id| id.to_string() == t.comment.id))
            })
            .collect();
        let mut builder = Builder::new(BuildOptions::new().with_profile(Profile::PdfUa1));
        let styles = Styles::new(&mut builder, options)?;
        let title = match (&options.title, self.info()?.title) {
            (Some(title), _) => title.clone(),
            (None, Some(name)) if !name.trim().is_empty() => format!("Comments on {name}"),
            (None, _) => "Comment summary".to_owned(),
        };
        let pages = self.summary_pages(&threads, options)?;
        let mut warnings = Vec::new();
        let forms = match options.layout {
            SummaryLayout::BesidePage => self.drawable_forms(&mut builder, &pages, &mut warnings)?,
            SummaryLayout::List => vec![None; pages.len()],
        };
        let size = self.summary_paper(options.paper)?;
        // An inch on A4 and Letter, less on smaller paper, so a small page keeps room for text.
        let margin = (size.width.get().min(size.height.get()) * 0.12).clamp(18.0, 72.0);
        let body_width = size.width.get() - 2.0 * margin;
        let body_height = size.height.get() - 2.0 * margin;
        let footer = PageRegion::new().with_center("{page} / {pages}");
        let template = PageTemplate::new(size)
            .with_margins(Margins::new(margin, margin, margin, margin))
            .with_furniture(None, Some(footer), &styles.small);
        let mut flow = Flow::new(template)
            .with_title(title.clone())
            .with_outline(false)
            .block(FlowBlock::Heading(1, Paragraph::plain(title.clone(), &styles.title)))
            .block(FlowBlock::Paragraph(Paragraph::plain(
                count_line(threads.len(), pages.iter().filter(|p| !p.threads.is_empty()).count()),
                &styles.body,
            )));
        let mut number = 1;
        for (page, form) in pages.iter().zip(forms) {
            let heading = FlowBlock::Heading(2, Paragraph::plain(format!("Page {}", page.label), &styles.heading));
            let mut blocks = Vec::new();
            if page.threads.is_empty() {
                blocks.push(FlowBlock::Paragraph(Paragraph::plain(
                    "No comments on this page.",
                    &styles.body,
                )));
            }
            let first = number;
            let mut marks = Vec::new();
            for thread in &page.threads {
                if let Some(rect) = thread.comment.rect {
                    marks.push((PointPt::new(rect.x0, rect.y0), number.to_string()));
                }
                blocks.extend(comment_blocks(thread, number, options, &styles));
                number += 1;
            }
            flow = flow.block(heading);
            match form {
                Some(form) => {
                    let width =
                        (body_width * PAGE_SHARE).min(body_height * 0.9 * form.width() / form.height().max(1.0));
                    let alt = figure_alt(&page.label, first, number - 1);
                    let figure = marks
                        .into_iter()
                        .fold(Figure::page(form, alt).with_width(width), |f, (at, label)| {
                            f.with_mark(at, label, &styles.mark)
                        });
                    flow = flow.block(FlowBlock::Beside(figure, blocks));
                }
                None => flow = blocks.into_iter().fold(flow, Flow::block),
            }
        }
        if threads.is_empty() && pages.is_empty() {
            flow = flow.block(FlowBlock::Paragraph(Paragraph::plain(
                "This document has no comments.",
                &styles.body,
            )));
        }
        builder.metadata(Metadata::default().with_title(title));
        builder.lang(options.language.clone());
        let report = builder.flow(&flow)?;
        warnings.extend(report.warnings.into_iter().map(Warning::from));
        let (doc, fonts) = builder.finish_with_warnings()?;
        warnings.extend(fonts.into_iter().map(Warning::from));
        Ok(Self::built(doc, warnings))
    }

    /// The pages a summary lists, in order, each with its threads and label.
    fn summary_pages<'t>(&self, threads: &'t [PageComment], options: &SummaryOptions) -> Result<Vec<SummaryPage<'t>>> {
        let all = self.ops().pages()?;
        let mut out = Vec::new();
        for page in all.iter() {
            let index = u32::try_from(page.index()).unwrap_or(u32::MAX);
            let on: Vec<&PageComment> = threads.iter().filter(|t| t.page == index).collect();
            if on.is_empty() && !options.empty_pages {
                continue;
            }
            let label = page.label()?.unwrap_or_else(|| (page.index() + 1).to_string());
            out.push(SummaryPage {
                index,
                label,
                threads: on,
            });
        }
        Ok(out)
    }

    /// Each page copied as a form, or `None` for one whose fonts aren't all embedded, which
    /// `warnings` then names.
    fn drawable_forms(
        &self,
        builder: &mut Builder,
        pages: &[SummaryPage<'_>],
        warnings: &mut Vec<Warning>,
    ) -> Result<Vec<Option<PageForm>>> {
        let all = self.ops().pages()?;
        let mut drawable = Vec::new();
        for page in pages {
            let embedded = match all.get(page.index as usize) {
                Some(p) => p.fonts()?.iter().all(|f| f.embedded),
                None => false,
            };
            if !embedded {
                warnings.push(Warning::Build(laminapdf_build::Warning::SummaryPageNotDrawn {
                    page: page.index,
                }));
            }
            drawable.push(embedded);
        }
        drop(all);
        let wanted: Vec<usize> = pages
            .iter()
            .zip(&drawable)
            .filter(|(_, d)| **d)
            .map(|(p, _)| p.index as usize)
            .collect();
        let mut forms = builder.page_forms(self.ops(), &wanted)?.into_iter();
        Ok(drawable
            .into_iter()
            .map(|d| if d { forms.next() } else { None })
            .collect())
    }

    /// The summary's page size.
    fn summary_paper(&self, paper: SummaryPaper) -> Result<PageSize> {
        if let SummaryPaper::Size(size) = paper {
            return Ok(size);
        }
        let pages = self.ops().pages()?;
        Ok(match pages.iter().next() {
            Some(page) => {
                let frame = page.frame()?;
                PageSize::new(frame.width(), frame.height())
            }
            None => PageSize::A4,
        })
    }
}

/// A page the summary lists.
struct SummaryPage<'t> {
    index: u32,
    label: String,
    threads: Vec<&'t PageComment>,
}

/// "3 comments on 2 pages."
fn count_line(comments: usize, pages: usize) -> String {
    let plural = |n: usize, one: &str, many: &str| {
        if n == 1 {
            format!("1 {one}")
        } else {
            format!("{n} {many}")
        }
    };
    format!(
        "{} on {}.",
        plural(comments, "comment", "comments"),
        plural(pages, "page", "pages")
    )
}

/// The drawn page's alternative text: which page, and which comments it marks.
fn figure_alt(label: &str, first: usize, last: usize) -> String {
    match last.checked_sub(first) {
        None => format!("Page {label}"),
        Some(0) => format!("Page {label}, with comment {first} marked"),
        Some(_) => format!("Page {label}, with comments {first} to {last} marked"),
    }
}

/// The blocks of comment `number`: its line, the text it marks, its note, the annotations
/// grouped with it, its status and its replies.
fn comment_blocks(thread: &PageComment, number: usize, options: &SummaryOptions, styles: &Styles) -> Vec<FlowBlock> {
    let tight = ParagraphStyle::new().with_space(0.0, 2.0);
    let comment = &thread.comment;
    let mut head = Paragraph::new(ParagraphStyle::new().with_space(8.0, 2.0)).text(format!("{number}. "), &styles.body);
    head = head.text(kind_name(&comment.subtype), &styles.body);
    if let Some(meta) = byline(comment.author.as_deref(), comment.created.as_deref()) {
        head = head.text(format!(" · {meta}"), &styles.small);
    }
    let mut blocks = vec![FlowBlock::Paragraph(head)];
    if let Some(quoted) = thread.quoted.as_deref().filter(|q| !q.is_empty()) {
        blocks.push(FlowBlock::Paragraph(
            Paragraph::plain(format!("“{quoted}”"), &styles.small).with_style(tight.clone()),
        ));
    }
    if let Some(note) = comment.contents.as_deref().filter(|n| !n.trim().is_empty()) {
        blocks.push(FlowBlock::Paragraph(
            Paragraph::plain(note, &styles.body).with_style(tight.clone()),
        ));
    }
    for part in &thread.group {
        let note = part.annotation.contents.as_deref().unwrap_or_default();
        let text = format!("{}: {note}", kind_name(&part.annotation.subtype));
        blocks.push(FlowBlock::Paragraph(
            Paragraph::plain(text, &styles.body).with_style(tight.clone()),
        ));
    }
    if options.status
        && let Some(status) = status_line(thread)
    {
        blocks.push(FlowBlock::Paragraph(
            Paragraph::plain(status, &styles.small).with_style(tight.clone()),
        ));
    }
    let replies: Vec<&CommentPart> = thread.replies.iter().filter(|r| r.annotation.state.is_none()).collect();
    if options.replies && !replies.is_empty() {
        let list = replies
            .into_iter()
            .fold(List::new(ListKind::Bullet, &styles.small), |list, reply| {
                let a = &reply.annotation;
                let mut p = Paragraph::new(tight.clone());
                if let Some(meta) = byline(a.author.as_deref(), a.created.as_deref()) {
                    p = p.text(format!("{meta}: "), &styles.small);
                }
                list.item(ListItem::new(
                    p.text(a.contents.clone().unwrap_or_default(), &styles.body),
                ))
            });
        blocks.push(FlowBlock::List(list));
    }
    blocks
}

/// "Ada, 2026-11-01 10:00", or the author or the date alone.
fn byline(author: Option<&str>, created: Option<&str>) -> Option<String> {
    let author = author.map(str::trim).filter(|a| !a.is_empty());
    let date = created.and_then(|d| d.parse::<PdfDate>().ok()).map(|d| {
        let iso = d.to_iso8601();
        let day = iso.get(..10).unwrap_or_default();
        let time = iso.get(11..16).unwrap_or_default();
        format!("{day} {time}")
    });
    match (author, date) {
        (Some(a), Some(d)) => Some(format!("{a}, {d}")),
        (Some(a), None) => Some(a.to_owned()),
        (None, Some(d)) => Some(d),
        (None, None) => None,
    }
}

/// "Status: Accepted · Marked", or `None` with neither set.
fn status_line(thread: &PageComment) -> Option<String> {
    let review = thread.review.as_deref().filter(|s| *s != "None");
    let marked = thread.marked.as_deref().filter(|s| *s == "Marked");
    match (review, marked) {
        (Some(r), Some(_)) => Some(format!("Status: {r} · Marked")),
        (Some(r), None) => Some(format!("Status: {r}")),
        (None, Some(_)) => Some("Marked".to_owned()),
        (None, None) => None,
    }
}

/// A subtype as a reader names it.
fn kind_name(subtype: &str) -> String {
    match subtype {
        "Text" => "Note",
        "FreeText" => "Text box",
        "StrikeOut" => "Strike-out",
        "Caret" => "Inserted text",
        "Ink" => "Drawing",
        "PolyLine" => "Polyline",
        "FileAttachment" => "File attachment",
        other => other,
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::{count_line, figure_alt, kind_name};

    #[test]
    fn the_count_line_says_one_or_many() {
        assert_eq!(count_line(1, 1), "1 comment on 1 page.");
        assert_eq!(count_line(5, 2), "5 comments on 2 pages.");
    }

    #[test]
    fn a_drawn_pages_alt_names_the_comments_it_marks() {
        assert_eq!(figure_alt("3", 4, 4), "Page 3, with comment 4 marked");
        assert_eq!(figure_alt("iv", 1, 3), "Page iv, with comments 1 to 3 marked");
        assert_eq!(figure_alt("2", 5, 4), "Page 2");
    }

    #[test]
    fn subtypes_get_their_readers_names() {
        assert_eq!(kind_name("Text"), "Note");
        assert_eq!(kind_name("Highlight"), "Highlight");
    }
}
