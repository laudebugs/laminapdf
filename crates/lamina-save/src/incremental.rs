//! The incremental writer ([ADR 0007] §5, ISO 32000-2 §7.5.6): the base bytes, unchanged,
//! then the overlay as an appended revision.
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use lamina_cos::{ByteSource, Dict, Document, Name, Object, OverlayEntry, SourceError};

use crate::error::{Result, SaveError};
use crate::id::{ContentHash, id_array, permanent_id};
use crate::legacy::is_xref_key;
use crate::options::{IncrementalOutput, SaveMode, SaveOptions, SaveReport};
use crate::sink::{CountingSink, WriteSink};
use crate::xref::{Row, Rows, classic_table, link_free_list, stream_object, stream_table};

/// Bytes copied from the base per read.
const CHUNK: usize = 64 * 1024;

/// Writes a document's overlay as an incremental update of its base.
///
/// * The base bytes are copied from the [`ByteSource`] in 64 KiB chunks, unchanged
///   ([`IncrementalOutput::AppendedOnly`] skips them). Before anything is written every chunk
///   is checked, so a source still missing bytes fails with
///   [`DataPending`](lamina_cos::Error::DataPending) listing all of them and the sink stays
///   empty.
/// * The overlay follows in object-number order: new objects (generation 0) and modified ones
///   (their base generation), each `n g obj … endobj`; deleted objects become free entries with
///   the generation incremented.
/// * Then an xref section of the same kind as the base's newest (a classic table, or an xref
///   stream numbered with the next free number), with `/Prev`, `/Size` never below the
///   base's ([`Document::size_floor`]) nor any number reserved in the session
///   ([`Document::next_number`]), the trailer's document keys as they are now (xref-section
///   keys dropped, ch. 07 §4) and `/ID` (§14.4): `/ID[0]` kept (derived like `/ID[1]` when missing),
///   `/ID[1]` the hash of the base length and the appended objects.
/// * An empty overlay writes the base unchanged (nothing at all with `AppendedOnly`).
///
/// The output is byte-deterministic (ADR 0001).
#[derive(Debug, Clone, Copy)]
pub struct IncrementalWriter<'d> {
    doc: &'d Document,
    options: SaveOptions,
}

/// What [`IncrementalWriter::write`] produced: the report and the appended revision, which
/// the document needs to rebuild its base (old base ‖ revision, ADR 0005 §3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncrementalSave {
    report: SaveReport,
    appended: Vec<u8>,
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
        Self { doc, options }
    }

    /// Writes to `sink`.
    ///
    /// # Errors
    /// Before anything is written: [`SaveError::RepairedBase`], [`SaveError::EncryptedBase`],
    /// [`lamina_cos::Error::DataPending`] / [`lamina_cos::Error::Source`] for base bytes or
    /// objects that can't be read, [`lamina_cos::Error::InvalidStructure`] when object numbers
    /// run out. While writing: [`SaveError::Sink`] (and a source that fails between the check
    /// and the copy).
    pub fn write<S: WriteSink + ?Sized>(&self, sink: &mut S) -> Result<IncrementalSave> {
        let doc = self.doc;
        let prev = doc.startxref().ok_or(SaveError::RepairedBase)?;
        if doc.is_encrypted() {
            return Err(SaveError::EncryptedBase);
        }
        let source = doc.source();
        let base_len = source.len();
        let appended = if doc.is_dirty() {
            self.revision(base_len, prev)?
        } else {
            Vec::new()
        };
        let whole = self.options.output() == IncrementalOutput::Whole;
        if whole {
            check_available(source.as_ref())?;
        }
        let mut out = CountingSink::new(sink);
        if whole {
            copy(source.as_ref(), &mut out)?;
        }
        out.write_all(&appended)?;
        Ok(IncrementalSave {
            report: SaveReport {
                bytes_written: out.position(),
                mode: SaveMode::Incremental,
                renumbered: None,
                append_offset: Some(base_len),
            },
            appended,
        })
    }

    /// The appended revision for a base of `base_len` bytes whose newest section is at
    /// `prev`.
    fn revision(&self, base_len: u64, prev: usize) -> Result<Vec<u8>> {
        let doc = self.doc;
        let mut out = Vec::new();
        if !matches!(last_byte(doc.source().as_ref(), base_len)?, Some(b'\n' | b'\r') | None) {
            out.push(b'\n');
        }
        let mut rows = Rows::new();
        let mut highest = 0u32;
        for (num, entry) in doc.overlay().iter() {
            highest = highest.max(num);
            let generation = doc.base_generation(num).unwrap_or(0);
            let offset = base_len.saturating_add(u64::try_from(out.len()).unwrap_or(u64::MAX));
            match entry {
                OverlayEntry::New(object) | OverlayEntry::Modified(object) => {
                    rows.insert(num, Row::InFile { offset, generation });
                    out.extend_from_slice(format!("{num} {generation} obj\n").as_bytes());
                    object.write_to(&mut out);
                    out.extend_from_slice(b"\nendobj\n");
                }
                // Deleted (and any later kind): a free entry, so the reused number gets a
                // new generation (§7.5.4).
                _ => {
                    rows.insert(
                        num,
                        Row::Free {
                            next: 0,
                            generation: generation.saturating_add(1),
                        },
                    );
                }
            }
        }
        let mut hash = ContentHash::default();
        hash.update(&base_len.to_be_bytes());
        hash.update(&out);

        let size = u64::try_from(doc.size_floor().max(0))
            .unwrap_or(0)
            .max(doc.next_number())
            .max(u64::from(highest) + 1);
        let out_of_numbers = || {
            SaveError::Cos(lamina_cos::Error::InvalidStructure {
                kind: "incremental update",
                message: "object numbers exhausted",
            })
        };
        let current = doc.trailer();
        let mut trailer = Dict::new();
        let id = id_array(permanent_id(doc, current)?, hash);
        let prev = i64::try_from(prev).unwrap_or(i64::MAX);
        if doc.newest_section_is_stream() {
            let xref_num = u32::try_from(size).map_err(|_| out_of_numbers())?;
            let new_size = xref_num.checked_add(1).ok_or_else(out_of_numbers)?;
            let offset = base_len.saturating_add(u64::try_from(out.len()).unwrap_or(u64::MAX));
            rows.insert(xref_num, Row::InFile { offset, generation: 0 });
            if rows.values().any(|r| matches!(r, Row::Free { .. })) {
                link_free_list(&mut rows);
            }
            let (mut dict, data) = stream_table(&rows);
            dict.insert("Size", new_size);
            for (k, v) in current.iter().filter(|(k, _)| !is_xref_key(k)) {
                dict.insert(k.clone(), v.clone());
            }
            dict.insert("ID", id);
            dict.insert("Prev", prev);
            out.extend_from_slice(&stream_object(xref_num, dict, &data));
            out.extend_from_slice(format!("startxref\n{offset}\n%%EOF\n").as_bytes());
        } else {
            let size = i64::try_from(size).map_err(|_| out_of_numbers())?;
            link_free_list(&mut rows);
            let xref = base_len.saturating_add(u64::try_from(out.len()).unwrap_or(u64::MAX));
            out.extend_from_slice(&classic_table(&rows, self.options.eol())?);
            // `/Size` keeps the previous trailer's position (first when it had none).
            for (k, v) in current
                .iter()
                .filter(|(k, _)| !is_xref_key(k) || k.as_bytes() == b"Size")
            {
                trailer.insert(k.clone(), v.clone());
            }
            if !trailer.contains_key("Size") {
                let mut sized = Dict::new();
                sized.insert("Size", size);
                for (k, v) in trailer.iter() {
                    sized.insert(k.clone(), v.clone());
                }
                trailer = sized;
            }
            trailer.insert("Size", size);
            trailer.insert("ID", id);
            trailer.insert(Name::from("Prev"), prev);
            out.extend_from_slice(b"trailer\n");
            Object::Dict(trailer).write_to(&mut out);
            out.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
        }
        Ok(out)
    }
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
    let n = source.read_at(at, &mut byte).map_err(lamina_cos::Error::from)?;
    Ok(byte.first().copied().filter(|_| n == 1))
}

/// Checks that every byte of a source without an in-memory slice can be read now, reporting
/// all missing ranges in one [`DataPending`](lamina_cos::Error::DataPending).
fn check_available(source: &dyn ByteSource) -> Result<()> {
    if source.as_slice().is_some() {
        return Ok(());
    }
    let len = source.len();
    let mut pending: Vec<(u64, u64)> = Vec::new();
    let mut buf = vec![0u8; CHUNK];
    let mut at = 0u64;
    while at < len {
        let want = usize::try_from((len - at).min(CHUNK as u64)).unwrap_or(CHUNK);
        let chunk = buf.get_mut(..want).unwrap_or_default();
        match read_full(source, at, chunk) {
            Ok(()) => {}
            Err(SourceError::Pending { .. }) => {
                let want = want as u64;
                match pending.last_mut() {
                    Some((start, n)) if *start + *n == at => *n += want,
                    _ => pending.push((at, want)),
                }
            }
            Err(e) => return Err(lamina_cos::Error::from(e).into()),
        }
        at += want as u64;
    }
    if pending.is_empty() {
        Ok(())
    } else {
        Err(lamina_cos::Error::DataPending { ranges: pending }.into())
    }
}

/// Fills `buf` from `offset`, calling `read_at` again for short reads; a source that ends
/// early is a failure.
fn read_full(source: &dyn ByteSource, offset: u64, buf: &mut [u8]) -> core::result::Result<(), SourceError> {
    let mut filled = 0;
    while filled < buf.len() {
        let at = offset + filled as u64;
        let rest = buf.get_mut(filled..).unwrap_or_default();
        let n = source.read_at(at, rest)?;
        if n == 0 {
            return Err(SourceError::Failed {
                offset: at,
                len: rest.len() as u64,
            });
        }
        filled += n;
    }
    Ok(())
}

/// Copies the whole source to `out` in chunks.
fn copy<S: WriteSink + ?Sized>(source: &dyn ByteSource, out: &mut CountingSink<&mut S>) -> Result<()> {
    if let Some(slice) = source.as_slice() {
        for chunk in slice.chunks(CHUNK) {
            out.write_all(chunk)?;
        }
        return Ok(());
    }
    let len = source.len();
    let mut buf = vec![0u8; CHUNK];
    let mut at = 0u64;
    while at < len {
        let want = usize::try_from((len - at).min(CHUNK as u64)).unwrap_or(CHUNK);
        let chunk = buf.get_mut(..want).unwrap_or_default();
        read_full(source, at, chunk).map_err(lamina_cos::Error::from)?;
        out.write_all(chunk)?;
        at += want as u64;
    }
    Ok(())
}
