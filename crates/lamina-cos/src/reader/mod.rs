//! Tolerant reader: xref/xref-stream loading with repair, object resolution, page tree.

mod objstm;
mod options;
mod repair;
mod warning;
mod xref;

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops;
use std::sync::Arc;

pub use options::OpenOptions;
pub use warning::Warning;
use warning::WarningLog;
pub(crate) use xref::XREF_STREAM_KEYS;
use xref::{XrefEntry, XrefTable};

use lamina_syntax::filters;
use lamina_syntax::{DecodeBudget, Dict, Limit, Limits, ObjRef, Object, Parser, Stream, find};

use crate::error::{Error, Result};
use crate::source::{ByteSource, Source};

/// How deep [`Document::decode_stream`] resolves references nested in `/Filter` and
/// `/DecodeParms` (array → parameter dictionary → value). An internal constant: it bounds a
/// fixed structure, not input size (ADR 0003 §3).
const MAX_FILTER_PARAM_DEPTH: usize = 3;

/// A page with its inheritable attributes resolved (§7.7.3.4).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Page {
    /// The page object's reference (`None` for a non-standard direct page dictionary).
    pub obj_ref: Option<ObjRef>,
    /// The page dictionary itself.
    pub dict: Dict,
    /// `/MediaBox`, own or inherited.
    pub media_box: Option<[f64; 4]>,
    /// `/CropBox`, own or inherited.
    pub crop_box: Option<[f64; 4]>,
    /// `/Rotate`, own or inherited (0 if absent).
    pub rotate: i64,
    /// `/Resources`, own or inherited, resolved. Always an [`Object::Dict`]; shared (not
    /// copied) between pages that inherit the same resources. See [`Page::resources_dict`].
    pub resources: Option<Arc<Object>>,
}

impl Page {
    /// The resolved `/Resources` dictionary, if any.
    #[must_use]
    pub fn resources_dict(&self) -> Option<&Dict> {
        match self.resources.as_deref() {
            Some(Object::Dict(d)) => Some(d),
            _ => None,
        }
    }
}

/// The result of [`Document::deref`]: either the direct object that was passed in, or the
/// shared, cached object a reference resolved to. Dereferences to [`Object`].
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'o> {
    /// The input was not a reference; this is the input itself.
    Direct(&'o Object),
    /// The input was a reference; this is the (cached) object it resolved to.
    Indirect(Arc<Object>),
}

impl Resolved<'_> {
    /// An owned copy of the object (clones unless this is the only handle to it).
    #[must_use]
    pub fn into_owned(self) -> Object {
        match self {
            Self::Direct(obj) => obj.clone(),
            Self::Indirect(arc) => Arc::unwrap_or_clone(arc),
        }
    }

    /// The object behind a shared pointer (a cheap clone for indirect objects).
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
type ObjStmMembers = Arc<Vec<(u32, lamina_syntax::Result<Arc<Object>>)>>;

/// An opened PDF ([ADR 0005] §2). Owns its input as a shared [`ByteSource`] and has no
/// lifetime parameter; objects are parsed lazily on first [`Document::resolve`] and then
/// cached, so repeated lookups share one [`Arc<Object>`].
///
/// **Threads (ADR 0005 §7):** a `Document` is `Send` but deliberately `!Sync`. It can move
/// to another thread, but one thread uses it at a time (its caches are `RefCell`s). For
/// parallel work, open one `Document` per thread over the same `Arc<dyn ByteSource>`;
/// opening is cheap because everything is lazy.
///
/// ```compile_fail
/// fn is_sync<T: Sync>() {}
/// is_sync::<lamina_cos::Document>();
/// ```
///
/// **Limits and warnings (ADR 0003):** every call is bounded by the [`OpenOptions`]'
/// [`Limits`]; decoding shares one [`Limits::max_decoded_total`] budget per top-level call.
/// Repairs are reported as [`Warning`]s, raised at open or lazily as objects are resolved
/// ([`Document::warnings`]), or refused with [`Error::Strict`] in strict mode.
///
/// [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about the opened file, not states of one machine"
)]
pub struct Document {
    /// The input; a source without an in-memory slice is read once and the copy is shared
    /// by every document opened from clones of the same [`Source`].
    source: Source,
    entries: BTreeMap<u32, XrefEntry>,
    /// The highest object number in any xref section, including entries dropped for
    /// exceeding [`Limits::max_object_number`] (the incremental writer never goes below it).
    max_object_number: Option<u32>,
    trailer: Dict,
    revisions: usize,
    repaired: bool,
    has_xref_stream: bool,
    newest_is_stream: bool,
    startxref: Option<usize>,
    header_version: Option<String>,
    limits: Limits,
    strict: bool,
    warnings: RefCell<WarningLog>,
    /// Nesting of public calls in progress; the outermost one owns the decode budget.
    call_depth: Cell<usize>,
    /// What is left of the current top-level call's [`Limits::max_decoded_total`].
    budget: Cell<DecodeBudget>,
    objstm_cache: RefCell<BTreeMap<u32, ObjStmMembers>>,
    /// Successfully loaded objects by number (errors are not cached).
    object_cache: RefCell<BTreeMap<u32, Arc<Object>>>,
}

// ADR 0005 §7: `Document` (and what it hands out) can move between threads.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<Document>();
    assert_send::<Page>();
    assert_send::<Error>();
};

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("source", &self.source)
            .field("objects", &self.entries.len())
            .field("revisions", &self.revisions)
            .field("repaired", &self.repaired)
            .field("header_version", &self.header_version)
            .field("strict", &self.strict)
            .finish_non_exhaustive()
    }
}

/// Marks a public call in progress; the outermost one starts a fresh decode budget.
struct Call<'d> {
    doc: &'d Document,
}

impl Drop for Call<'_> {
    fn drop(&mut self) {
        self.doc.call_depth.set(self.doc.call_depth.get().saturating_sub(1));
    }
}

/// Errors no tolerant path may swallow: strict-mode refusals (the caller asked for them) and
/// input that is missing or unreadable (the call has to be retried or fail).
const fn must_propagate(e: &Error) -> bool {
    matches!(e, Error::Strict { .. } | Error::DataPending { .. } | Error::Source(_))
}

/// Errors that end the page-tree walk instead of skipping a node: cycles (ADR 0003 §1) and
/// [`must_propagate`] ones. Hard limits are not among them: the walk is one of ADR 0003's
/// tolerant paths, so it skips the node with `page-tree-node-skipped` (§3).
const fn ends_page_walk(e: &Error) -> bool {
    matches!(e, Error::ReferenceCycle(_)) || must_propagate(e)
}

/// For reads that are not one of ADR 0003's tolerant paths but fall back to "absent" (an
/// inherited page attribute, `/Count`): `Ok(None)` for an object that is merely malformed,
/// the error for anything else. Cycles, hard limits ([`Error::LimitExceeded`]) and
/// [`must_propagate`] errors fail the call, as ADR 0003 §3 requires outside tolerant paths.
fn absent_if_malformed<T>(result: Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(e) if ends_page_walk(&e) || matches!(e, Error::LimitExceeded { .. }) => Err(e),
        Err(_) => Ok(None),
    }
}

fn read_header_version(data: &[u8], header_pos: usize) -> Option<String> {
    let start = header_pos + b"%PDF-".len();
    let rest = data.get(start..)?;
    let len = rest
        .iter()
        .take(8)
        .take_while(|b| b.is_ascii_digit() || **b == b'.')
        .count();
    let v = std::str::from_utf8(rest.get(..len)?).ok()?;
    (!v.is_empty()).then(|| v.to_owned())
}

/// True when every in-file xref entry points at its own `num gen obj` header (number and
/// generation).
fn entries_are_valid(data: &[u8], table: &XrefTable, limits: &Limits) -> bool {
    table.entries.iter().all(|(&num, entry)| match *entry {
        XrefEntry::InFile { offset, gen_nr } => Parser::new(data, offset)
            .with_limits(limits)
            .object_header()
            .is_ok_and(|id| id.num == num && id.gen_nr == gen_nr),
        XrefEntry::Free | XrefEntry::InStream { .. } => true,
    })
}

/// `obj` as a rectangle; `None` when it is absent, malformed or not four numbers.
fn as_rect(doc: &Document, obj: &Object) -> Result<Option<[f64; 4]>> {
    let Some(arr) = absent_if_malformed(doc.deref(obj))? else {
        return Ok(None);
    };
    let Some(items) = arr.as_array().filter(|items| items.len() == 4) else {
        return Ok(None);
    };
    let mut out = [0.0; 4];
    for (slot, item) in out.iter_mut().zip(items) {
        match absent_if_malformed(doc.deref(item))?.and_then(|n| n.as_f64()) {
            Some(n) => *slot = n,
            None => return Ok(None),
        }
    }
    Ok(Some(out))
}

/// Inheritable page attributes carried down the page tree. Values are shared via [`Arc`],
/// so passing them to every kid (and every page) never copies a dictionary.
#[derive(Debug, Clone, Default)]
struct Inherited {
    media_box: Option<Arc<Object>>,
    crop_box: Option<Arc<Object>>,
    rotate: Option<Arc<Object>>,
    resources: Option<Arc<Object>>,
}

const INHERITABLE: [&str; 4] = ["MediaBox", "CropBox", "Rotate", "Resources"];

impl Inherited {
    /// `parent` with any attributes `dict` sets itself replacing the inherited ones.
    /// Returns `parent` unchanged (a pointer copy) when `dict` sets none of them.
    ///
    /// A value that is `null`, or a reference that `is_null` says resolves to `null` (a missing
    /// or free object), counts as absent (§7.3.7, §7.3.10): the inherited value is kept.
    ///
    /// # Errors
    /// Whatever `is_null` fails with (it resolves references).
    fn overlay(parent: &Arc<Self>, dict: &Dict, is_null: impl Fn(&Object) -> Result<bool>) -> Result<Arc<Self>> {
        let mut own: [Option<&Object>; 4] = [None; 4];
        for (slot, key) in own.iter_mut().zip(INHERITABLE) {
            if let Some(value) = dict.get(key)
                && !is_null(value)?
            {
                *slot = Some(value);
            }
        }
        if own.iter().all(Option::is_none) {
            return Ok(Arc::clone(parent));
        }
        let [media_box, crop_box, rotate, resources] = own;
        let pick = |own: Option<&Object>, inherited: &Option<Arc<Object>>| {
            own.map(|o| Arc::new(o.clone())).or_else(|| inherited.clone())
        };
        Ok(Arc::new(Self {
            media_box: pick(media_box, &parent.media_box),
            crop_box: pick(crop_box, &parent.crop_box),
            rotate: pick(rotate, &parent.rotate),
            resources: pick(resources, &parent.resources),
        }))
    }
}

/// What [`Document::new`] needs besides the xref table.
struct Base {
    source: Source,
    header_version: Option<String>,
    limits: Limits,
    strict: bool,
}

impl Document {
    /// Opens a PDF from `source` (a `Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`, an
    /// `Arc<dyn ByteSource>` or any other [`ByteSource`]).
    ///
    /// The xref chain at `startxref` is used when it is valid — every in-file entry points at
    /// a matching `num gen obj` header and the trailer's `/Root` resolves to a dictionary.
    /// Otherwise the xref is rebuilt by scanning (falling back to a `/Type /Catalog` object),
    /// [`Document::was_repaired`] reports `true` and [`Warning::XrefRepaired`] is raised.
    ///
    /// ```
    /// use lamina_cos::{Document, OpenOptions};
    ///
    /// let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n";
    /// let doc = Document::open(bytes.to_vec(), OpenOptions::default())?;
    /// assert!(doc.was_repaired()); // no xref table: rebuilt by scanning
    /// assert_eq!(doc.warnings().iter().map(|w| w.code()).collect::<Vec<_>>(), ["xref-repaired"]);
    ///
    /// let strict = Document::open(bytes.to_vec(), OpenOptions::default().with_strict(true));
    /// assert_eq!(strict.map_err(|e| e.code()).err(), Some("strict"));
    /// # Ok::<(), lamina_cos::Error>(())
    /// ```
    ///
    /// # Errors
    /// [`Error::NotPdf`] for empty input or input without any `n g obj`;
    /// [`Error::MissingRoot`] when no catalog can be found; [`Error::Strict`] in strict mode
    /// when the file needs a repair; [`Error::DataPending`] / [`Error::Source`] when the
    /// source cannot deliver its bytes.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "ADR 0005's signature: the options are moved in so decryption (B2b) can keep the password"
    )]
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        let source = source.into();
        let data = source.bytes()?;
        if data.is_empty() {
            return Err(Error::NotPdf("empty input"));
        }
        let limits = *options.limits();
        let strict = options.strict();
        let mut log = WarningLog::default();
        let head = data.get(..data.len().min(1024)).unwrap_or_default();
        // Like Acrobat, accept the header anywhere in the first 1 KiB, and still try without it.
        let header_version = find(head, b"%PDF-", 0).and_then(|pos| read_header_version(data, pos));
        if header_version.is_none() {
            log.raise(Warning::MissingHeader, &limits, strict)?;
        }
        let base = || Base {
            source: source.clone(),
            header_version: header_version.clone(),
            limits,
            strict,
        };

        let mut budget = DecodeBudget::new(&limits);
        let startxref = xref::find_startxref(data).ok();
        let loaded = startxref
            .and_then(|start| xref::load(data, start, &limits, &mut budget).ok())
            .filter(|table| table.trailer.contains_key("Root") && entries_are_valid(data, table, &limits));
        if let Some(table) = loaded {
            let doc = Self::new(base(), table, startxref, log.clone())?;
            // Entries that look valid can still yield an unusable catalog (damaged object
            // stream, syntax error inside the catalog, …): treat that like a broken xref.
            let checked = {
                let _call = doc.call();
                doc.budget.set(budget);
                let checked = doc.catalog();
                budget = doc.budget.get();
                checked
            };
            match checked {
                Ok(_) => return Ok(doc),
                // A cycle is a broken catalog like any other: repair may still find one.
                Err(e @ (Error::Strict { .. } | Error::DataPending { .. })) => return Err(e),
                Err(_) => {}
            }
        }
        log.raise(Warning::XrefRepaired, &limits, strict)?;
        let table = repair::rebuild(data, &limits, &mut budget)?;
        let mut doc = Self::new(base(), table, None, log)?;
        doc.repaired = true;
        Ok(doc)
    }

    /// A document over `base` using `table` (not marked repaired). Raises the warnings the
    /// xref loader recorded.
    fn new(base: Base, table: XrefTable, startxref: Option<usize>, mut log: WarningLog) -> Result<Self> {
        let XrefTable {
            entries,
            trailer,
            sections,
            has_xref_stream,
            newest_is_stream,
            ignored_xref_streams,
            dropped_objects,
        } = table;
        let Base {
            source,
            header_version,
            limits,
            strict,
        } = base;
        for offset in ignored_xref_streams {
            log.raise(Warning::XrefStmIgnored { offset }, &limits, strict)?;
        }
        let max_object_number = entries.keys().chain(&dropped_objects).max().copied();
        for object in dropped_objects {
            log.raise(Warning::ObjectNumberOutOfRange { object }, &limits, strict)?;
        }
        Ok(Self {
            source,
            entries,
            max_object_number,
            trailer,
            revisions: sections,
            repaired: false,
            has_xref_stream,
            newest_is_stream,
            startxref,
            header_version,
            limits,
            strict,
            warnings: RefCell::new(log),
            call_depth: Cell::new(0),
            budget: Cell::new(DecodeBudget::new(&limits)),
            objstm_cache: RefCell::new(BTreeMap::new()),
            object_cache: RefCell::new(BTreeMap::new()),
        })
    }

    /// Enters a public call; the outermost call starts a fresh decode budget.
    fn call(&self) -> Call<'_> {
        let depth = self.call_depth.get();
        if depth == 0 {
            self.budget.set(DecodeBudget::new(&self.limits));
        }
        self.call_depth.set(depth + 1);
        Call { doc: self }
    }

    /// Records `warning`, or fails with [`Error::Strict`] in strict mode.
    fn warn(&self, warning: Warning) -> Result<()> {
        self.warnings.borrow_mut().raise(warning, &self.limits, self.strict)
    }

    /// The newest trailer dictionary (for xref-stream files, the stream dictionary).
    #[must_use]
    pub const fn trailer(&self) -> &Dict {
        &self.trailer
    }

    /// Number of xref sections reached via `/Prev` (1 for a file never updated).
    /// For repaired files: the number of trailers found by scanning (at least 1).
    #[must_use]
    pub const fn revisions(&self) -> usize {
        self.revisions
    }

    /// True when the trailer has `/Encrypt`. Nothing is decrypted yet (decryption lands in Round B2).
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        self.trailer.contains_key("Encrypt")
    }

    /// True when the xref was rebuilt by scanning because the stored one was unusable.
    #[must_use]
    pub const fn was_repaired(&self) -> bool {
        self.repaired
    }

    /// True when any xref section read was an xref stream.
    #[must_use]
    pub const fn has_xref_stream(&self) -> bool {
        self.has_xref_stream
    }

    /// The warnings raised so far: at open, and lazily as objects are resolved and streams
    /// decoded (so the list can grow after `open`). Deduplicated by code and object, and
    /// capped at [`Limits::max_warnings`].
    #[must_use]
    pub fn warnings(&self) -> Vec<Warning> {
        self.warnings.borrow().list().to_vec()
    }

    /// The limits this document was opened with.
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Whether the document was opened in strict mode.
    #[must_use]
    pub const fn is_strict(&self) -> bool {
        self.strict
    }

    /// The version in the `%PDF-x.y` header, e.g. `"1.7"` (`None` when the header is missing).
    #[must_use]
    pub fn header_version(&self) -> Option<&str> {
        self.header_version.as_deref()
    }

    /// Offset of the newest xref section (`None` for repaired files).
    #[must_use]
    pub const fn startxref(&self) -> Option<usize> {
        self.startxref
    }

    /// The input bytes.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.source.loaded()
    }

    /// The input source, shareable with other documents (one per thread).
    #[must_use]
    pub fn source(&self) -> &Arc<dyn ByteSource> {
        self.source.get()
    }

    /// Whether the newest xref section is an xref stream (the incremental writer appends the
    /// same kind).
    pub(crate) const fn newest_section_is_stream(&self) -> bool {
        self.newest_is_stream
    }

    /// The highest object number in any xref section, free entries and entries dropped for
    /// exceeding [`Limits::max_object_number`] included.
    pub(crate) const fn max_object_number(&self) -> Option<u32> {
        self.max_object_number
    }

    /// Numbers of all objects that are not free, ascending.
    pub fn object_numbers(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries
            .iter()
            .filter(|(_, e)| !matches!(e, XrefEntry::Free))
            .map(|(&n, _)| n)
    }

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
            let obj = self.load(current.num, Some(current.gen_nr), &mut Vec::new())?;
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
    pub fn object(&self, num: u32) -> Result<Arc<Object>> {
        let _call = self.call();
        self.load(num, None, &mut Vec::new())
    }

    /// Returns `obj` itself, or the object it refers to.
    ///
    /// # Errors
    /// As [`Document::resolve`].
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
    fn load(&self, num: u32, gen_nr: Option<u16>, in_flight: &mut Vec<u32>) -> Result<Arc<Object>> {
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
            XrefEntry::InFile { offset, .. } => self.load_in_file(num, offset, in_flight),
            XrefEntry::InStream { stream, index } => self.load_compressed(num, stream, index, in_flight),
        };
        in_flight.pop();
        if let Ok(obj) = &result {
            self.object_cache.borrow_mut().insert(num, Arc::clone(obj));
        }
        result
    }

    fn load_in_file(&self, num: u32, offset: usize, in_flight: &mut Vec<u32>) -> Result<Arc<Object>> {
        // An unresolvable indirect `/Length` (missing, malformed, a cycle, over a hard limit)
        // is one of ADR 0003's tolerant paths: the extent is recovered by scanning, with
        // `stream-length-recovered`. Strict-mode refusals and missing input end the call.
        let mut fatal = None;
        let parsed = {
            let mut resolve_length = |r: ObjRef| match self.load(r.num, Some(r.gen_nr), in_flight) {
                Ok(obj) => obj.as_int(),
                Err(e) => {
                    if must_propagate(&e) {
                        fatal.get_or_insert(e);
                    }
                    None
                }
            };
            Parser::new(self.data(), offset)
                .with_limits(&self.limits)
                .indirect_object(&mut resolve_length)
        };
        if let Some(e) = fatal {
            return Err(e);
        }
        let parsed = parsed?;
        if parsed.length_recovered {
            self.warn(Warning::StreamLengthRecovered { object: num })?;
        }
        Ok(Arc::new(parsed.object))
    }

    fn load_compressed(&self, num: u32, stream: u32, index: u32, in_flight: &mut Vec<u32>) -> Result<Arc<Object>> {
        let members = self.object_stream(stream, in_flight)?;
        let by_index = usize::try_from(index)
            .ok()
            .and_then(|i| members.get(i))
            .filter(|(n, _)| *n == num);
        if let Some((_, member)) = by_index {
            return member.clone().map_err(Error::from);
        }
        // Fall back to a search when the index is stale or wrong.
        match members.iter().find(|(n, _)| *n == num) {
            Some((_, member)) => {
                self.warn(Warning::ObjStmIndexMismatch { object: num })?;
                member.clone().map_err(Error::from)
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
        let members: Vec<_> = objstm::parse(&s.dict, &decoded, &self.limits)?
            .into_iter()
            .map(|(n, member)| (n, member.map(Arc::new)))
            .collect();
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
    pub fn decode_stream(&self, stream: &Stream) -> Result<Vec<u8>> {
        let _call = self.call();
        self.decode_stream_of(stream, None)
    }

    /// [`Document::decode_stream`] for the stream object `object` (named in warnings).
    fn decode_stream_of(&self, stream: &Stream, object: Option<u32>) -> Result<Vec<u8>> {
        let mut params = Dict::new();
        for key in ["Filter", "DecodeParms"] {
            if let Some(v) = stream.dict.get(key) {
                params.insert(key, self.resolve_nested(v, MAX_FILTER_PARAM_DEPTH)?);
            }
        }
        let mut budget = self.budget.get();
        let decoded = filters::decode(&params, &stream.data, &self.limits, &mut budget);
        self.budget.set(budget);
        let decoded = decoded?;
        if decoded.flate_truncated {
            self.warn(Warning::FlateTruncated { object })?;
        }
        Ok(decoded.data)
    }

    /// `obj` with references resolved, including those inside arrays and dictionaries down to
    /// `depth` levels (missing objects become `null`, which the filters treat as absent).
    fn resolve_nested(&self, obj: &Object, depth: usize) -> Result<Object> {
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

    /// The document catalog (`/Root`).
    ///
    /// # Errors
    /// [`Error::Type`] when `/Root` is not a dictionary; errors from [`Document::resolve`].
    pub fn catalog(&self) -> Result<Dict> {
        let _call = self.call();
        let root = self.trailer.get("Root").ok_or(Error::MissingRoot)?;
        match &*self.deref(root)? {
            Object::Dict(d) => Ok(d.clone()),
            _ => Err(Error::Type {
                expected: "dictionary",
                context: "/Root",
            }),
        }
    }

    /// Walks the page tree in document order, applying inherited attributes.
    ///
    /// Tolerant: a node that is missing, unparsable, not a dictionary, deeper than
    /// [`Limits::max_page_tree_depth`] or otherwise over a limit (including a missing `/Pages`
    /// root) is skipped, as is a `/Kids` that is not an array — each with
    /// [`Warning::PageTreeNodeSkipped`]. `/Count` is not trusted: the walk is authoritative,
    /// and a root `/Count` that differs raises [`Warning::PageCountMismatch`].
    ///
    /// A page attribute (`/MediaBox`, `/CropBox`, `/Rotate`, `/Resources`) or `/Count` that is
    /// malformed reads as absent.
    ///
    /// # Errors
    /// [`Error::ReferenceCycle`] when a page-tree node is reachable twice (e.g. a
    /// self-referencing `/Kids`) or a reference chain loops; [`Error::Strict`] in strict mode
    /// for any skip or any warning raised while reading attributes; [`Error::LimitExceeded`]
    /// when reading a page attribute exceeds a hard limit; errors from [`Document::catalog`].
    pub fn pages(&self) -> Result<Vec<Page>> {
        let _call = self.call();
        let catalog = self.catalog()?;
        let root = catalog.get("Pages").cloned().unwrap_or(Object::Null);
        let mut visited = BTreeSet::new();
        let mut stack = vec![(root.clone(), Arc::new(Inherited::default()), 0usize)];
        let mut pages = Vec::new();
        while let Some((node_obj, inherited, depth)) = stack.pop() {
            let obj_ref = node_obj.as_ref();
            let skip = || {
                self.warn(Warning::PageTreeNodeSkipped {
                    object: obj_ref.map(|r| r.num),
                })
            };
            // ADR 0003 §3: `max_page_tree_depth` is a hard limit hit inside the walk, one of
            // the tolerant paths, so the subtree is skipped with `page-tree-node-skipped`
            // (strict mode: `Error::Strict`) instead of failing the call.
            if depth > self.limits.max_page_tree_depth() {
                skip()?;
                continue;
            }
            if let Some(r) = obj_ref
                && !visited.insert(r.num)
            {
                return Err(Error::ReferenceCycle(r.num));
            }
            let resolved = match self.deref(&node_obj) {
                Ok(obj) => obj,
                Err(e) if ends_page_walk(&e) => return Err(e),
                Err(_) => {
                    skip()?;
                    continue;
                }
            };
            let Object::Dict(node) = &*resolved else {
                skip()?;
                continue;
            };
            let attrs = Inherited::overlay(&inherited, node, |o| match o {
                Object::Null => Ok(true),
                // A malformed object counts as present (it was before): the page keeps the
                // attribute and reading it later yields `None`.
                Object::Ref(_) => Ok(absent_if_malformed(self.deref(o))?.is_some_and(|r| matches!(*r, Object::Null))),
                _ => Ok(false),
            })?;
            let is_tree_node = node.get_name("Type") == Some(b"Pages")
                || (node.get_name("Type") != Some(b"Page") && node.contains_key("Kids"));
            if is_tree_node {
                let kids = match self.deref(node.get("Kids").unwrap_or(&Object::Null)) {
                    Ok(obj) => obj,
                    Err(e) if ends_page_walk(&e) => return Err(e),
                    Err(_) => {
                        skip()?;
                        continue;
                    }
                };
                if let Object::Array(kids) = &*kids {
                    stack.extend(
                        kids.iter()
                            .rev()
                            .map(|kid| (kid.clone(), Arc::clone(&attrs), depth + 1)),
                    );
                } else {
                    skip()?;
                }
            } else {
                pages.push(self.make_page(obj_ref, node.clone(), &attrs)?);
            }
        }
        self.check_page_count(&root, pages.len())?;
        Ok(pages)
    }

    /// Raises [`Warning::PageCountMismatch`] when the root node's `/Count` differs from `walked`.
    fn check_page_count(&self, root: &Object, walked: usize) -> Result<()> {
        let declared = match absent_if_malformed(self.deref(root))? {
            Some(node) => match node.as_dict().and_then(|d| d.get("Count")) {
                Some(count) => absent_if_malformed(self.deref(count))?.and_then(|c| c.as_int()),
                None => None,
            },
            None => None,
        };
        let walked = u64::try_from(walked).unwrap_or(u64::MAX);
        match declared {
            Some(declared) if u64::try_from(declared).ok() != Some(walked) => self.warn(Warning::PageCountMismatch {
                object: root.as_ref().map(|r| r.num),
                declared,
                walked,
            }),
            _ => Ok(()),
        }
    }

    /// A page from its dictionary and inherited attributes. A malformed attribute is `None`
    /// (`/Rotate`: 0); cycles, hard limits and strict-mode refusals fail the call.
    fn make_page(&self, obj_ref: Option<ObjRef>, dict: Dict, attrs: &Inherited) -> Result<Page> {
        let rect = |o: &Option<Arc<Object>>| match o.as_deref() {
            Some(o) => as_rect(self, o),
            None => Ok(None),
        };
        let rotate = match attrs.rotate.as_deref() {
            Some(o) => absent_if_malformed(self.deref(o))?.and_then(|r| r.as_int()),
            None => None,
        };
        // A direct dictionary is already shared through `attrs`; a reference resolves to
        // the cached object. Either way no dictionary is copied per page.
        let resources = match attrs.resources.as_ref() {
            Some(o) => match **o {
                Object::Ref(r) => absent_if_malformed(self.resolve(r))?,
                _ => Some(Arc::clone(o)),
            },
            None => None,
        }
        .filter(|shared| matches!(**shared, Object::Dict(_)));
        Ok(Page {
            obj_ref,
            media_box: rect(&attrs.media_box)?,
            crop_box: rect(&attrs.crop_box)?,
            rotate: rotate.unwrap_or(0),
            resources,
            dict,
        })
    }

    /// Number of leaf pages in the page tree.
    ///
    /// # Errors
    /// As [`Document::pages`].
    pub fn page_count(&self) -> Result<usize> {
        self.pages().map(|p| p.len())
    }

    /// The decoded content of a page: all `/Contents` streams joined with `\n`. `/Contents`
    /// (or an array element) that is `null`, missing or free contributes nothing. All the
    /// page's streams share one [`Limits::max_decoded_total`] budget.
    ///
    /// # Errors
    /// Decode errors, [`Error::LimitExceeded`], or [`Error::Type`] if `/Contents` is not a
    /// stream or array of streams.
    pub fn page_content(&self, page: &Page) -> Result<Vec<u8>> {
        let _call = self.call();
        let Some(contents) = page.dict.get("Contents") else {
            return Ok(Vec::new());
        };
        let resolved = self.deref(contents)?;
        let parts = match &*resolved {
            Object::Array(items) => items.as_slice(),
            single => std::slice::from_ref(single),
        };
        let mut out = Vec::new();
        let mut first = true;
        for part in parts {
            // The object number names the stream in a `flate-truncated` warning.
            let object = match part {
                Object::Ref(r) => Some(r.num),
                _ => contents.as_ref().map(|r| r.num),
            };
            let part = self.deref(part)?;
            let s = match &*part {
                Object::Stream(s) => s,
                // A missing or free object is null (§7.3.10): it contributes no content.
                Object::Null => continue,
                _ => {
                    return Err(Error::Type {
                        expected: "stream",
                        context: "/Contents",
                    });
                }
            };
            if !first {
                out.push(b'\n');
            }
            first = false;
            out.extend_from_slice(&self.decode_stream_of(s, object)?);
        }
        Ok(out)
    }
}
