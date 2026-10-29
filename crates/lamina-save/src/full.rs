//! The full-rewrite writer ([ADR 0007] §5): a new file holding only what the trailer
//! reaches.
//!
//! [ADR 0007]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0007-save-planner.md

use std::collections::BTreeMap;
use std::sync::Arc;

use lamina_cos::{Dict, Document, Limit, ObjRef, Object, OverlayEntry, Stream};

use crate::error::{Result, SaveError};
use crate::id::{ContentHash, id_array, permanent_id};
use crate::legacy::{HEADER_MARKER, is_xref_key};
use crate::options::{Renumber, SaveMode, SaveOptions, SaveReport, XrefKind};
use crate::sink::{CountingSink, WriteSink};
use crate::xref::{Row, Rows, classic_table, link_free_list, stream_object, stream_table};

/// Objects per object stream: small enough that reading one member stays cheap.
const OBJSTM_CAPACITY: usize = 100;

/// Writes a document as a new file: header, the reachable objects, object streams, xref.
///
/// * **Reachability** (iterative, each object visited once): from the trailer's document
///   keys (`/Root`, `/Info`, …) through every reference, skipping a stream's `/Length`
///   (written directly). Objects no path reaches (earlier revisions, orphans) are dropped.
///   A reference to an object that doesn't exist, or with a generation that isn't the
///   object's, resolves to `null` (§7.3.10) and is written as `null`; an object that can't
///   be parsed is written as `null` too, as the reader would see it.
/// * **Numbering**: [`Renumber::Keep`] keeps numbers and generations (gaps become free
///   entries); [`Renumber::Compact`] numbers the objects `1..=n` in their old order with
///   generation 0 and reports the map.
/// * **Layout** ([`XrefKind::Stream`], the default): stream objects and objects with a
///   generation above 0 top-level, every other object in Flate object streams of up to 100
///   members in number order (§7.5.7 forbids streams and non-zero generations there), then
///   an xref stream; the header is raised to 1.5 if lower. [`XrefKind::Classic`] writes every
///   object top-level and a classic table. Object streams and the xref stream are numbered
///   after every number in use or reserved ([`Document::next_number`]).
/// * **`/ID`**: `/ID[0]` kept, `/ID[1]` the hash of everything before the xref section.
///
/// Every object is loaded before the first byte is written, so unreadable input
/// ([`lamina_cos::Error::DataPending`]) leaves the sink empty. The output is
/// byte-deterministic (ADR 0001).
#[derive(Debug, Clone, Copy)]
pub struct FullWriter<'d> {
    doc: &'d Document,
    options: SaveOptions,
}

/// A reachable object: its number and generation in the input, and its value.
struct Reached {
    generation: u16,
    value: Arc<Object>,
}

impl<'d> FullWriter<'d> {
    /// A writer for `doc` as it is now (overlay over base); of the options, `renumber`,
    /// `xref` and `eol` apply.
    #[must_use]
    pub const fn new(doc: &'d Document, options: SaveOptions) -> Self {
        Self { doc, options }
    }

    /// Writes to `sink`.
    ///
    /// # Errors
    /// Before anything is written: [`SaveError::EncryptedBase`]; strict-mode refusals and
    /// unavailable input while loading objects; [`lamina_cos::Error::LimitExceeded`] for an
    /// object nested deeper than [`Limits::max_nesting_depth`](lamina_cos::Limits::max_nesting_depth);
    /// [`lamina_cos::Error::InvalidStructure`] when object numbers run out. While writing:
    /// [`SaveError::Sink`].
    pub fn write<S: WriteSink + ?Sized>(&self, sink: &mut S) -> Result<SaveReport> {
        let doc = self.doc;
        if doc.is_encrypted() {
            return Err(SaveError::EncryptedBase);
        }
        let trailer = doc.trailer();
        let depth = doc.limits().max_nesting_depth();
        let reached = self.reach(trailer, depth)?;
        let numbers = self.numbering(&reached);
        let map = |r: ObjRef| -> Option<ObjRef> {
            reached.get(&r.num).filter(|o| o.generation == r.gen_nr)?;
            let &(num, generation) = numbers.get(&r.num)?;
            Some(ObjRef::new(num, generation))
        };
        let remapper = Remap { map: &map, depth };
        let permanent = permanent_id(doc, trailer)?;
        let mut document_keys = Dict::new();
        for (k, v) in trailer.iter().filter(|(k, _)| !is_xref_key(k)) {
            document_keys.insert(k.clone(), remapper.object(v)?);
        }
        // New number order (the same as the old order: numbering keeps it).
        let objects: Vec<(u32, u16, &Object)> = reached
            .iter()
            .filter_map(|(old, o)| numbers.get(old).map(|&(num, generation)| (num, generation, &*o.value)))
            .collect();

        let compact = self.options.xref() == XrefKind::Stream;
        let highest = objects.last().map_or(0, |(num, _, _)| *num);
        let mut next = match self.options.renumber() {
            Renumber::Keep => doc.next_number().max(u64::from(highest) + 1),
            Renumber::Compact => u64::from(highest) + 1,
        };
        let mut allocate = || -> Result<u32> {
            let num = u32::try_from(next).map_err(|_| exhausted())?;
            next += 1;
            Ok(num)
        };

        let mut out = Output {
            sink: CountingSink::new(sink),
            hash: ContentHash::default(),
        };
        out.write(&header(doc.header_version(), compact))?;
        let mut rows = Rows::new();
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
        for group in packed.chunks(OBJSTM_CAPACITY) {
            let stream_num = allocate()?;
            let (stream, members) = object_stream(stream_num, group, &remapper)?;
            rows.extend(members);
            rows.insert(
                stream_num,
                Row::InFile {
                    offset: out.position(),
                    generation: 0,
                },
            );
            out.indirect(stream_num, 0, &Object::Stream(stream), &Remap { map: &Some, depth })?;
        }

        let Output { mut sink, hash } = out;
        let id = id_array(permanent, hash);
        let mut trailer = document_keys;
        trailer.insert("ID", id);
        if compact {
            let xref_num = allocate()?;
            write_xref_stream(&mut sink, rows, xref_num, &trailer)?;
        } else {
            let size = u32::try_from(next).map_err(|_| exhausted())?;
            write_classic_xref(&mut sink, rows, size, &trailer, self.options)?;
        }
        let renumbered = (self.options.renumber() == Renumber::Compact)
            .then(|| numbers.iter().map(|(&old, &(new, _))| (old, new)).collect());
        Ok(SaveReport {
            bytes_written: sink.position(),
            mode: SaveMode::FullRewrite,
            renumbered,
            append_offset: None,
        })
    }

    /// Every object reachable from `trailer`'s document keys, by number.
    fn reach(&self, trailer: &Dict, depth: usize) -> Result<BTreeMap<u32, Reached>> {
        let doc = self.doc;
        let mut reached = BTreeMap::new();
        let mut pending = Vec::new();
        for (_, value) in trailer.iter().filter(|(k, _)| !is_xref_key(k)) {
            references(value, depth, &mut pending)?;
        }
        while let Some(r) = pending.pop() {
            if reached.contains_key(&r.num) || generation(doc, r.num) != Some(r.gen_nr) {
                continue;
            }
            let value = match doc.object(r.num) {
                Ok(value) => value,
                Err(
                    e @ (lamina_cos::Error::Strict { .. }
                    | lamina_cos::Error::DataPending { .. }
                    | lamina_cos::Error::Source(_)),
                ) => return Err(e.into()),
                Err(_) => Arc::new(Object::Null),
            };
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

    /// Old number → (new number, new generation).
    fn numbering(&self, reached: &BTreeMap<u32, Reached>) -> BTreeMap<u32, (u32, u16)> {
        match self.options.renumber() {
            Renumber::Compact => reached.keys().zip(1u32..).map(|(&old, new)| (old, (new, 0))).collect(),
            _ => reached.iter().map(|(&num, o)| (num, (num, o.generation))).collect(),
        }
    }
}

/// The xref stream numbered `xref_num` (the last number) over `rows`, with `trailer`'s keys.
fn write_xref_stream<S: WriteSink>(
    sink: &mut CountingSink<S>,
    mut rows: Rows,
    xref_num: u32,
    trailer: &Dict,
) -> Result<()> {
    let size = xref_num.checked_add(1).ok_or_else(exhausted)?;
    let offset = sink.position();
    rows.insert(xref_num, Row::InFile { offset, generation: 0 });
    fill_gaps(&mut rows, size);
    let (mut dict, data) = stream_table(&rows);
    dict.insert("Size", size);
    for (k, v) in trailer.iter() {
        dict.insert(k.clone(), v.clone());
    }
    sink.write_all(&stream_object(xref_num, dict, &data))?;
    sink.write_all(format!("startxref\n{offset}\n%%EOF\n").as_bytes())?;
    Ok(())
}

/// A classic table of `size` entries over `rows`, then the trailer (`/Size` first).
fn write_classic_xref<S: WriteSink>(
    sink: &mut CountingSink<S>,
    mut rows: Rows,
    size: u32,
    trailer: &Dict,
    options: SaveOptions,
) -> Result<()> {
    fill_gaps(&mut rows, size);
    let offset = sink.position();
    sink.write_all(&classic_table(&rows, options.eol())?)?;
    let mut t = Dict::new();
    t.insert("Size", size);
    for (k, v) in trailer.iter() {
        t.insert(k.clone(), v.clone());
    }
    let mut tail = b"trailer\n".to_vec();
    Object::Dict(t).write_to(&mut tail);
    tail.extend_from_slice(format!("\nstartxref\n{offset}\n%%EOF\n").as_bytes());
    sink.write_all(&tail)?;
    Ok(())
}

const fn exhausted() -> SaveError {
    SaveError::Cos(lamina_cos::Error::InvalidStructure {
        kind: "full rewrite",
        message: "object numbers exhausted",
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

/// Pushes every reference inside `object` onto `out` (iteratively; a stream's `/Length` is
/// skipped because the writer replaces it with the data length), and checks that nothing is
/// nested deeper than `depth` (so [`Remap`] can't fail half-way through the output).
///
/// # Errors
/// [`lamina_cos::Error::LimitExceeded`] ([`Limit::NestingDepth`]).
fn references(object: &Object, depth: usize, out: &mut Vec<ObjRef>) -> Result<()> {
    let mut stack = vec![(object, 0usize)];
    while let Some((object, level)) = stack.pop() {
        if level >= depth {
            return Err(too_deep());
        }
        let inner = level + 1;
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

/// Rewrites references: through `map`, `None` becoming `null`.
struct Remap<'m> {
    map: &'m dyn Fn(ObjRef) -> Option<ObjRef>,
    depth: usize,
}

impl Remap<'_> {
    /// `object` with its references mapped (a stream's data is cloned: use
    /// [`Remap::dict`] for streams being written).
    fn object(&self, object: &Object) -> Result<Object> {
        self.at(object, self.depth)
    }

    /// A dictionary with its references mapped.
    fn dict(&self, dict: &Dict) -> Result<Dict> {
        self.dict_at(dict, self.depth)
    }

    fn at(&self, object: &Object, depth: usize) -> Result<Object> {
        let depth = depth.checked_sub(1).ok_or_else(too_deep)?;
        Ok(match object {
            Object::Ref(r) => (self.map)(*r).map_or(Object::Null, Object::Ref),
            Object::Array(items) => {
                Object::Array(items.iter().map(|item| self.at(item, depth)).collect::<Result<_>>()?)
            }
            Object::Dict(dict) => Object::Dict(self.dict_at(dict, depth)?),
            Object::Stream(stream) => Object::Stream(Stream {
                dict: self.dict_at(&stream.dict, depth)?,
                data: stream.data.clone(),
            }),
            other => other.clone(),
        })
    }

    fn dict_at(&self, dict: &Dict, depth: usize) -> Result<Dict> {
        let mut out = Dict::new();
        for (k, v) in dict.iter() {
            // `/Length` is rewritten from the data (in place), so a reference there is dropped.
            let value = if k.as_bytes() == b"Length" && matches!(v, Object::Ref(_)) {
                Object::Integer(0)
            } else {
                self.at(v, depth)?
            };
            out.insert(k.clone(), value);
        }
        Ok(out)
    }
}

/// Free entries for every number below `size` without an object (the free list links them).
fn fill_gaps(rows: &mut Rows, size: u32) {
    for num in 1..size {
        rows.entry(num).or_insert(Row::Free { next: 0, generation: 0 });
    }
    link_free_list(rows);
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
                let mut dict = remap.dict(&stream.dict)?;
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
