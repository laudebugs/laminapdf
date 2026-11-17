//! The words a comment summary writes itself, which a caller writing in another language replaces:
//! templates with named placeholders, and the names of annotation kinds and review states.

use std::collections::BTreeMap;

/// The words a comment summary writes itself, in English by default.
///
/// They are its default title, the line counting the comments, each page's heading, the
/// alternative text of each page drawn beside its comments, the status line, the quotation
/// marks round the text a comment marks, and the names of annotation kinds and review states. The
/// comments' own text, their authors and the ISO 8601 dates are never translated.
///
/// Each word is a template: the summary fills in the names in braces, such as `{label}`, and a
/// template may leave one out. Comments and pages each take two forms, one and other; a
/// language with more plural forms can phrase the count line to avoid them, such as
/// "Comments: {count}".
///
/// ```
/// use laminapdf::SummaryWords;
///
/// let words = SummaryWords::default()
///     .with_page("Seite {label}")
///     .with_status("Stand: {status}")
///     .with_kind("Text", "Notiz");
/// assert_eq!(words.page, "Seite {label}");
/// ```
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SummaryWords {
    /// The title of a document that has one: "Comments on {title}".
    pub titled: String,
    /// The title of a document that has none: "Comment summary".
    pub untitled: String,
    /// The line under the title when the document has no comments.
    pub no_comments: String,
    /// The line under the title when none of the chosen comments are in the document.
    pub none_chosen: String,
    /// The line under the title counting the comments and their pages: "{comments} on {pages}.",
    /// built from `comments_one` or `comments_other` and `pages_one` or `pages_other`.
    pub count: String,
    /// One comment: "1 comment".
    pub comments_one: String,
    /// Several comments: "{count} comments".
    pub comments_other: String,
    /// One page: "1 page".
    pub pages_one: String,
    /// Several pages: "{count} pages".
    pub pages_other: String,
    /// Each page's heading: "Page {label}". It is also the alternative text of a drawn page that
    /// marks no comment, where it may take `{first}` and `{last}` too.
    pub page: String,
    /// The line under a page without comments, listed when the summary lists those pages.
    pub page_empty: String,
    /// A drawn page's alternative text when it marks one comment:
    /// "Page {label}, with comment {first} marked".
    pub figure_one: String,
    /// A drawn page's alternative text when it marks several:
    /// "Page {label}, with comments {first} to {last} marked".
    pub figure_many: String,
    /// The status line: "Status: {status}".
    pub status: String,
    /// The check mark, after the status or alone: "Marked".
    pub marked: String,
    /// The text a comment marks, quoted under it: "“{text}”".
    pub quote: String,
    /// Annotation kinds by their PDF subtype: every kind a review holds, `Text` "Note", `Ink`
    /// "Drawing" and the rest. A subtype missing here is written as the file names it.
    pub kinds: BTreeMap<String, String>,
    /// Review states by their PDF name: `Accepted`, `Rejected`, `Cancelled` and `Completed`, each
    /// its own English name by default. A state missing here is written as the file names it.
    pub states: BTreeMap<String, String>,
}

/// Every kind a review holds (ISO 32000-2 Table 171's markup annotations), with its English name.
const KINDS: &[(&str, &str)] = &[
    ("Text", "Note"),
    ("FreeText", "Text box"),
    ("Line", "Line"),
    ("Square", "Rectangle"),
    ("Circle", "Ellipse"),
    ("Polygon", "Polygon"),
    ("PolyLine", "Polyline"),
    ("Highlight", "Highlight"),
    ("Underline", "Underline"),
    ("Squiggly", "Squiggly underline"),
    ("StrikeOut", "Strike-out"),
    ("Caret", "Inserted text"),
    ("Stamp", "Stamp"),
    ("Ink", "Drawing"),
    ("FileAttachment", "File attachment"),
    ("Sound", "Sound"),
    ("Redact", "Redaction"),
];

/// The review states of ISO 32000-2 Table 175's Review model, each its own English name.
const STATES: &[&str] = &["Accepted", "Rejected", "Cancelled", "Completed"];

impl Default for SummaryWords {
    fn default() -> Self {
        Self {
            titled: "Comments on {title}".to_owned(),
            untitled: "Comment summary".to_owned(),
            no_comments: "This document has no comments.".to_owned(),
            none_chosen: "None of the chosen comments are in this document.".to_owned(),
            count: "{comments} on {pages}.".to_owned(),
            comments_one: "1 comment".to_owned(),
            comments_other: "{count} comments".to_owned(),
            pages_one: "1 page".to_owned(),
            pages_other: "{count} pages".to_owned(),
            page: "Page {label}".to_owned(),
            page_empty: "No comments on this page.".to_owned(),
            figure_one: "Page {label}, with comment {first} marked".to_owned(),
            figure_many: "Page {label}, with comments {first} to {last} marked".to_owned(),
            status: "Status: {status}".to_owned(),
            marked: "Marked".to_owned(),
            quote: "“{text}”".to_owned(),
            kinds: KINDS.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect(),
            states: STATES.iter().map(|s| ((*s).to_owned(), (*s).to_owned())).collect(),
        }
    }
}

/// A `with_` builder for each template, which sets that field.
macro_rules! template_builders {
    ($($method:ident => $field:ident),* $(,)?) => {
        impl SummaryWords {
            $(
                #[doc = concat!("With [`SummaryWords::", stringify!($field), "`] set to `template`.")]
                ///
                /// Added in 0.6.0.
                #[must_use]
                pub fn $method(mut self, template: impl Into<String>) -> Self {
                    self.$field = template.into();
                    self
                }
            )*
        }
    };
}

template_builders! {
    with_titled => titled,
    with_untitled => untitled,
    with_no_comments => no_comments,
    with_none_chosen => none_chosen,
    with_count => count,
    with_comments_one => comments_one,
    with_comments_other => comments_other,
    with_pages_one => pages_one,
    with_pages_other => pages_other,
    with_page => page,
    with_page_empty => page_empty,
    with_figure_one => figure_one,
    with_figure_many => figure_many,
    with_status => status,
    with_marked => marked,
    with_quote => quote,
}

impl SummaryWords {
    /// With the annotation kind `subtype` (`Text`, `Ink`…) named `name`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_kind(mut self, subtype: impl Into<String>, name: impl Into<String>) -> Self {
        self.kinds.insert(subtype.into(), name.into());
        self
    }

    /// With the review state `state` (`Accepted`…) named `name`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_state(mut self, state: impl Into<String>, name: impl Into<String>) -> Self {
        self.states.insert(state.into(), name.into());
        self
    }

    /// The summary's title: `titled` with the document's, or `untitled`.
    pub(crate) fn title(&self, document: Option<&str>) -> String {
        match document.filter(|t| !t.trim().is_empty()) {
            Some(title) => fill(&self.titled, &[("title", title)]),
            None => self.untitled.clone(),
        }
    }

    /// "3 comments on 2 pages."
    pub(crate) fn count_line(&self, comments: usize, pages: usize) -> String {
        let form = |n: usize, one: &str, other: &str| {
            if n == 1 {
                fill(one, &[("count", "1")])
            } else {
                fill(other, &[("count", &n.to_string())])
            }
        };
        fill(
            &self.count,
            &[
                ("comments", &form(comments, &self.comments_one, &self.comments_other)),
                ("pages", &form(pages, &self.pages_one, &self.pages_other)),
            ],
        )
    }

    /// A page's heading.
    pub(crate) fn page_heading(&self, label: &str) -> String {
        fill(&self.page, &[("label", label)])
    }

    /// The drawn page's alternative text: which page, and which comments it marks.
    pub(crate) fn figure_alt(&self, label: &str, first: usize, last: usize) -> String {
        let (first_s, last_s) = (first.to_string(), last.to_string());
        let values = [("label", label), ("first", first_s.as_str()), ("last", last_s.as_str())];
        match last.checked_sub(first) {
            None => fill(&self.page, &values),
            Some(0) => fill(&self.figure_one, &values),
            Some(_) => fill(&self.figure_many, &values),
        }
    }

    /// "Status: Accepted · Marked", or `None` with neither set.
    pub(crate) fn status_line(&self, review: Option<&str>, marked: bool) -> Option<String> {
        let review = review.map(|r| fill(&self.status, &[("status", self.state_name(r))]));
        match (review, marked) {
            (Some(r), true) => Some(format!("{r} · {}", self.marked)),
            (Some(r), false) => Some(r),
            (None, true) => Some(self.marked.clone()),
            (None, false) => None,
        }
    }

    /// The text a comment marks, quoted.
    pub(crate) fn quoted(&self, text: &str) -> String {
        fill(&self.quote, &[("text", text)])
    }

    /// A subtype as a reader names it.
    pub(crate) fn kind_name(&self, subtype: &str) -> String {
        self.kinds.get(subtype).cloned().unwrap_or_else(|| subtype.to_owned())
    }

    /// A review state as a reader names it.
    fn state_name<'s>(&'s self, state: &'s str) -> &'s str {
        self.states.get(state).map_or(state, String::as_str)
    }
}

/// `template` with each `{name}` of `values` replaced, in one pass over the template, so a value
/// (a page label from the document, say) is never read as a placeholder; any other braces stay
/// as written.
fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let (before, from) = rest.split_at(open);
        out.push_str(before);
        let found = from.find('}').and_then(|close| {
            let name = from.get(1..close)?;
            let value = values.iter().find(|(n, _)| *n == name)?.1;
            Some((value, close))
        });
        if let Some((value, close)) = found {
            out.push_str(value);
            rest = from.get(close + 1..).unwrap_or_default();
        } else {
            out.push('{');
            rest = from.get(1..).unwrap_or_default();
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::SummaryWords;

    #[test]
    fn the_count_line_says_one_or_many() {
        let words = SummaryWords::default();
        assert_eq!(words.count_line(1, 1), "1 comment on 1 page.");
        assert_eq!(words.count_line(5, 2), "5 comments on 2 pages.");
    }

    #[test]
    fn a_drawn_pages_alt_names_the_comments_it_marks() {
        let words = SummaryWords::default();
        assert_eq!(words.figure_alt("3", 4, 4), "Page 3, with comment 4 marked");
        assert_eq!(words.figure_alt("iv", 1, 3), "Page iv, with comments 1 to 3 marked");
        assert_eq!(words.figure_alt("2", 5, 4), "Page 2");
    }

    #[test]
    fn subtypes_get_their_readers_names() {
        let words = SummaryWords::default();
        assert_eq!(words.kind_name("Text"), "Note");
        assert_eq!(words.kind_name("Highlight"), "Highlight");
    }

    #[test]
    fn the_status_line_names_the_state_and_the_mark() {
        let words = SummaryWords::default();
        assert_eq!(
            words.status_line(Some("Accepted"), true).as_deref(),
            Some("Status: Accepted · Marked")
        );
        assert_eq!(words.status_line(None, true).as_deref(), Some("Marked"));
        assert_eq!(words.status_line(None, false), None);
    }

    #[test]
    fn the_title_falls_back_when_the_document_has_none() {
        let words = SummaryWords::default();
        assert_eq!(words.title(Some("Q3 report")), "Comments on Q3 report");
        assert_eq!(words.title(Some("  ")), "Comment summary");
        assert_eq!(words.title(None), "Comment summary");
    }

    #[test]
    fn every_phrase_comes_from_the_callers_words() {
        let words = SummaryWords {
            titled: "Kommentare zu {title}".to_owned(),
            comments_one: "1 Kommentar".to_owned(),
            comments_other: "{count} Kommentare".to_owned(),
            pages_one: "1 Seite".to_owned(),
            pages_other: "{count} Seiten".to_owned(),
            count: "{comments} auf {pages}.".to_owned(),
            figure_many: "Seite {label}, Kommentare {first} bis {last}".to_owned(),
            status: "Stand: {status}".to_owned(),
            marked: "Markiert".to_owned(),
            ..SummaryWords::default()
        }
        .with_page("Seite {label}")
        .with_kind("Text", "Notiz")
        .with_state("Accepted", "Angenommen");
        assert_eq!(words.title(Some("Bericht")), "Kommentare zu Bericht");
        assert_eq!(words.count_line(3, 1), "3 Kommentare auf 1 Seite.");
        assert_eq!(words.page_heading("iv"), "Seite iv");
        assert_eq!(words.figure_alt("2", 1, 3), "Seite 2, Kommentare 1 bis 3");
        assert_eq!(words.kind_name("Text"), "Notiz");
        assert_eq!(
            words.status_line(Some("Accepted"), true).as_deref(),
            Some("Stand: Angenommen · Markiert")
        );
    }

    #[test]
    fn a_value_is_never_read_as_a_placeholder() {
        let words = SummaryWords::default();
        assert_eq!(
            words.figure_alt("{last}", 1, 3),
            "Page {last}, with comments 1 to 3 marked"
        );
        assert_eq!(words.title(Some("{title}")), "Comments on {title}");
    }

    #[test]
    fn a_template_may_leave_a_placeholder_out_and_keeps_other_braces() {
        let words = SummaryWords::default().with_page("Page");
        assert_eq!(words.page_heading("3"), "Page");
        let words = SummaryWords::default().with_page("{label} {unknown}");
        assert_eq!(words.page_heading("3"), "3 {unknown}");
    }
}
