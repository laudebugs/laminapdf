//! Tolerant reader: xref/xref-stream loading with repair, object resolution, page tree.

mod objstm;
mod repair;
mod xref;

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::ops;
use std::rc::Rc;

use xref::{XrefEntry, XrefTable};

use lamina_syntax::filters::{self, DEFAULT_MAX_DECODED};
use lamina_syntax::{Dict, ObjRef, Object, Parser, Stream};

use crate::error::{Error, Result};
use crate::scan::find;

/// How many indirect objects may be "in flight" at once while resolving (e.g. an indirect
/// `/Length` inside an object stream). Deeper chains are treated as cycles.
const MAX_RESOLVE_DEPTH: usize = 32;

/// Limits applied while reading.
///
/// Build it from [`OpenOptions::default`] and the `with_*` methods; more limits will be added
/// (ADR 0003, Round B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpenOptions {
    /// Refuse to decode any stream larger than this many bytes (default 64 MiB).
    pub max_decoded_size: usize,
}

impl OpenOptions {
    /// Sets [`OpenOptions::max_decoded_size`].
    #[must_use]
    pub const fn with_max_decoded_size(mut self, bytes: usize) -> Self {
        self.max_decoded_size = bytes;
        self
    }
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self {
            max_decoded_size: DEFAULT_MAX_DECODED,
        }
    }
}

/// A recoverable problem noticed while opening a document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Warning {
    /// No valid `%PDF-x.y` header in the first 1 024 bytes; parsing went ahead anyway.
    MissingHeader,
    /// The stored cross-reference data was unusable and was rebuilt by scanning.
    XrefRepaired,
}

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
    pub resources: Option<Rc<Object>>,
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
    Indirect(Rc<Object>),
}

impl Resolved<'_> {
    /// An owned copy of the object (clones unless this is the only handle to it).
    #[must_use]
    pub fn into_owned(self) -> Object {
        match self {
            Self::Direct(obj) => obj.clone(),
            Self::Indirect(rc) => Rc::unwrap_or_clone(rc),
        }
    }

    /// The object behind a shared pointer (a cheap clone for indirect objects).
    #[must_use]
    pub fn into_rc(self) -> Rc<Object> {
        match self {
            Self::Direct(obj) => Rc::new(obj.clone()),
            Self::Indirect(rc) => rc,
        }
    }
}

impl ops::Deref for Resolved<'_> {
    type Target = Object;

    fn deref(&self) -> &Object {
        match self {
            Self::Direct(obj) => obj,
            Self::Indirect(rc) => rc,
        }
    }
}

type ObjStmMembers = Rc<Vec<(u32, Rc<Object>)>>;

/// An opened PDF. Borrows the input bytes; objects are parsed lazily on first
/// [`Document::resolve`] and then cached, so repeated lookups share one [`Rc<Object>`].
///
/// The caches use [`RefCell`] and [`Rc`], so a `Document` is deliberately `!Sync` and
/// `!Send`: use one per thread (opening is cheap, and the input bytes can be shared).
#[derive(Debug)]
pub struct Document<'a> {
    data: &'a [u8],
    entries: BTreeMap<u32, XrefEntry>,
    trailer: Dict,
    revisions: usize,
    repaired: bool,
    has_xref_stream: bool,
    startxref: Option<usize>,
    header_version: Option<String>,
    warnings: Vec<Warning>,
    options: OpenOptions,
    objstm_cache: RefCell<BTreeMap<u32, ObjStmMembers>>,
    /// Successfully loaded objects by number (errors are not cached).
    object_cache: RefCell<BTreeMap<u32, Rc<Object>>>,
}

/// Opens a PDF with default [`OpenOptions`].
///
/// # Errors
/// Fails on input that is not a PDF or cannot be repaired into one with a `/Root`.
pub fn open(data: &[u8]) -> Result<Document<'_>> {
    open_with(data, OpenOptions::default())
}

/// Opens a PDF with explicit limits.
///
/// The xref chain at `startxref` is used when it is valid — every in-file entry points at
/// a matching `num gen obj` header and the trailer's `/Root` resolves to a dictionary.
/// Otherwise the xref is rebuilt by scanning (falling back to a `/Type /Catalog` object),
/// and [`Document::was_repaired`] reports `true`.
///
/// # Errors
/// [`Error::NotPdf`] for empty input or input without any `n g obj`;
/// [`Error::MissingRoot`] when no catalog can be found.
pub fn open_with(data: &[u8], options: OpenOptions) -> Result<Document<'_>> {
    if data.is_empty() {
        return Err(Error::NotPdf("empty input"));
    }
    let mut warnings = Vec::new();
    let head = data.get(..data.len().min(1024)).unwrap_or_default();
    // Like Acrobat, accept the header anywhere in the first 1 KiB, and still try without it.
    let header_version = find(head, b"%PDF-", 0).and_then(|pos| read_header_version(data, pos));
    if header_version.is_none() {
        warnings.push(Warning::MissingHeader);
    }

    let startxref = xref::find_startxref(data).ok();
    let loaded = startxref
        .and_then(|start| xref::load(data, start, options.max_decoded_size).ok())
        .filter(|table| table.trailer.contains_key("Root") && entries_are_valid(data, table));
    if let Some(table) = loaded {
        let doc = Document::new(
            data,
            table,
            startxref,
            header_version.clone(),
            warnings.clone(),
            options,
        );
        // Entries that look valid can still yield an unusable catalog (damaged object
        // stream, syntax error inside the catalog, …): treat that like a broken xref.
        if doc.catalog().is_ok() {
            return Ok(doc);
        }
    }
    warnings.push(Warning::XrefRepaired);
    let table = repair::rebuild(data, options.max_decoded_size)?;
    let mut doc = Document::new(data, table, None, header_version, warnings, options);
    doc.repaired = true;
    Ok(doc)
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

/// True when every in-file xref entry points at its own `num gen obj` header.
fn entries_are_valid(data: &[u8], table: &XrefTable) -> bool {
    table.entries.iter().all(|(&num, entry)| match *entry {
        XrefEntry::InFile { offset, .. } => Parser::new(data, offset).object_header().is_ok_and(|id| id.num == num),
        XrefEntry::Free | XrefEntry::InStream { .. } => true,
    })
}

fn as_rect(doc: &Document<'_>, obj: &Object) -> Option<[f64; 4]> {
    let arr = doc.deref(obj).ok()?;
    let items = arr.as_array()?;
    let mut out = [0.0; 4];
    if items.len() != 4 {
        return None;
    }
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = doc.deref(item).ok()?.as_f64()?;
    }
    Some(out)
}

/// Inheritable page attributes carried down the page tree. Values are shared via [`Rc`],
/// so passing them to every kid (and every page) never copies a dictionary.
#[derive(Debug, Clone, Default)]
struct Inherited {
    media_box: Option<Rc<Object>>,
    crop_box: Option<Rc<Object>>,
    rotate: Option<Rc<Object>>,
    resources: Option<Rc<Object>>,
}

const INHERITABLE: [&str; 4] = ["MediaBox", "CropBox", "Rotate", "Resources"];

impl Inherited {
    /// `parent` with any attributes `dict` sets itself replacing the inherited ones.
    /// Returns `parent` unchanged (a pointer copy) when `dict` sets none of them.
    fn overlay(parent: &Rc<Self>, dict: &Dict) -> Rc<Self> {
        if !INHERITABLE.iter().any(|key| dict.contains_key(key)) {
            return Rc::clone(parent);
        }
        let pick = |key, inherited: &Option<Rc<Object>>| {
            dict.get(key).map(|o| Rc::new(o.clone())).or_else(|| inherited.clone())
        };
        Rc::new(Self {
            media_box: pick("MediaBox", &parent.media_box),
            crop_box: pick("CropBox", &parent.crop_box),
            rotate: pick("Rotate", &parent.rotate),
            resources: pick("Resources", &parent.resources),
        })
    }
}

impl<'a> Document<'a> {
    /// A document over `data` using `table` (not marked repaired).
    fn new(
        data: &'a [u8],
        table: XrefTable,
        startxref: Option<usize>,
        header_version: Option<String>,
        warnings: Vec<Warning>,
        options: OpenOptions,
    ) -> Self {
        let XrefTable {
            entries,
            trailer,
            sections,
            has_xref_stream,
        } = table;
        Self {
            data,
            entries,
            trailer,
            revisions: sections,
            repaired: false,
            has_xref_stream,
            startxref,
            header_version,
            warnings,
            options,
            objstm_cache: RefCell::new(BTreeMap::new()),
            object_cache: RefCell::new(BTreeMap::new()),
        }
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

    /// Recoverable problems noticed while opening.
    #[must_use]
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
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
    pub const fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Numbers of all objects that are not free, ascending.
    pub fn object_numbers(&self) -> impl Iterator<Item = u32> + '_ {
        self.entries
            .iter()
            .filter(|(_, e)| !matches!(e, XrefEntry::Free))
            .map(|(&n, _)| n)
    }

    /// Resolves an indirect reference, following reference-to-reference chains.
    /// Missing or free objects resolve to `null` (§7.3.10).
    ///
    /// Objects are parsed once and cached: resolving the same reference again returns a
    /// clone of the same [`Rc`].
    ///
    /// # Errors
    /// [`Error::ReferenceCycle`] for chains that loop; parse errors for corrupt objects.
    pub fn resolve(&self, r: ObjRef) -> Result<Rc<Object>> {
        let mut chain = Vec::new();
        let mut current = r;
        loop {
            if chain.contains(&current.num) || chain.len() >= MAX_RESOLVE_DEPTH {
                return Err(Error::ReferenceCycle(current.num));
            }
            chain.push(current.num);
            let obj = self.load(current.num, &mut Vec::new())?;
            match *obj {
                Object::Ref(next) => current = next,
                _ => return Ok(obj),
            }
        }
    }

    /// The value of indirect object `num` exactly as stored, without following a top-level
    /// reference (`1 0 obj 2 0 R endobj` yields `2 0 R`). Missing/free objects are `null`.
    ///
    /// # Errors
    /// Parse errors for corrupt objects; [`Error::ReferenceCycle`] for self-referential streams.
    pub fn object(&self, num: u32) -> Result<Rc<Object>> {
        self.load(num, &mut Vec::new())
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
    fn load(&self, num: u32, in_flight: &mut Vec<u32>) -> Result<Rc<Object>> {
        if let Some(cached) = self.object_cache.borrow().get(&num) {
            return Ok(Rc::clone(cached));
        }
        if in_flight.contains(&num) || in_flight.len() >= MAX_RESOLVE_DEPTH {
            return Err(Error::ReferenceCycle(num));
        }
        let Some(&entry) = self.entries.get(&num) else {
            return Ok(Rc::new(Object::Null));
        };
        in_flight.push(num);
        let result = match entry {
            XrefEntry::Free => Ok(Rc::new(Object::Null)),
            XrefEntry::InFile { offset, .. } => {
                let mut resolve_length = |r: ObjRef| self.load(r.num, in_flight).ok().and_then(|o| o.as_int());
                Parser::new(self.data, offset)
                    .indirect_object(&mut resolve_length)
                    .map(|(_, obj)| Rc::new(obj))
                    .map_err(Error::from)
            }
            XrefEntry::InStream { stream, index } => self.load_compressed(num, stream, index, in_flight),
        };
        in_flight.pop();
        if let Ok(obj) = &result {
            self.object_cache.borrow_mut().insert(num, Rc::clone(obj));
        }
        result
    }

    fn load_compressed(&self, num: u32, stream: u32, index: u32, in_flight: &mut Vec<u32>) -> Result<Rc<Object>> {
        let members = self.object_stream(stream, in_flight)?;
        let by_index = usize::try_from(index)
            .ok()
            .and_then(|i| members.get(i))
            .filter(|(n, _)| *n == num);
        // Fall back to a search when the index is stale or wrong.
        let found = by_index.or_else(|| members.iter().find(|(n, _)| *n == num));
        Ok(found.map_or_else(|| Rc::new(Object::Null), |(_, obj)| Rc::clone(obj)))
    }

    fn object_stream(&self, stream: u32, in_flight: &mut Vec<u32>) -> Result<ObjStmMembers> {
        if let Some(cached) = self.objstm_cache.borrow().get(&stream) {
            return Ok(Rc::clone(cached));
        }
        if !matches!(self.entries.get(&stream), Some(XrefEntry::InFile { .. })) {
            return Err(Error::InvalidStructure {
                kind: "object stream",
                message: "container is not a top-level object",
            });
        }
        let container = self.load(stream, in_flight)?;
        let Object::Stream(s) = &*container else {
            return Err(Error::Type {
                expected: "stream",
                context: "object stream container",
            });
        };
        let decoded = self.decode_stream(s)?;
        let members: Vec<_> = objstm::parse(&s.dict, &decoded)?
            .into_iter()
            .map(|(n, obj)| (n, Rc::new(obj)))
            .collect();
        let members = Rc::new(members);
        self.objstm_cache.borrow_mut().insert(stream, Rc::clone(&members));
        Ok(members)
    }

    /// Decodes a stream's data (Flate + PNG predictors), resolving indirect filter parameters.
    ///
    /// # Errors
    /// Unsupported filters, corrupt data, or output over [`OpenOptions::max_decoded_size`].
    pub fn decode_stream(&self, stream: &Stream) -> Result<Vec<u8>> {
        let mut params = Dict::new();
        for key in ["Filter", "DecodeParms"] {
            if let Some(v) = stream.dict.get(key) {
                params.insert(key, self.deref(v)?.into_owned());
            }
        }
        Ok(filters::decode(&params, &stream.data, self.options.max_decoded_size)?)
    }

    /// The document catalog (`/Root`).
    ///
    /// # Errors
    /// [`Error::Type`] when `/Root` is not a dictionary.
    pub fn catalog(&self) -> Result<Dict> {
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
    /// Tolerant: a node that is missing, unparsable or not a dictionary (including a
    /// missing `/Pages` root) is skipped, as is a `/Kids` that is not an array — missing
    /// objects are `null` (§7.3.10). Only cycles are errors.
    ///
    /// # Errors
    /// [`Error::ReferenceCycle`] when a page-tree node is reachable twice (e.g. a
    /// self-referencing `/Kids`) or a reference chain loops; errors from [`Document::catalog`].
    pub fn pages(&self) -> Result<Vec<Page>> {
        let catalog = self.catalog()?;
        let root = catalog.get("Pages").cloned().unwrap_or(Object::Null);
        let mut visited = BTreeSet::new();
        let mut stack = vec![(root, Rc::new(Inherited::default()))];
        let mut pages = Vec::new();
        while let Some((node_obj, inherited)) = stack.pop() {
            let obj_ref = node_obj.as_ref();
            if let Some(r) = obj_ref
                && !visited.insert(r.num)
            {
                return Err(Error::ReferenceCycle(r.num));
            }
            let resolved = match self.deref(&node_obj) {
                Ok(obj) => obj,
                Err(e @ Error::ReferenceCycle(_)) => return Err(e),
                Err(_) => continue,
            };
            let Object::Dict(node) = &*resolved else {
                continue;
            };
            let attrs = Inherited::overlay(&inherited, node);
            let is_tree_node = node.get_name("Type") == Some(b"Pages")
                || (node.get_name("Type") != Some(b"Page") && node.contains_key("Kids"));
            if is_tree_node {
                let kids = match self.deref(node.get("Kids").unwrap_or(&Object::Null)) {
                    Ok(obj) => obj,
                    Err(e @ Error::ReferenceCycle(_)) => return Err(e),
                    Err(_) => continue,
                };
                if let Object::Array(kids) = &*kids {
                    stack.extend(kids.iter().rev().map(|kid| (kid.clone(), Rc::clone(&attrs))));
                }
            } else {
                pages.push(self.make_page(obj_ref, node.clone(), &attrs));
            }
        }
        Ok(pages)
    }

    fn make_page(&self, obj_ref: Option<ObjRef>, dict: Dict, attrs: &Inherited) -> Page {
        let rect = |o: &Option<Rc<Object>>| o.as_deref().and_then(|o| as_rect(self, o));
        let rotate = attrs
            .rotate
            .as_deref()
            .and_then(|o| self.deref(o).ok()?.as_int())
            .unwrap_or(0);
        // A direct dictionary is already shared through `attrs`; a reference resolves to
        // the cached object. Either way no dictionary is copied per page.
        let resources = attrs.resources.as_ref().and_then(|o| {
            let shared = match **o {
                Object::Ref(r) => self.resolve(r).ok()?,
                _ => Rc::clone(o),
            };
            matches!(*shared, Object::Dict(_)).then_some(shared)
        });
        Page {
            obj_ref,
            media_box: rect(&attrs.media_box),
            crop_box: rect(&attrs.crop_box),
            rotate,
            resources,
            dict,
        }
    }

    /// Number of leaf pages in the page tree.
    ///
    /// # Errors
    /// As [`Document::pages`].
    pub fn page_count(&self) -> Result<usize> {
        self.pages().map(|p| p.len())
    }

    /// The decoded content of a page: all `/Contents` streams joined with `\n`.
    ///
    /// # Errors
    /// Decode errors, or [`Error::Type`] if `/Contents` is not a stream or array of streams.
    pub fn page_content(&self, page: &Page) -> Result<Vec<u8>> {
        let Some(contents) = page.dict.get("Contents") else {
            return Ok(Vec::new());
        };
        let resolved = self.deref(contents)?;
        let parts = match &*resolved {
            Object::Array(items) => items.as_slice(),
            single => std::slice::from_ref(single),
        };
        let mut out = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            let part = self.deref(part)?;
            let Object::Stream(s) = &*part else {
                return Err(Error::Type {
                    expected: "stream",
                    context: "/Contents",
                });
            };
            if i > 0 {
                out.push(b'\n');
            }
            out.extend_from_slice(&self.decode_stream(s)?);
        }
        Ok(out)
    }
}
