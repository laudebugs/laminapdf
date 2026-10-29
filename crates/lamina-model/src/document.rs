//! [`Document`]: a COS document plus the model's interior state (the [`IdAllocator`]), and
//! the tolerant read helpers every view shares.

use std::fmt;
use std::ops;
use std::sync::Arc;

use lamina_cos::{Error, ObjRef, Object, OpenOptions, Result, Source, Warning};

use crate::id::IdAllocator;
use crate::info::Info;
use crate::labels::PageLabels;
use crate::outline::Outline;
use crate::page::Pages;
use crate::tree::{NameTree, NumberTree};

/// An opened PDF with typed views ([ADR 0005] §4): [`lamina_cos::Document`] plus the
/// document-level [`IdAllocator`].
///
/// It dereferences to the COS document, so every COS read (`resolve`, `catalog`,
/// `warnings`, …) is available directly. Views borrow it (`Page<'d>` holds `&'d Document`),
/// read their fields lazily, and never cache copies of the graph. Per-field tolerance
/// follows ADR 0003: a malformed field reads as absent (or its default) with a [`Warning`]
/// recorded on the document, and strict mode turns that warning into [`Error::Strict`].
///
/// Like the COS document it is `Send` but `!Sync`.
///
/// [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
pub struct Document {
    cos: lamina_cos::Document,
    ids: IdAllocator,
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

impl From<lamina_cos::Document> for Document {
    fn from(cos: lamina_cos::Document) -> Self {
        // Like the incremental writer: above every xref entry and the trailer's /Size, so a
        // dangling reference below /Size never resolves to a new object.
        let above_entries = cos.max_object_number().map_or(1, |n| n.saturating_add(1));
        let size = cos
            .trailer()
            .get_int("Size")
            .and_then(|n| u32::try_from(n).ok())
            .unwrap_or(0);
        let max = cos.limits().max_object_number();
        Self {
            ids: IdAllocator::new(above_entries.max(size), max),
            cos,
        }
    }
}

impl ops::Deref for Document {
    type Target = lamina_cos::Document;

    fn deref(&self) -> &lamina_cos::Document {
        &self.cos
    }
}

impl Document {
    /// Opens a PDF ([`lamina_cos::Document::open`]) for typed reading.
    ///
    /// # Errors
    /// As [`lamina_cos::Document::open`].
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        lamina_cos::Document::open(source, options).map(Self::from)
    }

    /// The COS document.
    #[must_use]
    pub const fn cos(&self) -> &lamina_cos::Document {
        &self.cos
    }

    /// The COS document, giving up the model state.
    #[must_use]
    pub fn into_cos(self) -> lamina_cos::Document {
        self.cos
    }

    /// The document-level ID allocator.
    #[must_use]
    pub const fn ids(&self) -> &IdAllocator {
        &self.ids
    }

    /// The pages, in page-tree order.
    ///
    /// # Errors
    /// As [`lamina_cos::Document::pages`].
    pub fn pages(&self) -> Result<Pages<'_>> {
        Pages::new(self)
    }

    /// The document outline (bookmarks); empty when the catalog has no `/Outlines`.
    ///
    /// # Errors
    /// [`Error::Strict`] in strict mode when an item is skipped; [`Error::DataPending`] /
    /// [`Error::Source`] when the input cannot be read; errors from
    /// [`lamina_cos::Document::catalog`].
    pub fn outline(&self) -> Result<Outline<'_>> {
        Outline::new(self)
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

    /// Records `warning` on the document ([`lamina_cos::Document::report`]).
    pub(crate) fn warn(&self, warning: Warning) -> Result<()> {
        self.cos.report(warning)
    }

    /// Resolves `obj` tolerantly: `Ok(None)` when it cannot be read (malformed, a cycle,
    /// over a limit), the error only for [`must_propagate`] ones.
    pub(crate) fn read(&self, obj: &Object) -> Result<Option<Arc<Object>>> {
        Ok(tolerant(self.cos.deref(obj))?.map(lamina_cos::Resolved::into_arc))
    }
}

/// Errors no view may swallow: strict-mode refusals (the caller asked for them) and input
/// that is not available (the call has to be retried or fail).
pub(crate) const fn must_propagate(e: &Error) -> bool {
    matches!(e, Error::Strict { .. } | Error::DataPending { .. } | Error::Source(_))
}

/// `Ok(Some(v))`, `Ok(None)` for an unreadable value, or a [`must_propagate`] error.
pub(crate) fn tolerant<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(e) if must_propagate(&e) => Err(e),
        Err(_) => Ok(None),
    }
}

/// The object number of a reference, for warnings.
pub(crate) fn object_number(obj: &Object) -> Option<u32> {
    obj.as_ref().map(|r: ObjRef| r.num)
}
