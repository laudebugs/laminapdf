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
use crate::xmp;

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
///
/// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        laminapdf_cos::Document::open(source, options).map(Self::from)
    }

    /// The COS document.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn cos(&self) -> &laminapdf_cos::Document {
        &self.cos
    }

    /// The COS document for writing through its overlay ([`laminapdf_cos::Document::edit`]).
    /// `laminapdf-ops` holds the model document privately and hands out write access only
    /// inside a transaction.
    ///
    /// Added in 0.2.0.
    pub const fn cos_mut(&mut self) -> &mut laminapdf_cos::Document {
        &mut self.cos
    }

    /// The COS document, giving up the typed views (the overlay and the allocator stay
    /// with it).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn into_cos(self) -> laminapdf_cos::Document {
        self.cos
    }

    /// The document-level ID allocator.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn ids(&self) -> IdAllocator<'_> {
        IdAllocator::new(&self.cos)
    }

    /// The XMP metadata packet (the catalog's `/Metadata` stream, ISO 32000-2 §14.3.2),
    /// decoded; `None` without one, or when its stream can't be decoded.
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.2.0.
    pub fn xmp_packet(&self) -> Result<Option<Vec<u8>>> {
        let catalog = self.catalog()?;
        let Some(metadata) = catalog.get("Metadata") else {
            return Ok(None);
        };
        let Some(object) = self.read(metadata)? else {
            return Ok(None);
        };
        let Object::Stream(stream) = &*object else {
            return Ok(None);
        };
        tolerant(self.decode_stream(stream))
    }

    /// The XMP metadata packet, parsed ([`xmp::Packet`]); [`xmp::PacketState::Malformed`]
    /// with [`Warning::XmpInvalid`] raised when the packet can't be read, so it counts as
    /// absent until an edit chooses to replace it.
    ///
    /// # Errors
    /// As [`Document::xmp_packet`]; [`Error::Strict`](laminapdf_cos::Error::Strict) in strict
    /// mode for a malformed packet.
    ///
    /// Added in 0.2.0.
    pub fn xmp(&self) -> Result<xmp::PacketState> {
        let Some(bytes) = self.xmp_packet()? else {
            return Ok(xmp::PacketState::Absent);
        };
        match xmp::Packet::parse(&bytes) {
            Ok(packet) => Ok(xmp::PacketState::Parsed(packet)),
            Err(error) => {
                let object = self.catalog()?.get("Metadata").and_then(Object::as_ref).map(|r| r.num);
                self.warn(Warning::XmpInvalid {
                    object,
                    offset: error.offset(),
                    reason: error.reason(),
                })?;
                Ok(xmp::PacketState::Malformed { bytes, error })
            }
        }
    }

    /// The PDF version the file declares (ISO 32000-2 §7.2.2): the catalog's `/Version` when
    /// it is later than the `%PDF-x.y` header's, else the header's (`"1.7"`); `None` with
    /// neither. A `/Version` that isn't `<digits>.<digits>` is ignored.
    ///
    /// # Errors
    /// Reading errors (the catalog).
    ///
    /// Added in 0.2.0.
    pub fn version(&self) -> Result<Option<String>> {
        let header = self.header_version().filter(|v| version_number(v).is_some());
        let catalog = self.catalog()?;
        let declared = catalog
            .get_name("Version")
            .and_then(|name| std::str::from_utf8(name).ok())
            .filter(|v| version_number(v).is_some());
        let later = match (header, declared) {
            (Some(header), Some(declared)) if version_number(declared) > version_number(header) => Some(declared),
            (Some(header), _) => Some(header),
            (None, declared) => declared,
        };
        Ok(later.map(str::to_owned))
    }

    /// Whether the file declares PDF 2.0 or later ([`Document::version`]), where ISO 32000-2
    /// §14.3.3 deprecates every Info entry but `/CreationDate` and `/ModDate`.
    ///
    /// # Errors
    /// As [`Document::version`].
    ///
    /// Added in 0.2.0.
    pub fn is_pdf2(&self) -> Result<bool> {
        Ok(self
            .version()?
            .as_deref()
            .and_then(version_number)
            .is_some_and(|(major, _)| major >= 2))
    }

    /// The pages, in page-tree order.
    ///
    /// # Errors
    /// As [`laminapdf_cos::Document::pages`].
    ///
    /// Added in 0.2.0.
    pub fn pages(&self) -> Result<Pages<'_>> {
        Pages::new(self)
    }

    /// Whether the document says it is tagged: its catalog's `/MarkInfo` has `/Marked true`
    /// (ISO 32000-2 §14.7.1). It doesn't check the structure tree.
    ///
    /// # Errors
    /// Reading errors (the catalog, `/MarkInfo`).
    ///
    /// Added in 0.2.0.
    pub fn is_tagged(&self) -> Result<bool> {
        let catalog = self.catalog()?;
        let Some(mark_info) = catalog.get("MarkInfo") else {
            return Ok(false);
        };
        let mark_info = self.deref(mark_info)?;
        let marked = mark_info.as_dict().and_then(|d| d.get("Marked"));
        Ok(matches!(marked, Some(Object::Bool(true))))
    }

    /// The document outline (bookmarks); empty when the catalog has no `/Outlines`.
    ///
    /// # Errors
    /// [`Error::Strict`](laminapdf_cos::Error::Strict) in strict mode when an item is skipped; [`Error::DataPending`](laminapdf_cos::Error::DataPending) /
    /// [`Error::Source`](laminapdf_cos::Error::Source) when the input cannot be read; errors from
    /// [`laminapdf_cos::Document::catalog`].
    ///
    /// Added in 0.2.0.
    pub fn outline(&self) -> Result<Outline<'_>> {
        Outline::new(self)
    }

    /// The logical structure (the catalog's `/StructTreeRoot`, §14.7), if any.
    ///
    /// # Errors
    /// As [`Document::outline`].
    ///
    /// Added in 0.2.0.
    pub fn structure(&self) -> Result<Option<crate::StructTree>> {
        crate::structure::read(self)
    }

    /// The document information dictionary (the trailer's `/Info`), if any.
    ///
    /// # Errors
    /// As [`Document::outline`].
    ///
    /// Added in 0.2.0.
    pub fn info(&self) -> Result<Option<Info<'_>>> {
        Info::new(self)
    }

    /// The page labels (the catalog's `/PageLabels` number tree); empty when absent.
    ///
    /// # Errors
    /// As [`Document::outline`].
    ///
    /// Added in 0.2.0.
    pub fn page_labels(&self) -> Result<PageLabels> {
        PageLabels::new(self)
    }

    /// The name tree under the catalog's `/Names` dictionary at `key` (e.g. `"Dests"`,
    /// `"EmbeddedFiles"`), if present.
    ///
    /// # Errors
    /// As [`Document::outline`].
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
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

/// A version string's numbers, `"1.7"` → `(1, 7)`; `None` unless it is `<digits>.<digits>`.
fn version_number(version: &str) -> Option<(u32, u32)> {
    let (major, minor) = version.split_once('.')?;
    // `u32::from_str` takes a leading `+`, which a version never has.
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !(digits(major) && digits(minor)) {
        return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?))
}
