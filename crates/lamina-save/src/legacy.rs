//! The spike-era writer, moved here from `lamina-cos` in B3b (ADR 0007 §7) with its tests:
//! classic files, compact (object + xref stream) files built from objects numbered `1..n`,
//! and [`incremental_update`] from an explicit list of changes. It builds whole files in
//! memory and returns `lamina_cos` errors.
//!
//! Saving an edited document goes through [`IncrementalWriter`](crate::IncrementalWriter) and
//! [`FullWriter`](crate::FullWriter) instead, which read the overlay and stream to a
//! [`WriteSink`](crate::WriteSink). [`Writer`] stays as the way tests (and, until
//! `lamina-build`, generation code) build files from scratch. Its output is byte-deterministic
//! (ADR 0001): every map it iterates is ordered, and running it twice on the same input gives
//! identical bytes.

use std::collections::BTreeMap;

use lamina_cos::{Document, Error, Result, XREF_STREAM_KEYS};
use lamina_syntax::{Dict, Name, ObjRef, Object, Stream};

/// The 20-byte cross-reference entry ends with a two-byte EOL; both forms are legal (§7.5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum XrefEol {
    /// `\r\n`.
    #[default]
    CrLf,
    /// ` \n`.
    SpaceLf,
}

impl XrefEol {
    pub(crate) const fn bytes(self) -> &'static [u8] {
        match self {
            Self::CrLf => b"\r\n",
            Self::SpaceLf => b" \n",
        }
    }
}

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
/// use lamina_cos::{Dict, Document, Object, OpenOptions};
/// use lamina_save::Writer;
/// let mut w = Writer::new(1, 7);
/// let pages = w.reserve();
/// let page = w.add([("Type", Object::name("Page")), ("Parent", pages.into())].into_iter().collect::<Dict>());
/// w.set(pages, [("Type", Object::name("Pages")), ("Kids", vec![page.into()].into()), ("Count", 1.into())]
///     .into_iter().collect::<Dict>());
/// let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
/// let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
/// assert_eq!(Document::open(bytes, OpenOptions::default())?.page_count()?, 1);
/// # Ok::<(), lamina_cos::Error>(())
/// ```
#[derive(Debug, Clone)]
pub struct Writer {
    version: (u8, u8),
    eol: XrefEol,
    objects: Vec<Option<Body>>,
}

pub(crate) const HEADER_MARKER: &[u8] = b"%\xE2\xE3\xCF\xD3\n";

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

    fn header(&self, min_minor: u8) -> Vec<u8> {
        let (major, minor) = if self.version < (1, min_minor) {
            (1, min_minor)
        } else {
            self.version
        };
        let mut out = format!("%PDF-{major}.{minor}\n").into_bytes();
        out.extend_from_slice(HEADER_MARKER);
        out
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
    /// [`Error::UnsetObject`] if a reserved object was never set.
    pub fn finish(&self, trailer: &Dict) -> Result<Vec<u8>> {
        let mut out = self.header(0);
        let mut offsets = Vec::with_capacity(self.objects.len());
        for item in self.bodies() {
            let (num, body) = item?;
            offsets.push(out.len());
            write_indirect(num, body, &mut out);
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n", self.size()).as_bytes());
        write_free_head(self.eol, &mut out);
        for off in offsets {
            write_in_use(off, 0, self.eol, &mut out);
        }
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
    /// (`/W [1 4 2]`). The header is raised to 1.5 if lower.
    ///
    /// # Errors
    /// [`Error::UnsetObject`] for unset reservations; [`Error::InvalidStructure`] if offsets
    /// exceed the 4-byte field or the object stream holds more than 65 535 objects.
    pub fn finish_compact(&self, trailer: &Dict) -> Result<Vec<u8>> {
        let too_big = |message| Error::InvalidStructure {
            kind: "compact output",
            message,
        };
        let mut out = self.header(5);
        let mut rows: BTreeMap<u32, (u8, u32, u16)> = BTreeMap::new();
        let mut packed: Vec<(u32, &Object)> = Vec::new();
        for item in self.bodies() {
            let (num, body) = item?;
            match body {
                Body::Object(obj) if !matches!(obj, Object::Stream(_)) => packed.push((num, obj)),
                _ => {
                    let off = u32::try_from(out.len()).map_err(|_| too_big("offset over 4 GiB"))?;
                    rows.insert(num, (1, off, 0));
                    write_indirect(num, body, &mut out);
                }
            }
        }
        let mut next = self.size();
        if !packed.is_empty() {
            let objstm_num = next;
            next += 1;
            let mut index_part = Vec::new();
            let mut body_part = Vec::new();
            for (i, (num, obj)) in packed.iter().enumerate() {
                let idx = u16::try_from(i).map_err(|_| too_big("more than 65535 packed objects"))?;
                rows.insert(*num, (2, objstm_num, idx));
                if i > 0 {
                    index_part.push(b' ');
                }
                index_part.extend_from_slice(format!("{num} {}", body_part.len()).as_bytes());
                obj.write_to(&mut body_part);
                body_part.push(b'\n');
            }
            index_part.push(b'\n');
            let first = index_part.len();
            index_part.extend_from_slice(&body_part);
            let dict: Dict = [
                ("Type", Object::name("ObjStm")),
                ("N", i64::try_from(packed.len()).unwrap_or(i64::MAX).into()),
                ("First", i64::try_from(first).unwrap_or(i64::MAX).into()),
            ]
            .into_iter()
            .collect();
            let off = u32::try_from(out.len()).map_err(|_| too_big("offset over 4 GiB"))?;
            rows.insert(objstm_num, (1, off, 0));
            write_indirect(
                objstm_num,
                &Body::Object(Stream::flate(dict, &index_part).into()),
                &mut out,
            );
        }
        let xref_num = next;
        let size = xref_num + 1;
        let xref_off = out.len();
        rows.insert(
            xref_num,
            (1, u32::try_from(xref_off).map_err(|_| too_big("offset over 4 GiB"))?, 0),
        );
        let mut table = Vec::with_capacity(usize::try_from(size).unwrap_or(0) * 7);
        for num in 0..size {
            let (kind, f2, f3) = rows
                .get(&num)
                .copied()
                .unwrap_or((0, 0, if num == 0 { 65535 } else { 0 }));
            table.push(kind);
            table.extend_from_slice(&f2.to_be_bytes());
            table.extend_from_slice(&f3.to_be_bytes());
        }
        let mut dict: Dict = [
            ("Type", Object::name("XRef")),
            ("W", vec![1.into(), 4.into(), 2.into()].into()),
            ("Size", size.into()),
            ("Index", vec![0.into(), size.into()].into()),
        ]
        .into_iter()
        .collect();
        for (k, v) in trailer.iter().filter(|(k, _)| !is_xref_key(k)) {
            dict.insert(k.clone(), v.clone());
        }
        write_indirect(xref_num, &Body::Object(Stream::flate(dict, &table).into()), &mut out);
        out.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
        Ok(out)
    }
}

/// Whether `key` describes an xref section rather than the document (see [`XREF_STREAM_KEYS`]).
pub(crate) fn is_xref_key(key: &Name) -> bool {
    XREF_STREAM_KEYS.iter().any(|k| k.as_bytes() == key.as_bytes())
}

fn write_indirect(num: u32, body: &Body, out: &mut Vec<u8>) {
    out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
    match body {
        Body::Object(obj) => obj.write_to(out),
        Body::Raw(raw) => out.extend_from_slice(raw),
    }
    out.extend_from_slice(b"\nendobj\n");
}

fn write_free_head(eol: XrefEol, out: &mut Vec<u8>) {
    out.extend_from_slice(b"0000000000 65535 f");
    out.extend_from_slice(eol.bytes());
}

fn write_in_use(offset: usize, gen_nr: u16, eol: XrefEol, out: &mut Vec<u8>) {
    out.extend_from_slice(format!("{offset:010} {gen_nr:05} n").as_bytes());
    out.extend_from_slice(eol.bytes());
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
/// use lamina_cos::{Document, ObjRef, Object, OpenOptions};
/// use lamina_save::{Writer, XrefEol, incremental_update};
///
/// let mut w = Writer::new(1, 7);
/// let root = w.add([("Type", Object::name("Catalog"))].into_iter().collect::<lamina_cos::Dict>());
/// let base = Document::open(w.finish(&[("Root", root)].into_iter().collect())?, OpenOptions::default())?;
/// let updated = incremental_update(&base, &[(ObjRef::new(2, 0), Object::Integer(42))], XrefEol::CrLf)?;
/// let doc = Document::open(updated, OpenOptions::default())?;
/// assert_eq!((doc.revisions(), doc.object(2)?.as_int()), (2, Some(42)));
/// # Ok::<(), lamina_cos::Error>(())
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
    let mut offsets: BTreeMap<u32, (usize, u16)> = BTreeMap::new();
    for (&num, &(gen_nr, obj)) in &by_num {
        offsets.insert(num, (out.len(), gen_nr));
        out.extend_from_slice(format!("{num} {gen_nr} obj\n").as_bytes());
        obj.write_to(&mut out);
        out.extend_from_slice(b"\nendobj\n");
    }

    // The base's trailer: an overlay trailer may name objects this update does not write.
    let old = doc.base_trailer();
    let size = doc
        .size_floor()
        .max(by_num.keys().next_back().map_or(0, |&n| i64::from(n) + 1));
    let prev = i64::try_from(prev).unwrap_or(i64::MAX);
    if doc.newest_section_is_stream() {
        append_xref_stream(&mut out, old, &offsets, size, prev)?;
    } else {
        append_xref_table(&mut out, old, &offsets, size, prev, eol);
    }
    Ok(out)
}

/// The previous trailer's document keys (no xref-section keys), in their order.
fn document_keys(old: &Dict) -> impl Iterator<Item = (&Name, &Object)> {
    old.iter().filter(|(k, _)| !is_xref_key(k))
}

/// Like [`document_keys`], but keeps `/Size` so a classic trailer can hold it in its old position.
fn document_keys_and_size(old: &Dict) -> impl Iterator<Item = (&Name, &Object)> {
    old.iter().filter(|(k, _)| !is_xref_key(k) || k.as_bytes() == b"Size")
}

fn append_xref_table(
    out: &mut Vec<u8>,
    old: &Dict,
    offsets: &BTreeMap<u32, (usize, u16)>,
    size: i64,
    prev: i64,
    eol: XrefEol,
) {
    let xref = out.len();
    out.extend_from_slice(b"xref\n");
    let nums: Vec<u32> = std::iter::once(0)
        .chain(offsets.keys().copied().filter(|&n| n != 0))
        .collect();
    for run in nums.chunk_by(|a, b| b.checked_sub(*a) == Some(1)) {
        let (Some(&start), len) = (run.first(), run.len()) else {
            continue;
        };
        out.extend_from_slice(format!("{start} {len}\n").as_bytes());
        for num in run {
            match offsets.get(num) {
                Some(&(off, gen_nr)) if *num != 0 => write_in_use(off, gen_nr, eol, out),
                _ => write_free_head(eol, out),
            }
        }
    }
    // `/Size` keeps the previous trailer's position (first when it had none).
    let mut trailer = Dict::new();
    for (k, v) in document_keys_and_size(old) {
        trailer.insert(k.clone(), v.clone());
    }
    trailer.insert("Size", size);
    trailer.insert(Name::from("Prev"), prev);
    write_trailer(&trailer, xref, out);
}

fn append_xref_stream(
    out: &mut Vec<u8>,
    old: &Dict,
    offsets: &BTreeMap<u32, (usize, u16)>,
    size: i64,
    prev: i64,
) -> Result<()> {
    let out_of_numbers = || Error::InvalidStructure {
        kind: "incremental update",
        message: "object numbers exhausted",
    };
    let xref_num = u32::try_from(size).map_err(|_| out_of_numbers())?;
    let new_size = xref_num.checked_add(1).ok_or_else(out_of_numbers)?;
    let xref_off = out.len();
    let mut rows: BTreeMap<u32, (usize, u16)> = offsets
        .iter()
        .filter(|(n, _)| **n != 0)
        .map(|(n, e)| (*n, *e))
        .collect();
    rows.insert(xref_num, (xref_off, 0));
    // Offset field width: 4 bytes like `finish_compact`, wider only for files over 4 GiB.
    let max_off = u64::try_from(xref_off).unwrap_or(u64::MAX);
    let width = usize::try_from(8 - max_off.leading_zeros() / 8).unwrap_or(8).max(4);
    let mut table = Vec::with_capacity(rows.len() * (3 + width));
    let mut index = Vec::new();
    let nums: Vec<u32> = rows.keys().copied().collect();
    for run in nums.chunk_by(|a, b| b.checked_sub(*a) == Some(1)) {
        let (Some(&start), len) = (run.first(), run.len()) else {
            continue;
        };
        index.push(Object::from(start));
        index.push(Object::from(i64::try_from(len).unwrap_or(i64::MAX)));
        for num in run {
            let Some(&(off, gen_nr)) = rows.get(num) else {
                continue;
            };
            let off = u64::try_from(off).unwrap_or(u64::MAX).to_be_bytes();
            table.push(1);
            table.extend_from_slice(off.get(8 - width..).unwrap_or(&off));
            table.extend_from_slice(&gen_nr.to_be_bytes());
        }
    }
    let width = i64::try_from(width).unwrap_or(8);
    let mut dict: Dict = [
        ("Type", Object::name("XRef")),
        ("W", vec![1.into(), width.into(), 2.into()].into()),
        ("Size", new_size.into()),
        ("Index", Object::Array(index)),
    ]
    .into_iter()
    .collect();
    for (k, v) in document_keys(old) {
        dict.insert(k.clone(), v.clone());
    }
    dict.insert("Prev", prev);
    write_indirect(xref_num, &Body::Object(Stream::flate(dict, &table).into()), out);
    out.extend_from_slice(format!("startxref\n{xref_off}\n%%EOF\n").as_bytes());
    Ok(())
}
