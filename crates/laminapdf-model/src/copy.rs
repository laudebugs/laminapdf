//! [`ObjectCopier`]: pages and what they reach, deep-copied from one document into another's
//! overlay (E2 page organisation plan §2.2; ISO 32000-2 §7.3.10, §7.7.3).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use laminapdf_cos::{CosMut, Dict, Error, Limit, Limits, ObjRef, Object, Position, Result, strip_crypt_filter};
use sha2::{Digest as _, Sha256};

/// What an object hashes to once its references are replaced by what they point at: two
/// streams with the same key are the same stream in the target.
type Key = [u8; 32];

/// What a source object becomes in the target ([`ObjectCopier::copy_of`]).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Copied {
    /// This target object.
    Object(ObjRef),
    /// Nothing: references to it are written as `null` (a missing object, or one outside the
    /// copy).
    Null,
}

impl Copied {
    /// The reference a copied reference to the source object becomes.
    fn reference(self) -> Object {
        match self {
            Self::Object(r) => Object::Ref(r),
            Self::Null => Object::Null,
        }
    }
}

/// Copies pages, and every object they reach, from a source document into a target
/// document's overlay, with fresh object numbers ([ADR 0005] §3).
///
/// * **Once per source.** An object reached from several copied pages (a font, an image) is
///   copied once, and so is a stream stored twice in the source with the same dictionary
///   and bytes: streams are matched by a hash of their content, references included. The
///   copier remembers what it copied, so later calls with the same copier reuse it. Its
///   records name objects in the target: drop it when the target loses them (a transaction
///   rolled back, an undo), and use a new one.
/// * **Pages stay pages.** A page's `/Parent` is left out and the attributes it inherited
///   (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`; §7.7.3.4) are set on the copy, which
///   the caller then puts in the target's page tree. A reference to a page that is not being
///   copied, to a page-tree node, to the catalog (a link's destination) or to an annotation
///   whose `/P` names a page that is not being copied (a field's widget on another page) is
///   written as `null`, and the object it named is listed in [`ObjectCopier::outside`]. An
///   annotation a copied page lists in `/Annots` always comes with it, and one left out
///   earlier comes with its page when a later call copies that page. A widget's field comes
///   with it; fitting it into the target's form is the caller's job.
/// * **Bounded.** The copy walks the source before it writes anything. Streams whose stored
///   (still encoded) data passes [`Limits::max_decoded_total`] bytes in one call, the bytes
///   the copy holds in memory, or objects nested deeper than [`Limits::max_nesting_depth`],
///   stop it with [`Error::LimitExceeded`], leaving the target and the copier as they were.
///   Reference cycles are followed once.
/// * **Decrypted.** Objects of an encrypted source are read decrypted, and their streams lose
///   the `/Crypt` filter (§7.4.10), so the copies are written as plain objects (encrypted
///   only if the target is).
///
/// ```
/// use laminapdf_cos::{Document, OpenOptions};
/// use laminapdf_model::ObjectCopier;
/// # let file = |objects: &[&str]| {
/// #     let mut bytes = b"%PDF-1.7\n".to_vec();
/// #     let mut offsets = Vec::new();
/// #     for (i, body) in objects.iter().enumerate() {
/// #         offsets.push(bytes.len());
/// #         bytes.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
/// #     }
/// #     let xref = bytes.len();
/// #     bytes.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
/// #     for offset in offsets {
/// #         bytes.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
/// #     }
/// #     bytes.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
/// #     bytes
/// # };
/// # let source_bytes = file(&[
/// #     "<< /Type /Catalog /Pages 2 0 R >>",
/// #     "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>",
/// #     "<< /Type /Page /Parent 2 0 R >>",
/// # ]);
/// # let target_bytes = file(&["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [] /Count 0 >>"]);
/// let source = Document::open(source_bytes, OpenOptions::default())?;
/// let mut target = Document::open(target_bytes, OpenOptions::default())?;
///
/// let pages = source.pages()?;
/// let mut copier = ObjectCopier::new(&source, *target.limits());
/// let copies = copier.copy_pages(&mut target.edit(), &[&pages[0]])?;
///
/// // The copy has no /Parent yet, and carries the media box it inherited.
/// let copy = target.edit().get(copies[0])?.unwrap();
/// let dict = copy.as_dict().unwrap();
/// assert!(dict.get("Parent").is_none());
/// assert!(dict.get("MediaBox").is_some());
/// # Ok::<(), laminapdf_cos::Error>(())
/// ```
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
///
/// Added in 0.2.0.
#[derive(Debug)]
pub struct ObjectCopier<'s> {
    source: &'s laminapdf_cos::Document,
    limits: Limits,
    /// Each source object reached so far → what it became.
    copies: BTreeMap<ObjRef, Copied>,
    /// Each copied stream's key → the copy.
    streams: BTreeMap<Key, ObjRef>,
    /// Each keyed source object's key; `None` when it can't be keyed (in a cycle, or too far).
    keys: BTreeMap<ObjRef, Option<Key>>,
    outside: BTreeSet<ObjRef>,
}

/// What one call reads from the source before it writes.
#[derive(Default)]
struct Walk {
    /// The pages being copied.
    roots: BTreeSet<ObjRef>,
    /// The source objects to copy, in the order they were reached.
    order: Vec<(ObjRef, Arc<Object>)>,
    /// The same, by reference.
    objects: BTreeMap<ObjRef, Arc<Object>>,
    /// References written as `null`: missing objects, and what lies outside the copy.
    nulls: BTreeSet<ObjRef>,
    /// What lies outside the copy, for [`ObjectCopier::outside`] once the call succeeds.
    outside: BTreeSet<ObjRef>,
    /// The annotations the pages being copied list in `/Annots`: never outside.
    annots: BTreeSet<ObjRef>,
    /// Stream bytes read.
    bytes: usize,
}

/// What one call changed in the copier's records, undone if the call fails.
#[derive(Default)]
struct Journal {
    /// Each source object mapped, with what it was mapped to before.
    copies: Vec<(ObjRef, Option<Copied>)>,
    /// Each stream key added.
    streams: Vec<Key>,
}

/// The key of a reference written as `null`.
const NULL_KEY: Key = [0; 32];

impl<'s> ObjectCopier<'s> {
    /// A copier from `source` that stops at `limits` (the target's, usually).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(source: &'s laminapdf_cos::Document, limits: Limits) -> Self {
        Self {
            source,
            limits,
            copies: BTreeMap::new(),
            streams: BTreeMap::new(),
            keys: BTreeMap::new(),
            outside: BTreeSet::new(),
        }
    }

    /// Copies `pages` of the source into `target`, each as a new page object without a
    /// `/Parent`, and returns the copies in order. References between the pages copied
    /// together (a link from one to another, an annotation's `/P`) point at the copies.
    ///
    /// # Errors
    /// [`Error::LimitExceeded`] past the copier's [`Limits`], with `target` unchanged; errors
    /// reading the source ([`Error::ReferenceCycle`] in a chain of references, parse errors,
    /// [`Error::DataPending`]), also with `target` unchanged; errors allocating object
    /// numbers in `target` after some objects were written (roll back the caller's
    /// transaction).
    ///
    /// Added in 0.2.0.
    pub fn copy_pages(&mut self, target: &mut CosMut<'_>, pages: &[&laminapdf_cos::Page]) -> Result<Vec<ObjRef>> {
        let roots: Vec<Dict> = pages.iter().map(|page| page_dict(page)).collect();
        let walk = self.walk(&roots, pages.iter().filter_map(|page| page.obj_ref).collect())?;
        let mut journal = Journal::default();
        let written = self
            .key_streams(&walk)
            .and_then(|()| self.write(target, pages, &roots, &walk, &mut journal));
        if written.is_err() {
            self.undo(journal);
        } else {
            self.outside.extend(&walk.outside);
            // A page or annotation left out before may be copied now.
            let copies = &self.copies;
            self.outside
                .retain(|r| !matches!(copies.get(r), Some(Copied::Object(_))));
        }
        written
    }

    /// Copies `value` of the source (a resource dictionary, a destination array) into `target`:
    /// every object it reaches that this copier hasn't copied yet is copied, and the value is
    /// returned with its references pointing at the copies. A page, page-tree node or the
    /// catalog it reaches is written as `null`, as [`ObjectCopier::copy_pages`] writes it.
    ///
    /// # Errors
    /// As [`ObjectCopier::copy_pages`].
    ///
    /// Added in 0.2.0.
    pub fn copy_value(&mut self, target: &mut CosMut<'_>, value: &Object) -> Result<Object> {
        let root: Dict = [("V", value.clone())].into_iter().collect();
        let walk = self.walk(std::slice::from_ref(&root), BTreeSet::new())?;
        let mut journal = Journal::default();
        let written = self
            .key_streams(&walk)
            .and_then(|()| self.write(target, &[], &[], &walk, &mut journal));
        if let Err(e) = written {
            self.undo(journal);
            return Err(e);
        }
        self.outside.extend(&walk.outside);
        Ok(self.rewrite(value))
    }

    /// Records that `source` now became `copied`, so a failed call can undo it.
    fn map(&mut self, journal: &mut Journal, source: ObjRef, copied: Copied) {
        journal.copies.push((source, self.copies.insert(source, copied)));
    }

    /// Puts the copier's records back as they were before a failed call.
    fn undo(&mut self, journal: Journal) {
        for (source, before) in journal.copies.into_iter().rev() {
            match before {
                Some(copied) => self.copies.insert(source, copied),
                None => self.copies.remove(&source),
            };
        }
        for key in journal.streams {
            self.streams.remove(&key);
        }
    }

    /// Keys every stream the walk read, so equal ones are copied once.
    fn key_streams(&mut self, walk: &Walk) -> Result<()> {
        for (r, object) in &walk.order {
            if matches!(**object, Object::Stream(_)) {
                self.key_of(*r, walk, &mut Vec::new())?;
            }
        }
        Ok(())
    }

    /// Reserves the numbers of the copies and writes them: the pages last, as `roots`.
    fn write(
        &mut self,
        target: &mut CosMut<'_>,
        pages: &[&laminapdf_cos::Page],
        roots: &[Dict],
        walk: &Walk,
        journal: &mut Journal,
    ) -> Result<Vec<ObjRef>> {
        let doc = target.document();
        let copies = pages.iter().map(|_| doc.reserve_number()).collect::<Result<Vec<_>>>()?;
        for (page, &copy) in pages.iter().zip(&copies) {
            if let Some(r) = page.obj_ref {
                self.map(journal, r, Copied::Object(copy));
            }
        }
        for &r in &walk.nulls {
            self.map(journal, r, Copied::Null);
        }
        let mut writes = Vec::new();
        for (r, object) in &walk.order {
            let key = match **object {
                Object::Stream(_) => self.keys.get(r).copied().flatten(),
                _ => None,
            };
            if let Some(&copy) = key.and_then(|k| self.streams.get(&k)) {
                self.map(journal, *r, Copied::Object(copy));
                continue;
            }
            let copy = doc.reserve_number()?;
            self.map(journal, *r, Copied::Object(copy));
            if let Some(key) = key {
                self.streams.insert(key, copy);
                journal.streams.push(key);
            }
            writes.push((copy, Arc::clone(object)));
        }
        let encrypted = self.source.encryption().is_some();
        for (copy, object) in writes {
            let mut object = self.rewrite(&object);
            if let Object::Stream(stream) = &mut object {
                // The writer sets /Length, and the data was decrypted when it was read.
                stream.dict.remove("Length");
                if encrypted {
                    strip_crypt_filter(&mut stream.dict);
                }
            }
            target.set(copy, object)?;
        }
        for (root, &copy) in roots.iter().zip(&copies) {
            target.set(copy, Object::Dict(self.rewrite_dict(root)))?;
        }
        Ok(copies)
    }

    /// What source object `source` became in the target; `None` when nothing copied so far
    /// reached it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn copy_of(&self, source: ObjRef) -> Option<Copied> {
        self.copies.get(&source).copied()
    }

    /// The source pages, page-tree nodes, catalog and annotations of other pages that copied
    /// objects referred to without being copied; those references were written as `null`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn outside(&self) -> &BTreeSet<ObjRef> {
        &self.outside
    }

    /// Reads every source object the `roots` reach that isn't copied yet, checking the limits.
    fn walk(&self, roots: &[Dict], root_refs: BTreeSet<ObjRef>) -> Result<Walk> {
        let mut walk = Walk {
            roots: root_refs,
            ..Walk::default()
        };
        let mut queue = VecDeque::new();
        for root in roots {
            self.scan_dict(root, None, 0, &mut queue)?;
            walk.annots.extend(self.annots(root)?);
        }
        while let Some(r) = queue.pop_front() {
            // Something copied before is reused; something left out before is looked at again,
            // since the pages copied now may be what it belongs to.
            if walk.roots.contains(&r)
                || matches!(self.copies.get(&r), Some(Copied::Object(_)))
                || walk.objects.contains_key(&r)
                || walk.nulls.contains(&r)
            {
                continue;
            }
            let object = self.source.resolve(r)?;
            if matches!(*object, Object::Null) {
                walk.nulls.insert(r);
                continue;
            }
            if !walk.annots.contains(&r) && self.is_outside(&object, &walk.roots)? {
                walk.outside.insert(r);
                walk.nulls.insert(r);
                continue;
            }
            match &*object {
                Object::Stream(stream) => {
                    walk.bytes = walk.bytes.saturating_add(stream.data.len());
                    if walk.bytes > self.limits.max_decoded_total() {
                        return Err(limit(Limit::DecodedTotal, None));
                    }
                    self.scan_dict(&stream.dict, Some(r), 1, &mut queue)?;
                }
                other => self.scan(other, Some(r), 0, &mut queue)?,
            }
            walk.order.push((r, Arc::clone(&object)));
            walk.objects.insert(r, object);
        }
        Ok(walk)
    }

    /// The annotations `page` lists in `/Annots`: they come with it whatever their `/P` says.
    fn annots(&self, page: &Dict) -> Result<Vec<ObjRef>> {
        let Some(annots) = page.get("Annots") else {
            return Ok(Vec::new());
        };
        let annots = self.source.deref(annots)?;
        Ok(annots
            .as_array()
            .map(|items| items.iter().filter_map(Object::as_ref).collect())
            .unwrap_or_default())
    }

    /// Whether `object` lies outside the copy: a page not being copied (and not copied
    /// earlier), a page-tree node, the catalog, or an annotation whose `/P` names such a page.
    fn is_outside(&self, object: &Object, roots: &BTreeSet<ObjRef>) -> Result<bool> {
        let Some(dict) = object.as_dict() else {
            return Ok(false);
        };
        if is_page_tree_or_catalog(dict) {
            return Ok(true);
        }
        let Some(page) = dict.get("P").and_then(Object::as_ref) else {
            return Ok(false);
        };
        if roots.contains(&page) || matches!(self.copies.get(&page), Some(Copied::Object(_))) {
            return Ok(false);
        }
        Ok(self
            .source
            .resolve(page)?
            .as_dict()
            .is_some_and(is_page_tree_or_catalog))
    }

    /// Queues the references in `object`, checking its nesting.
    fn scan(&self, object: &Object, at: Option<ObjRef>, depth: usize, queue: &mut VecDeque<ObjRef>) -> Result<()> {
        if depth > self.limits.max_nesting_depth() {
            return Err(limit(Limit::NestingDepth, at.map(|r| Position::Object(r.num))));
        }
        match object {
            Object::Ref(r) => queue.push_back(*r),
            Object::Array(items) => {
                for item in items {
                    self.scan(item, at, depth + 1, queue)?;
                }
            }
            Object::Dict(dict) => self.scan_dict(dict, at, depth + 1, queue)?,
            Object::Stream(stream) => self.scan_dict(&stream.dict, at, depth + 1, queue)?,
            _ => {}
        }
        Ok(())
    }

    fn scan_dict(&self, dict: &Dict, at: Option<ObjRef>, depth: usize, queue: &mut VecDeque<ObjRef>) -> Result<()> {
        for (_, value) in dict.iter() {
            self.scan(value, at, depth, queue)?;
        }
        Ok(())
    }

    /// The key of source object `r`, computed once: its content with each reference replaced
    /// by the key of what it points at. `None` for an object in a reference cycle, or more
    /// than [`Limits::max_resolve_depth`] references away from the stream being keyed: such a
    /// stream is copied, just never matched with another.
    fn key_of(&mut self, r: ObjRef, walk: &Walk, stack: &mut Vec<ObjRef>) -> Result<Option<Key>> {
        if let Some(&known) = self.keys.get(&r) {
            return Ok(known);
        }
        if walk.nulls.contains(&r) || self.copies.get(&r) == Some(&Copied::Null) {
            return Ok(Some(NULL_KEY));
        }
        let mut hash = Sha256::new();
        // A page copied with this call, or an object an earlier call copied, stands for itself.
        if walk.roots.contains(&r) {
            hash.update(b"page");
            hash.update(r.num.to_be_bytes());
            hash.update(r.gen_nr.to_be_bytes());
            return Ok(Some(hash.finalize().into()));
        }
        if let Some(Copied::Object(copy)) = self.copies.get(&r) {
            // A later call keys such a stream apart from an equal one keyed by its content:
            // it is copied again, never matched wrongly.
            hash.update(b"copy");
            hash.update(copy.num.to_be_bytes());
            return Ok(Some(hash.finalize().into()));
        }
        let Some(object) = walk.objects.get(&r) else {
            return Ok(None);
        };
        if stack.contains(&r) || stack.len() >= self.limits.max_resolve_depth() {
            return Ok(None);
        }
        stack.push(r);
        let keyed = self.feed(&mut hash, object, walk, stack, 0);
        stack.pop();
        let key = keyed?.then(|| hash.finalize().into());
        self.keys.insert(r, key);
        Ok(key)
    }

    /// Hashes `object` into `hash`; `false` when a reference in it can't be keyed.
    fn feed(
        &mut self,
        hash: &mut Sha256,
        object: &Object,
        walk: &Walk,
        stack: &mut Vec<ObjRef>,
        depth: usize,
    ) -> Result<bool> {
        if depth > self.limits.max_nesting_depth() {
            return Ok(false);
        }
        match object {
            Object::Null => put(hash, tag::NULL, &[]),
            Object::Bool(b) => put(hash, tag::BOOL, &[u8::from(*b)]),
            Object::Integer(i) => put(hash, tag::INTEGER, &i.to_be_bytes()),
            Object::Real(f) => put(hash, tag::REAL, &f.to_bits().to_be_bytes()),
            Object::String(s) => put(hash, tag::STRING, &s.bytes),
            Object::Name(n) => put(hash, tag::NAME, n.as_bytes()),
            Object::Ref(r) => match self.key_of(*r, walk, stack)? {
                Some(key) => put(hash, tag::REFERENCE, &key),
                None => return Ok(false),
            },
            Object::Array(items) => {
                put(hash, tag::ARRAY, &(items.len() as u64).to_be_bytes());
                for item in items {
                    if !self.feed(hash, item, walk, stack, depth + 1)? {
                        return Ok(false);
                    }
                }
            }
            Object::Dict(dict) => return self.feed_dict(hash, dict, walk, stack, depth),
            Object::Stream(stream) => {
                if !self.feed_dict(hash, &stream.dict, walk, stack, depth)? {
                    return Ok(false);
                }
                put(hash, tag::STREAM_DATA, &stream.data);
            }
        }
        Ok(true)
    }

    fn feed_dict(
        &mut self,
        hash: &mut Sha256,
        dict: &Dict,
        walk: &Walk,
        stack: &mut Vec<ObjRef>,
        depth: usize,
    ) -> Result<bool> {
        put(hash, tag::DICT, &(dict.len() as u64).to_be_bytes());
        for (name, value) in dict.iter() {
            // The writer sets /Length; an indirect one would only make equal streams differ.
            if name.as_bytes() == b"Length" {
                continue;
            }
            put(hash, tag::NAME, name.as_bytes());
            if !self.feed(hash, value, walk, stack, depth + 1)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `object` with each source reference replaced by its copy (`null` where there is none).
    fn rewrite(&self, object: &Object) -> Object {
        match object {
            Object::Ref(r) => self.copies.get(r).map_or(Object::Null, |copied| copied.reference()),
            Object::Array(items) => Object::Array(items.iter().map(|item| self.rewrite(item)).collect()),
            Object::Dict(dict) => Object::Dict(self.rewrite_dict(dict)),
            Object::Stream(stream) => {
                let mut copy = stream.clone();
                copy.dict = self.rewrite_dict(&stream.dict);
                Object::Stream(copy)
            }
            other => other.clone(),
        }
    }

    fn rewrite_dict(&self, dict: &Dict) -> Dict {
        let mut copy = Dict::new();
        for (name, value) in dict.iter() {
            copy.insert(name.clone(), self.rewrite(value));
        }
        copy
    }
}

/// The dictionary a copy of `page` starts from: its own, without `/Parent`, with the
/// attributes it inherited set on it.
fn page_dict(page: &laminapdf_cos::Page) -> Dict {
    let mut dict = page.dict.clone();
    dict.remove("Parent");
    for (key, value) in page.inherited.entries() {
        if let Some(value) = value
            && matches!(dict.get(key), None | Some(Object::Null))
        {
            dict.insert(key, (**value).clone());
        }
    }
    dict
}

/// The tags that keep the hashed encodings of different kinds of value apart.
mod tag {
    pub(super) const NULL: u8 = 0;
    pub(super) const BOOL: u8 = 1;
    pub(super) const INTEGER: u8 = 2;
    pub(super) const REAL: u8 = 3;
    pub(super) const STRING: u8 = 4;
    pub(super) const NAME: u8 = 5;
    pub(super) const REFERENCE: u8 = 6;
    pub(super) const ARRAY: u8 = 7;
    pub(super) const DICT: u8 = 8;
    pub(super) const STREAM_DATA: u8 = 9;
}

/// Whether `dict` is a page, a page-tree node or the catalog.
fn is_page_tree_or_catalog(dict: &Dict) -> bool {
    let Some(Object::Name(kind)) = dict.get("Type") else {
        return false;
    };
    matches!(kind.as_bytes(), b"Page" | b"Pages" | b"Catalog")
}

/// Feeds one tagged, length-prefixed value to `hash`, so no two encodings collide.
fn put(hash: &mut Sha256, tag: u8, data: &[u8]) {
    hash.update([tag]);
    hash.update((data.len() as u64).to_be_bytes());
    hash.update(data);
}

fn limit(limit: Limit, at: Option<Position>) -> Error {
    Error::LimitExceeded { limit, at }
}
