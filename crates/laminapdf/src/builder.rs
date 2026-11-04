//! [`Builder`]: `laminapdf-build`'s builder, finishing into a façade [`Document`].

use laminapdf_build::{Attachment, ViewerPreferences};
use laminapdf_build::{
    BuildOptions, Fit, Metadata, OutlineEntry, OutlineItemId, PageBuilder, PageLabel, PageSize, TagDiagnostic,
};
#[cfg(feature = "text")]
use laminapdf_build::{EmbedPolicy, Standard14};

use crate::document::Document;
use crate::error::Result;
#[cfg(feature = "text")]
use crate::{Font, FontHandle};

/// Builds a new document ([ADR 0005] §6): fonts, pages (each drawn on a
/// [`Surface`](crate::Surface) in page space, [ADR 0027]), navigation and metadata, then
/// [`Builder::finish`] gives a [`Document`] that saves as a full rewrite.
///
/// This is `laminapdf-build`'s builder with the façade's error and document types. Its pages
/// are borrowed builders ([`PageBuilder`]) and paths are drawn with a closure: a Rust API.
/// The binding form (owned page handles, path records) is defined by the edge crates, such as
/// `laminapdf-wasm`, which wrap this one.
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#[derive(Debug)]
pub struct Builder(laminapdf_build::Builder);

impl Builder {
    /// A builder for a new, empty document. Everything that would come from the
    /// environment (the file ID, dates, the producer) is in `options`, so the same calls give
    /// the same bytes.
    #[must_use]
    pub fn new(options: BuildOptions) -> Self {
        Self(laminapdf_build::Builder::new(options))
    }

    /// Registers `font` for text; it is subset to the glyphs drawn and embedded at
    /// [`Builder::finish`].
    ///
    /// # Errors
    /// `font-not-embeddable` when the font's `fsType` forbids embedding.
    #[cfg(feature = "text")]
    pub fn font(&mut self, font: Font) -> Result<FontHandle> {
        Ok(self.0.font(font)?)
    }

    /// Loads a font from its bytes (TrueType or OpenType/CFF; F7) and registers it.
    ///
    /// # Errors
    /// `invalid-font`; as [`Builder::font`].
    #[cfg(feature = "text")]
    pub fn font_bytes(&mut self, bytes: Vec<u8>) -> Result<FontHandle> {
        self.font(Font::from_bytes(bytes)?)
    }

    /// Registers `font` with an embedding `policy` (`EmbedPolicy::Override` embeds a font
    /// whose licence restricts it: the caller's responsibility).
    ///
    /// # Errors
    /// As [`Builder::font`].
    #[cfg(feature = "text")]
    pub fn font_with(&mut self, font: Font, policy: EmbedPolicy) -> Result<FontHandle> {
        Ok(self.0.font_with(font, policy)?)
    }

    /// One of the standard 14 fonts: not embedded, Latin only, no shaping.
    ///
    /// # Errors
    /// None in practice.
    #[cfg(feature = "text")]
    pub fn standard_font(&mut self, font: Standard14) -> Result<FontHandle> {
        Ok(self.0.standard_font(font)?)
    }

    /// Defines the named destination `name`: page `index` (0-based) shown as `fit`.
    pub fn destination(&mut self, name: &str, index: u32, fit: Fit) -> &mut Self {
        self.0.destination(name, index, fit);
        self
    }

    /// Adds `entries` (with their children) to the outline; returns their IDs in pre-order.
    ///
    /// # Errors
    /// `limit-exceeded` when object numbers run out.
    pub fn outline(&mut self, entries: Vec<OutlineEntry>) -> Result<Vec<OutlineItemId>> {
        Ok(self.0.outline(entries)?)
    }

    /// The page labels, ranges by first page (checked at [`Builder::finish`]).
    pub fn page_labels(&mut self, ranges: Vec<PageLabel>) -> &mut Self {
        self.0.page_labels(ranges);
        self
    }

    /// The metadata, written as the Info dictionary and an XMP packet with the same values.
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.0.metadata(metadata);
        self
    }

    /// The natural language (`/Lang`), a BCP 47 tag such as `en-GB`.
    pub fn lang(&mut self, lang: &str) -> &mut Self {
        self.0.lang(lang);
        self
    }

    /// Attaches a file (`/EmbeddedFiles` and the catalog's `/AF`): PDF/A-3b's associated
    /// files, such as an e-invoice's XML. The same name again replaces the file.
    pub fn attach(&mut self, file: Attachment) -> &mut Self {
        self.0.attach(file);
        self
    }

    /// How viewers present the document when it opens.
    pub fn viewer_preferences(&mut self, prefs: ViewerPreferences) -> &mut Self {
        self.0.viewer_preferences(prefs);
        self
    }

    /// What the tagged document lacks so far: content drawn outside every structure element
    /// and artifact, and figures without alternate text, per finished page. Empty while the
    /// document isn't tagged ([`BuildOptions::with_tagged`], or a page using
    /// [`Surface::tag`](crate::Surface::tag)).
    #[must_use]
    pub fn tagging_diagnostics(&self) -> Vec<TagDiagnostic> {
        self.0.tagging_diagnostics().to_vec()
    }

    /// A page of `size`, added by [`PageBuilder::finish`].
    pub fn page(&mut self, size: PageSize) -> PageBuilder<'_> {
        self.0.page(size)
    }

    /// Writes the fonts, links, page tree, outline, destinations, labels, catalog and
    /// metadata, and returns the document (unsaved: [`Document::save`] writes it). Warnings
    /// embedding the fonts raised are in its [`Document::warnings`].
    ///
    /// # Errors
    /// `page-index-out-of-range`, `unknown-destination`, `invalid-page-labels`,
    /// `invalid-geometry`, `subset-failed`; under a [`Profile`](crate::Profile), the first rule
    /// the document breaks (see `laminapdf_build::Builder::finish`).
    pub fn finish(self) -> Result<Document> {
        // Only font embedding raises build warnings.
        #[cfg(feature = "text")]
        let (doc, warnings) = {
            let (doc, warnings) = self.0.finish_with_warnings()?;
            (doc, warnings.into_iter().map(crate::Warning::from).collect())
        };
        #[cfg(not(feature = "text"))]
        let (doc, warnings) = (self.0.finish()?, Vec::new());
        Ok(Document::built(doc, warnings))
    }
}
