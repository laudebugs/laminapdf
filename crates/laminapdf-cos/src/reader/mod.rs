//! Tolerant reader: opening a file (the header, the xref table or stream, repair and the
//! security handler) and the [`Document`] it returns. Resolving and loading objects is in
//! `load`; the page tree is in `pages`.

mod encrypt;
pub use encrypt::strip_crypt_filter;
mod load;
mod objstm;
mod options;
mod pages;
mod repair;
mod revisions;

pub use revisions::{Revision, revision_starts};
mod warning;
mod window;
mod xref;

use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt;
use std::ops;
use std::sync::Arc;

pub use encrypt::Encryption;
use encrypt::Security;
use load::ObjStmMembers;
pub use load::Resolved;
pub use options::OpenOptions;
pub use pages::{DirectPath, Inherited, Page, PathSegment};
pub use warning::Warning;
use warning::WarningLog;
pub use xref::XREF_STREAM_KEYS;
use xref::{XrefEntry, XrefTable};

use laminapdf_syntax::crypt::{PasswordKind, Permissions};
use laminapdf_syntax::{DecodeBudget, Dict, Limits, ObjRef, Object, Parser, find};

use crate::error::{Error, Result, tolerant};
use crate::overlay::{InOverlay, Overlay, OverlayEntry};
use crate::source::{ByteSource, Fetch, Pending, Source, WINDOW};
use window::{Reach, parse_at, within_read_window};

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
///
/// Added in 0.2.0.
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
    /// What each of those sections lists, in the same order.
    section_objects: Vec<xref::SectionObjects>,
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
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
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
            section_objects: table.section_objects,
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
    ///
    /// Added in 0.2.0.
    pub fn report(&self, warning: Warning) -> Result<()> {
        self.warn(warning)
    }

    /// The trailer as it is now: the overlay's when this session changed it
    /// ([`CosMut::set_trailer`](crate::CosMut::set_trailer)), else the newest trailer
    /// dictionary of the file (for xref-stream files, the stream dictionary).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn trailer(&self) -> &Dict {
        self.overlay.trailer().unwrap_or(&self.trailer)
    }

    /// The newest trailer dictionary of the file, ignoring the overlay.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn base_trailer(&self) -> &Dict {
        &self.trailer
    }

    /// Number of xref sections reached via `/Prev` (1 for a file never updated).
    /// For repaired files: the number of trailers found by scanning (at least 1).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn revisions(&self) -> usize {
        self.revisions
    }

    /// The offsets of the xref sections reached via `/Prev` from the newest one, newest first
    /// (hybrid `/XRefStm` sections not included); empty for a repaired file. A signature's
    /// `/ByteRange` end, compared with them, tells which revision it closes.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn xref_section_offsets(&self) -> &[usize] {
        &self.section_offsets
    }

    /// True when the document is encrypted: its trailer has an `/Encrypt` dictionary (one that
    /// is `null` counts as absent). [`Document::open`] only returns such a document once a
    /// password (possibly the empty user password) authenticated it; it is decrypted as it is
    /// read, and [`Document::matched_password`] says which password worked.
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn encryption(&self) -> Option<&Encryption> {
        self.security.as_ref().map(Security::info)
    }

    /// The user access permissions (`/P`) of an encrypted document; `None` for an unencrypted
    /// one. They are what the file asks for, not access control (research ch. 04 §7.5):
    /// whether to honour them, and whether opening with the owner password lifts them, is the
    /// caller's decision.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn permissions(&self) -> Option<Permissions> {
        self.security.as_ref().map(Security::permissions)
    }

    /// Which password opened an encrypted document (the empty user password counts as
    /// [`PasswordKind::User`]); `None` for an unencrypted one.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn matched_password(&self) -> Option<PasswordKind> {
        self.security.as_ref().map(Security::matched)
    }

    /// True when the xref was rebuilt by scanning because the stored one was unusable.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn was_repaired(&self) -> bool {
        self.repaired
    }

    /// True when any xref section read was an xref stream.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn has_xref_stream(&self) -> bool {
        self.has_xref_stream
    }

    /// The warnings raised so far: at open, and lazily as objects are resolved and streams
    /// decoded (so the list can grow after `open`). Deduplicated by code and object, and
    /// capped at [`Limits::max_warnings`].
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn warnings(&self) -> Vec<Warning> {
        self.warnings.borrow().list().to_vec()
    }

    /// The limits this document was opened with.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn limits(&self) -> &Limits {
        &self.limits
    }

    /// Whether the document was opened in strict mode.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn is_strict(&self) -> bool {
        self.strict
    }

    /// The version in the `%PDF-x.y` header, e.g. `"1.7"` (`None` when the header is missing).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn header_version(&self) -> Option<&str> {
        self.header_version.as_deref()
    }

    /// Offset of the newest xref section (`None` for repaired files).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn startxref(&self) -> Option<usize> {
        self.startxref
    }

    /// The input bytes, when the source holds them in memory ([`ByteSource::as_slice`]);
    /// `None` for a source read in windows, which is never read whole.
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
    pub fn whole_input(&self) -> Result<Cow<'_, [u8]>> {
        self.source.whole()
    }

    /// The input source, shareable with other documents (one per thread).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn source(&self) -> &Arc<dyn ByteSource> {
        self.source.get()
    }

    /// Whether the newest xref section is an xref stream (the incremental writer appends the
    /// same kind).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn newest_section_is_stream(&self) -> bool {
        self.newest_is_stream
    }

    /// The highest object number in any xref section, free entries and entries dropped for
    /// exceeding [`Limits::max_object_number`] included. This describes the base (the
    /// file's xref), not objects created in this session ([`Document::object_numbers`] lists
    /// those too).
    ///
    /// Added in 0.2.0.
    pub const fn max_object_number(&self) -> Option<u32> {
        self.max_object_number
    }

    /// Numbers of all objects that exist now, ascending: the base's in-use objects not
    /// deleted in this session, then the ones created in it (always numbered above the base's).
    ///
    /// Added in 0.2.0.
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

    /// The document catalog (`/Root`).
    ///
    /// # Errors
    /// [`Error::Type`] when `/Root` is not a dictionary; errors from [`Document::resolve`].
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
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
    ///
    /// Added in 0.2.0.
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
