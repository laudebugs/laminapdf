//! The incremental writer ([ADR 0007] §5, ISO 32000-2 §7.5.6): the base bytes, unchanged,
//! then the overlay as an appended revision.
//!
//! [ADR 0007]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0007-save-planner.md

use laminapdf_cos::{ByteSource, Dict, Document, Name, Object, OverlayEntry, merge_ranges, read_exact};

use crate::detect::Patches;
use crate::error::{Result, SaveError};
use crate::id::{ContentHash, id_array, permanent_id};
use crate::legacy::{document_keys, document_keys_and_size};
use crate::options::{IncrementalOutput, SaveMode, SaveOptions, SaveReport};
use crate::sink::{CountingSink, WriteSink, require_empty};
use crate::xref::{Row, Rows, classic_table, link_free_list, stream_object, stream_table};

/// Bytes read from the base at a time.
const CHUNK: usize = 64 * 1024;

/// Writes a document's overlay as an incremental update of its base.
///
/// * **Base.** Copied from the [`ByteSource`] in 64 KiB chunks, unchanged
///   ([`IncrementalOutput::AppendedOnly`] skips the copy). A source without an in-memory slice
///   is read twice: once, before anything is written, to check that every chunk is available
///   (a source still missing bytes fails with [`DataPending`](laminapdf_cos::Error::DataPending)
///   listing all of them, and the sink stays empty) and to hash it for `/ID`; then to copy it.
///   A source that loses bytes between the two passes fails the save with the sink holding a
///   partial output (see [`SaveError`]).
/// * **Revision.** The overlay in object-number order: new objects (generation 0) and modified
///   ones (their base generation), each `n g obj … endobj`; deleted objects become free entries
///   with the generation incremented, linked into the free list in front of the base's
///   ([`Document::free_list_head`]).
/// * **Xref.** A section of the same kind as the base's newest (a classic table, or an xref
///   stream numbered with the next free number, within
///   [`Limits::max_object_number`](laminapdf_cos::Limits::max_object_number)), with `/Prev`,
///   `/Size` never below the base's ([`Document::size_floor`]) nor any number reserved in the
///   session ([`Document::next_number`]), the trailer's document keys as they are now
///   (xref-section and xref-stream keys dropped, ch. 07 §4), and `/ID` (§14.4): `/ID[0]` kept
///   (derived like `/ID[1]` when missing), `/ID[1]` the first 16 bytes of the SHA-256 of the
///   base and of the revision up to its trailer.
/// * **Nothing to append.** With an empty overlay the base is written unchanged (nothing at
///   all with `AppendedOnly`), even for a repaired or encrypted base, and the report has no
///   append offset.
///
/// Everything that can fail on input happens before the first byte is written. The output is
/// byte-deterministic (ADR 0001).
#[derive(Debug, Clone, Copy)]
pub struct IncrementalWriter<'d> {
    doc: &'d Document,
    options: SaveOptions,
    patches: Option<&'d Patches>,
}

/// What [`IncrementalWriter::write`] produced: the report and the appended revision, which
/// the document needs to rebuild its base (old base ‖ revision, ADR 0005 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalSave {
    report: SaveReport,
    appended: Vec<u8>,
    freed: bool,
}

impl IncrementalSave {
    /// What was written.
    #[must_use]
    pub const fn report(&self) -> &SaveReport {
        &self.report
    }

    /// The appended revision (empty when the overlay was).
    #[must_use]
    pub fn appended(&self) -> &[u8] {
        &self.appended
    }

    /// Whether the revision freed objects (deleted base objects).
    #[must_use]
    pub const fn freed_objects(&self) -> bool {
        self.freed
    }

    /// The report and the appended revision.
    #[must_use]
    pub fn into_parts(self) -> (SaveReport, Vec<u8>) {
        (self.report, self.appended)
    }
}

impl<'d> IncrementalWriter<'d> {
    /// A writer for `doc`'s overlay; of the options, `output` and `eol` apply.
    #[must_use]
    pub const fn new(doc: &'d Document, options: SaveOptions) -> Self {
        Self {
            doc,
            options,
            patches: None,
        }
    }

    /// With the override's changes written over the document's values (in the revision).
    pub(crate) const fn with_patches(mut self, patches: &'d Patches) -> Self {
        self.patches = Some(patches);
        self
    }

    /// Writes to `sink`, which must be empty (or, for `AppendedOnly`, hold exactly the base).
    ///
    /// # Errors
    /// Before anything is written: [`SinkError::NotEmpty`](crate::SinkError::NotEmpty);
    /// [`laminapdf_cos::Error::RepairedBase`] and [`SaveError::EncryptedBase`] when there
    /// is something to append; [`laminapdf_cos::Error::DataPending`] /
    /// [`laminapdf_cos::Error::Source`] for base bytes or objects that can't be read;
    /// [`laminapdf_cos::Error::InvalidStructure`] when object numbers run out. While writing:
    /// [`SaveError::Sink`], and source errors from the copy pass.
    pub fn write<S: WriteSink + ?Sized>(&self, sink: &mut S) -> Result<IncrementalSave> {
        let doc = self.doc;
        let source = doc.source().as_ref();
        let base_len = source.len();
        let whole = self.options.output() == IncrementalOutput::Whole;
        require_empty(sink, (!whole).then_some(base_len))?;
        let patched = self.patches.is_some_and(|p| !p.is_empty());
        let (appended, freed, append_offset) = if doc.is_dirty() || patched {
            let prev = doc.startxref().ok_or(laminapdf_cos::Error::RepairedBase)?;
            if doc.is_encrypted() {
                return Err(SaveError::EncryptedBase);
            }
            let mut hash = ContentHash::default();
            check_and_hash(source, Some(&mut hash))?;
            let (appended, freed) = self.revision(base_len, prev, hash)?;
            (appended, freed, Some(base_len))
        } else {
            if whole {
                check_and_hash(source, None)?;
            }
            (Vec::new(), false, None)
        };
        let mut out = CountingSink::new(sink);
        if whole {
            copy(source, &mut out)?;
        }
        out.write_all(&appended)?;
        Ok(IncrementalSave {
            report: SaveReport {
                bytes_written: out.position(),
                mode: SaveMode::Incremental,
                renumbered: None,
                append_offset,
                invalidated: Vec::new(),
                encryption_removed: false,
            },
            appended,
            freed,
        })
    }

    /// What the revision holds, by number: the value to write (the override's replacement
    /// first, else the overlay's new or modified object), or `None` for a free entry.
    fn entries(&self) -> Vec<(u32, Option<&'d Object>)> {
        let doc = self.doc;
        let patches = self.patches.map(|p| &p.objects);
        let numbers: std::collections::BTreeSet<u32> = doc
            .overlay()
            .iter()
            .map(|(num, _)| num)
            .chain(patches.into_iter().flat_map(|p| p.keys().copied()))
            .collect();
        numbers
            .into_iter()
            .map(|num| {
                let written = patches
                    .and_then(|p| p.get(&num))
                    .or_else(|| match doc.overlay().get(num) {
                        Some(OverlayEntry::New(object) | OverlayEntry::Modified(object)) => Some(&**object),
                        _ => None,
                    });
                (num, written)
            })
            .collect()
    }

    /// The appended revision for a base of `base_len` bytes whose newest section is at
    /// `prev`, `hash` holding the base; and whether it frees objects.
    fn revision(&self, base_len: u64, prev: usize, mut hash: ContentHash) -> Result<(Vec<u8>, bool)> {
        let doc = self.doc;
        let mut out = Vec::new();
        if !matches!(last_byte(doc.source().as_ref(), base_len)?, Some(b'\n' | b'\r') | None) {
            out.push(b'\n');
        }
        let offset_of = |out: &Vec<u8>| base_len.saturating_add(u64::try_from(out.len()).unwrap_or(u64::MAX));
        let mut rows = Rows::new();
        let mut highest = 0u32;
        let mut freed = false;
        for (num, written) in self.entries() {
            highest = highest.max(num);
            let generation = doc.base_generation(num).unwrap_or(0);
            if let Some(object) = written {
                rows.insert(
                    num,
                    Row::InFile {
                        offset: offset_of(&out),
                        generation,
                    },
                );
                out.extend_from_slice(format!("{num} {generation} obj\n").as_bytes());
                object.write_to(&mut out);
                out.extend_from_slice(b"\nendobj\n");
            } else {
                // Deleted (and any later kind): a free entry, so the reused number gets a
                // new generation (§7.5.4).
                freed = true;
                rows.insert(
                    num,
                    Row::Free {
                        next: 0,
                        generation: generation.saturating_add(1),
                    },
                );
            }
        }
        hash.update(&out);

        let size = u64::try_from(doc.size_floor().max(0))
            .unwrap_or(0)
            .max(doc.next_number())
            .max(u64::from(highest) + 1);
        let current = self
            .patches
            .and_then(|p| p.trailer.as_ref())
            .unwrap_or_else(|| doc.trailer());
        let permanent = permanent_id(doc, current)?;
        let head = doc.free_list_head();
        let prev = i64::try_from(prev).unwrap_or(i64::MAX);
        if doc.newest_section_is_stream() {
            let xref_num = u32::try_from(size)
                .ok()
                .filter(|&n| n <= doc.limits().max_object_number())
                .ok_or_else(exhausted)?;
            let new_size = xref_num.checked_add(1).ok_or_else(exhausted)?;
            let offset = offset_of(&out);
            rows.insert(xref_num, Row::InFile { offset, generation: 0 });
            // Object 0 only when there is something to link in front of the base's list.
            if freed {
                link_free_list(&mut rows, head);
            }
            let (mut dict, data) = stream_table(&rows);
            hash.update(&data);
            dict.insert("Size", new_size);
            for (k, v) in document_keys(current) {
                dict.insert(k.clone(), v.clone());
            }
            dict.insert("ID", id_array(permanent, hash));
            dict.insert("Prev", prev);
            out.extend_from_slice(&stream_object(xref_num, dict, &data));
            out.extend_from_slice(format!("startxref\n{offset}\n%%EOF\n").as_bytes());
        } else {
            let size = i64::try_from(size).map_err(|_| exhausted())?;
            link_free_list(&mut rows, head);
            let xref = offset_of(&out);
            let table = classic_table(&rows, self.options.eol())?;
            hash.update(&table);
            out.extend_from_slice(&table);
            // `/Size` keeps the previous trailer's position (first when it had none).
            let mut trailer = Dict::new();
            if !current.contains_key("Size") {
                trailer.insert("Size", size);
            }
            for (k, v) in document_keys_and_size(current) {
                trailer.insert(k.clone(), v.clone());
            }
            trailer.insert("Size", size);
            trailer.insert("ID", id_array(permanent, hash));
            trailer.insert(Name::from("Prev"), prev);
            out.extend_from_slice(b"trailer\n");
            Object::Dict(trailer).write_to(&mut out);
            out.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        }
        Ok((out, freed))
    }
}

const fn exhausted() -> SaveError {
    SaveError::Cos(laminapdf_cos::Error::InvalidStructure {
        kind: "incremental update",
        message: "object numbers exhausted (Limits::max_object_number)",
    })
}

/// The last byte of the base (`None` when it is empty).
fn last_byte(source: &dyn ByteSource, len: u64) -> Result<Option<u8>> {
    if let Some(slice) = source.as_slice() {
        return Ok(slice.last().copied());
    }
    let Some(at) = len.checked_sub(1) else {
        return Ok(None);
    };
    let mut byte = [0u8; 1];
    read_exact(source, at, &mut byte)?;
    Ok(byte.first().copied())
}

/// The base in chunks: `(offset, length)` pairs covering `len` bytes.
fn chunks(len: u64) -> impl Iterator<Item = (u64, usize)> {
    let step = CHUNK as u64;
    (0..len.div_ceil(step)).map(move |i| {
        let at = i * step;
        (at, usize::try_from((len - at).min(step)).unwrap_or(CHUNK))
    })
}

/// Reads the whole base once before anything is written: every missing range is reported in
/// one [`DataPending`](laminapdf_cos::Error::DataPending), and the bytes go into `hash` when one
/// is given.
fn check_and_hash(source: &dyn ByteSource, mut hash: Option<&mut ContentHash>) -> Result<()> {
    if let Some(slice) = source.as_slice() {
        if let Some(hash) = hash {
            hash.update(slice);
        }
        return Ok(());
    }
    let mut pending = Vec::new();
    let mut buf = vec![0u8; CHUNK];
    for (at, len) in chunks(source.len()) {
        let chunk = buf.get_mut(..len).unwrap_or_default();
        match read_exact(source, at, chunk) {
            Ok(()) => {
                if let Some(hash) = hash.as_deref_mut() {
                    hash.update(chunk);
                }
            }
            Err(laminapdf_cos::Error::DataPending { ranges }) => pending.extend(ranges),
            Err(e) => return Err(e.into()),
        }
    }
    if pending.is_empty() {
        Ok(())
    } else {
        Err(laminapdf_cos::Error::DataPending {
            ranges: merge_ranges(pending),
        }
        .into())
    }
}

/// Copies the whole source to `out` in chunks.
fn copy<S: WriteSink + ?Sized>(source: &dyn ByteSource, out: &mut CountingSink<&mut S>) -> Result<()> {
    if let Some(slice) = source.as_slice() {
        for chunk in slice.chunks(CHUNK) {
            out.write_all(chunk)?;
        }
        return Ok(());
    }
    let mut buf = vec![0u8; CHUNK];
    for (at, len) in chunks(source.len()) {
        let chunk = buf.get_mut(..len).unwrap_or_default();
        read_exact(source, at, chunk)?;
        out.write_all(chunk)?;
    }
    Ok(())
}
