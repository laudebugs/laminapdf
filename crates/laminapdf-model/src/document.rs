//! [`Document`]: a COS document with typed views and the [`IdAllocator`], and
//! the tolerant read helpers every view shares.

use std::fmt;
use std::ops;
use std::sync::Arc;

use laminapdf_cos::{ObjRef, Object, OpenOptions, Result, Source, Warning};

use crate::id::IdAllocator;
use crate::info::Info;
use crate::labels::PageLabels;
use crate::outline::Outline;
use crate::page::Pages;
use crate::tree::{NameTree, NumberTree};

/// An opened PDF with typed views ([ADR 0005] §4): [`laminapdf_cos::Document`] plus the
/// document-level [`IdAllocator`] (a view of the COS overlay's number allocator).
///
/// It dereferences to the COS document, so every COS read (`resolve`, `catalog`,
/// `warnings`, …) is available directly. Views borrow it (`Page<'d>` holds `&'d Document`),
/// read their fields lazily, and never cache copies of the graph. Per-field tolerance
/// follows ADR 0003: a malformed field reads as absent (or its default) with a [`Warning`]
/// recorded on the document, and strict mode turns that warning into [`Error::Strict`](laminapdf_cos::Error::Strict).
///
/// Like the COS document it is `Send` but `!Sync`.
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
pub struct Document {
    cos: laminapdf_cos::Document,
}

// ADR 0005 §7: a document can move between threads.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<Document>();
};

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("cos", &self.cos)
            .finish_non_exhaustive()
    }
}

impl From<laminapdf_cos::Document> for Document {
    fn from(cos: laminapdf_cos::Document) -> Self {
        Self { cos }
    }
}

impl ops::Deref for Document {
    type Target = laminapdf_cos::Document;

    fn deref(&self) -> &laminapdf_cos::Document {
        &self.cos
    }
}

impl Document {
    /// Opens a PDF ([`laminapdf_cos::Document::open`]) for typed reading.
    ///
    /// # Errors
    /// As [`laminapdf_cos::Document::open`].
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        laminapdf_cos::Document::open(source, options).map(Self::from)
    }

    /// The COS document.
    #[must_use]
    pub const fn cos(&self) -> &laminapdf_cos::Document {
        &self.cos
    }

    /// The COS document for writing through its overlay ([`laminapdf_cos::Document::edit`]).
    /// `laminapdf-ops` holds the model document privately and hands out write access only
    /// inside a transaction.
    pub const fn cos_mut(&mut self) -> &mut laminapdf_cos::Document {
        &mut self.cos
    }

    /// The COS document, giving up the typed views (the overlay and the allocator stay
    /// with it).
    #[must_use]
    pub fn into_cos(self) -> laminapdf_cos::Document {
        self.cos
    }

    /// The document-level ID allocator.
    #[must_use]
    pub const fn ids(&self) -> IdAllocator<'_> {
        IdAllocator::new(&self.cos)
    }

    /// The pages, in page-tree order.
    ///
    /// # Errors
    /// As [`laminapdf_cos::Document::pages`].
    pub fn pages(&self) -> Result<Pages<'_>> {
        Pages::new(self)
    }

    /// The document outline (bookmarks); empty when the catalog has no `/Outlines`.
    ///
    /// # Errors
    /// [`Error::Strict`](laminapdf_cos::Error::Strict) in strict mode when an item is skipped; [`Error::DataPending`](laminapdf_cos::Error::DataPending) /
    /// [`Error::Source`](laminapdf_cos::Error::Source) when the input cannot be read; errors from
    /// [`laminapdf_cos::Document::catalog`].
    pub fn outline(&self) -> Result<Outline<'_>> {
        Outline::new(self)
    }

    /// The logical structure (the catalog's `/StructTreeRoot`, §14.7), if any.
    ///
    /// # Errors
    /// As [`Document::outline`].
    pub fn structure(&self) -> Result<Option<crate::StructTree>> {
        crate::structure::read(self)
    }

    /// The document information dictionary (the trailer's `/Info`), if any.
    ///
    /// # Errors
    /// As [`Document::outline`].
    pub fn info(&self) -> Result<Option<Info<'_>>> {
        Info::new(self)
    }

    /// The page labels (the catalog's `/PageLabels` number tree); empty when absent.
    ///
    /// # Errors
    /// As [`Document::outline`].
    pub fn page_labels(&self) -> Result<PageLabels> {
        PageLabels::new(self)
    }

    /// The name tree under the catalog's `/Names` dictionary at `key` (e.g. `"Dests"`,
    /// `"EmbeddedFiles"`), if present.
    ///
    /// # Errors
    /// As [`Document::outline`].
    pub fn name_tree(&self, key: &str) -> Result<Option<NameTree<'_>>> {
        let catalog = self.catalog()?;
        let Some(names) = catalog.get("Names") else {
            return Ok(None);
        };
        let resolved = tolerant(self.cos.deref(names))?;
        match resolved.as_deref() {
            Some(Object::Dict(names)) => Ok(names.get(key).map(|root| NameTree::new(self, root.clone()))),
            Some(Object::Null) => Ok(None),
            _ => {
                self.warn(Warning::InvalidEntry {
                    object: self.trailer().get("Root").and_then(Object::as_ref).map(|r| r.num),
                    key: "Names",
                })?;
                Ok(None)
            }
        }
    }

    /// A number tree rooted at `root` (a dictionary or a reference to one).
    #[must_use]
    pub fn number_tree(&self, root: Object) -> NumberTree<'_> {
        NumberTree::new(self, root)
    }

    /// Records `warning` on the document ([`laminapdf_cos::Document::report`]).
    pub(crate) fn warn(&self, warning: Warning) -> Result<()> {
        self.cos.report(warning)
    }

    /// The text string (§7.9.2.2) at `key` of `dict`, decoded to Unicode; `None` when absent,
    /// unreadable or not a string.
    pub(crate) fn text_entry(&self, dict: &laminapdf_cos::Dict, key: &str) -> Result<Option<String>> {
        let Some(value) = dict.get(key) else {
            return Ok(None);
        };
        Ok(match self.read(value)?.as_deref() {
            Some(Object::String(s)) => Some(laminapdf_syntax::text::decode_text_string(&s.bytes)),
            _ => None,
        })
    }

    /// Resolves `obj` tolerantly: `Ok(None)` when it cannot be read (malformed, a cycle,
    /// over a limit), the error only for those that [must propagate](laminapdf_cos::Error::must_propagate).
    pub(crate) fn read(&self, obj: &Object) -> Result<Option<Arc<Object>>> {
        Ok(tolerant(self.cos.deref(obj))?.map(laminapdf_cos::Resolved::into_arc))
    }
}

pub(crate) use laminapdf_cos::tolerant;

/// The object number of a reference, for warnings.
pub(crate) fn object_number(obj: &Object) -> Option<u32> {
    obj.as_ref().map(|r: ObjRef| r.num)
}
