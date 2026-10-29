//! The full-rewrite writer ([ADR 0007] §5): a new file holding only what the trailer
//! reaches.
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::collections::BTreeMap;
use std::sync::Arc;

use lamina_cos::{Dict, Document, Limit, ObjRef, Object, OverlayEntry, Stream};

use crate::error::{Result, SaveError};
use crate::id::{ContentHash, id_array, permanent_id};
use crate::legacy::{HEADER_MARKER, document_keys};
use crate::options::{Renumber, SaveMode, SaveOptions, SaveReport, XrefKind};
use crate::sink::{CountingSink, WriteSink, require_empty};
use crate::xref::{Row, Rows, classic_table, link_free_list, stream_object, stream_table};

/// Objects per object stream: small enough that reading one member stays cheap.
const OBJSTM_CAPACITY: usize = 100;

/// Writes a document as a new file: header, the reachable objects, object streams, xref.
///
/// * **Reachability** (iterative, each object visited once): from the trailer's document
///   keys (`/Root`, `/Info`, …; not `/ID`, which is rewritten) through every reference,
///   skipping a stream's own `/Length` (written directly). Objects no path reaches (earlier
///   revisions, orphans) are dropped. A reference to an object that doesn't exist, or with a
///   generation that isn't the object's, resolves to `null` (§7.3.10) and is written as
///   `null`. An object that can't be loaded fails the save: nothing is silently dropped.
/// * **Numbering**: [`Renumber::Keep`] keeps numbers and generations; base objects that are
///   not written (deleted or unreachable) become free entries with the next generation
///   (§7.5.4), other numbers are simply absent from the xref. [`Renumber::Compact`] numbers
///   the objects `1..=n` in their old order with generation 0 and reports the map. Either
///   way the xref lists only numbers that hold something, so its size follows the data, not
///   the input's `/Size`.
/// * **Layout** ([`XrefKind::Stream`], the default): stream objects and objects with a
///   generation above 0 top-level, every other object in Flate object streams of up to 100
///   members in number order (§7.5.7 forbids streams and non-zero generations there), then
///   an xref stream; the header is raised to 1.5 if lower. [`XrefKind::Classic`] writes every
///   object top-level and a classic table. Object streams and the xref stream are numbered
///   after every other number in the xref, within
///   [`Limits::max_object_number`](lamina_cos::Limits::max_object_number).
/// * **`/ID`**: `/ID[0]` kept (derived like `/ID[1]` when missing), `/ID[1]` the first 16
///   bytes of the SHA-256 of everything before the xref section.
///
/// Every object is loaded and every number allocated before the first byte is written, so
/// unreadable input ([`lamina_cos::Error::DataPending`]) and exhausted numbers leave the sink
/// empty. The one failure that can only show while writing is a classic table entry past ten
/// digits (an output over 10 GB): use the xref-stream layout for that. The output is
/// byte-deterministic (ADR 0001).
#[derive(Debug, Clone, Copy)]
pub struct FullWriter<'d> {
    doc: &'d Document,
    options: SaveOptions,
}

/// A reachable object: its generation in the input, and its value.
struct Reached {
    generation: u16,
    value: Arc<Object>,
}

/// The numbers a rewrite uses, all allocated before writing.
struct Plan {
    /// Old number → (new number, new generation), for every reachable object.
    numbers: BTreeMap<u32, (u32, u16)>,
    /// Free entries (number → generation) for base objects that are not written.
    freed: BTreeMap<u32, u16>,
    /// The object streams' numbers, one per group of packed objects.
    streams: Vec<u32>,
    /// The xref stream's number (`/Size` - 1), or the classic table's `/Size`.
    last: u32,
}

impl<'d> FullWriter<'d> {
    /// A writer for `doc` as it is now (overlay over base); of the options, `renumber`,
    /// `xref`, `eol` and `remove_encryption` apply. With `remove_encryption`, an encrypted
    /// document is written as plaintext: the objects as the reader decrypted them, streams
    /// without a leading `/Crypt` filter, and no `/Encrypt` in the trailer (`/ID[0]` kept).
    #[must_use]
    pub const fn new(doc: &'d Document, options: SaveOptions) -> Self {
        Self { doc, options }
    }

    /// Writes to `sink`, which must be empty.
    ///
    /// # Errors
    /// Before anything is written: [`SinkError::NotEmpty`](crate::SinkError::NotEmpty);
    /// [`SaveError::EncryptedBase`] unless the options remove the encryption; any error loading a reachable object or `/ID`
    /// (unavailable input, strict mode, limits, unparsable objects);
    /// [`lamina_cos::Error::LimitExceeded`] for an object nested deeper than
    /// [`Limits::max_nesting_depth`](lamina_cos::Limits::max_nesting_depth);
    /// [`lamina_cos::Error::InvalidStructure`] when object numbers run out. While writing:
    /// [`SaveError::Sink`]; [`lamina_cos::Error::InvalidStructure`] for a classic offset over
    /// ten digits.
    pub fn write<S: WriteSink + ?Sized>(&self, sink: &mut S) -> Result<SaveReport> {
        let doc = self.doc;
        require_empty(sink, None)?;
        let decrypt = doc.is_encrypted();
        if decrypt && !self.options.remove_encryption() {
            return Err(SaveError::EncryptedBase);
        }
        let trailer = doc.trailer();
        let depth = doc.limits().max_nesting_depth();
        let reached = self.reach(trailer, depth)?;
        let compact = self.options.xref() == XrefKind::Stream;
        let plan = self.plan(&reached, self.object_streams(&reached))?;
        let map = |r: ObjRef| -> Option<ObjRef> {
            reached.get(&r.num).filter(|o| o.generation == r.gen_nr)?;
            let &(num, generation) = plan.numbers.get(&r.num)?;
            Some(ObjRef::new(num, generation))
        };
        let remapper = Remap {
            map: &map,
            depth,
            decrypted: decrypt,
        };
        let permanent = permanent_id(doc, trailer)?;
        let trailer_out = written_keys(trailer)
            .map(|(k, v)| Ok((k.clone(), remapper.object(v)?)))
            .collect::<Result<Dict>>()?;
        // New number order (the same as the old order: numbering keeps it).
        let objects: Vec<(u32, u16, &Object)> = reached
            .iter()
            .filter_map(|(old, o)| {
                plan.numbers
                    .get(old)
                    .map(|&(num, generation)| (num, generation, &*o.value))
            })
            .collect();

        let mut out = Output {
            sink: CountingSink::new(sink),
            hash: ContentHash::default(),
        };
        out.write(&header(doc.header_version(), compact))?;
        let mut rows: Rows = plan
            .freed
            .iter()
            .map(|(&num, &generation)| (num, Row::Free { next: 0, generation }))
            .collect();
        let mut packed = Vec::new();
        for &(num, generation, value) in &objects {
            if compact && generation == 0 && !matches!(value, Object::Stream(_)) {
                packed.push((num, value));
                continue;
            }
            rows.insert(
                num,
                Row::InFile {
                    offset: out.position(),
                    generation,
                },
            );
            out.indirect(num, generation, value, &remapper)?;
        }
        for (group, &stream_num) in packed.chunks(OBJSTM_CAPACITY).zip(&plan.streams) {
            let (stream, members) = object_stream(stream_num, group, &remapper)?;
            rows.extend(members);
            rows.insert(
                stream_num,
                Row::InFile {
                    offset: out.position(),
                    generation: 0,
                },
            );
            let identity = Remap {
                map: &Some,
                depth,
                decrypted: false,
            };
            out.indirect(stream_num, 0, &Object::Stream(stream), &identity)?;
        }

        let Output { mut sink, hash } = out;
        link_free_list(&mut rows, 0);
        if compact {
            write_xref_stream(&mut sink, rows, plan.last, &trailer_out, permanent, hash)?;
        } else {
            write_classic_xref(&mut sink, &rows, plan.last, &trailer_out, permanent, hash, self.options)?;
        }
        let renumbered = (self.options.renumber() == Renumber::Compact)
            .then(|| plan.numbers.iter().map(|(&old, &(new, _))| (old, new)).collect());
        Ok(SaveReport {
            bytes_written: sink.position(),
            mode: SaveMode::FullRewrite,
            renumbered,
            append_offset: None,
            invalidated: Vec::new(),
            encryption_removed: decrypt,
        })
    }

    /// How many object streams the packable objects of `reached` fill (none for a classic
    /// layout).
    fn object_streams(&self, reached: &BTreeMap<u32, Reached>) -> usize {
        if self.options.xref() != XrefKind::Stream {
            return 0;
        }
        reached
            .values()
            .filter(|o| self.packed_generation(o) == 0 && !matches!(*o.value, Object::Stream(_)))
            .count()
            .div_ceil(OBJSTM_CAPACITY)
    }

    /// The generation an object is written with.
    fn packed_generation(&self, o: &Reached) -> u16 {
        match self.options.renumber() {
            Renumber::Compact => 0,
            _ => o.generation,
        }
    }

    /// Allocates every number: the objects', the freed entries', `groups` object streams'
    /// and the xref's.
    fn plan(&self, reached: &BTreeMap<u32, Reached>, groups: usize) -> Result<Plan> {
        let doc = self.doc;
        let (numbers, freed): (BTreeMap<u32, (u32, u16)>, BTreeMap<u32, u16>) =
            if self.options.renumber() == Renumber::Compact {
                (
                    reached.keys().zip(1u32..).map(|(&old, new)| (old, (new, 0))).collect(),
                    BTreeMap::new(),
                )
            } else {
                // Base objects that existed (some deleted in this session) and are not
                // written: freed, with the next generation.
                let deleted = doc
                    .overlay()
                    .iter()
                    .filter(|(_, e)| matches!(e, OverlayEntry::Deleted))
                    .map(|(n, _)| n);
                let freed = doc
                    .object_numbers()
                    .chain(deleted)
                    .filter(|n| !reached.contains_key(n))
                    .filter_map(|n| doc.base_generation(n).map(|g| (n, g.saturating_add(1))))
                    .collect();
                (
                    reached.iter().map(|(&num, o)| (num, (num, o.generation))).collect(),
                    freed,
                )
            };
        let highest = numbers
            .values()
            .map(|&(n, _)| n)
            .chain(freed.keys().copied())
            .max()
            .unwrap_or(0);
        let max = doc.limits().max_object_number();
        let mut next = u64::from(highest) + 1;
        let mut allocate = || -> Result<u32> {
            let num = u32::try_from(next).ok().filter(|&n| n <= max).ok_or_else(exhausted)?;
            next += 1;
            Ok(num)
        };
        let streams = (0..groups).map(|_| allocate()).collect::<Result<Vec<_>>>()?;
        let last = if self.options.xref() == XrefKind::Stream {
            allocate()?
        } else {
            // The classic table's `/Size`: one past the highest number.
            u32::try_from(next).map_err(|_| exhausted())?
        };
        Ok(Plan {
            numbers,
            freed,
            streams,
            last,
        })
    }

    /// Every object reachable from `trailer`'s document keys (except `/ID`), by number.
    fn reach(&self, trailer: &Dict, depth: usize) -> Result<BTreeMap<u32, Reached>> {
        let doc = self.doc;
        let mut reached = BTreeMap::new();
        let mut pending = Vec::new();
        for (_, value) in written_keys(trailer) {
            references(value, depth, &mut pending)?;
        }
        while let Some(r) = pending.pop() {
            if reached.contains_key(&r.num) || generation(doc, r.num) != Some(r.gen_nr) {
                continue;
            }
            let value = doc.object(r.num)?;
            references(&value, depth, &mut pending)?;
            reached.insert(
                r.num,
                Reached {
                    generation: r.gen_nr,
                    value,
                },
            );
        }
        Ok(reached)
    }
}

/// The xref stream numbered `xref_num` (the last number) over `rows`, with `trailer`'s keys
/// and the `/ID` from `permanent` and `hash` (which takes the rows too).
fn write_xref_stream<S: WriteSink>(
    sink: &mut CountingSink<S>,
    mut rows: Rows,
    xref_num: u32,
    trailer: &Dict,
    permanent: Option<lamina_cos::PdfString>,
    mut hash: ContentHash,
) -> Result<()> {
    let size = xref_num.checked_add(1).ok_or_else(exhausted)?;
    let offset = sink.position();
    rows.insert(xref_num, Row::InFile { offset, generation: 0 });
    let (mut dict, data) = stream_table(&rows);
    hash.update(&data);
    dict.insert("Size", size);
    for (k, v) in trailer.iter() {
        dict.insert(k.clone(), v.clone());
    }
    dict.insert("ID", id_array(permanent, hash));
    sink.write_all(&stream_object(xref_num, dict, &data))?;
    sink.write_all(format!("startxref\n{offset}\n%%EOF\n").as_bytes())?;
    Ok(())
}

/// A classic table over `rows`, then the trailer (`/Size` first, then `trailer`'s keys and
/// the `/ID` from `permanent` and `hash`, which takes the table too).
fn write_classic_xref<S: WriteSink>(
    sink: &mut CountingSink<S>,
    rows: &Rows,
    size: u32,
    trailer: &Dict,
    permanent: Option<lamina_cos::PdfString>,
    mut hash: ContentHash,
    options: SaveOptions,
) -> Result<()> {
    let table = classic_table(rows, options.eol())?;
    hash.update(&table);
    let offset = sink.position();
    sink.write_all(&table)?;
    let mut t = Dict::new();
    t.insert("Size", size);
    for (k, v) in trailer.iter() {
        t.insert(k.clone(), v.clone());
    }
    t.insert("ID", id_array(permanent, hash));
    let mut tail = b"trailer\n".to_vec();
    Object::Dict(t).write_to(&mut tail);
    tail.extend_from_slice(format!("\nstartxref\n{offset}\n%%EOF\n").as_bytes());
    sink.write_all(&tail)?;
    Ok(())
}

const fn exhausted() -> SaveError {
    SaveError::Cos(lamina_cos::Error::InvalidStructure {
        kind: "full rewrite",
        message: "object numbers exhausted (Limits::max_object_number)",
    })
}

/// The generation of object `num` if it exists now: 0 for one created in this session, the
/// base's for any other not deleted.
fn generation(doc: &Document, num: u32) -> Option<u16> {
    match doc.overlay().get(num) {
        Some(OverlayEntry::New(_)) => Some(0),
        Some(OverlayEntry::Deleted) => None,
        _ => doc.base_generation(num),
    }
}

/// Pushes every reference inside `object` onto `out` (iteratively; a stream's own `/Length`
/// is skipped because the writer replaces it with the data length), and checks that no
/// container is nested deeper than `depth`, counted like the parser does (an object's
/// outermost array or dictionary is level 1, scalars add nothing), so [`Remap`] can't fail
/// half-way through the output.
///
/// # Errors
/// [`lamina_cos::Error::LimitExceeded`] ([`Limit::NestingDepth`]).
fn references(object: &Object, depth: usize, out: &mut Vec<ObjRef>) -> Result<()> {
    // Each entry: a value and the number of containers around it.
    let mut stack = vec![(object, 0usize)];
    while let Some((object, around)) = stack.pop() {
        let inner = around + 1;
        if matches!(object, Object::Array(_) | Object::Dict(_) | Object::Stream(_)) && inner > depth {
            return Err(too_deep());
        }
        match object {
            Object::Ref(r) => out.push(*r),
            Object::Array(items) => stack.extend(items.iter().rev().map(|v| (v, inner))),
            Object::Dict(dict) => stack.extend(dict.iter().map(|(_, v)| (v, inner))),
            Object::Stream(stream) => stack.extend(
                stream
                    .dict
                    .iter()
                    .filter(|(k, _)| k.as_bytes() != b"Length")
                    .map(|(_, v)| (v, inner)),
            ),
            _ => {}
        }
    }
    Ok(())
}

const fn too_deep() -> SaveError {
    SaveError::Cos(lamina_cos::Error::LimitExceeded {
        limit: Limit::NestingDepth,
        at: None,
    })
}

/// The trailer's document keys a full rewrite writes: not `/ID` (rewritten) and not
/// `/Encrypt` (a rewrite either refuses an encrypted document or removes the encryption).
fn written_keys(trailer: &Dict) -> impl Iterator<Item = (&lamina_cos::Name, &Object)> {
    document_keys(trailer).filter(|(k, _)| !matches!(k.as_bytes(), b"ID" | b"Encrypt"))
}

/// Rewrites references: through `map`, `None` becoming `null`.
struct Remap<'m> {
    map: &'m dyn Fn(ObjRef) -> Option<ObjRef>,
    depth: usize,
    /// The objects were decrypted as they were read and are written as plaintext: streams lose
    /// a leading `/Crypt` filter (§7.4.10), which the reader already applied.
    decrypted: bool,
}

impl Remap<'_> {
    /// `object` with its references mapped (a stream's data is cloned: [`Output::indirect`]
    /// maps a stream's dictionary only).
    fn object(&self, object: &Object) -> Result<Object> {
        self.at(object, self.depth)
    }

    /// A stream's dictionary with its references mapped; its own `/Length` (rewritten from
    /// the data, in place) loses a reference value.
    fn stream_dict(&self, dict: &Dict) -> Result<Dict> {
        let mut out = self.dict_at(dict, self.depth)?;
        if dict.get("Length").is_some_and(|v| matches!(v, Object::Ref(_))) {
            out.insert("Length", 0);
        }
        if self.decrypted {
            strip_crypt_filter(&mut out);
        }
        Ok(out)
    }

    /// `depth` is the number of containers still allowed around and inside `object`.
    fn at(&self, object: &Object, depth: usize) -> Result<Object> {
        Ok(match object {
            Object::Ref(r) => (self.map)(*r).map_or(Object::Null, Object::Ref),
            Object::Array(items) => {
                let depth = depth.checked_sub(1).ok_or_else(too_deep)?;
                Object::Array(items.iter().map(|item| self.at(item, depth)).collect::<Result<_>>()?)
            }
            Object::Dict(dict) => Object::Dict(self.dict_at(dict, depth)?),
            Object::Stream(stream) => Object::Stream(Stream {
                dict: self.stream_dict(&stream.dict)?,
                data: stream.data.clone(),
            }),
            other => other.clone(),
        })
    }

    fn dict_at(&self, dict: &Dict, depth: usize) -> Result<Dict> {
        let depth = depth.checked_sub(1).ok_or_else(too_deep)?;
        let mut out = Dict::new();
        for (k, v) in dict.iter() {
            out.insert(k.clone(), self.at(v, depth)?);
        }
        Ok(out)
    }
}

/// An object stream (§7.5.7) numbered `num` holding `members` (references mapped), and their
/// xref rows.
fn object_stream(num: u32, members: &[(u32, &Object)], remap: &Remap<'_>) -> Result<(Stream, Vec<(u32, Row)>)> {
    let mut index = Vec::new();
    let mut body = Vec::new();
    let mut rows = Vec::with_capacity(members.len());
    for ((member, object), position) in members.iter().zip(0u16..) {
        rows.push((
            *member,
            Row::InStream {
                stream: num,
                index: position,
            },
        ));
        if position > 0 {
            index.push(b' ');
        }
        index.extend_from_slice(format!("{member} {}", body.len()).as_bytes());
        remap.object(object)?.write_to(&mut body);
        body.push(b'\n');
    }
    index.push(b'\n');
    let first = index.len();
    index.extend_from_slice(&body);
    let dict: Dict = [
        ("Type", Object::name("ObjStm")),
        ("N", Object::from(i64::try_from(members.len()).unwrap_or(i64::MAX))),
        ("First", Object::from(i64::try_from(first).unwrap_or(i64::MAX))),
    ]
    .into_iter()
    .collect();
    Ok((Stream::flate(dict, &index), rows))
}

/// `%PDF-x.y` from the base header (1.7 when it has none), raised to 1.5 for object and xref
/// streams, then the binary marker.
fn header(version: Option<&str>, compact: bool) -> Vec<u8> {
    let parsed = version.and_then(|v| {
        let (major, minor) = v.split_once('.')?;
        Some((major.parse::<u8>().ok()?, minor.parse::<u8>().ok()?))
    });
    let mut version = parsed.unwrap_or((1, 7));
    if compact && version < (1, 5) {
        version = (1, 5);
    }
    let mut out = format!("%PDF-{}.{}\n", version.0, version.1).into_bytes();
    out.extend_from_slice(HEADER_MARKER);
    out
}

/// The sink, counting, and the hash of everything before the xref section.
struct Output<S> {
    sink: CountingSink<S>,
    hash: ContentHash,
}

impl<S: WriteSink> Output<S> {
    const fn position(&self) -> u64 {
        self.sink.position()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<()> {
        self.hash.update(bytes);
        self.sink.write_all(bytes)?;
        Ok(())
    }

    /// `num gen obj … endobj` with references mapped; a stream's data goes to the sink
    /// without a copy.
    fn indirect(&mut self, num: u32, generation: u16, value: &Object, remap: &Remap<'_>) -> Result<()> {
        let mut head = format!("{num} {generation} obj\n").into_bytes();
        match value {
            Object::Stream(stream) => {
                let mut dict = remap.stream_dict(&stream.dict)?;
                dict.insert("Length", i64::try_from(stream.data.len()).unwrap_or(i64::MAX));
                Object::Dict(dict).write_to(&mut head);
                head.extend_from_slice(b"\nstream\n");
                self.write(&head)?;
                self.write(&stream.data)?;
                self.write(b"\nendstream\nendobj\n")
            }
            other => {
                remap.object(other)?.write_to(&mut head);
                head.extend_from_slice(b"\nendobj\n");
                self.write(&head)
            }
        }
    }
}

/// Removes a leading `/Crypt` filter and its `/DecodeParms` entry from a stream dictionary
/// (§7.4.10: `/Crypt` comes first in the chain). The reader decrypted the data already.
fn strip_crypt_filter(dict: &mut Dict) {
    let is_crypt = |o: &Object| matches!(o, Object::Name(n) if n.as_bytes() == b"Crypt");
    match dict.get_mut("Filter") {
        Some(Object::Array(filters)) if filters.first().is_some_and(is_crypt) => {
            filters.remove(0);
            let empty = filters.is_empty();
            match dict.get_mut("DecodeParms") {
                Some(Object::Array(parms)) if !parms.is_empty() => {
                    parms.remove(0);
                }
                Some(Object::Dict(_)) => {
                    dict.remove("DecodeParms");
                }
                _ => {}
            }
            if empty {
                dict.remove("Filter");
                dict.remove("DecodeParms");
            }
        }
        Some(filter) if is_crypt(filter) => {
            dict.remove("Filter");
            dict.remove("DecodeParms");
        }
        _ => {}
    }
}
