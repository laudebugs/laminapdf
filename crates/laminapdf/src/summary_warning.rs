//! What a comment summary ([`Document::comment_summary`](crate::Document::comment_summary)) left
//! out, as warnings. Only a build with the `text` feature makes summaries; every surface knows
//! the code from `fixtures/codes.json`'s generated tables.

/// Why a comment summary lists a page's comments without drawing the page.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PageNotDrawn {
    /// The page uses a font the file doesn't embed, which PDF/UA-1 forbids.
    FontsNotEmbedded,
    /// The page's content can't be read to its end within the document's limits, so its
    /// marked content can't all be taken out of the drawing.
    ContentUnreadable,
}

impl PageNotDrawn {
    /// `fonts-not-embedded` or `content-unreadable`, as the warning's data names it.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FontsNotEmbedded => "fonts-not-embedded",
            Self::ContentUnreadable => "content-unreadable",
        }
    }
}

/// What a comment summary left out, in its [`Document::warnings`](crate::Document::warnings).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SummaryWarning {
    /// A page's comments are listed without the page beside them (`summary-page-not-drawn`).
    PageNotDrawn {
        /// The page of the summarised document, from 0.
        page: u32,
        /// Why.
        reason: PageNotDrawn,
    },
}

impl SummaryWarning {
    /// Every code, in the order `fixtures/codes.json`'s `facadeWarnings` lists them after
    /// [`EditWarning`](crate::EditWarning)'s.
    ///
    /// Added in 0.6.0.
    pub const CODES: &'static [&'static str] = &["summary-page-not-drawn"];

    /// The stable kebab-case code.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::PageNotDrawn { .. } => "summary-page-not-drawn",
        }
    }

    /// Where it is: nowhere in the summary. The page it names belongs to the summarised
    /// document, so it is in the warning's data instead.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn location(&self) -> laminapdf_cos::Location {
        laminapdf_cos::Location::NONE
    }
}
