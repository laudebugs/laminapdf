//! Text extraction (todos B9b, B9d; feature `extract`, which `text` implies): [`Document::page_text`](crate::Document::page_text)
//! returns a page's [`PageText`]: glyphs with their text, codes and boxes in page space
//! (ADR 0027), runs with their font, size, MCID and `ActualText`, and [`PageText::lines`].
//!
//! ```
//! # fn sample() -> Vec<u8> { std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/text-std14.pdf")).unwrap() }
//! use lamina::{Document, OpenOptions, extract::TextOptions};
//!
//! let doc = Document::open_bytes(sample(), OpenOptions::default())?;
//! let text = doc.page_text(0, &TextOptions::new())?;
//! assert_eq!(text.lines()[0], "Courier: Hello, world! 0123456789");
//! let first = &text.glyphs[0];
//! assert!(first.quad[0].y > first.quad[3].y); // the baseline is below the top: y grows down
//! # Ok::<(), lamina::Error>(())
//! ```

pub use lamina_text::extract::{Code, Glyph, PageText, Quad, TextRun, UserSpaceVector};

/// What [`Document::page_text`](crate::Document::page_text) returns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextOptions {
    /// Leave out text drawn invisibly (rendering modes 3 and 7: OCR layers, hidden text).
    /// Default `false`: invisible text is extracted, flagged on its glyphs and runs.
    pub skip_invisible: bool,
}

impl TextOptions {
    /// The defaults: every glyph, visible or not.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Leave out invisible text ([`TextOptions::skip_invisible`]).
    #[must_use]
    pub const fn with_skip_invisible(mut self, skip: bool) -> Self {
        self.skip_invisible = skip;
        self
    }
}
