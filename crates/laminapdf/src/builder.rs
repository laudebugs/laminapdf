//! [`Builder`]: `laminapdf-build`'s builder, finishing into a façade [`Document`].

use laminapdf_build::{Attachment, ViewerPreferences};
use laminapdf_build::{
    BuildOptions, ElementId, Fit, Metadata, OutlineEntry, OutlineItemId, PageBuilder, PageLabel, PageSize,
    TagDiagnostic,
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
pub struct Builder(laminapdf_build::Builder, Vec<crate::Warning>);

impl Builder {
    /// A builder for a new, empty document. Everything that would come from the
    /// environment (the file ID, dates, the producer) is in `options`, so the same calls give
    /// the same bytes.
    #[must_use]
    pub fn new(options: BuildOptions) -> Self {
        Self(laminapdf_build::Builder::new(options), Vec::new())
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

    /// Writes `element`, a structure element opened across pages by
    /// [`Surface::open_element`](crate::Surface::open_element), once its last page is finished.
    ///
    /// # Errors
    /// `tag-element-not-open` when `element` isn't open (closed already, opened by another
    /// builder or inside an artifact, or on a page that failed).
    pub fn close_element(&mut self, element: ElementId) -> Result<()> {
        Ok(self.0.close_element(element)?)
    }

    /// Lays `flow` out and adds its pages ([`laminapdf::flow`](crate::flow), ADR 0032): blocks
    /// poured through a page template into as many pages as they need. Its warnings
    /// (`flow-keep-broken`) are in the finished document's [`Document::warnings`].
    ///
    /// # Errors
    /// `flow-content-too-large` for a line or a figure taller than a page's body; shaping
    /// errors (`missing-glyph`).
    #[cfg(feature = "text")]
    pub fn flow(&mut self, flow: &crate::flow::Flow) -> Result<crate::flow::FlowReport> {
        let report = self.0.flow(flow)?;
        self.1.extend(report.warnings.iter().cloned().map(crate::Warning::from));
        Ok(report)
    }

    /// Lays `markdown` out as a flow and adds its pages ([`laminapdf::markdown`](crate::markdown),
    /// ADR 0032 §6): `CommonMark` with tables, footnotes, strikethrough and task lists, its front
    /// matter as the document's metadata. Without fonts in `options` it sets the standard 14
    /// Helvetica family and Courier: Latin text only, and refused under PDF/UA-1
    /// (`font-not-embedded`). Its warnings (`markdown-html-ignored`, `markdown-image-missing`,
    /// the flow's) are in the finished document's [`Document::warnings`].
    ///
    /// # Errors
    /// As [`Builder::flow`]; `missing-glyph` for a character the fonts lack.
    #[cfg(feature = "markdown")]
    pub fn markdown(
        &mut self,
        markdown: &str,
        options: &crate::markdown::MarkdownBuildOptions,
    ) -> Result<crate::flow::FlowReport> {
        let report = self.0.markdown(markdown, options)?;
        self.1.extend(report.warnings.iter().cloned().map(crate::Warning::from));
        Ok(report)
    }

    /// Lays a `laminapdf.document/1` document out and adds its pages after any already added
    /// ([`laminapdf::flow::FlowDocument`](crate::flow::FlowDocument), ADR 0032 §6), with its
    /// fonts and images from `resources`; its title, author, subject and language become the
    /// document's metadata. Pass its [profiles](crate::flow::FlowDocument::profiles) to the
    /// builder's options. Its warnings are in the finished document's [`Document::warnings`].
    ///
    /// # Errors
    /// `invalid-document` for a style, font or image it names but doesn't have; as
    /// [`Builder::flow`].
    #[cfg(feature = "document")]
    pub fn document(
        &mut self,
        document: &crate::flow::FlowDocument,
        resources: &crate::flow::Resources,
    ) -> Result<crate::flow::FlowReport> {
        let report = self.0.document(document, resources)?;
        self.1.extend(report.warnings.iter().cloned().map(crate::Warning::from));
        Ok(report)
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
        // Flows' warnings first, in the order they were raised, then the fonts'.
        let Self(builder, flows) = self;
        #[cfg(feature = "text")]
        let (doc, warnings) = {
            let (doc, fonts) = builder.finish_with_warnings()?;
            let mut warnings = flows;
            warnings.extend(fonts.into_iter().map(crate::Warning::from));
            (doc, warnings)
        };
        #[cfg(not(feature = "text"))]
        let (doc, warnings) = (builder.finish()?, flows);
        Ok(Document::built(doc, warnings))
    }
}
