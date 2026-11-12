//! [`ObjectCopier`]: pages and what they reach, deep-copied from one document into another's
//! overlay (E2 page organisation plan §2.2; ISO 32000-2 §7.3.10, §7.7.3).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use laminapdf_cos::{CosMut, Dict, Error, Limit, Limits, ObjRef, Object, Position, Result, strip_crypt_filter};
use sha2::{Digest as _, Sha256};

/// What an object hashes to once its references are replaced by what they point at: two
/// streams with the same key are the same stream in the target.
type Key = [u8; 32];

/// Copies pages, and every object they reach, from a source document into a target
/// document's overlay, with fresh object numbers ([ADR 0005] §3).
///
/// * **Once per source.** An object reached from several copied pages (a font, an image) is
///   copied once, and so is a stream stored twice in the source with the same dictionary
///   and bytes: streams are matched by a hash of their content, references included. The
///   copier remembers what it copied, so later calls with the same copier reuse it; keep one
///   copier per source and target for as long as the target keeps its copies.
/// * **Pages stay pages.** A page's `/Parent` is left out and the attributes it inherited
///   (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`; §7.7.3.4) are set on the copy, which
///   the caller then puts in the target's page tree. A reference to a page that is not being
///   copied, to a page-tree node or to the catalog (a link's destination, a widget's field
///   and its other widgets) is written as `null`, and the object it named is listed in
///   [`ObjectCopier::outside`].
/// * **Bounded.** The copy walks the source before it writes anything: stream data past
///   [`Limits::max_decoded_total`] bytes, or objects nested deeper than
///   [`Limits::max_nesting_depth`], stop it with [`Error::LimitExceeded`] and leave the
///   target as it was. Reference cycles are followed once.
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
    /// Each source object reached so far → its copy, or `None` where it is written as `null`.
    copies: BTreeMap<ObjRef, Option<ObjRef>>,
    /// Each copied stream's key → the copy.
    streams: BTreeMap<Key, ObjRef>,
    /// Each keyed source object's key; `None` when it can't be keyed (in a cycle, or too far).
    keys: BTreeMap<ObjRef, Option<Key>>,
    outside: BTreeSet<ObjRef>,
}

/// What one call reads from the source before it writes.
#[derive(Default)]
struct Walk {
    /// The source objects to copy, in the order they were reached.
    order: Vec<(ObjRef, Arc<Object>)>,
    /// The same, by reference.
    objects: BTreeMap<ObjRef, Arc<Object>>,
    /// References written as `null`: missing objects, and what lies outside the copy.
    nulls: BTreeSet<ObjRef>,
    /// Stream bytes read.
    bytes: usize,
}

/// Inheritable page attributes (ISO 32000-2 §7.7.3.4), as [`laminapdf_cos::Inherited`] holds them.
const INHERITED: [&str; 4] = ["Resources", "MediaBox", "CropBox", "Rotate"];

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
        let root_refs: BTreeSet<ObjRef> = pages.iter().filter_map(|page| page.obj_ref).collect();
        let walk = self.walk(&roots, &root_refs)?;
        for (r, object) in &walk.order {
            if matches!(**object, Object::Stream(_)) {
                self.key_of(*r, &walk, &root_refs, &mut Vec::new())?;
            }
        }

        // Every read is done: from here on, numbers are reserved and objects written.
        let doc = target.document();
        let copies = pages.iter().map(|_| doc.reserve_number()).collect::<Result<Vec<_>>>()?;
        for (page, &copy) in pages.iter().zip(&copies) {
            if let Some(r) = page.obj_ref {
                self.copies.insert(r, Some(copy));
            }
        }
        for &r in &walk.nulls {
            self.copies.insert(r, None);
        }
        let mut writes = Vec::new();
        for (r, object) in &walk.order {
            let key = match **object {
                Object::Stream(_) => self.keys.get(r).copied().flatten(),
                _ => None,
            };
            if let Some(&copy) = key.and_then(|k| self.streams.get(&k)) {
                self.copies.insert(*r, Some(copy));
                continue;
            }
            let copy = doc.reserve_number()?;
            self.copies.insert(*r, Some(copy));
            if let Some(key) = key {
                self.streams.insert(key, copy);
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

    /// The copy of source object `source`: `Some(None)` when references to it are written as
    /// `null`, `None` when nothing copied so far reached it.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn copy_of(&self, source: ObjRef) -> Option<Option<ObjRef>> {
        self.copies.get(&source).copied()
    }

    /// The source pages, page-tree nodes and catalog that copied objects referred to without
    /// being copied; those references were written as `null`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn outside(&self) -> &BTreeSet<ObjRef> {
        &self.outside
    }

    /// Reads every source object the `roots` reach that isn't copied yet, checking the limits.
    fn walk(&mut self, roots: &[Dict], root_refs: &BTreeSet<ObjRef>) -> Result<Walk> {
        let mut walk = Walk::default();
        let mut queue = VecDeque::new();
        for root in roots {
            self.scan_dict(root, None, 0, &mut queue)?;
        }
        while let Some(r) = queue.pop_front() {
            if root_refs.contains(&r)
                || self.copies.contains_key(&r)
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
            if is_outside(&object) {
                self.outside.insert(r);
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
    fn key_of(
        &mut self,
        r: ObjRef,
        walk: &Walk,
        roots: &BTreeSet<ObjRef>,
        stack: &mut Vec<ObjRef>,
    ) -> Result<Option<Key>> {
        if let Some(&known) = self.keys.get(&r) {
            return Ok(known);
        }
        if walk.nulls.contains(&r) || self.copies.get(&r) == Some(&None) {
            return Ok(Some(NULL_KEY));
        }
        let mut hash = Sha256::new();
        // A page copied with this call, or an object an earlier call copied, stands for itself.
        if roots.contains(&r) {
            hash.update(b"page");
            hash.update(r.num.to_be_bytes());
            hash.update(r.gen_nr.to_be_bytes());
            return Ok(Some(hash.finalize().into()));
        }
        if let Some(Some(copy)) = self.copies.get(&r) {
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
        let keyed = self.feed(&mut hash, object, walk, roots, stack, 0);
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
        roots: &BTreeSet<ObjRef>,
        stack: &mut Vec<ObjRef>,
        depth: usize,
    ) -> Result<bool> {
        if depth > self.limits.max_nesting_depth() {
            return Ok(false);
        }
        match object {
            Object::Null => put(hash, 0, &[]),
            Object::Bool(b) => put(hash, 1, &[u8::from(*b)]),
            Object::Integer(i) => put(hash, 2, &i.to_be_bytes()),
            Object::Real(f) => put(hash, 3, &f.to_bits().to_be_bytes()),
            Object::String(s) => put(hash, 4, &s.bytes),
            Object::Name(n) => put(hash, 5, n.as_bytes()),
            Object::Ref(r) => match self.key_of(*r, walk, roots, stack)? {
                Some(key) => put(hash, 6, &key),
                None => return Ok(false),
            },
            Object::Array(items) => {
                put(hash, 7, &(items.len() as u64).to_be_bytes());
                for item in items {
                    if !self.feed(hash, item, walk, roots, stack, depth + 1)? {
                        return Ok(false);
                    }
                }
            }
            Object::Dict(dict) => return self.feed_dict(hash, dict, walk, roots, stack, depth),
            Object::Stream(stream) => {
                if !self.feed_dict(hash, &stream.dict, walk, roots, stack, depth)? {
                    return Ok(false);
                }
                put(hash, 9, &stream.data);
            }
        }
        Ok(true)
    }

    fn feed_dict(
        &mut self,
        hash: &mut Sha256,
        dict: &Dict,
        walk: &Walk,
        roots: &BTreeSet<ObjRef>,
        stack: &mut Vec<ObjRef>,
        depth: usize,
    ) -> Result<bool> {
        put(hash, 8, &(dict.len() as u64).to_be_bytes());
        for (name, value) in dict.iter() {
            // The writer sets /Length; an indirect one would only make equal streams differ.
            if name.as_bytes() == b"Length" {
                continue;
            }
            put(hash, 5, name.as_bytes());
            if !self.feed(hash, value, walk, roots, stack, depth + 1)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `object` with each source reference replaced by its copy (`null` where there is none).
    fn rewrite(&self, object: &Object) -> Object {
        match object {
            Object::Ref(r) => self.copies.get(r).copied().flatten().map_or(Object::Null, Object::Ref),
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
    let inherited = &page.inherited;
    for (key, value) in INHERITED.into_iter().zip([
        &inherited.resources,
        &inherited.media_box,
        &inherited.crop_box,
        &inherited.rotate,
    ]) {
        if let Some(value) = value
            && matches!(dict.get(key), None | Some(Object::Null))
        {
            dict.insert(key, (**value).clone());
        }
    }
    dict
}

/// Whether `object` is a page, a page-tree node or the catalog: what a copy points at only
/// when it is copied too.
fn is_outside(object: &Object) -> bool {
    let Some(Object::Name(kind)) = object.as_dict().and_then(|d| d.get("Type")) else {
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
