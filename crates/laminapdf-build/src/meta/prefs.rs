//! Viewer preferences (§12.2), the page mode and the page layout (§7.7.2, Table 29).

use laminapdf_cos::{Dict, Name};

/// Which panel a viewer shows when the document opens (`/PageMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageMode {
    /// No panel (`UseNone`).
    None,
    /// The outline (`UseOutlines`).
    Outlines,
    /// Page thumbnails (`UseThumbs`).
    Thumbnails,
    /// Full screen (`FullScreen`).
    FullScreen,
    /// The optional content panel (`UseOC`, PDF 1.5).
    OptionalContent,
    /// The attachments panel (`UseAttachments`, PDF 1.6).
    Attachments,
}

impl PageMode {
    pub(crate) const fn pdf_name(self) -> &'static str {
        match self {
            Self::None => "UseNone",
            Self::Outlines => "UseOutlines",
            Self::Thumbnails => "UseThumbs",
            Self::FullScreen => "FullScreen",
            Self::OptionalContent => "UseOC",
            Self::Attachments => "UseAttachments",
        }
    }

    pub(crate) const fn version(self) -> (u8, u8) {
        match self {
            Self::OptionalContent => (1, 5),
            Self::Attachments => (1, 6),
            _ => (1, 0),
        }
    }
}

/// How a viewer lays pages out when the document opens (`/PageLayout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PageLayout {
    /// One page at a time.
    SinglePage,
    /// One continuous column.
    OneColumn,
    /// Two columns, odd pages on the left.
    TwoColumnLeft,
    /// Two columns, odd pages on the right.
    TwoColumnRight,
    /// Two pages at a time, odd pages on the left (PDF 1.5).
    TwoPageLeft,
    /// Two pages at a time, odd pages on the right (PDF 1.5).
    TwoPageRight,
}

impl PageLayout {
    pub(crate) const fn pdf_name(self) -> &'static str {
        match self {
            Self::SinglePage => "SinglePage",
            Self::OneColumn => "OneColumn",
            Self::TwoColumnLeft => "TwoColumnLeft",
            Self::TwoColumnRight => "TwoColumnRight",
            Self::TwoPageLeft => "TwoPageLeft",
            Self::TwoPageRight => "TwoPageRight",
        }
    }

    pub(crate) const fn version(self) -> (u8, u8) {
        match self {
            Self::TwoPageLeft | Self::TwoPageRight => (1, 5),
            _ => (1, 0),
        }
    }
}

/// How a viewer presents the document when it opens.
///
/// ```
/// use laminapdf_build::{PageLayout, PageMode, ViewerPreferences};
///
/// let prefs = ViewerPreferences::default()
///     .with_display_doc_title(true)
///     .with_page_mode(PageMode::Outlines)
///     .with_page_layout(PageLayout::OneColumn);
/// assert!(prefs.display_doc_title());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct ViewerPreferences {
    display_doc_title: bool,
    page_mode: Option<PageMode>,
    page_layout: Option<PageLayout>,
}

impl ViewerPreferences {
    /// Show the Info title in the window's title bar instead of the file name
    /// (`/DisplayDocTitle`, PDF 1.4).
    #[must_use]
    pub const fn with_display_doc_title(mut self, display: bool) -> Self {
        self.display_doc_title = display;
        self
    }

    /// The panel shown on opening.
    #[must_use]
    pub const fn with_page_mode(mut self, mode: PageMode) -> Self {
        self.page_mode = Some(mode);
        self
    }

    /// The page layout on opening.
    #[must_use]
    pub const fn with_page_layout(mut self, layout: PageLayout) -> Self {
        self.page_layout = Some(layout);
        self
    }

    /// Whether the title bar shows the document title.
    #[must_use]
    pub const fn display_doc_title(&self) -> bool {
        self.display_doc_title
    }

    /// The page mode, if set.
    #[must_use]
    pub const fn page_mode(&self) -> Option<PageMode> {
        self.page_mode
    }

    /// The page layout, if set.
    #[must_use]
    pub const fn page_layout(&self) -> Option<PageLayout> {
        self.page_layout
    }

    /// The PDF version these preferences need.
    pub(crate) fn version(self) -> (u8, u8) {
        let mut version = if self.display_doc_title { (1, 4) } else { (1, 0) };
        if let Some(mode) = self.page_mode {
            version = version.max(mode.version());
        }
        if let Some(layout) = self.page_layout {
            version = version.max(layout.version());
        }
        version
    }

    /// Adds these preferences to the catalog.
    pub(crate) fn write(self, catalog: &mut Dict) {
        if self.display_doc_title {
            let mut prefs = Dict::new();
            prefs.insert("DisplayDocTitle", true);
            catalog.insert("ViewerPreferences", prefs);
        }
        if let Some(mode) = self.page_mode {
            catalog.insert("PageMode", Name::from(mode.pdf_name()));
        }
        if let Some(layout) = self.page_layout {
            catalog.insert("PageLayout", Name::from(layout.pdf_name()));
        }
    }
}
