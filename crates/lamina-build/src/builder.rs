//! [`Builder`] (a new document) and [`Appender`] (pages added to an open one).

use lamina_cos::{ObjRef, OpenOptions};
use lamina_model::{PageId, PdfDate};

use crate::error::Result;
use crate::finish;
use crate::page::{PageBuilder, PageSize};
use crate::resources::DocResources;

/// The header version of a new document (§7.5.2). Saving with object and cross-reference
/// streams (`lamina-save`'s default) raises anything below 1.5 to 1.5.
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
/// use lamina_build::{BuildOptions, PdfVersion};
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
    open: OpenOptions,
}

impl BuildOptions {
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

    /// `/Producer` in a minimal Info dictionary (the full Info and XMP are B5b's).
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
    /// The pages written so far, in order.
    pub(crate) pages: Vec<ObjRef>,
    pub(crate) resources: DocResources,
    pub(crate) compress: bool,
    /// Write `/Rotate` and `/CropBox` on every page, so nothing is inherited from an
    /// existing page tree's root.
    pub(crate) explicit_inherited: bool,
}

/// What a page writes into: the document and the build's shared state.
#[derive(Debug)]
pub(crate) struct Target<'b> {
    pub(crate) doc: &'b mut lamina_cos::Document,
    pub(crate) state: &'b mut BuildState,
}

/// Builds a new document ([ADR 0005] §6): pages, then [`Builder::finish`], which returns an
/// editable [`lamina_ops::Document`] whose objects all live in the overlay over an empty base.
/// Saving it (`lamina_save::Save`) writes a full file; it is `Dirty` until then.
///
/// ```
/// use lamina_build::{BuildOptions, Builder, PageSize};
///
/// let mut b = Builder::new(BuildOptions::default());
/// b.page(PageSize::A4).finish()?;
/// b.page(PageSize::LETTER.landscape()).finish()?;
/// let doc = b.finish()?;
/// assert_eq!((doc.pages()?.len(), doc.is_dirty()), (2, true));
/// # Ok::<(), lamina_build::Error>(())
/// ```
///
/// [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
#[derive(Debug)]
pub struct Builder {
    doc: lamina_cos::Document,
    options: BuildOptions,
    catalog: ObjRef,
    state: BuildState,
}

impl Builder {
    /// A builder for a new, empty document. The catalog and the page tree root are reserved
    /// first, so they are objects 1 and 2.
    #[must_use]
    pub fn new(options: BuildOptions) -> Self {
        let doc = lamina_cos::Document::empty(Some(options.version.as_str()), &options.open);
        // A fresh document starts numbering at 1, far below any limit; should reserving still
        // fail, `finish` reports it when it writes the reserved objects.
        let catalog = doc.reserve_number().unwrap_or(ObjRef::new(1, 0));
        let pages_root = doc.reserve_number().unwrap_or(ObjRef::new(2, 0));
        let compress = !options.uncompressed;
        Self {
            doc,
            options,
            catalog,
            state: BuildState {
                pages_root,
                pages: Vec::new(),
                resources: DocResources::default(),
                compress,
                explicit_inherited: false,
            },
        }
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
    /// result saves incrementally). See [`Appender`].
    ///
    /// # Errors
    /// As [`Appender::new`].
    ///
    /// [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
    pub fn on(doc: &mut lamina_model::Document) -> Result<Appender<'_>> {
        Appender::new(doc)
    }

    /// Writes the page tree, the catalog, the Info dictionary when [`BuildOptions`] asked
    /// for one, and the trailer (`/Root`, `/Info`, `/ID`), and returns the document.
    ///
    /// # Errors
    /// [`Error::Cos`](crate::Error::Cos) when object numbers run out.
    pub fn finish(mut self) -> Result<lamina_ops::Document> {
        finish::new_document(&mut self.doc, &self.options, self.catalog, &self.state)?;
        Ok(lamina_ops::Document::from(lamina_model::Document::from(self.doc)))
    }
}

/// Adds pages at the end of an open document's page tree, through its overlay: existing
/// objects (and so every existing [`PageId`]) stay as they are, the root `/Pages` node gets
/// the new `/Kids` and `/Count`, and the document saves incrementally with `lamina-save`'s
/// [`IncrementalWriter`](https://docs.rs/lamina-save). New pages write `/Rotate` and
/// `/CropBox` themselves, so nothing is inherited from the existing tree.
///
/// The changes are not op-log transactions (there is no op for drawn content yet), so they
/// are made on a [`lamina_model::Document`], not through `lamina_ops`.
#[derive(Debug)]
pub struct Appender<'d> {
    doc: &'d mut lamina_model::Document,
    state: BuildState,
}

impl<'d> Appender<'d> {
    /// An appender for `doc`.
    ///
    /// # Errors
    /// [`Error::Cos`](crate::Error::Cos) with `invalid-structure` when the catalog's `/Pages`
    /// is not an indirect page tree node; errors reading the catalog.
    pub fn new(doc: &'d mut lamina_model::Document) -> Result<Self> {
        let pages_root = finish::page_tree_root(doc.cos())?;
        Ok(Self {
            doc,
            state: BuildState {
                pages_root,
                pages: Vec::new(),
                resources: DocResources::default(),
                compress: true,
                explicit_inherited: true,
            },
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

    /// Links the new pages into the page tree; returns their IDs in order.
    ///
    /// # Errors
    /// As [`Appender::new`], when the root node changed meanwhile.
    pub fn finish(self) -> Result<Vec<PageId>> {
        finish::append_pages(self.doc.cos_mut(), &self.state)?;
        Ok(self.state.pages.iter().map(|r| PageId::object(*r)).collect())
    }
}
