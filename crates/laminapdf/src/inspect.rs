//! Read-only inspection for power users and developers: the version and the standards a file
//! claims, its fonts and colour profiles, and its objects (each with its value, stream data raw
//! or decoded within [`Limits`](crate::Limits), and what refers to it). Nothing here changes the
//! document, and nothing is validated: a claim is what the file says. The revisions are
//! [`Document::revisions`].

use std::sync::Arc;

use laminapdf_cos::{Dict, Object, StringFormat};
use laminapdf_model::{ColorProfiles, ConformanceClaims, ObjectEntry, ReferenceIndex, Referrer};

use crate::document::Document;
use crate::error::Result;

/// One object with its value ([`Document::object`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct InspectedObject {
    /// Its number, kind, `/Type` and where it is stored.
    pub entry: ObjectEntry,
    /// Its value as this session's edits leave it (a stream's dictionary for a stream);
    /// `null` when it can't be read.
    pub value: CosValue,
    /// A stream's filters and stored length; `None` for any other object.
    pub stream: Option<StreamFacts>,
    /// The 0-based revision whose cross-reference section last lists it; `None` for an object
    /// this session created, or a file whose cross-reference table was rebuilt.
    pub revision: Option<u32>,
}

/// A PDF value as plain data (ISO 32000-2 §7.3), for showing an object.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CosValue {
    /// `null`.
    Null,
    /// `true` or `false`.
    Boolean(bool),
    /// An integer.
    Integer(i64),
    /// A real number.
    Real(f64),
    /// A string's bytes, as text, and whether the file wrote it in hexadecimal.
    String {
        /// The bytes.
        bytes: Vec<u8>,
        /// The bytes decoded as a text string (§7.9.2.2: UTF-16 or UTF-8 with a byte order
        /// mark, else `PDFDocEncoding`).
        text: String,
        /// Written as `<…>`.
        hex: bool,
    },
    /// A name, without its `/`, lossily decoded.
    Name(String),
    /// An array.
    Array(Vec<CosValue>),
    /// A dictionary's entries, in the file's order, keys without their `/`.
    Dictionary(Vec<(String, CosValue)>),
    /// A reference to an indirect object.
    Reference {
        /// Its number.
        num: u32,
        /// Its generation.
        generation: u16,
    },
}

impl CosValue {
    fn of(object: &Object) -> Self {
        let name = |bytes: &[u8]| String::from_utf8_lossy(bytes).into_owned();
        match object {
            Object::Null => Self::Null,
            Object::Bool(b) => Self::Boolean(*b),
            Object::Integer(i) => Self::Integer(*i),
            Object::Real(r) => Self::Real(*r),
            Object::String(s) => Self::String {
                bytes: s.bytes.clone(),
                text: laminapdf_syntax::text::decode_text_string(&s.bytes),
                hex: s.format == StringFormat::Hex,
            },
            Object::Name(n) => Self::Name(name(n.as_bytes())),
            Object::Array(items) => Self::Array(items.iter().map(Self::of).collect()),
            Object::Dict(dict) => Self::dictionary(dict),
            Object::Stream(stream) => Self::dictionary(&stream.dict),
            Object::Ref(r) => Self::Reference {
                num: r.num,
                generation: r.gen_nr,
            },
        }
    }

    fn dictionary(dict: &Dict) -> Self {
        Self::Dictionary(
            dict.iter()
                .map(|(k, v)| (String::from_utf8_lossy(k.as_bytes()).into_owned(), Self::of(v)))
                .collect(),
        )
    }
}

/// A stream's filters and stored length ([`InspectedObject::stream`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StreamFacts {
    /// The `/Filter` names, in the order they decode (`FlateDecode`); empty for none.
    pub filters: Vec<String>,
    /// The data's length as stored, in bytes.
    pub length: u64,
}

/// What the inspection reads keep while the document is unchanged ([`Document`]'s cache, as its
/// page facts are kept).
#[derive(Debug, Default)]
pub(crate) struct InspectionCache {
    /// Who holds a reference to each object, indexed on the first `references_to`.
    references: Option<Arc<ReferenceIndex>>,
    /// The revisions' cross-reference facts.
    revisions: Option<Arc<Vec<laminapdf_cos::Revision>>>,
}

/// Which bytes of a stream to read ([`Document::stream_bytes`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum StreamView {
    /// The bytes as stored in the file, still encoded by its `/Filter`s.
    Raw,
    /// The bytes decoded, within the document's [`Limits`](crate::Limits).
    Decoded,
}

/// The first bytes of a stream ([`Document::stream_bytes`]).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct StreamBytes {
    /// At most the bytes asked for, from the start.
    pub bytes: Vec<u8>,
    /// The stream's whole length in this view, in bytes; `None` for a decoded stream longer than
    /// the bytes asked for, which is never decoded further to find out.
    pub total: Option<u64>,
}

impl Document {
    /// The version the file declares and the standards it claims (PDF/A, PDF/UA, PDF/X), each
    /// claimed, not verified.
    ///
    /// ```
    /// # use laminapdf::{Document, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/examples/pdf_a3_invoice_with_xml.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let claims = doc.conformance()?;
    /// let pdfa = claims.pdfa.expect("the invoice claims PDF/A-3");
    /// assert_eq!((pdfa.part, pdfa.conformance.as_deref()), (3, Some("B")));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors (the catalog, the metadata stream).
    ///
    /// Added in 0.6.0.
    pub fn conformance(&self) -> Result<ConformanceClaims> {
        Ok(self.ops().conformance()?)
    }

    /// The output intents and every ICC profile the document carries, each profile with what
    /// its header says. It reads every object once, so call it when a view opens.
    ///
    /// # Errors
    /// Reading errors (`data-pending` for a range document, a failed source).
    ///
    /// Added in 0.6.0.
    pub fn color_profiles(&self) -> Result<ColorProfiles> {
        Ok(self.ops().color_profiles()?)
    }

    /// The trailer as it is now (for a cross-reference stream, the stream's dictionary): the
    /// root of the object tree, with `/Root`, `/Info`, `/ID` and `/Size`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn trailer(&self) -> CosValue {
        CosValue::dictionary(self.ops().trailer())
    }

    /// Every object that exists now, by ascending number, with its kind, `/Type` and where it
    /// is stored. Each is read once (object streams decoded within
    /// [`Limits`](crate::Limits)); one that can't be read is listed as `null`.
    ///
    /// ```
    /// # use laminapdf::{Document, ObjectKind, OpenOptions};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let streams = doc.objects()?.iter().filter(|o| o.kind == ObjectKind::Stream).count();
    /// assert!(streams > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// Reading errors that aren't repairs (`data-pending`, a failed source).
    ///
    /// Added in 0.6.0.
    pub fn objects(&self) -> Result<Vec<ObjectEntry>> {
        Ok(self.ops().objects().collect::<laminapdf_cos::Result<_>>()?)
    }

    /// Object `num` with its value and the revision that last wrote it; `None` when the
    /// document has no such object.
    ///
    /// # Errors
    /// Reading errors that aren't repairs.
    ///
    /// Added in 0.6.0.
    pub fn object(&self, num: u32) -> Result<Option<InspectedObject>> {
        let model = self.ops();
        if model.object_location(num).is_none() {
            return Ok(None);
        }
        // Read once: the entry is made from the same value.
        let object = laminapdf_cos::tolerant(model.object(num))?;
        let Some(entry) = model.entry_of_number(num, object.as_deref()) else {
            return Ok(None);
        };
        let object = object.as_deref().unwrap_or(&Object::Null);
        let stream = match object {
            Object::Stream(s) => Some(StreamFacts {
                filters: model.stream_filters(&s.dict),
                length: u64::try_from(s.data.len()).unwrap_or(u64::MAX),
            }),
            _ => None,
        };
        let revision = self
            .revision_index()?
            .iter()
            .rposition(|r| r.in_use.binary_search(&num).is_ok())
            .and_then(|i| u32::try_from(i).ok());
        Ok(Some(InspectedObject {
            entry,
            value: CosValue::of(object),
            stream,
            revision,
        }))
    }

    /// The first `max` bytes of stream object `num`, raw or decoded, with the stream's whole
    /// length in that view; `None` when object `num` isn't a stream. Decoding never goes past the document's
    /// [`Limits`](crate::Limits): a stream that decodes to more is refused, and its raw bytes
    /// can still be read.
    ///
    /// ```
    /// # use laminapdf::{Document, ObjectKind, OpenOptions, StreamView};
    /// # let bytes = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/corpus/hello.pdf"))?;
    /// let doc = Document::open_bytes(bytes, OpenOptions::default())?;
    /// let objects = doc.objects()?;
    /// let stream = objects.iter().find(|o| o.kind == ObjectKind::Stream).expect("a page's contents");
    /// let start = doc.stream_bytes(stream.id.num, StreamView::Decoded, 64)?.expect("a stream");
    /// assert!(start.bytes.len() <= 64);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// `limit-exceeded` when it decodes to more than
    /// [`Limits::max_stream_decoded`](crate::Limits::max_stream_decoded); decoding errors.
    ///
    /// Added in 0.6.0.
    pub fn stream_bytes(&self, num: u32, view: StreamView, max: usize) -> Result<Option<StreamBytes>> {
        let model = self.ops();
        let object = model.object(num)?;
        let Object::Stream(stream) = &*object else {
            return Ok(None);
        };
        let len = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
        if view == StreamView::Raw {
            return Ok(Some(StreamBytes {
                bytes: stream
                    .data
                    .get(..max.min(stream.data.len()))
                    .unwrap_or_default()
                    .to_vec(),
                total: Some(len(stream.data.len())),
            }));
        }
        // Decoded only as far as is shown, and one byte more to know whether there is more.
        let (bytes, complete) = model.decode_stream_prefix(stream, max)?;
        let total = complete.then(|| len(bytes.len()));
        Ok(Some(StreamBytes { bytes, total }))
    }

    /// The places that refer to object `num`: the trailer's keys, then each object's, with the
    /// path to the reference inside it (`/Kids[2]`). The first call reads every object to index
    /// them; later calls reuse the index until the document changes.
    ///
    /// # Errors
    /// Reading errors that aren't repairs.
    ///
    /// Added in 0.6.0.
    pub fn references_to(&self, num: u32) -> Result<Vec<Referrer>> {
        let index = self.reference_index()?;
        Ok(self.ops().referrers(&index, num)?)
    }

    /// How many objects exist now, without reading any: the inspector's count.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn object_count(&self) -> u32 {
        u32::try_from(self.ops().object_numbers().count()).unwrap_or(u32::MAX)
    }

    /// Object `num`'s number, kind, `/Type` and where it is stored; `None` when the document has
    /// no such object.
    ///
    /// # Errors
    /// Reading errors that aren't repairs.
    ///
    /// Added in 0.6.0.
    pub fn object_entry(&self, num: u32) -> Result<Option<ObjectEntry>> {
        Ok(self.ops().object_entry(num)?)
    }

    /// The object `num` written as PDF syntax (`12 0 obj … endobj`), to copy as text. A
    /// stream's dictionary is written with a comment in place of its data.
    ///
    /// # Errors
    /// Reading errors that aren't repairs.
    ///
    /// Added in 0.6.0.
    pub fn object_syntax(&self, num: u32) -> Result<Option<String>> {
        let model = self.ops();
        let Some(entry) = model.object_entry(num)? else {
            return Ok(None);
        };
        let object = laminapdf_cos::tolerant(model.object(num))?.unwrap_or_else(|| Arc::new(Object::Null));
        let mut out = format!("{} {} obj\n", entry.id.num, entry.id.gen_nr).into_bytes();
        match &*object {
            Object::Stream(stream) => {
                Object::Dict(stream.dict.clone()).write_to(&mut out);
                out.extend_from_slice(format!("\nstream\n% {} bytes\nendstream", stream.data.len()).as_bytes());
            }
            value => value.write_to(&mut out),
        }
        out.extend_from_slice(b"\nendobj\n");
        Ok(Some(String::from_utf8_lossy(&out).into_owned()))
    }

    fn reference_index(&self) -> Result<Arc<ReferenceIndex>> {
        if let Some(index) = self.inspection.borrow().references.as_ref() {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(self.ops().reference_index()?);
        self.inspection.borrow_mut().references = Some(Arc::clone(&index));
        Ok(index)
    }

    /// The file's revisions with what each lists ([`laminapdf_cos::Document::revision_index`]),
    /// read once and kept until the document changes.
    pub(crate) fn revision_index(&self) -> Result<Arc<Vec<laminapdf_cos::Revision>>> {
        if let Some(index) = self.inspection.borrow().revisions.as_ref() {
            return Ok(Arc::clone(index));
        }
        let index = Arc::new(self.ops().revision_index()?);
        self.inspection.borrow_mut().revisions = Some(Arc::clone(&index));
        Ok(index)
    }
}
