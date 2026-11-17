//! Attached files (ISO 32000-2 §7.11.3 file specifications, §7.11.4 embedded file streams,
//! §14.13 associated files, §12.3.5 collections): the files a document carries, what holds
//! each one, their bytes with their checksum checked, and a portfolio's schema.
//!
//! One file specification can be held from several places at once: the catalog's
//! `/Names /EmbeddedFiles` tree, the `/AF` (associated files) arrays of the catalog, pages and
//! annotations, and a `FileAttachment` annotation's `/FS`. [`Document::attachments`] lists each
//! file specification once with all its holders; its [`AttachmentId`] is the specification's
//! object, or a path ID when it is a direct dictionary.

mod bytes;
mod collection;
mod relationship;
#[cfg(feature = "tree-write")]
mod write;

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, DirectPath, ObjRef, Object, PathSegment, Result};

pub use bytes::{AttachmentBytes, Checksum};
pub use collection::{
    Collection, CollectionField, CollectionFieldKind, CollectionFile, CollectionValue, CollectionView,
};
pub use relationship::AfRelationship;
#[cfg(feature = "tree-write")]
pub use write::{EmbeddedFileSpec, write_embedded_file};

use crate::PdfDate;
use crate::annot::AnnotSubtype;
use crate::document::Document;
use crate::id::{AnnotId, AttachmentId, PageId};
use crate::values::{file_spec_name, read_name, text_of};

/// A file the document carries: a file specification (§7.11.3) and, when the file is embedded,
/// the facts of its embedded file stream (§7.11.4).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AttachedFile {
    /// Its ID, for the attachment ops and [`Document::attachment_bytes`].
    pub id: AttachmentId,
    /// Its key in the `/EmbeddedFiles` tree, decoded; `None` when the tree doesn't list it.
    pub key: Option<String>,
    /// The file's name: `/UF`, else `/F`.
    pub name: Option<String>,
    /// `/Desc`, a description viewers show beside the file.
    pub description: Option<String>,
    /// The embedded stream's `/Subtype`: the file's MIME type, such as `text/xml`.
    pub mime: Option<String>,
    /// `/Params /Size`: the file's length in bytes, as the file states it.
    pub size: Option<u64>,
    /// `/Params /CreationDate`.
    pub created: Option<PdfDate>,
    /// `/Params /ModDate`.
    pub modified: Option<PdfDate>,
    /// `/Params /CheckSum`: the MD5 digest of the file's bytes, as the file states it.
    pub checksum: Option<[u8; 16]>,
    /// `/AFRelationship`; `None` when absent or not a name the standard defines.
    pub relationship: Option<AfRelationship>,
    /// Whether the file's bytes are in the document (`/EF`): a specification without them
    /// names a file elsewhere, which Lamina PDF never fetches.
    pub embedded: bool,
    /// What holds it, in the order found: the tree, then the catalog's, the pages' and the
    /// annotations' `/AF`, and `FileAttachment` annotations.
    pub holders: Vec<AttachmentHolder>,
}

/// What holds an attached file.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AttachmentHolder {
    /// The document's `/EmbeddedFiles` name tree, the list viewers show.
    EmbeddedFiles,
    /// The catalog's `/AF`: the file is associated with the whole document (§14.13.3).
    Document,
    /// A page's `/AF` (§14.13.4).
    Page(PageId),
    /// An annotation's `/AF` (§14.13.7).
    Annotation {
        /// The page the annotation is on.
        page: PageId,
        /// The annotation.
        annotation: AnnotId,
    },
    /// A `FileAttachment` annotation's `/FS` (§12.5.6.15): an icon on the page that holds the
    /// file.
    FileAttachment {
        /// The page the annotation is on.
        page: PageId,
        /// The annotation.
        annotation: AnnotId,
    },
    /// The `/AF` array of another object, such as a structure element or an XObject (§14.13.5,
    /// §14.13.6): only [`Disclosure`](crate::Disclosure) reads these, since it reads every object.
    Object {
        /// The object.
        object: u32,
    },
}

/// Where a file specification is stored, for a writer to change it.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileSpecAt {
    /// Its own indirect object.
    Object(ObjRef),
    /// A direct dictionary inside another object.
    Direct(DirectPath),
}

/// A file specification found while listing, with where it is stored.
pub(crate) struct Found {
    pub(crate) attachment: AttachedFile,
    pub(crate) at: FileSpecAt,
    pub(crate) spec: Dict,
}

impl Document {
    /// Every file the document carries, each file specification once: those of the
    /// `/EmbeddedFiles` tree, of the `/AF` arrays of the catalog, the pages and their
    /// annotations, and of `FileAttachment` annotations, with what holds each.
    ///
    /// Reading is tolerant: an entry that isn't a file specification is left out.
    ///
    /// # Errors
    /// Only errors that must propagate ([`laminapdf_cos::Error::must_propagate`]).
    ///
    /// Added in 0.6.0.
    pub fn attachments(&self) -> Result<Vec<AttachedFile>> {
        Ok(self.found_attachments()?.into_iter().map(|f| f.attachment).collect())
    }

    /// The attachment with this ID, if the document still holds it.
    ///
    /// # Errors
    /// As [`Document::attachments`].
    ///
    /// Added in 0.6.0.
    pub fn attachment(&self, id: &AttachmentId) -> Result<Option<AttachedFile>> {
        Ok(self.find_attachment(id)?.map(|f| f.attachment))
    }

    /// Where the attachment `id`'s file specification is stored, and its dictionary as read.
    ///
    /// # Errors
    /// As [`Document::attachments`].
    ///
    /// Added in 0.6.0.
    pub fn file_spec_at(&self, id: &AttachmentId) -> Result<Option<(FileSpecAt, Dict)>> {
        Ok(self.find_attachment(id)?.map(|f| (f.at, f.spec)))
    }

    fn find_attachment(&self, id: &AttachmentId) -> Result<Option<Found>> {
        Ok(self.found_attachments()?.into_iter().find(|f| &f.attachment.id == id))
    }

    /// Every file specification, in the order of [`Document::attachments`].
    fn found_attachments(&self) -> Result<Vec<Found>> {
        let mut list = List::default();
        self.list_attachments(&mut list)?;
        Ok(list.found)
    }

    /// [`Document::attachments`]' file specifications, then those the `/AF` arrays of every other
    /// object hold and those nothing holds: one walk for the listing and the disclosure scan,
    /// which reads every object.
    pub(crate) fn every_file_spec(&self) -> Result<Vec<Found>> {
        let mut list = List::default();
        let seen = self.list_attachments(&mut list)?;
        for num in self.object_numbers() {
            if seen.contains(&num) {
                continue;
            }
            let Some(object) = laminapdf_cos::tolerant(self.object(num))? else {
                continue;
            };
            let Some(dict) = object.as_dict() else { continue };
            let holder = AttachmentHolder::Object { object: num };
            self.hold_associated(
                &mut list,
                dict,
                DirectPath::new(Some(ObjRef::new(num, 0)), Vec::new()),
                &holder,
            )?;
            if dict.get("EF").is_some() {
                list.add_unheld(self, ObjRef::new(num, 0), dict)?;
            }
        }
        Ok(list.found)
    }

    /// Lists the file specifications [`Document::attachments`] reads into `list`; returns the
    /// objects whose `/AF` it read (the catalog's, the pages' and the annotations').
    fn list_attachments(&self, list: &mut List) -> Result<std::collections::BTreeSet<u32>> {
        let mut seen = std::collections::BTreeSet::new();
        if let Some(tree) = self.name_tree("EmbeddedFiles")? {
            let root_path = self.embedded_files_root_path()?;
            for entry in tree.located_entries()? {
                let at = match entry.node.as_ref() {
                    Some(node) => Some(DirectPath::new(
                        Some(node),
                        vec![PathSegment::Key("Names"), PathSegment::Index(entry.index)],
                    )),
                    None if &entry.node == tree.root() => root_path.clone().map(|mut p| {
                        p.segments
                            .extend([PathSegment::Key("Names"), PathSegment::Index(entry.index)]);
                        p
                    }),
                    None => None,
                };
                let key = Some(laminapdf_syntax::text::decode_text_string(&entry.key));
                list.hold(self, &entry.value, at, AttachmentHolder::EmbeddedFiles, key)?;
            }
        }
        let catalog = self.catalog()?;
        let catalog_path = match self.trailer().get("Root") {
            Some(Object::Ref(r)) => {
                seen.insert(r.num);
                DirectPath::new(Some(*r), Vec::new())
            }
            _ => DirectPath::new(None, vec![PathSegment::Key("Root")]),
        };
        self.hold_associated(list, &catalog, catalog_path, &AttachmentHolder::Document)?;
        let pages = self.pages()?;
        for page in pages.iter() {
            let page_path = page.obj_ref().map(|r| DirectPath::new(Some(r), Vec::new()));
            if let Some(path) = page_path.clone() {
                seen.extend(path.anchor.map(|r| r.num));
                self.hold_associated(list, page.dict(), path, &AttachmentHolder::Page(page.id().clone()))?;
            }
            let mut annots: Vec<(AnnotId, DirectPath, Dict)> = Vec::new();
            self.each_annotation(&page, |id, index, dict| {
                annots.push((id.clone(), annotation_path(&page, &id, index), dict.clone()));
                Ok(())
            })?;
            for (annotation, anchor, dict) in annots {
                let holder = AttachmentHolder::Annotation {
                    page: page.id().clone(),
                    annotation: annotation.clone(),
                };
                if anchor.segments.is_empty() {
                    seen.extend(anchor.anchor.map(|r| r.num));
                }
                self.hold_associated(list, &dict, anchor.clone(), &holder)?;
                let subtype = AnnotSubtype::from_name(&read_name(self, dict.get("Subtype"))?.unwrap_or_default());
                if subtype == AnnotSubtype::FileAttachment
                    && let Some(fs) = dict.get("FS")
                {
                    let mut at = anchor.clone();
                    at.segments.push(PathSegment::Key("FS"));
                    let holder = AttachmentHolder::FileAttachment {
                        page: page.id().clone(),
                        annotation,
                    };
                    list.hold(self, fs, Some(at), holder, None)?;
                }
            }
        }
        Ok(seen)
    }

    /// Marks the files of `owner`'s `/AF` array as held by `holder`. `owner_path` is where
    /// `owner` is, for the path IDs of direct file specifications in the array.
    fn hold_associated(
        &self,
        list: &mut List,
        owner: &Dict,
        owner_path: DirectPath,
        holder: &AttachmentHolder,
    ) -> Result<()> {
        let Some(af) = owner.get("AF") else { return Ok(()) };
        let Some(items) = self.read(af)? else { return Ok(()) };
        let base = if let Some(array) = af.as_ref() {
            DirectPath::new(Some(array), Vec::new())
        } else {
            let mut p = owner_path;
            p.segments.push(PathSegment::Key("AF"));
            p
        };
        for (i, item) in items.as_array().unwrap_or_default().iter().enumerate() {
            let mut at = base.clone();
            at.segments.push(PathSegment::Index(i));
            list.hold(self, item, Some(at), holder.clone(), None)?;
        }
        Ok(())
    }

    /// Where the `/EmbeddedFiles` tree's root dictionary is when it is direct: in the `/Names`
    /// dictionary's object, or in the catalog when `/Names` is direct too.
    fn embedded_files_root_path(&self) -> Result<Option<DirectPath>> {
        let catalog = self.catalog()?;
        Ok(match catalog.get("Names") {
            Some(Object::Ref(r)) => Some(DirectPath::new(Some(*r), vec![PathSegment::Key("EmbeddedFiles")])),
            Some(Object::Dict(_)) => match self.trailer().get("Root") {
                Some(Object::Ref(root)) => Some(DirectPath::new(
                    Some(*root),
                    vec![PathSegment::Key("Names"), PathSegment::Key("EmbeddedFiles")],
                )),
                _ => None,
            },
            _ => None,
        })
    }
}

/// Where the annotation `id` at `index` of `page`'s `/Annots` is: its own object, else its place
/// in the array (a direct annotation).
fn annotation_path(page: &crate::Page<'_>, id: &AnnotId, index: usize) -> DirectPath {
    if let Some(r) = id.as_lamina_id().obj_ref() {
        return DirectPath::new(Some(r), Vec::new());
    }
    crate::annot::direct_path(page, page.dict().get("Annots").and_then(Object::as_ref), index)
}

impl Document {
    /// Where the annotation `id` is stored: its own object, or, for a direct annotation, its place
    /// in its page's `/Annots`; `None` when no page lists it.
    ///
    /// # Errors
    /// As [`Document::attachments`].
    ///
    /// Added in 0.6.0.
    pub fn annotation_at(&self, id: &AnnotId) -> Result<Option<DirectPath>> {
        if let Some(r) = id.as_lamina_id().obj_ref() {
            return Ok(Some(DirectPath::new(Some(r), Vec::new())));
        }
        for page in self.pages()?.iter() {
            let mut at = None;
            self.each_annotation(&page, |found, index, _| {
                if &found == id {
                    at = Some(annotation_path(&page, &found, index));
                }
                Ok(())
            })?;
            if at.is_some() {
                return Ok(at);
            }
        }
        Ok(None)
    }
}

/// The file specifications found so far, each once.
#[derive(Default)]
struct List {
    found: Vec<Found>,
    by_object: BTreeMap<u32, usize>,
}

impl List {
    /// Adds the file specification object `r` (`spec`) with no holder, unless it is listed.
    fn add_unheld(&mut self, doc: &Document, r: ObjRef, spec: &Dict) -> Result<()> {
        if self.by_object.contains_key(&r.num) {
            return Ok(());
        }
        let attachment = read_spec(doc, AttachmentId::object(r), spec)?;
        self.by_object.insert(r.num, self.found.len());
        self.found.push(Found {
            attachment,
            at: FileSpecAt::Object(r),
            spec: spec.clone(),
        });
        Ok(())
    }

    /// Adds `holder` to the file specification `value` names (a reference, or a direct
    /// dictionary at `at`), reading it the first time.
    fn hold(
        &mut self,
        doc: &Document,
        value: &Object,
        at: Option<DirectPath>,
        holder: AttachmentHolder,
        key: Option<String>,
    ) -> Result<()> {
        if let Some(r) = value.as_ref()
            && let Some(&i) = self.by_object.get(&r.num)
        {
            if let Some(found) = self.found.get_mut(i) {
                found.attachment.holders.push(holder);
                if found.attachment.key.is_none() {
                    found.attachment.key = key;
                }
            }
            return Ok(());
        }
        let Some(spec) = doc.read(value)? else { return Ok(()) };
        let Some(spec) = spec.as_dict() else { return Ok(()) };
        let (id, at) = match (value.as_ref(), at) {
            (Some(r), _) => (AttachmentId::object(r), FileSpecAt::Object(r)),
            (None, Some(path)) => (AttachmentId::path(&path), FileSpecAt::Direct(path)),
            (None, None) => return Ok(()),
        };
        let mut attachment = read_spec(doc, id, spec)?;
        attachment.key = key;
        attachment.holders.push(holder);
        if let Some(r) = value.as_ref() {
            self.by_object.insert(r.num, self.found.len());
        }
        self.found.push(Found {
            attachment,
            at,
            spec: spec.clone(),
        });
        Ok(())
    }
}

/// The facts of the file specification `spec`.
fn read_spec(doc: &Document, id: AttachmentId, spec: &Dict) -> Result<AttachedFile> {
    let relationship = read_name(doc, spec.get("AFRelationship"))?.and_then(|n| AfRelationship::from_name(&n));
    let mut out = AttachedFile {
        id,
        key: None,
        name: file_spec_name(doc, spec)?,
        description: doc.text_entry(spec, "Desc")?,
        mime: None,
        size: None,
        created: None,
        modified: None,
        checksum: None,
        relationship,
        embedded: false,
        holders: Vec::new(),
    };
    let Some(Object::Stream(file)) = embedded_stream(doc, spec)?.as_deref().cloned() else {
        return Ok(out);
    };
    out.embedded = true;
    out.mime = read_name(doc, file.dict.get("Subtype"))?.map(|n| String::from_utf8_lossy(&n).into_owned());
    let params = match file.dict.get("Params") {
        Some(p) => doc.read(p)?,
        None => None,
    };
    if let Some(params) = params.as_deref().and_then(Object::as_dict) {
        out.size = match params.get("Size") {
            Some(s) => doc
                .read(s)?
                .and_then(|s| s.as_int())
                .and_then(|s| u64::try_from(s).ok()),
            None => None,
        };
        out.created = date(doc, params.get("CreationDate"))?;
        out.modified = date(doc, params.get("ModDate"))?;
        out.checksum = match params.get("CheckSum") {
            Some(c) => doc.read(c)?.and_then(|c| match &*c {
                Object::String(s) => <[u8; 16]>::try_from(s.bytes.as_slice()).ok(),
                _ => None,
            }),
            None => None,
        };
    }
    Ok(out)
}

/// The MD5 digest of `bytes`, as `/Params /CheckSum` records it (§7.11.4.2): what the writer
/// records and what reading the bytes back compares.
pub(crate) fn md5(bytes: &[u8]) -> [u8; 16] {
    use md5::{Digest as _, Md5};
    Md5::digest(bytes).into()
}

/// The embedded file stream of `spec`: `/EF /UF`, else `/EF /F`.
pub(crate) fn embedded_stream(doc: &Document, spec: &Dict) -> Result<Option<std::sync::Arc<Object>>> {
    let Some(ef) = spec.get("EF") else { return Ok(None) };
    let Some(ef) = doc.read(ef)? else { return Ok(None) };
    let Some(ef) = ef.as_dict() else { return Ok(None) };
    match ef.get("UF").or_else(|| ef.get("F")) {
        Some(f) => doc.read(f),
        None => Ok(None),
    }
}

/// A date entry, when it parses.
fn date(doc: &Document, value: Option<&Object>) -> Result<Option<PdfDate>> {
    let Some(value) = value else { return Ok(None) };
    Ok(doc
        .read(value)?
        .as_deref()
        .and_then(text_of)
        .and_then(|t| t.parse().ok()))
}
