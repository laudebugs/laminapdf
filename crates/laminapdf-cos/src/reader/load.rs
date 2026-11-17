//! Loading objects: resolving references, objects in the file and in object streams, and
//! decoding streams, each parsed once and cached.

use std::ops;
use std::sync::Arc;

use laminapdf_syntax::{Dict, Limit, ObjRef, Object, Stream, filters};

use super::window::read_indirect;
use super::xref::XrefEntry;
use super::{Document, Warning, objstm, strip_crypt_filter};
use crate::error::{Error, Result};
use crate::source::Pending;

/// How deep [`Document::decode_stream`] resolves references nested in `/Filter` and
/// `/DecodeParms` (array → parameter dictionary → value). An internal constant: it bounds a
/// fixed structure, not input size (ADR 0003 §3).
const MAX_FILTER_PARAM_DEPTH: usize = 3;

/// The result of [`Document::deref`]: either the direct object that was passed in, or the
/// shared, cached object a reference resolved to. Dereferences to [`Object`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'o> {
    /// The input was not a reference; this is the input itself.
    Direct(&'o Object),
    /// The input was a reference; this is the (cached) object it resolved to.
    Indirect(Arc<Object>),
}

impl Resolved<'_> {
    /// An owned copy of the object (clones unless this is the only handle to it).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn into_owned(self) -> Object {
        match self {
            Self::Direct(obj) => obj.clone(),
            Self::Indirect(arc) => Arc::unwrap_or_clone(arc),
        }
    }

    /// The object behind a shared pointer (a cheap clone for indirect objects).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn into_arc(self) -> Arc<Object> {
        match self {
            Self::Direct(obj) => Arc::new(obj.clone()),
            Self::Indirect(arc) => arc,
        }
    }
}

impl ops::Deref for Resolved<'_> {
    type Target = Object;

    fn deref(&self) -> &Object {
        match self {
            Self::Direct(obj) => obj,
            Self::Indirect(arc) => arc,
        }
    }
}

/// A decoded object stream's members. A member that does not parse keeps its error, which
/// resolving that member returns, exactly as for an object stored in the file (ADR 0003 §1).
pub(super) type ObjStmMembers = Arc<Vec<(u32, laminapdf_syntax::Result<Arc<Object>>)>>;

impl Document {
    /// Resolves an indirect reference, following reference-to-reference chains.
    /// Missing or free objects, and references whose generation number does not match the
    /// xref entry's, resolve to `null` (§7.3.10; the latter raises
    /// [`Warning::StaleGeneration`]).
    ///
    /// Objects are parsed once and cached: resolving the same reference again returns a
    /// clone of the same [`Arc`].
    ///
    /// # Errors
    /// [`Error::ReferenceCycle`] for chains that loop; [`Error::LimitExceeded`] for chains
    /// longer than [`Limits::max_resolve_depth`] or any other hard limit; parse errors for
    /// corrupt objects; [`Error::Strict`] in strict mode.
    ///
    /// [`Limits::max_resolve_depth`]: laminapdf_syntax::Limits::max_resolve_depth
    ///
    /// Added in 0.2.0.
    pub fn resolve(&self, r: ObjRef) -> Result<Arc<Object>> {
        let _call = self.call();
        let mut chain = Vec::new();
        let mut current = r;
        loop {
            if chain.contains(&current.num) {
                return Err(Error::ReferenceCycle(current.num));
            }
            if chain.len() >= self.limits.max_resolve_depth() {
                return Err(Error::limit_at_object(Limit::ResolveDepth, current.num));
            }
            chain.push(current.num);
            let obj = self.load_current(current.num, Some(current.gen_nr))?;
            match *obj {
                Object::Ref(next) => current = next,
                _ => return Ok(obj),
            }
        }
    }

    /// The value of indirect object `num` exactly as stored, without following a top-level
    /// reference (`1 0 obj 2 0 R endobj` yields `2 0 R`). Missing/free objects are `null`.
    /// Any generation is accepted: this looks up the number's current definition.
    ///
    /// # Errors
    /// Parse errors for corrupt objects; [`Error::ReferenceCycle`] for self-referential
    /// streams; [`Error::LimitExceeded`]; [`Error::Strict`] in strict mode.
    ///
    /// Added in 0.2.0.
    pub fn object(&self, num: u32) -> Result<Arc<Object>> {
        let _call = self.call();
        self.load_current(num, None)
    }

    /// Returns `obj` itself, or the object it refers to.
    ///
    /// # Errors
    /// As [`Document::resolve`].
    ///
    /// Added in 0.2.0.
    pub fn deref<'o>(&self, obj: &'o Object) -> Result<Resolved<'o>> {
        match obj {
            Object::Ref(r) => self.resolve(*r).map(Resolved::Indirect),
            other => Ok(Resolved::Direct(other)),
        }
    }

    /// Loads object `num` without following a top-level reference. `in_flight` holds the
    /// objects currently being loaded further up the stack, to break `/Length` cycles.
    ///
    /// Successful loads are cached. Caching is sound even though a nested load sees a
    /// non-empty `in_flight`: the only thing that depends on it is an indirect `/Length`,
    /// which must resolve to an integer, and integers never depend on other objects.
    ///
    /// With `gen_nr`, a reference whose generation does not match the entry's loads `null`
    /// (§7.3.10). The cache is keyed by number, which is sound because each number has one
    /// entry, and the generation is checked before the cache is consulted.
    pub(super) fn load(&self, num: u32, gen_nr: Option<u16>, in_flight: &mut Vec<u32>) -> Result<Arc<Object>> {
        if let (Some(gen_nr), Some(entry)) = (gen_nr, self.entries.get(&num))
            && !entry.matches_gen(gen_nr)
        {
            if !matches!(entry, XrefEntry::Free) {
                self.warn(Warning::StaleGeneration {
                    object: num,
                    generation: gen_nr,
                })?;
            }
            return Ok(Arc::new(Object::Null));
        }
        if let Some(cached) = self.object_cache.borrow().get(&num) {
            return Ok(Arc::clone(cached));
        }
        if in_flight.contains(&num) {
            return Err(Error::ReferenceCycle(num));
        }
        if in_flight.len() >= self.limits.max_resolve_depth() {
            return Err(Error::limit_at_object(Limit::ResolveDepth, num));
        }
        let Some(&entry) = self.entries.get(&num) else {
            return Ok(Arc::new(Object::Null));
        };
        in_flight.push(num);
        let result = match entry {
            XrefEntry::Free => Ok(Arc::new(Object::Null)),
            XrefEntry::InFile { offset, gen_nr } => self.load_in_file(num, gen_nr, offset, in_flight),
            XrefEntry::InStream { stream, index } => self.load_compressed(num, stream, index, in_flight),
        };
        in_flight.pop();
        if let Ok(obj) = &result {
            self.object_cache.borrow_mut().insert(num, Arc::clone(obj));
        }
        result
    }

    fn load_in_file(&self, num: u32, gen_nr: u16, offset: usize, in_flight: &mut Vec<u32>) -> Result<Arc<Object>> {
        // An unresolvable indirect `/Length` (missing, malformed, a cycle, over a hard limit)
        // is one of ADR 0003's tolerant paths: the extent is recovered by scanning, with
        // `stream-length-recovered`. Strict-mode refusals and missing input end the call.
        let mut fatal = None;
        let parsed = {
            let mut resolve_length = |r: ObjRef| match self.load(r.num, Some(r.gen_nr), in_flight) {
                Ok(obj) => obj.as_int(),
                Err(e) => {
                    if e.must_propagate() {
                        fatal.get_or_insert(e);
                    }
                    None
                }
            };
            read_indirect(&self.source, offset, &self.limits, &mut resolve_length)
        };
        match (fatal, parsed) {
            // The `/Length` object and a longer window are both missing: ask for both at once.
            (Some(e @ Error::DataPending { .. }), Err(more @ Error::DataPending { .. })) => {
                let mut pending = Pending::default();
                pending.absorb(e)?;
                pending.absorb(more)?;
                Err(pending.into_error())
            }
            (Some(e), _) => Err(e),
            (None, parsed) => self.finish_in_file(num, gen_nr, parsed??, in_flight),
        }
    }

    /// The rest of [`Document::load_in_file`] once `parsed` is read: the length warning and
    /// decryption.
    fn finish_in_file(
        &self,
        num: u32,
        gen_nr: u16,
        mut parsed: laminapdf_syntax::IndirectObject,
        in_flight: &mut Vec<u32>,
    ) -> Result<Arc<Object>> {
        if parsed.length_recovered {
            self.warn(Warning::StreamLengthRecovered { object: num })?;
        }
        if parsed.skipped_keys {
            self.warn(Warning::DictionaryKeySkipped { object: Some(num) })?;
        }
        if let Some(security) = &self.security {
            let mut fatal = None;
            let mut resolve = |o: &Object| match o {
                Object::Ref(r) => match self.load(r.num, Some(r.gen_nr), in_flight) {
                    Ok(obj) => Some((*obj).clone()),
                    Err(e) => {
                        if e.must_propagate() {
                            fatal.get_or_insert(e);
                        }
                        None
                    }
                },
                other => Some(other.clone()),
            };
            let well_formed = security.decrypt_object(&mut parsed.object, num, gen_nr, &mut resolve);
            if let Some(e) = fatal {
                return Err(e);
            }
            if !well_formed {
                self.warn(Warning::AesPaddingInvalid { object: num })?;
            }
        }
        Ok(Arc::new(parsed.object))
    }

    fn load_compressed(&self, num: u32, stream: u32, index: u32, in_flight: &mut Vec<u32>) -> Result<Arc<Object>> {
        let members = self.object_stream(stream, in_flight)?;
        let by_index = usize::try_from(index)
            .ok()
            .and_then(|i| members.get(i))
            .filter(|(n, _)| *n == num);
        // Member offsets are in the stream's decoded data, not the file (ADR 0026 §1.1).
        let tagged = |member: &laminapdf_syntax::Result<Arc<Object>>| {
            member
                .clone()
                .map_err(|e| Error::from(e.map_position(|p| p.in_stream(stream))))
        };
        if let Some((_, member)) = by_index {
            return tagged(member);
        }
        // Fall back to a search when the index is stale or wrong.
        match members.iter().find(|(n, _)| *n == num) {
            Some((_, member)) => {
                self.warn(Warning::ObjStmIndexMismatch { object: num })?;
                tagged(member)
            }
            None => Ok(Arc::new(Object::Null)),
        }
    }

    fn object_stream(&self, stream: u32, in_flight: &mut Vec<u32>) -> Result<ObjStmMembers> {
        if let Some(cached) = self.objstm_cache.borrow().get(&stream) {
            return Ok(Arc::clone(cached));
        }
        if !matches!(self.entries.get(&stream), Some(XrefEntry::InFile { .. })) {
            return Err(Error::InvalidStructure {
                kind: "object stream",
                message: "container is not a top-level object",
            });
        }
        let container = self.load(stream, None, in_flight)?;
        let Object::Stream(s) = &*container else {
            return Err(Error::Type {
                expected: "stream",
                context: "object stream container",
            });
        };
        let decoded = self.decode_stream_of(s, Some(stream))?;
        let mut members = Vec::new();
        for member in objstm::parse(&s.dict, &decoded, &self.limits)? {
            if member.skipped_keys {
                self.warn(Warning::DictionaryKeySkipped {
                    object: Some(member.num),
                })?;
            }
            members.push((member.num, member.object.map(Arc::new)));
        }
        let members = Arc::new(members);
        self.objstm_cache.borrow_mut().insert(stream, Arc::clone(&members));
        Ok(members)
    }

    /// Decodes a stream's data (Flate + PNG predictors), resolving indirect filter parameters.
    ///
    /// # Errors
    /// Unsupported filters, corrupt data, or [`Error::LimitExceeded`] for output over
    /// [`Limits::max_stream_decoded`] or [`Limits::max_decoded_total`], or a chain longer than
    /// [`Limits::max_filter_chain`]; [`Error::Strict`] in strict mode for truncated Flate data.
    ///
    /// [`Limits::max_stream_decoded`]: laminapdf_syntax::Limits::max_stream_decoded
    /// [`Limits::max_decoded_total`]: laminapdf_syntax::Limits::max_decoded_total
    /// [`Limits::max_filter_chain`]: laminapdf_syntax::Limits::max_filter_chain
    ///
    /// Added in 0.2.0.
    pub fn decode_stream(&self, stream: &Stream) -> Result<Vec<u8>> {
        let _call = self.call();
        self.decode_stream_of(stream, None)
    }

    /// The first `max` bytes `stream` decodes to, and whether they are all of it, decoding at
    /// most one byte more ([`filters::decode_prefix`]): a view that shows the start of a stream
    /// never decodes the rest. The document's limits still apply.
    ///
    /// # Errors
    /// As [`Document::decode_stream`].
    ///
    /// Added in 0.6.0.
    pub fn decode_stream_prefix(&self, stream: &Stream, max: usize) -> Result<(Vec<u8>, bool)> {
        let _call = self.call();
        let params = self.filter_params(stream)?;
        let mut budget = self.budget.get();
        let read = filters::decode_prefix(&params, &stream.data, &self.limits, &mut budget, max);
        self.budget.set(budget);
        let read = read?;
        Ok((read.decoded.data, read.complete))
    }

    /// The filter names of the stream whose dictionary is `dict`, in decoding order, an indirect
    /// `/Filter` resolved and inline-image abbreviations written out (`DCT` is `DCTDecode`);
    /// empty without filters or when `/Filter` can't be read.
    ///
    /// ```
    /// use laminapdf_cos::{Dict, Document, Object, OpenOptions};
    ///
    /// let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\n2 0 obj [/AHx /Fl] endobj\n\
    ///     trailer << /Root 1 0 R >>\n";
    /// let doc = Document::open(bytes.to_vec(), OpenOptions::default())?;
    /// let dict: Dict = [("Filter", Object::Ref(laminapdf_cos::ObjRef::new(2, 0)))].into_iter().collect();
    /// assert_eq!(doc.stream_filters(&dict), ["ASCIIHexDecode", "FlateDecode"]);
    /// # Ok::<(), laminapdf_cos::Error>(())
    /// ```
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn stream_filters(&self, dict: &Dict) -> Vec<String> {
        let full = |name: &[u8]| String::from_utf8_lossy(filters::full_name(name)).into_owned();
        let Some(filter) = dict.get("Filter") else {
            return Vec::new();
        };
        let Ok(resolved) = self.deref(filter) else {
            return Vec::new();
        };
        match &*resolved {
            Object::Name(name) => vec![full(name.as_bytes())],
            Object::Array(items) => items
                .iter()
                .filter_map(|item| match item {
                    Object::Name(name) => Some(full(name.as_bytes())),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// `stream`'s `/Filter` and `/DecodeParms`, resolved, less a `/Crypt` filter already applied.
    fn filter_params(&self, stream: &Stream) -> Result<Dict> {
        let mut params = Dict::new();
        for key in ["Filter", "DecodeParms"] {
            if let Some(v) = stream.dict.get(key) {
                params.insert(key, self.resolve_nested(v, MAX_FILTER_PARAM_DEPTH)?);
            }
        }
        if self.security.is_some() {
            // Streams of an encrypted document were decrypted when they were loaded, which
            // applied the `/Crypt` filter (§7.4.10); the rest of the chain decodes as usual.
            strip_crypt_filter(&mut params);
        }
        Ok(params)
    }

    /// [`Document::decode_stream`] for the stream object `object` (named in warnings).
    pub(super) fn decode_stream_of(&self, stream: &Stream, object: Option<u32>) -> Result<Vec<u8>> {
        let params = self.filter_params(stream)?;
        let mut budget = self.budget.get();
        let decoded = filters::decode(&params, &stream.data, &self.limits, &mut budget);
        self.budget.set(budget);
        let decoded = decoded?;
        if decoded.flate_truncated {
            self.warn(Warning::FlateTruncated { object })?;
        }
        if decoded.characters_skipped {
            self.warn(Warning::FilterCharactersSkipped { object })?;
        }
        Ok(decoded.data)
    }

    /// `obj` with references resolved, including those inside arrays and dictionaries down to
    /// `depth` levels (missing objects become `null`, which the filters treat as absent).
    pub(super) fn resolve_nested(&self, obj: &Object, depth: usize) -> Result<Object> {
        let resolved = self.deref(obj)?;
        let Some(depth) = depth.checked_sub(1) else {
            return Ok(resolved.into_owned());
        };
        Ok(match &*resolved {
            Object::Array(items) => Object::Array(
                items
                    .iter()
                    .map(|item| self.resolve_nested(item, depth))
                    .collect::<Result<_>>()?,
            ),
            Object::Dict(dict) => Object::Dict(
                dict.iter()
                    .map(|(k, v)| Ok((k.clone(), self.resolve_nested(v, depth)?)))
                    .collect::<Result<Dict>>()?,
            ),
            _ => resolved.into_owned(),
        })
    }
}
