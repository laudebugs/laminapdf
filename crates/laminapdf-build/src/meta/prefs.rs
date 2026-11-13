//! Viewer preferences (§12.2), the page mode and the page layout (§7.7.2, Table 29).

use laminapdf_cos::{Dict, Name};
use laminapdf_model::{PageLayout, PageMode};

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
///
/// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_display_doc_title(mut self, display: bool) -> Self {
        self.display_doc_title = display;
        self
    }

    /// The panel shown on opening.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_page_mode(mut self, mode: PageMode) -> Self {
        self.page_mode = Some(mode);
        self
    }

    /// The page layout on opening.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn with_page_layout(mut self, layout: PageLayout) -> Self {
        self.page_layout = Some(layout);
        self
    }

    /// Whether the title bar shows the document title.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn display_doc_title(&self) -> bool {
        self.display_doc_title
    }

    /// The page mode, if set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn page_mode(&self) -> Option<PageMode> {
        self.page_mode
    }

    /// The page layout, if set.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn page_layout(&self) -> Option<PageLayout> {
        self.page_layout
    }

    /// The PDF version these preferences need.
    pub(crate) fn version(self) -> (u8, u8) {
        let mut version = if self.display_doc_title { (1, 4) } else { (1, 0) };
        if let Some(mode) = self.page_mode {
            version = version.max(mode.since());
        }
        if let Some(layout) = self.page_layout {
            version = version.max(layout.since());
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
