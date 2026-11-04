//! The spike-era writer, kept with its tests beside the product writers (ADR 0007 §7):
//! classic files, compact (object + xref stream) files built from objects numbered `1..n`,
//! and [`incremental_update`] from an explicit list of changes. It builds whole files in
//! memory and returns `laminapdf_cos` errors. Its xref sections, object stream and header come
//! from the helpers the product writers use (`xref.rs`, `full.rs`), so it writes nothing they
//! don't; only the order of an xref stream's `/Size` and `/Index` is its own (the frozen
//! reference files keep it).
//!
//! Saving an edited document goes through [`IncrementalWriter`](crate::IncrementalWriter) and
//! [`FullWriter`](crate::FullWriter) instead, which read the overlay and stream to a
//! [`WriteSink`](crate::WriteSink). [`Writer`] stays as the way tests (and, until
//! `laminapdf-build`, generation code) build files from scratch. Its output is byte-deterministic
//! (ADR 0001): every map it iterates is ordered, and running it twice on the same input gives
//! identical bytes.

use std::collections::BTreeMap;

use laminapdf_cos::{Document, Error, Result};
use laminapdf_syntax::{Dict, ObjRef, Object};

use crate::SaveError;
use crate::full::header;
use crate::options::XrefEol;
use crate::xref::{
    Row, Rows, classic_table, document_keys, document_keys_and_size, is_xref_key, object_stream, stream_object,
    stream_table,
};

/// The body of an indirect object: a structured [`Object`], or pre-serialized bytes for
/// callers that need exact control over formatting (e.g. reproducing hand-written files).
#[derive(Debug, Clone, PartialEq)]
enum Body {
    Object(Object),
    Raw(Vec<u8>),
}

/// Builds a PDF from indirect objects numbered `1..=n` in insertion order.
///
/// # Examples
/// ```
/// use laminapdf_cos::{Dict, Document, Object, OpenOptions};
/// use laminapdf_save::test_support::Writer;
/// let mut w = Writer::new(1, 7);
/// let pages = w.reserve();
/// let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>());
/// w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())]
///     .into_iter().collect::<Dict>());
/// let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
/// assert_eq!(Document::open(bytes, OpenOptions::default())?.page_count()?, 1);
/// # Ok::<(), laminapdf_cos::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Writer {
    version: (u8, u8),
    eol: XrefEol,
    objects: Vec<Option<Body>>,
}

impl Writer {
    /// A writer emitting `%PDF-major.minor`.
    #[must_use]
    pub const fn new(major: u8, minor: u8) -> Self {
        Self {
            version: (major, minor),
            eol: XrefEol::CrLf,
            objects: Vec::new(),
        }
    }

    /// Chooses the xref entry EOL form.
    #[must_use]
    pub const fn with_xref_eol(mut self, eol: XrefEol) -> Self {
        self.eol = eol;
        self
    }

    fn next_ref(&self) -> ObjRef {
        // Object numbers beyond u32 cannot be expressed in PDF; saturate rather than panic.
        ObjRef::new(u32::try_from(self.objects.len() + 1).unwrap_or(u32::MAX), 0)
    }

    /// Adds an object, returning its reference.
    pub fn add(&mut self, obj: impl Into<Object>) -> ObjRef {
        let r = self.next_ref();
        self.objects.push(Some(Body::Object(obj.into())));
        r
    }

    /// Adds an object whose body is written verbatim between `n 0 obj\n` and `\nendobj`.
    pub fn add_raw(&mut self, body: impl Into<Vec<u8>>) -> ObjRef {
        let r = self.next_ref();
        self.objects.push(Some(Body::Raw(body.into())));
        r
    }

    /// Reserves an object number to be filled later with [`Writer::set`] (for forward references).
    pub fn reserve(&mut self) -> ObjRef {
        let r = self.next_ref();
        self.objects.push(None);
        r
    }

    /// Sets the value of a reserved (or existing) object.
    pub fn set(&mut self, r: ObjRef, obj: impl Into<Object>) {
        let Some(idx) = usize::try_from(r.num).ok().and_then(|n| n.checked_sub(1)) else {
            return;
        };
        if idx >= self.objects.len() {
            self.objects.resize(idx + 1, None);
        }
        if let Some(slot) = self.objects.get_mut(idx) {
            *slot = Some(Body::Object(obj.into()));
        }
    }

    fn header(&self, compact: bool) -> Vec<u8> {
        let (major, minor) = self.version;
        header(Some(&format!("{major}.{minor}")), compact)
    }

    fn bodies(&self) -> impl Iterator<Item = Result<(u32, &Body)>> {
        self.objects
            .iter()
            .zip(1u32..)
            .map(|(b, num)| b.as_ref().map(|b| (num, b)).ok_or(Error::UnsetObject(num)))
    }

    fn size(&self) -> u32 {
        self.next_ref().num
    }

    /// Writes a classic file: header, objects, xref table, trailer, `startxref`, `%%EOF`.
    ///
    /// The trailer is `trailer` with `/Size` set to the object count: written first when
    /// absent, kept in place (value overwritten) when the caller supplied one.
    ///
    /// # Errors
    /// [`Error::UnsetObject`] if a reserved object was never set; [`Error::InvalidStructure`]
    /// for an offset over ten digits.
    pub fn finish(&self, trailer: &Dict) -> Result<Vec<u8>> {
        let mut out = self.header(false);
        let mut rows = Rows::from([(0, FREE_HEAD)]);
        for item in self.bodies() {
            let (num, body) = item?;
            rows.insert(num, in_file(out.len(), 0));
            write_indirect(num, body, &mut out);
        }
        let xref = out.len();
        out.extend_from_slice(&classic_table(&rows, self.eol).map_err(cos)?);
        // `/Size` goes first unless the caller placed it; its value is always recomputed.
        let mut t = if trailer.contains_key("Size") {
            trailer.clone()
        } else {
            let mut fresh = Dict::new();
            fresh.insert("Size", self.size()); // placeholder that fixes `/Size` as the first key
            for (k, v) in trailer.iter() {
                fresh.insert(k.clone(), v.clone());
            }
            fresh
        };
        t.insert("Size", self.size());
        write_trailer(&t, xref, &mut out);
        Ok(out)
    }

    /// Writes a compact PDF 1.5+ file: stream (and raw) objects stay top-level, every other
    /// object is packed into one Flate object stream, and the xref is a Flate xref stream
    /// (`/W [1 4 2]`, wider past 4 GiB). The header is raised to 1.5 if lower.
    ///
    /// # Errors
    /// [`Error::UnsetObject`] for unset reservations; [`Error::InvalidStructure`] if the object
    /// stream would hold more than 65 536 objects.
    pub fn finish_compact(&self, trailer: &Dict) -> Result<Vec<u8>> {
        let mut out = self.header(true);
        let mut rows = Rows::from([(0, FREE_HEAD)]);
        let mut packed: Vec<(u32, &Object)> = Vec::new();
        for item in self.bodies() {
            let (num, body) = item?;
            match body {
                Body::Object(obj) if !matches!(obj, Object::Stream(_)) => packed.push((num, obj)),
                _ => {
                    rows.insert(num, in_file(out.len(), 0));
                    write_indirect(num, body, &mut out);
                }
            }
        }
        let mut next = self.size();
        if !packed.is_empty() {
            if packed.len() > usize::from(u16::MAX) + 1 {
                return Err(Error::InvalidStructure {
                    kind: "compact output",
                    message: "more than 65535 packed objects",
                });
            }
            let objstm_num = next;
            next += 1;
            let (stream, members) = object_stream(objstm_num, &packed, |obj, body| {
                obj.write_to(body);
                Ok(())
            })
            .map_err(cos)?;
            rows.extend(members);
            rows.insert(objstm_num, in_file(out.len(), 0));
            write_indirect(objstm_num, &Body::Object(stream.into()), &mut out);
        }
        let xref_num = next;
        let size = xref_num + 1;
        let xref_off = out.len();
        rows.insert(xref_num, in_file(xref_off, 0));
        let (table, data) = stream_table(&rows);
        let mut dict = with_size(&table, size);
        for (k, v) in trailer.iter().filter(|(k, _)| !is_xref_key(k)) {
            dict.insert(k.clone(), v.clone());
        }
        out.extend_from_slice(&stream_object(xref_num, dict, &data));
        out.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        Ok(out)
    }
}

/// Object 0, the head of an empty free list (§7.5.4).
const FREE_HEAD: Row = Row::Free {
    next: 0,
    generation: u16::MAX,
};

/// The row of an object at byte `offset`.
fn in_file(offset: usize, generation: u16) -> Row {
    Row::InFile {
        offset: u64::try_from(offset).unwrap_or(u64::MAX),
        generation,
    }
}

/// The xref helpers fail only with a `laminapdf_cos` error, which this writer's API returns.
fn cos(error: SaveError) -> Error {
    match error {
        SaveError::Cos(e) => e,
        _ => Error::InvalidStructure {
            kind: "writer",
            message: "an unexpected save error",
        },
    }
}

/// `table` (from [`stream_table`]: `/Type`, `/W`, `/Index`) with `/Size` before `/Index`, the
/// order this writer has always used (last if `table` has no `/Index`).
fn with_size(table: &Dict, size: impl Into<Object>) -> Dict {
    let mut size = Some(size.into());
    let mut dict = Dict::new();
    for (k, v) in table.iter() {
        if k.as_bytes() == b"Index"
            && let Some(size) = size.take()
        {
            dict.insert("Size", size);
        }
        dict.insert(k.clone(), v.clone());
    }
    if let Some(size) = size {
        dict.insert("Size", size);
    }
    dict
}

fn write_indirect(num: u32, body: &Body, out: &mut Vec<u8>) {
    out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
    match body {
        Body::Object(obj) => obj.write_to(out),
        Body::Raw(raw) => out.extend_from_slice(raw),
    }
    out.extend_from_slice(b"\nendobj\n");
}

fn write_trailer(trailer: &Dict, xref: usize, out: &mut Vec<u8>) {
    out.extend_from_slice(b"trailer\n");
    Object::Dict(trailer.clone()).write_to(out);
    out.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
}

/// Appends an incremental update (§7.5.6) to `base`'s input bytes: the changed/new objects,
/// then a new cross-reference section of the same kind as the newest one in `base`, so `/Prev` chains
/// never mix kinds and objects in object streams stay visible to every reader:
///
/// * after a classic table (including a hybrid file's): a classic section (a `0 1` free-list
///   head plus one subsection per run of consecutive numbers) and a trailer repeating the
///   previous trailer's keys, with `/Size` updated and `/Prev` last;
/// * after an xref stream: an uncompressed-row, Flate-compressed xref stream
///   (`/Type /XRef /W [1 n 2] /Size /Index`, then the previous trailer's document keys, then
///   `/Prev`), numbered with the next free object number.
///
/// `/Size` never shrinks: it is the largest of the previous `/Size`, the highest object number
/// in any section + 1, and the highest new number + 1. The original bytes are copied untouched.
///
/// ```
/// use laminapdf_cos::{Document, ObjRef, Object, OpenOptions};
/// use laminapdf_save::XrefEol;
/// use laminapdf_save::test_support::{Writer, incremental_update};
///
/// let mut w = Writer::new(1, 7);
/// let root = w.add([("Type", Object::name("Catalog"))].into_iter().collect::<laminapdf_cos::Dict>());
/// let base = Document::open(w.finish(&[("Root", root)].into_iter().collect())?, OpenOptions::default())?;
/// let updated = incremental_update(&base, &[(ObjRef::new(2, 0), Object::Integer(42))], XrefEol::CrLf)?;
/// let doc = Document::open(updated, OpenOptions::default())?;
/// assert_eq!((doc.revisions(), doc.object(2)?.as_int()), (2, Some(42)));
/// # Ok::<(), laminapdf_cos::Error>(())
/// ```
///
/// # Errors
/// [`Error::RepairedBase`] when `base`'s xref was repaired (there is no valid section for
/// `/Prev` to point at); [`Error::EncryptedBase`] when it is encrypted (the new objects would
/// have to be encrypted too); [`Error::InvalidStructure`] when object numbers run out;
/// [`Error::DataPending`] / [`Error::Source`] when `base` was opened from a source without an
/// in-memory slice and its bytes can't all be read (the original bytes are copied whole).
pub fn incremental_update(base: &Document, changes: &[(ObjRef, Object)], eol: XrefEol) -> Result<Vec<u8>> {
    let doc = base;
    let prev = doc.startxref().ok_or(Error::RepairedBase)?;
    if doc.is_encrypted() {
        return Err(Error::EncryptedBase);
    }
    let original = doc.whole_input()?;
    let by_num: BTreeMap<u32, (u16, &Object)> = changes.iter().map(|(r, o)| (r.num, (r.gen_nr, o))).collect();

    let mut out = original.into_owned();
    if !matches!(out.last(), Some(b'\n' | b'\r')) {
        out.push(b'\n');
    }
    let mut rows = Rows::new();
    for (&num, &(gen_nr, obj)) in &by_num {
        rows.insert(num, in_file(out.len(), gen_nr));
        out.extend_from_slice(format!("{num} {gen_nr} obj\n").as_bytes());
        obj.write_to(&mut out);
        out.extend_from_slice(b"\nendobj\n");
    }
    // Object 0 stays the free-list head whatever the changes say.
    rows.remove(&0);

    // The base's trailer: an overlay trailer may name objects this update does not write.
    let old = doc.base_trailer();
    let size = doc
        .size_floor()
        .max(by_num.keys().next_back().map_or(0, |&n| i64::from(n) + 1));
    let prev = i64::try_from(prev).unwrap_or(i64::MAX);
    if doc.newest_section_is_stream() {
        append_xref_stream(&mut out, old, rows, size, prev)?;
    } else {
        rows.insert(0, FREE_HEAD);
        append_xref_table(&mut out, old, &rows, size, prev, eol)?;
    }
    Ok(out)
}

/// A classic section over `rows` and a trailer repeating `old`'s keys, `/Size` in its old place
/// (first when it had none) and `/Prev` last.
fn append_xref_table(out: &mut Vec<u8>, old: &Dict, rows: &Rows, size: i64, prev: i64, eol: XrefEol) -> Result<()> {
    let xref = out.len();
    out.extend_from_slice(&classic_table(rows, eol).map_err(cos)?);
    let mut trailer = Dict::new();
    for (k, v) in document_keys_and_size(old) {
        trailer.insert(k.clone(), v.clone());
    }
    trailer.insert("Size", size);
    trailer.insert("Prev", prev);
    write_trailer(&trailer, xref, out);
    Ok(())
}

/// An xref stream over `rows` plus itself, numbered `size`, with `old`'s document keys and
/// `/Prev`.
fn append_xref_stream(out: &mut Vec<u8>, old: &Dict, mut rows: Rows, size: i64, prev: i64) -> Result<()> {
    let out_of_numbers = || Error::InvalidStructure {
        kind: "incremental update",
        message: "object numbers exhausted",
    };
    let xref_num = u32::try_from(size).map_err(|_| out_of_numbers())?;
    let new_size = xref_num.checked_add(1).ok_or_else(out_of_numbers)?;
    let xref_off = out.len();
    rows.insert(xref_num, in_file(xref_off, 0));
    let (table, data) = stream_table(&rows);
    let mut dict = with_size(&table, new_size);
    for (k, v) in document_keys(old) {
        dict.insert(k.clone(), v.clone());
    }
    dict.insert("Prev", prev);
    out.extend_from_slice(&stream_object(xref_num, dict, &data));
    out.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
    Ok(())
}
