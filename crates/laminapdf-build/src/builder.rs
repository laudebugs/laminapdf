//! [`Builder`] (a new document) and [`Appender`] (pages added to an open one).

use std::collections::BTreeMap;

use laminapdf_cos::{ObjRef, OpenOptions};
use laminapdf_model::{IdAllocator, Kind, OutlineItemId, PageFrame, PageId, PdfDate};
#[cfg(feature = "text")]
use laminapdf_text::{EmbedPolicy, FontRegistry, Standard14};

use crate::annot::link::PendingLink;
use crate::attach::Attachment;
use crate::error::Result;
use crate::finish;
use crate::meta::{Metadata, ViewerPreferences};
use crate::nav::{Fit, OutlineEntry, PageLabel};
use crate::page::{PageBuilder, PageSize};
use crate::profile::{Conformance, Profile};
use crate::resources::DocResources;
#[cfg(feature = "text")]
use crate::surface::text::Font;
use crate::tag::TagDiagnostic;
use crate::tag::elem::TagState;

/// The header version of a new document (§7.5.2). Saving with object and cross-reference
/// streams (`laminapdf-save`'s default) raises anything below 1.5 to 1.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[non_exhaustive]
pub enum PdfVersion {
    /// PDF 1.4 (transparency).
    V1_4,
    /// PDF 1.5 (object streams).
    V1_5,
    /// PDF 1.6 (`/UserUnit`).
    V1_6,
    /// PDF 1.7 (ISO 32000-1), the default.
    #[default]
    V1_7,
    /// PDF 2.0 (ISO 32000-2).
    V2_0,
}

impl PdfVersion {
    /// `"1.7"` and so on.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V1_4 => "1.4",
            Self::V1_5 => "1.5",
            Self::V1_6 => "1.6",
            Self::V1_7 => "1.7",
            Self::V2_0 => "2.0",
        }
    }
}

/// Options for a new document. Everything that would otherwise come from the environment
/// (the file identifier, dates) is given here, so the same calls give the same bytes
/// (ADR 0001).
///
/// ```
/// use laminapdf_build::{BuildOptions, PdfVersion};
///
/// let options = BuildOptions::default()
///     .with_version(PdfVersion::V2_0)
///     .with_id([7; 16])
///     .with_producer("lamina")
///     .with_creation_date("D:20261029120000Z".parse()?);
/// assert_eq!(options.version(), PdfVersion::V2_0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BuildOptions {
    version: PdfVersion,
    id: Option<[u8; 16]>,
    producer: Option<String>,
    creation_date: Option<PdfDate>,
    uncompressed: bool,
    tagged: bool,
    conformance: Conformance,
    open: OpenOptions,
}

impl BuildOptions {
    /// The defaults, as [`BuildOptions::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The header version (default 1.7).
    #[must_use]
    pub const fn with_version(mut self, version: PdfVersion) -> Self {
        self.version = version;
        self
    }

    /// A fixed permanent identifier (`/ID[0]`, §14.4); without one, the writer derives it
    /// from the content, which is just as deterministic.
    #[must_use]
    pub const fn with_id(mut self, id: [u8; 16]) -> Self {
        self.id = Some(id);
        self
    }

    /// `/Producer` in the Info dictionary ([`Builder::metadata`] sets the rest, and its own
    /// producer wins).
    #[must_use]
    pub fn with_producer(mut self, producer: impl Into<String>) -> Self {
        self.producer = Some(producer.into());
        self
    }

    /// `/CreationDate` in the Info dictionary: given, never read from a clock.
    #[must_use]
    pub const fn with_creation_date(mut self, date: PdfDate) -> Self {
        self.creation_date = Some(date);
        self
    }

    /// Whether content streams are Flate-compressed (default `true`; `false` keeps them
    /// readable, for goldens and debugging). Images are always compressed.
    #[must_use]
    pub const fn with_compress(mut self, compress: bool) -> Self {
        self.uncompressed = !compress;
        self
    }

    /// Whether the document is tagged (§14.8): a structure tree with a `Document` element
    /// and `/MarkInfo << /Marked true >>`, even before (or without) any
    /// [`Surface::tag`](crate::Surface::tag), so content drawn outside every element is
    /// reported by [`Builder::tagging_diagnostics`]. Default `false`; a page that uses an
    /// element or artifact makes the document tagged either way. Appended pages
    /// ([`Builder::on`]) are never tagged: merging into an existing structure tree isn't
    /// supported, so their elements and artifacts are drawn as plain content.
    #[must_use]
    pub const fn with_tagged(mut self, tagged: bool) -> Self {
        self.tagged = tagged;
        self
    }

    /// Whether the document is asked to be tagged (PDF/UA-1 asks for it too).
    #[must_use]
    pub const fn tagged(&self) -> bool {
        self.tagged || self.conformance.ua
    }

    /// Adds a conformance profile ([`Profile`]): PDF/UA-1, or one of PDF/A-2b, PDF/A-2u and
    /// PDF/A-3b (a second PDF/A profile replaces the first; [`Profile::None`] clears them).
    /// [`Builder::finish`] then fails with a coded error for each machine-checkable rule the
    /// document breaks, and writes what the profiles need: the identification in the XMP
    /// packet (always written), `/DisplayDocTitle` for PDF/UA-1, and for PDF/A the output
    /// intent with an embedded sRGB profile. PDF/UA-1 makes the document tagged.
    ///
    /// Only new documents ([`Builder::new`]) take a profile; [`Builder::on`] ignores it.
    #[must_use]
    pub const fn with_profile(mut self, profile: Profile) -> Self {
        self.conformance = self.conformance.with(profile);
        self
    }

    /// The profiles chosen, PDF/UA-1 first.
    pub fn profiles(&self) -> impl Iterator<Item = Profile> {
        self.conformance.profiles()
    }

    pub(crate) const fn conformance(&self) -> Conformance {
        self.conformance
    }

    /// The limits and strict mode the new document carries (default
    /// [`OpenOptions::default`]); no password applies.
    #[must_use]
    pub fn with_open_options(mut self, open: OpenOptions) -> Self {
        self.open = open;
        self
    }

    /// The header version.
    #[must_use]
    pub const fn version(&self) -> PdfVersion {
        self.version
    }

    /// The permanent identifier, if fixed.
    #[must_use]
    pub const fn id(&self) -> Option<&[u8; 16]> {
        self.id.as_ref()
    }

    /// `/Producer`, if set.
    #[must_use]
    pub fn producer(&self) -> Option<&str> {
        self.producer.as_deref()
    }

    /// `/CreationDate`, if set.
    #[must_use]
    pub const fn creation_date(&self) -> Option<&PdfDate> {
        self.creation_date.as_ref()
    }
}

/// What every page of one build shares.
#[derive(Debug)]
pub(crate) struct BuildState {
    /// The `/Pages` node new pages hang off (reserved for a new document).
    pub(crate) pages_root: ObjRef,
    /// The pages written so far, in order, and their frames.
    pub(crate) pages: Vec<ObjRef>,
    pub(crate) frames: Vec<PageFrame>,
    /// The fonts text was drawn with (one registry per build).
    #[cfg(feature = "text")]
    pub(crate) fonts: FontRegistry,
    /// Link annotations waiting for their targets.
    pub(crate) links: Vec<PendingLink>,
    pub(crate) resources: DocResources,
    pub(crate) compress: bool,
    /// Write `/Rotate` and `/CropBox` on every page, so nothing is inherited from an
    /// existing page tree's root.
    pub(crate) explicit_inherited: bool,
    /// The lowest PDF version with every feature written so far.
    pub(crate) required: (u8, u8),
    /// What embedding the fonts raised (ToUnicode conflicts, whole fonts kept).
    #[cfg(feature = "text")]
    pub(crate) font_warnings: Vec<laminapdf_text::Warning>,
    /// Every font registered, in order (the profiles check their usage).
    #[cfg(feature = "text")]
    pub(crate) font_keys: Vec<laminapdf_text::FontKey>,
    /// The first page that draws with `DeviceCMYK` (PDF/A refuses it under the sRGB intent).
    pub(crate) device_cmyk: Option<u32>,
    /// Whether pages may build structure (not for an appender).
    pub(crate) tagging: bool,
    /// The structure elements, `ParentTree` and diagnostics so far.
    pub(crate) tag: TagState,
}

impl BuildState {
    fn new(pages_root: ObjRef, compress: bool, explicit_inherited: bool, tagged: Option<bool>) -> Self {
        Self {
            pages_root,
            pages: Vec::new(),
            frames: Vec::new(),
            #[cfg(feature = "text")]
            fonts: FontRegistry::new(),
            links: Vec::new(),
            resources: DocResources::default(),
            compress,
            explicit_inherited,
            required: (1, 0),
            #[cfg(feature = "text")]
            font_warnings: Vec::new(),
            #[cfg(feature = "text")]
            font_keys: Vec::new(),
            device_cmyk: None,
            tagging: tagged.is_some(),
            tag: TagState::new(tagged.unwrap_or(false)),
        }
    }

    /// Notes that something written needs PDF `version`.
    pub(crate) fn require(&mut self, version: (u8, u8)) {
        self.required = self.required.max(version);
    }

    #[cfg(feature = "text")]
    fn add_font(&mut self, font: laminapdf_text::Font, policy: EmbedPolicy) -> Result<Font> {
        let key = self.fonts.add(font.clone(), policy)?;
        self.font_keys.push(key);
        Ok(Font { key, font })
    }
}

/// The outline of a new document: its root and items' reserved numbers, pre-order.
#[derive(Debug)]
pub(crate) struct OutlineState {
    pub(crate) root: ObjRef,
    pub(crate) entries: Vec<OutlineEntry>,
    pub(crate) ids: Vec<ObjRef>,
}

/// What a new document has besides its pages.
#[derive(Debug, Default)]
pub(crate) struct DocFeatures {
    pub(crate) names: BTreeMap<String, (u32, Fit)>,
    pub(crate) outline: Option<OutlineState>,
    pub(crate) labels: Option<Vec<PageLabel>>,
    pub(crate) metadata: Option<Metadata>,
    pub(crate) lang: Option<String>,
    pub(crate) prefs: Option<ViewerPreferences>,
    pub(crate) attachments: BTreeMap<String, Attachment>,
}

/// What a page writes into: the document and the build's shared state.
#[derive(Debug)]
pub(crate) struct Target<'b> {
    pub(crate) doc: &'b mut laminapdf_cos::Document,
    pub(crate) state: &'b mut BuildState,
}

/// Builds a new document ([ADR 0005] §6): pages, then [`Builder::finish`], which returns an
/// editable [`laminapdf_ops::Document`] whose objects all live in the overlay over an empty base.
/// Saving it (`laminapdf_save::Save`) writes a full file; it is `Dirty` until then.
///
/// ```
/// use laminapdf_build::{BuildOptions, Builder, PageSize};
///
/// let mut b = Builder::new(BuildOptions::default());
/// b.page(PageSize::A4).finish()?;
/// b.page(PageSize::LETTER.landscape()).finish()?;
/// let doc = b.finish()?;
/// assert_eq!((doc.pages()?.len(), doc.is_dirty()), (2, true));
/// # Ok::<(), laminapdf_build::Error>(())
/// ```
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
#[derive(Debug)]
pub struct Builder {
    doc: laminapdf_cos::Document,
    options: BuildOptions,
    catalog: ObjRef,
    state: BuildState,
    features: DocFeatures,
}

impl Builder {
    /// A builder for a new, empty document. The catalog and the page tree root are reserved
    /// first, so they are objects 1 and 2.
    #[must_use]
    pub fn new(options: BuildOptions) -> Self {
        let doc = laminapdf_cos::Document::empty(Some(options.version.as_str()), &options.open);
        // A fresh document starts numbering at 1, far below any limit; should reserving still
        // fail, `finish` reports it when it writes the reserved objects.
        let catalog = doc.reserve_number().unwrap_or(ObjRef::new(1, 0));
        let pages_root = doc.reserve_number().unwrap_or(ObjRef::new(2, 0));
        let compress = !options.uncompressed;
        let tagged = options.tagged();
        Self {
            doc,
            options,
            catalog,
            state: BuildState::new(pages_root, compress, false, Some(tagged)),
            features: DocFeatures::default(),
        }
    }

    /// Registers `font` for text ([`Surface::text`](crate::Surface::text),
    /// [`Surface::text_box`](crate::Surface::text_box)); it is subset to the glyphs drawn and
    /// embedded when the build finishes.
    ///
    /// # Errors
    /// `font-not-embeddable` when the font's `fsType` forbids embedding (see
    /// [`Builder::font_with`]).
    #[cfg(feature = "text")]
    pub fn font(&mut self, font: laminapdf_text::Font) -> Result<Font> {
        self.state.add_font(font, EmbedPolicy::Respect)
    }

    /// Registers `font` with an embedding `policy` (`EmbedPolicy::Override` embeds a font
    /// whose licence restricts it: the caller's responsibility).
    ///
    /// # Errors
    /// As [`Builder::font`].
    #[cfg(feature = "text")]
    pub fn font_with(&mut self, font: laminapdf_text::Font, policy: EmbedPolicy) -> Result<Font> {
        self.state.add_font(font, policy)
    }

    /// One of the standard 14 fonts: not embedded, one glyph per character (no shaping),
    /// WinAnsi-encoded (Latin) or in its built-in encoding (Symbol, `ZapfDingbats`).
    ///
    /// # Errors
    /// None in practice (the registry accepts every standard 14 font); kept for symmetry
    /// with [`Builder::font`].
    #[cfg(feature = "text")]
    pub fn standard_font(&mut self, font: Standard14) -> Result<Font> {
        self.state
            .add_font(laminapdf_text::Font::standard(font), EmbedPolicy::Respect)
    }

    /// Defines the named destination `name` (`/Names /Dests`, §12.3.2.4): page `index`
    /// (0-based, resolved when the build finishes) shown as `fit`. Links and outline items
    /// reach it with [`LinkTarget::Named`](crate::LinkTarget::Named). Defining a name again
    /// replaces it. The tree is written sorted by name, so the output doesn't depend on the
    /// order of the calls.
    pub fn destination(&mut self, name: impl Into<String>, index: u32, fit: Fit) -> &mut Self {
        self.features.names.insert(name.into(), (index, fit));
        self
    }

    /// Adds `entries` (with their children) at the end of the document's outline. Each item
    /// gets its object number, and so its [`OutlineItemId`], now (ADR 0005 §5); the IDs come
    /// back in pre-order (an item before its children). Targets are resolved when the build
    /// finishes.
    ///
    /// # Errors
    /// [`Error::Cos`](crate::Error::Cos) when object numbers run out.
    pub fn outline(&mut self, entries: impl IntoIterator<Item = OutlineEntry>) -> Result<Vec<OutlineItemId>> {
        let ids = IdAllocator::new(&self.doc);
        if self.features.outline.is_none() {
            let root = self.doc.reserve_number()?;
            self.features.outline = Some(OutlineState {
                root,
                entries: Vec::new(),
                ids: Vec::new(),
            });
        }
        let Some(outline) = self.features.outline.as_mut() else {
            return Ok(Vec::new());
        };
        let mut new = Vec::new();
        for entry in entries {
            for _ in 0..entry.len() {
                let (r, _) = ids.allocate(Kind::OutlineItem)?;
                outline.ids.push(r);
                new.push(OutlineItemId::object(r));
            }
            outline.entries.push(entry);
        }
        Ok(new)
    }

    /// The page labels (`/PageLabels`, §12.4.2): ranges by first page, checked when the build
    /// finishes (the first must start at page 0, starts must increase and be pages of the
    /// document, numbers start at 1).
    pub fn page_labels(&mut self, ranges: impl IntoIterator<Item = PageLabel>) -> &mut Self {
        self.features.labels = Some(ranges.into_iter().collect());
        self
    }

    /// The document's metadata, written both as the Info dictionary and as an XMP packet
    /// (the catalog's uncompressed `/Metadata` stream) with the same values. Values left
    /// `None` fall back to [`BuildOptions::with_producer`] and
    /// [`BuildOptions::with_creation_date`].
    pub fn metadata(&mut self, metadata: Metadata) -> &mut Self {
        self.features.metadata = Some(metadata);
        self
    }

    /// The document's natural language (`/Lang`, §14.9.2.2), a BCP 47 tag such as `en-GB`.
    pub fn lang(&mut self, lang: impl Into<String>) -> &mut Self {
        self.features.lang = Some(lang.into());
        self
    }

    /// How viewers present the document when it opens.
    pub fn viewer_preferences(&mut self, prefs: ViewerPreferences) -> &mut Self {
        self.features.prefs = Some(prefs);
        self
    }

    /// Attaches `file` to the document (`/EmbeddedFiles` and the catalog's `/AF`, with its
    /// `/AFRelationship`): PDF/A-3b's associated files, such as an e-invoice's XML. Attaching a
    /// file with the same name again replaces it; files are written sorted by name. PDF/A-2b
    /// and PDF/A-2u refuse attachments at [`Builder::finish`] (`embedded-file-forbidden`).
    pub fn attach(&mut self, file: Attachment) -> &mut Self {
        self.features.attachments.insert(file.name().to_owned(), file);
        self
    }

    /// What the tagged document lacks so far ([B8a]): for each finished page, the content
    /// drawn outside every structure element and artifact, and the figures without alternate
    /// text, in page order. Empty while the document isn't tagged (see
    /// [`BuildOptions::with_tagged`]).
    ///
    /// [B8a]: https://github.com/laudebugs/laminapdf/blob/main/docs/todos/done/B8a-structure-tree.md
    #[must_use]
    pub fn tagging_diagnostics(&self) -> &[TagDiagnostic] {
        self.state.tag.diagnostics()
    }

    /// A page of `size`, appended to the document by [`PageBuilder::finish`].
    pub fn page(&mut self, size: PageSize) -> PageBuilder<'_> {
        PageBuilder::new(
            Target {
                doc: &mut self.doc,
                state: &mut self.state,
            },
            size,
        )
    }

    /// Adds pages to `doc`, an open document ([ADR 0005] §6: the same code path, and the
    /// result saves incrementally). Of `options`, only [`BuildOptions::with_compress`]
    /// applies (the document keeps its own version, `/ID` and Info). See [`Appender`].
    ///
    /// # Errors
    /// As [`Appender::new`].
    ///
    /// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
    pub fn on<'d>(doc: &'d mut laminapdf_model::Document, options: &BuildOptions) -> Result<Appender<'d>> {
        Appender::new(doc, options)
    }

    /// Writes the fonts (subset and embedded), the links' targets, the page tree, the
    /// outline, named destinations, page labels, the catalog, the Info dictionary and XMP
    /// packet, and the trailer (`/Root`, `/Info`, `/ID`), and returns the document.
    ///
    /// # Errors
    /// [`Error::PageIndexOutOfRange`](crate::Error::PageIndexOutOfRange) for a link,
    /// outline item or destination naming a page the document doesn't have;
    /// [`Error::UnknownDestination`](crate::Error::UnknownDestination) for a name never
    /// defined; [`Error::InvalidPageLabels`](crate::Error::InvalidPageLabels);
    /// [`Error::InvalidGeometry`](crate::Error::InvalidGeometry) for a destination coordinate
    /// that isn't finite; [`Error::Text`](crate::Error::Text) when subsetting a font fails;
    /// [`Error::Cos`](crate::Error::Cos) when object numbers run out. Under a [`Profile`], the
    /// first rule the document breaks, checked before anything is written: one of the
    /// profile variants of [`Error`](crate::Error), from
    /// [`Error::ProfileVersionConflict`](crate::Error::ProfileVersionConflict) on (each code's
    /// page in `docs/errors/` names its profile and rule).
    pub fn finish(self) -> Result<laminapdf_ops::Document> {
        self.finish_inner().map(|(doc, _)| doc)
    }

    /// [`Builder::finish`], also returning the warnings embedding the fonts raised
    /// (`tounicode-conflict`, `whole-font-rebuilt`), in the order they were raised.
    ///
    /// # Errors
    /// As [`Builder::finish`].
    #[cfg(feature = "text")]
    pub fn finish_with_warnings(self) -> Result<(laminapdf_ops::Document, Vec<laminapdf_text::Warning>)> {
        self.finish_inner().map(|(doc, state)| (doc, state.font_warnings))
    }

    /// Writes the document's objects; returns it with the build's final state.
    fn finish_inner(mut self) -> Result<(laminapdf_ops::Document, BuildState)> {
        finish::new_document(
            &mut self.doc,
            &self.options,
            self.catalog,
            &mut self.state,
            &self.features,
        )?;
        let doc = laminapdf_ops::Document::from(laminapdf_model::Document::from(self.doc));
        Ok((doc, self.state))
    }
}

/// Adds pages at the end of an open document's page tree, through its overlay: existing
/// objects (and so every existing [`PageId`]) stay as they are, the root `/Pages` node gets
/// the new `/Kids` and `/Count`, and the document saves incrementally with `laminapdf-save`'s
/// [`IncrementalWriter`](https://docs.rs/laminapdf-save). New pages write `/Rotate` and
/// `/CropBox` themselves, so nothing is inherited from the existing tree.
///
/// The changes are not op-log transactions (there is no op for drawn content yet), so they
/// are made on a [`laminapdf_model::Document`], not through `laminapdf_ops`.
///
/// Images, ICC profiles and `ExtGState`s are written once per appender. Each appender (each
/// [`Builder::on`] call) writes its own: it doesn't look for equal objects already in the
/// document, so adding the same image in two separate calls embeds it twice. Add pages that
/// share resources through one appender.
///
/// A feature newer than the document's version (see [`PageBuilder::finish`]) raises it with
/// the catalog's `/Version` (§7.7.2) when the appender finishes.
#[derive(Debug)]
pub struct Appender<'d> {
    doc: &'d mut laminapdf_model::Document,
    state: BuildState,
}

impl<'d> Appender<'d> {
    /// An appender for `doc`; of `options`, only the compression of content streams applies.
    ///
    /// # Errors
    /// [`Error::Cos`](crate::Error::Cos) with `invalid-structure` when the catalog's `/Pages`
    /// is not an indirect page tree node; errors reading the catalog.
    pub fn new(doc: &'d mut laminapdf_model::Document, options: &BuildOptions) -> Result<Self> {
        let pages_root = finish::page_tree_root(doc.cos())?;
        Ok(Self {
            doc,
            state: BuildState::new(pages_root, !options.uncompressed, true, None),
        })
    }

    /// A page of `size`, appended by [`PageBuilder::finish`].
    pub fn page(&mut self, size: PageSize) -> PageBuilder<'_> {
        PageBuilder::new(
            Target {
                doc: self.doc.cos_mut(),
                state: &mut self.state,
            },
            size,
        )
    }

    /// Registers `font` for the new pages' text (as [`Builder::font`]).
    ///
    /// # Errors
    /// As [`Builder::font`].
    #[cfg(feature = "text")]
    pub fn font(&mut self, font: laminapdf_text::Font) -> Result<Font> {
        self.state.add_font(font, EmbedPolicy::Respect)
    }

    /// Registers `font` with an embedding `policy` (as [`Builder::font_with`]).
    ///
    /// # Errors
    /// As [`Builder::font`].
    #[cfg(feature = "text")]
    pub fn font_with(&mut self, font: laminapdf_text::Font, policy: EmbedPolicy) -> Result<Font> {
        self.state.add_font(font, policy)
    }

    /// One of the standard 14 fonts (as [`Builder::standard_font`]).
    ///
    /// # Errors
    /// As [`Builder::standard_font`].
    #[cfg(feature = "text")]
    pub fn standard_font(&mut self, font: Standard14) -> Result<Font> {
        self.state
            .add_font(laminapdf_text::Font::standard(font), EmbedPolicy::Respect)
    }

    /// Writes the fonts, links the new pages into the page tree and resolves the new links'
    /// targets against every page of the document (named destinations are not checked:
    /// the document may define them); returns the new pages' IDs in order.
    ///
    /// # Errors
    /// As [`Appender::new`], when the root node changed meanwhile;
    /// [`Error::PageIndexOutOfRange`](crate::Error::PageIndexOutOfRange) for a link to a page
    /// the document doesn't have; [`Error::Text`](crate::Error::Text) when subsetting fails.
    ///
    /// Targets are checked before anything changes, and a write that fails rolls the
    /// document's overlay back to where this call started.
    pub fn finish(self) -> Result<Vec<PageId>> {
        finish::finish_appended(self.doc, self.state)
    }
}
