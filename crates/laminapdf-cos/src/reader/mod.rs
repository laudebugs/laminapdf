//! Tolerant reader: xref/xref-stream loading with repair, object resolution, page tree.

mod encrypt;
pub use encrypt::strip_crypt_filter;
mod objstm;
mod options;
mod repair;
mod warning;
mod window;
mod xref;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops;
use std::sync::Arc;

pub use encrypt::Encryption;
use encrypt::Security;
pub use options::OpenOptions;
pub use warning::Warning;
use warning::WarningLog;
pub use xref::XREF_STREAM_KEYS;
use xref::{XrefEntry, XrefTable};

use laminapdf_syntax::crypt::{PasswordKind, Permissions};
use laminapdf_syntax::filters;
use laminapdf_syntax::{DecodeBudget, Dict, Limit, Limits, ObjRef, Object, Parser, Stream, find};

use crate::error::{Error, Result, absent_if_malformed, tolerant};
use crate::overlay::{InOverlay, Overlay, OverlayEntry};
use crate::source::{ByteSource, Fetch, Pending, Source, WINDOW};
use window::{Reach, parse_at, read_indirect, within_read_window};

/// How deep [`Document::decode_stream`] resolves references nested in `/Filter` and
/// `/DecodeParms` (array → parameter dictionary → value). An internal constant: it bounds a
/// fixed structure, not input size (ADR 0003 §3).
const MAX_FILTER_PARAM_DEPTH: usize = 3;

/// Where the `%PDF-` header is looked for: the first 1 KiB, like Acrobat.
const HEADER_SEARCH: usize = 1024;

// The header check reads the search area, `%PDF-` and up to 8 version bytes: the first window
// holds all of them (or the whole input), so it never needs a second one.
const _: () = assert!(WINDOW >= HEADER_SEARCH + b"%PDF-".len() + 8);

/// Bytes at the end of the input read with the first window: `startxref`, its offset and
/// `%%EOF` are normally all there, even when they cross into the last window.
const TAIL: usize = 1024;

/// Bytes read for each xref entry's `n g obj` header check (on a source without an in-memory
/// slice); a header that runs past them is read again with a growing probe.
const HEADER_PROBE: usize = 32;

/// Header probes this close together are read as one range (fewer `read_at` calls and fewer,
/// larger `DataPending` ranges for the many small objects of a typical file).
const PROBE_GAP: usize = 256;

/// A page with its inheritable attributes (§7.7.3.4): `/Resources` resolved, the others as
/// stored. `laminapdf-model`'s page view reads the boxes and `/Rotate` from them.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Page {
    /// The page object's reference (`None` for a non-standard direct page dictionary).
    pub obj_ref: Option<ObjRef>,
    /// The page dictionary itself.
    pub dict: Dict,
    /// `/Resources`, own or inherited, resolved. Always an [`Object::Dict`]; shared (not
    /// copied) between pages that inherit the same resources. See [`Page::resources_dict`].
    pub resources: Option<Arc<Object>>,
    /// The inheritable attributes as stored (unresolved, own or inherited).
    pub inherited: Arc<Inherited>,
    /// Where a direct page dictionary sits (`None` when the page is an indirect object).
    pub direct_path: Option<DirectPath>,
}

/// The location of a direct (inline) object: its nearest indirect ancestor and the keys and
/// array indices that lead from there to it, e.g. `5 0 R` → `/Kids` → `3` (ADR 0005 §5).
/// The same bytes always give the same path.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct DirectPath {
    /// The nearest indirect ancestor (`None` when the chain starts at the trailer, i.e. the
    /// trailer's `/Root` is itself a direct dictionary).
    pub anchor: Option<ObjRef>,
    /// The steps from the anchor (from the trailer when `anchor` is `None`).
    pub segments: Vec<PathSegment>,
}

/// One step of a [`DirectPath`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathSegment {
    /// A dictionary key, e.g. `Kids`.
    Key(&'static str),
    /// An array index.
    Index(usize),
}

impl DirectPath {
    /// The path `segments` from `anchor` (`None`: from the trailer).
    #[must_use]
    pub const fn new(anchor: Option<ObjRef>, segments: Vec<PathSegment>) -> Self {
        Self { anchor, segments }
    }

    fn child(&self, step: PathSegment) -> Self {
        let mut segments = Vec::with_capacity(self.segments.len() + 1);
        segments.extend_from_slice(&self.segments);
        segments.push(step);
        Self {
            anchor: self.anchor,
            segments,
        }
    }
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
type ObjStmMembers = Arc<Vec<(u32, laminapdf_syntax::Result<Arc<Object>>)>>;

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
/// is_sync::<laminapdf_cos::Document>();
/// ```
///
/// **Limits and warnings (ADR 0003):** every call is bounded by the [`OpenOptions`]'
/// [`Limits`]; decoding shares one [`Limits::max_decoded_total`] budget per top-level call.
/// Repairs are reported as [`Warning`]s, raised at open or lazily as objects are resolved
/// ([`Document::warnings`]), or refused with [`Error::Strict`] in strict mode.
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent facts about the opened file, not states of one machine"
)]
pub struct Document {
    /// The input; a source without an in-memory slice is read in windows, cached and shared
    /// by every document opened from clones of the same [`Source`].
    source: Source,
    entries: BTreeMap<u32, XrefEntry>,
    /// The highest object number in any xref section, including entries dropped for
    /// exceeding [`Limits::max_object_number`] (the incremental writer never goes below it).
    max_object_number: Option<u32>,
    /// Object 0's next-free number in the newest section listing it (0 when none does).
    free_head: u32,
    trailer: Dict,
    revisions: usize,
    /// The offsets of the xref sections reached via `/Prev`, newest first (empty when repaired).
    section_offsets: Vec<usize>,
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
    /// The authenticated standard security handler of an encrypted document: objects are
    /// decrypted as they are loaded.
    security: Option<Security>,
    /// What this session changed (ADR 0005 §3); resolution reads it before the base.
    overlay: Overlay,
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
            .field("encryption", &self.encryption())
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
///
/// On a source without an in-memory slice the headers are read with small probes, not whole
/// windows, so checking a table does not read the whole file: probes within [`PROBE_GAP`] of
/// each other are read as one range, cached windows are used, and the ranges the source is
/// missing are all reported in one [`Error::DataPending`]. A header that can't be read within
/// [`Limits::max_read_window`] counts as wrong (repair).
///
/// # Errors
/// The source's errors.
fn entries_are_valid(source: &Source, table: &XrefTable, limits: &Limits) -> Result<bool> {
    let matches = |data: &[u8], pos: usize, num: u32, gen_nr: u16| {
        let mut parser = Parser::new(data, pos).with_limits(limits);
        let ok = parser
            .object_header()
            .is_ok_and(|id| id.num == num && id.gen_nr == gen_nr);
        (ok, parser.reached_end())
    };
    let mut probes: Vec<(usize, u32, u16)> = table
        .entries
        .iter()
        .filter_map(|(&num, entry)| match *entry {
            XrefEntry::InFile { offset, gen_nr } => Some((offset, num, gen_nr)),
            XrefEntry::Free | XrefEntry::InStream { .. } => None,
        })
        .collect();
    if let Some(data) = source.in_memory() {
        return Ok(probes
            .into_iter()
            .all(|(offset, num, gen_nr)| matches(data, offset, num, gen_nr).0));
    }
    probes.sort_unstable();
    let total = source.len()?;
    let mut spans: Vec<ops::Range<usize>> = Vec::new();
    for &(offset, ..) in &probes {
        let probe = offset.min(total)..offset.saturating_add(HEADER_PROBE).min(total);
        match spans.last_mut() {
            Some(span) if probe.start <= span.end.saturating_add(PROBE_GAP) => span.end = span.end.max(probe.end),
            _ => spans.push(probe),
        }
    }
    let mut pending = Pending::default();
    let mut windows = Vec::new();
    for span in spans {
        match source.read(span.start, span.len(), Fetch::Exact) {
            Ok(window) => windows.push(window),
            Err(e) => {
                pending.absorb(e)?;
                if pending.is_full() {
                    break;
                }
            }
        }
    }
    // Check what arrived first: one bad entry means repair, whatever the missing ones say.
    let mut next = 0;
    for (offset, num, gen_nr) in probes {
        while windows
            .get(next)
            .is_some_and(|w| w.end() <= offset.min(total) && w.start() < w.end())
        {
            next += 1;
        }
        let Some(window) = windows.get(next).filter(|w| w.start() <= offset.min(total)) else {
            continue;
        };
        let (mut ok, reached_end) = matches(window.bytes(), offset - window.start(), num, gen_nr);
        if reached_end && !window.at_end() {
            let probe = parse_at(source, offset, Reach::Probe(2 * HEADER_PROBE), limits, |v| {
                matches(v.data, v.pos, num, gen_nr)
            });
            match within_read_window(probe) {
                Ok(again) => ok = again.unwrap_or(false),
                Err(e) => {
                    pending.absorb(e)?;
                    continue;
                }
            }
        }
        if !ok {
            return Ok(false);
        }
    }
    pending.finish()?;
    Ok(true)
}

/// Inheritable page attributes (§7.7.3.4) as stored, own or inherited and unresolved (a
/// value may be a reference). Carried down the page tree and shared via [`Arc`], so passing
/// them to every kid (and every page) never copies a dictionary.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Inherited {
    /// `/MediaBox`.
    pub media_box: Option<Arc<Object>>,
    /// `/CropBox`.
    pub crop_box: Option<Arc<Object>>,
    /// `/Rotate`.
    pub rotate: Option<Arc<Object>>,
    /// `/Resources`.
    pub resources: Option<Arc<Object>>,
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
    /// use laminapdf_cos::{Document, OpenOptions};
    ///
    /// let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n";
    /// let doc = Document::open(bytes.to_vec(), OpenOptions::default())?;
    /// assert!(doc.was_repaired()); // no xref table: rebuilt by scanning
    /// assert_eq!(doc.warnings().iter().map(|w| w.code()).collect::<Vec<_>>(), ["xref-repaired"]);
    ///
    /// let strict = Document::open(bytes.to_vec(), OpenOptions::default().with_strict(true));
    /// assert_eq!(strict.map_err(|e| e.code()).err(), Some("strict"));
    /// # Ok::<(), laminapdf_cos::Error>(())
    /// ```
    ///
    /// An encrypted file (standard security handler, R2–R6) is authenticated here with
    /// [`OpenOptions::with_password`], or with the empty user password when none is given, and
    /// its strings and streams are then decrypted as objects are loaded (see
    /// [`Document::encryption`]).
    ///
    /// # Errors
    /// [`Error::NotPdf`] for empty input or input without any `n g obj`;
    /// [`Error::MissingRoot`] when no catalog can be found; [`Error::Strict`] in strict mode
    /// when the file needs a repair; [`Error::DataPending`] / [`Error::Source`] when the
    /// source cannot deliver its bytes. For encrypted files: [`Error::PasswordRequired`],
    /// [`Error::WrongPassword`], [`Error::UnsupportedSecurityHandler`], or
    /// [`Error::InvalidStructure`] for a malformed `/Encrypt` dictionary.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "ADR 0005's signature: the options are moved in (they may carry a password)"
    )]
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        let source = source.into();
        let len = source.len()?;
        if len == 0 {
            return Err(Error::NotPdf("empty input"));
        }
        // The header and `startxref` come first: a source still missing them hears about
        // the first window and the last kilobyte (one or two windows) at once.
        source.prefetch(&[(0, WINDOW), (len.saturating_sub(TAIL), TAIL)])?;
        let limits = *options.limits();
        let strict = options.strict();
        let mut log = WarningLog::default();
        // Like Acrobat, accept the header anywhere in the first 1 KiB, and still try without it.
        let header_version = parse_at(&source, 0, Reach::Object, &limits, |v| {
            let data = v.data;
            let head = data.get(..data.len().min(HEADER_SEARCH)).unwrap_or_default();
            let version = find(head, b"%PDF-", 0).and_then(|pos| read_header_version(data, pos));
            (version, false)
        })?;
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
        let startxref = tolerant(xref::find_startxref(&source, &limits))?;
        let loaded = match startxref {
            Some(start) => tolerant(xref::load(&source, start, &limits, &mut budget))?,
            None => None,
        };
        let loaded = match loaded {
            Some(table) if table.trailer.contains_key("Root") && entries_are_valid(&source, &table, &limits)? => {
                Some(table)
            }
            _ => None,
        };
        if let Some(table) = loaded {
            let mut doc = Self::new(base(), table, startxref, log.clone())?;
            // A password problem or an unsupported handler is never repaired around; an
            // `/Encrypt` that is merely damaged is, like a broken catalog.
            let unlocked = match doc.read_security(options.password()) {
                Ok(security) => security.map_or(Ok(()), |s| doc.install_security(s)),
                Err(e) => Err(e),
            };
            // Entries that look valid can still yield an unusable catalog (damaged object
            // stream, syntax error inside the catalog, …): treat that like a broken xref.
            let checked = unlocked.and_then(|()| {
                let _call = doc.call();
                doc.budget.set(budget);
                let checked = doc.catalog();
                budget = doc.budget.get();
                checked
            });
            match checked {
                Ok(_) => return Ok(doc),
                // A cycle is a broken catalog like any other: repair may still find one.
                // Strict refusals, missing data and source failures are never repaired around.
                Err(e) if e.must_propagate() || e.is_security_refusal() => return Err(e),
                Err(_) => {}
            }
        }
        log.raise(Warning::XrefRepaired, &limits, strict)?;
        // Repair in two phases around authentication: in-file objects and trailers first, then
        // `/Encrypt` (never in an object stream), then the object streams, decrypted. The file
        // is scanned once and the password checked once.
        let scan = repair::scan(&source, &limits)?;
        let security = {
            let in_file = Self::new(base(), scan.in_file_table(), None, WarningLog::default())?;
            in_file.read_security(options.password())?
        };
        let (table, malformed) = scan.finish(&source, &limits, &mut budget, security.as_ref())?;
        let mut doc = Self::new(base(), table, None, log)?;
        if let Some(security) = security {
            doc.install_security(security)?;
        }
        for object in malformed {
            doc.warn(Warning::AesPaddingInvalid { object })?;
        }
        doc.repaired = true;
        Ok(doc)
    }

    /// A document with no base bytes: no objects, an empty trailer and the header version
    /// `version` (for example `"1.7"`; the writers fall back to 1.7 for `None` or anything
    /// that is not `major.minor`). The builder (`laminapdf-build`) creates every object in the
    /// overlay; saving it is always a full rewrite (`laminapdf-save`'s `NoBase`). Reads behave as
    /// for a file whose objects are all free until the overlay sets a `/Root`.
    #[must_use]
    pub fn empty(version: Option<&str>, options: &OpenOptions) -> Self {
        let base = Base {
            source: Source::from(Vec::new()),
            header_version: version.map(str::to_owned),
            limits: *options.limits(),
            strict: options.strict(),
        };
        Self::from_parts(base, XrefTable::default(), None, WarningLog::default(), 0, None)
    }

    /// A document over `base` using `table` (not marked repaired). Raises the warnings the
    /// xref loader recorded.
    fn new(base: Base, mut table: XrefTable, startxref: Option<usize>, mut log: WarningLog) -> Result<Self> {
        let (limits, strict) = (base.limits, base.strict);
        for offset in std::mem::take(&mut table.ignored_xref_streams) {
            log.raise(Warning::XrefStmIgnored { offset }, &limits, strict)?;
        }
        let dropped = std::mem::take(&mut table.dropped_objects);
        let max_object_number = table.entries.keys().chain(&dropped).max().copied();
        for object in dropped {
            log.raise(Warning::ObjectNumberOutOfRange { object }, &limits, strict)?;
        }
        let free_head = table.free_head.unwrap_or(0);
        Ok(Self::from_parts(
            base,
            table,
            startxref,
            log,
            free_head,
            max_object_number,
        ))
    }

    /// The document's fields (no checks, no warnings raised).
    fn from_parts(
        base: Base,
        table: XrefTable,
        startxref: Option<usize>,
        log: WarningLog,
        free_head: u32,
        max_object_number: Option<u32>,
    ) -> Self {
        let Base {
            source,
            header_version,
            limits,
            strict,
        } = base;
        Self {
            source,
            entries: table.entries,
            max_object_number,
            free_head,
            trailer: table.trailer,
            revisions: table.sections,
            section_offsets: table.section_offsets,
            repaired: false,
            has_xref_stream: table.has_xref_stream,
            newest_is_stream: table.newest_is_stream,
            startxref,
            header_version,
            limits,
            strict,
            warnings: RefCell::new(log),
            call_depth: Cell::new(0),
            budget: Cell::new(DecodeBudget::new(&limits)),
            objstm_cache: RefCell::new(BTreeMap::new()),
            object_cache: RefCell::new(BTreeMap::new()),
            overlay: Overlay::default(),
            security: None,
        }
    }

    /// Reads `/Encrypt` and authenticates `password` (`None` for an unencrypted document: no
    /// `/Encrypt`, or one that is `null` or resolves to `null`, which counts as absent, §7.3.10).
    /// Objects resolved on the way (an indirect `/Encrypt`, its `/CF`, the `/ID`) are read
    /// without decryption, which is what the spec asks for them.
    fn read_security(&self, password: Option<&[u8]>) -> Result<Option<Security>> {
        let Some(encrypt) = self.trailer.get("Encrypt") else {
            return Ok(None);
        };
        let _call = self.call();
        let dict = match self.resolve_nested(encrypt, 4)? {
            Object::Dict(dict) => dict,
            Object::Null => return Ok(None),
            _ => return Err(encrypt::invalid("/Encrypt is not a dictionary")),
        };
        let id = match self.trailer.get("ID") {
            Some(id) => self.resolve_nested(id, 2)?,
            None => Object::Null,
        };
        let id0 = match id.as_array().and_then(<[Object]>::first) {
            Some(Object::String(s)) => s.bytes.as_slice(),
            _ => &[],
        };
        Security::unlock(&dict, encrypt.as_ref().map(|r| r.num), id0, password).map(Some)
    }

    /// Makes `security` decrypt every object loaded from now on. Anything cached while
    /// reading `/Encrypt` is dropped, so it is loaded again (and decrypted, unless exempt).
    fn install_security(&mut self, security: Security) -> Result<()> {
        let perms_valid = security.perms_valid();
        self.security = Some(security);
        self.object_cache.get_mut().clear();
        self.objstm_cache.get_mut().clear();
        if perms_valid {
            Ok(())
        } else {
            self.warn(Warning::PermsMismatch)
        }
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

    /// Records a warning raised by a layer above this one (the typed views of
    /// `laminapdf-model`), so every warning about a document lives in one list
    /// ([`Document::warnings`]), deduplicated and capped like the reader's own.
    ///
    /// # Errors
    /// [`Error::Strict`] in strict mode.
    pub fn report(&self, warning: Warning) -> Result<()> {
        self.warn(warning)
    }

    /// The trailer as it is now: the overlay's when this session changed it
    /// ([`CosMut::set_trailer`](crate::CosMut::set_trailer)), else the newest trailer
    /// dictionary of the file (for xref-stream files, the stream dictionary).
    #[must_use]
    pub fn trailer(&self) -> &Dict {
        self.overlay.trailer().unwrap_or(&self.trailer)
    }

    /// The newest trailer dictionary of the file, ignoring the overlay.
    #[must_use]
    pub const fn base_trailer(&self) -> &Dict {
        &self.trailer
    }

    /// Number of xref sections reached via `/Prev` (1 for a file never updated).
    /// For repaired files: the number of trailers found by scanning (at least 1).
    #[must_use]
    pub const fn revisions(&self) -> usize {
        self.revisions
    }

    /// The offsets of the xref sections reached via `/Prev` from the newest one, newest first
    /// (hybrid `/XRefStm` sections not included); empty for a repaired file. A signature's
    /// `/ByteRange` end, compared with them, tells which revision it closes.
    #[must_use]
    pub fn xref_section_offsets(&self) -> &[usize] {
        &self.section_offsets
    }

    /// True when the document is encrypted: its trailer has an `/Encrypt` dictionary (one that
    /// is `null` counts as absent). [`Document::open`] only returns such a document once a
    /// password (possibly the empty user password) authenticated it; it is decrypted as it is
    /// read, and [`Document::matched_password`] says which password worked.
    #[must_use]
    pub const fn is_encrypted(&self) -> bool {
        self.security.is_some()
    }

    /// The encryption of an encrypted document: `/V`, `/R`, ciphers, key length and
    /// `/EncryptMetadata`. `None` for an unencrypted one.
    ///
    /// ```
    /// use laminapdf_cos::{Document, OpenOptions};
    ///
    /// let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R >>\n";
    /// let doc = Document::open(bytes.to_vec(), OpenOptions::default())?;
    /// assert_eq!((doc.encryption(), doc.permissions(), doc.matched_password()), (None, None, None));
    /// # Ok::<(), laminapdf_cos::Error>(())
    /// ```
    #[must_use]
    pub fn encryption(&self) -> Option<&Encryption> {
        self.security.as_ref().map(Security::info)
    }

    /// The user access permissions (`/P`) of an encrypted document; `None` for an unencrypted
    /// one. They are what the file asks for, not access control (research ch. 04 §7.5):
    /// whether to honour them, and whether opening with the owner password lifts them, is the
    /// caller's decision.
    #[must_use]
    pub fn permissions(&self) -> Option<Permissions> {
        self.security.as_ref().map(Security::permissions)
    }

    /// Which password opened an encrypted document (the empty user password counts as
    /// [`PasswordKind::User`]); `None` for an unencrypted one.
    #[must_use]
    pub fn matched_password(&self) -> Option<PasswordKind> {
        self.security.as_ref().map(Security::matched)
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

    /// The input bytes, when the source holds them in memory ([`ByteSource::as_slice`]);
    /// `None` for a source read in windows, which is never read whole.
    #[must_use]
    pub fn data(&self) -> Option<&[u8]> {
        self.source.in_memory()
    }

    /// The whole input, borrowed when it is in memory, otherwise read window by window
    /// (without filling the window cache). Only for callers whose output holds the whole
    /// input anyway (`laminapdf-save`'s `incremental_update`).
    ///
    /// # Errors
    /// [`Error::DataPending`] with every missing window; [`Error::Source`].
    pub fn whole_input(&self) -> Result<Cow<'_, [u8]>> {
        self.source.whole()
    }

    /// The input source, shareable with other documents (one per thread).
    #[must_use]
    pub fn source(&self) -> &Arc<dyn ByteSource> {
        self.source.get()
    }

    /// Whether the newest xref section is an xref stream (the incremental writer appends the
    /// same kind).
    #[must_use]
    pub const fn newest_section_is_stream(&self) -> bool {
        self.newest_is_stream
    }

    /// The highest object number in any xref section, free entries and entries dropped for
    /// exceeding [`Limits::max_object_number`] included. This describes the base (the
    /// file's xref), not objects created in this session ([`Document::object_numbers`] lists
    /// those too).
    pub const fn max_object_number(&self) -> Option<u32> {
        self.max_object_number
    }

    /// Numbers of all objects that exist now, ascending: the base's in-use objects not
    /// deleted in this session, then the ones created in it (always numbered above the base's).
    pub fn object_numbers(&self) -> impl Iterator<Item = u32> + '_ {
        let base = self
            .entries
            .iter()
            .filter(|(n, e)| {
                !matches!(e, XrefEntry::Free) && !matches!(self.overlay.get(**n), Some(OverlayEntry::Deleted))
            })
            .map(|(&n, _)| n);
        let new = self
            .overlay
            .iter()
            .filter(|(_, e)| matches!(e, OverlayEntry::New(_)))
            .map(|(n, _)| n);
        base.chain(new)
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
    pub fn object(&self, num: u32) -> Result<Arc<Object>> {
        let _call = self.call();
        self.load_current(num, None)
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
        if self.security.is_some() {
            // Streams of an encrypted document were decrypted when they were loaded, which
            // applied the `/Crypt` filter (§7.4.10); the rest of the chain decodes as usual.
            strip_crypt_filter(&mut params);
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
        let root = self.trailer().get("Root").ok_or(Error::MissingRoot)?;
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
    /// Page attributes are inherited as stored (an own reference is resolved only to tell an
    /// explicit `null` from a value); `laminapdf-model` reads the boxes and `/Rotate`. A
    /// malformed `/Resources` or `/Count` reads as absent.
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
        // A direct `/Pages` sits in the catalog: its path starts there (ADR 0005 §5).
        let root_path = (!matches!(root, Object::Ref(_))).then(|| {
            let catalog_ref = self.trailer().get("Root").and_then(Object::as_ref);
            let segments = match catalog_ref {
                Some(_) => vec![PathSegment::Key("Pages")],
                None => vec![PathSegment::Key("Root"), PathSegment::Key("Pages")],
            };
            DirectPath::new(catalog_ref, segments)
        });
        let mut visited = BTreeSet::new();
        let mut stack = vec![(root.clone(), Arc::new(Inherited::default()), 0usize, root_path)];
        let mut pages = Vec::new();
        while let Some((node_obj, inherited, depth, path)) = stack.pop() {
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
                Err(e) if e.ends_page_walk() => return Err(e),
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
                let kids_obj = node.get("Kids").unwrap_or(&Object::Null);
                let kids = match self.deref(kids_obj) {
                    Ok(obj) => obj,
                    Err(e) if e.ends_page_walk() => return Err(e),
                    Err(_) => {
                        skip()?;
                        continue;
                    }
                };
                if let Object::Array(kids) = &*kids {
                    // A direct kid is named from its nearest indirect ancestor: the `/Kids`
                    // array when that is an indirect object, else the node, else the node's
                    // own path. Indirect kids need no path.
                    let base = match (kids_obj.as_ref(), obj_ref) {
                        (Some(array), _) => DirectPath::new(Some(array), Vec::new()),
                        (None, Some(r)) => DirectPath::new(Some(r), vec![PathSegment::Key("Kids")]),
                        (None, None) => {
                            let mut base = path.unwrap_or_else(|| DirectPath::new(None, Vec::new()));
                            base.segments.push(PathSegment::Key("Kids"));
                            base
                        }
                    };
                    stack.extend(kids.iter().enumerate().rev().map(|(i, kid)| {
                        let kid_path = (!matches!(kid, Object::Ref(_))).then(|| base.child(PathSegment::Index(i)));
                        (kid.clone(), Arc::clone(&attrs), depth + 1, kid_path)
                    }));
                } else {
                    skip()?;
                }
            } else {
                let direct_path = if obj_ref.is_none() { path } else { None };
                pages.push(self.make_page(obj_ref, node.clone(), attrs, direct_path)?);
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

    /// A page from its dictionary and inherited attributes. Malformed `/Resources` is `None`;
    /// cycles, hard limits and strict-mode refusals fail the call.
    fn make_page(
        &self,
        obj_ref: Option<ObjRef>,
        dict: Dict,
        attrs: Arc<Inherited>,
        direct_path: Option<DirectPath>,
    ) -> Result<Page> {
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
            resources,
            dict,
            inherited: attrs,
            direct_path,
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

/// The overlay's view of the base (ADR 0005 §3), used by the overlay module and the writers.
impl Document {
    pub(crate) const fn overlay_ref(&self) -> &Overlay {
        &self.overlay
    }

    pub(crate) const fn overlay_mut(&mut self) -> &mut Overlay {
        &mut self.overlay
    }

    /// Object `num` as it is now: the overlay's entry, else the base's (`null` when deleted
    /// or, with `gen_nr`, when the generation does not match, which raises
    /// `stale-generation` as it does for base objects).
    fn load_current(&self, num: u32, gen_nr: Option<u16>) -> Result<Arc<Object>> {
        match self.overlay_value(num, gen_nr) {
            InOverlay::Value(o) => Ok(Arc::clone(o)),
            InOverlay::Absent => {
                if let Some(generation) = gen_nr
                    && !self.overlay_generation_matches(num, generation)
                {
                    self.warn(Warning::StaleGeneration {
                        object: num,
                        generation,
                    })?;
                }
                Ok(Arc::new(Object::Null))
            }
            InOverlay::NoEntry => self.load(num, gen_nr, &mut Vec::new()),
        }
    }

    /// The generation of base object `num` when the base's xref has it in use, ignoring the
    /// overlay (objects in object streams have generation 0). The writers use it for
    /// modified objects (same generation) and deleted ones (a free entry with the next
    /// generation, §7.5.4).
    #[must_use]
    pub fn base_generation(&self, num: u32) -> Option<u16> {
        self.entries
            .get(&num)
            .filter(|e| !matches!(e, XrefEntry::Free))
            .map(|e| e.gen_nr())
    }

    /// The head of the base's free list (§7.5.4): the next-free number of object 0 in the
    /// newest xref section that lists it, 0 when none does (or the xref was repaired). An
    /// incremental update links its own free entries in front of it.
    #[must_use]
    pub const fn free_list_head(&self) -> u32 {
        self.free_head
    }

    /// Puts `older`'s warnings before this document's own (a rebase keeps the diagnostics
    /// raised so far).
    pub(crate) fn carry_warnings(&self, older: &Self) {
        let mut merged = WarningLog::default();
        let old = older.warnings.borrow();
        let own = self.warnings.borrow();
        for warning in old.list().iter().chain(own.list()) {
            // Not strict: these were raised (and accepted) already.
            let _accepted = merged.raise(*warning, &self.limits, false);
        }
        drop((old, own));
        *self.warnings.borrow_mut() = merged;
    }

    /// Whether the base has an in-use object `r` (number and generation).
    pub(crate) fn base_has(&self, r: ObjRef) -> bool {
        self.entries
            .get(&r.num)
            .is_some_and(|e| !matches!(e, XrefEntry::Free) && e.matches_gen(r.gen_nr))
    }

    /// Base object `num`, ignoring the overlay. One that cannot be loaded is `null` to the
    /// overlay; strict-mode refusals and unavailable input are errors.
    pub(crate) fn base_value(&self, num: u32) -> Result<Arc<Object>> {
        let _call = self.call();
        match self.load(num, None, &mut Vec::new()) {
            Ok(base) => Ok(base),
            Err(e) if e.must_propagate() => Err(e),
            Err(_) => Ok(Arc::new(Object::Null)),
        }
    }
}
