//! The attachment ops (ISO 32000-2 §7.11, §14.13): [`AttachFile`] embeds a file and lists it in
//! the `/EmbeddedFiles` tree, and in the `/AF` (associated files) array of the document, a page
//! or an annotation when it is associated with one; [`UpdateAttachment`] renames, describes or
//! retypes it; [`DeleteAttachment`] takes it out of the tree and every `/AF` array. All three
//! are class `Document`.
//!
//! The file's bytes come from the session's blob store ([`Document::add_blob`](crate::Document::add_blob)),
//! and its dates come in the op, never from a clock. `laminapdf-model` writes the embedded
//! file and its specification, as it does for the builder's attachments, with the MD5
//! `/CheckSum`. No op raises the file's `/Version`: an `/AF` array in a PDF 1.7 file is a
//! second-class entry readers skip.

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_model::{
    AfRelationship, AnnotId, AttachmentHolder, AttachmentId, EmbeddedFileSpec, FileSpecAt, PageId, PdfDate, TreeWriter,
    set_at_path, set_catalog_entry, set_names_entry, write_embedded_file,
};
use laminapdf_syntax::text::encode_text_string;
use serde::{Deserialize, Serialize};

use crate::annot::{Found, every_annotation, object_dict};
use crate::blob::{BlobId, BlobStore};
use crate::error::{Error, Result};
use crate::op::Effect;
use crate::outline::target::page_ref;
use crate::wire;

/// What an attached file is associated with, besides being listed in the `/EmbeddedFiles` tree,
/// which every attached file is.
///
/// On the wire: `"embeddedFiles"` (the list only), `"document"`, `{ "page": "p3.0" }` or
/// `{ "annotation": "a8.0" }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub enum AttachTarget {
    /// Nothing: the file is only in the document's list of attachments.
    #[default]
    EmbeddedFiles,
    /// The whole document: the catalog's `/AF` (§14.13.3), as an e-invoice's XML is.
    Document,
    /// A page's `/AF` (§14.13.4).
    Page(
        #[serde(with = "wire::page_id")]
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        PageId,
    ),
    /// An annotation's `/AF` (§14.13.7).
    Annotation(
        #[serde(with = "wire::annot_id")]
        #[cfg_attr(feature = "tsify", tsify(type = "string"))]
        AnnotId,
    ),
}

/// Attaches a file: its bytes, named by `file` in the session's blob store, become an embedded
/// file stream with `/Params` (`/Size`, the dates given and an MD5 `/CheckSum`), described by a
/// file specification with `/F` and `/UF` set to `name`, `/Desc` and `/AFRelationship`. The
/// file is listed in the `/EmbeddedFiles` tree under `name` (or `name (2)`, `name (3)`, … when
/// another file has that key), and in the `/AF` array of what `attach_to` names.
///
/// The new attachment's ID is in [`Applied::created_attachments`](crate::Applied::created_attachments).
/// On a file that claims PDF/A-1 or PDF/A-2, the save planner warns `pdfa-attachment-forbidden`;
/// on PDF/A-3, `pdfa-attachment-incomplete` unless the file has an `/AF` target and a
/// `modified` date (ISO 19005-3 §6.8). Neither stops the op.
///
/// An empty name is `invalid-op`; bytes the session doesn't hold are `unknown-blob`.
///
/// On the wire: `{ "op": "attachFile", "args": { "name": "factur-x.xml", "file": "<64 hex
/// digits>", "mime": "text/xml", "modified": "D:20261117120000Z", "attachTo": "document",
/// "relationship": "alternative" } }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct AttachFile {
    /// The file's name.
    pub name: String,
    /// The file's bytes, by their ID in the blob store.
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub file: BlobId,
    /// Its MIME type, such as `text/xml` or `application/pdf`; empty for none.
    pub mime: String,
    /// `/Desc`: a description viewers show beside the file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub description: Option<String>,
    /// When the file was made (`/Params /CreationDate`), as a PDF date.
    #[serde(default, with = "wire::opt_date", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub created: Option<PdfDate>,
    /// When it last changed (`/Params /ModDate`), as a PDF date.
    #[serde(default, with = "wire::opt_date", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string"))]
    pub modified: Option<PdfDate>,
    /// What it is associated with.
    #[serde(default)]
    pub attach_to: AttachTarget,
    /// How it relates to what it is associated with (default `unspecified`).
    #[serde(default, with = "wire::af_relationship")]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "\"source\" | \"data\" | \"alternative\" | \"supplement\" | \"encryptedPayload\" | \"formData\" | \"schema\" | \"unspecified\""
        )
    )]
    pub relationship: AfRelationship,
}

impl AttachFile {
    /// The op that attaches the bytes `file` as `name`, of type `mime`, to the document's list
    /// only, with no description or dates and an unspecified relationship.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn new(name: impl Into<String>, file: BlobId, mime: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            file,
            mime: mime.into(),
            description: None,
            created: None,
            modified: None,
            attach_to: AttachTarget::EmbeddedFiles,
            relationship: AfRelationship::Unspecified,
        }
    }

    /// The same op, associating the file with `target` as `relationship`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub fn with_target(mut self, target: AttachTarget, relationship: AfRelationship) -> Self {
        self.attach_to = target;
        self.relationship = relationship;
        self
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document, blobs: &BlobStore) -> Result<Effect> {
        let name = non_empty(&self.name)?;
        let bytes = blobs.get(&self.file).ok_or(Error::UnknownBlob(self.file))?;
        // Find what the file is associated with before writing anything.
        let owner = match &self.attach_to {
            AttachTarget::EmbeddedFiles => None,
            AttachTarget::Document => Some(Owner::Catalog),
            AttachTarget::Page(page) => Some(Owner::Object(page_ref(doc, page)?)),
            AttachTarget::Annotation(annot) => {
                Some(Owner::Object(Found::locate(&every_annotation(doc)?, annot)?.object))
            }
        };
        let mut spec = EmbeddedFileSpec::new(name, bytes, &self.mime);
        spec.description = self.description.as_deref();
        spec.created = self.created.as_ref();
        spec.modified = self.modified.as_ref();
        spec.relationship = self.relationship;
        let spec = write_embedded_file(&mut doc.cos_mut().edit(), &spec)?;
        let mut tree = Tree::read(doc)?;
        let key = tree.free_key(name);
        tree.entries.insert(key, Object::Ref(spec));
        tree.write(doc)?;
        if let Some(owner) = owner {
            owner.edit_af(doc, |items| items.push(Object::Ref(spec)))?;
        }
        Ok(Effect {
            created_attachments: vec![AttachmentId::object(spec)],
            ..Effect::default()
        })
    }
}

/// Changes an attachment's name (its `/F` and `/UF`, and its key in the `/EmbeddedFiles`
/// tree), its description (`null` removes it), its MIME type or its relationship. An absent
/// field is left alone, and so are the file's bytes and dates. A new name taken by another file
/// in the tree gets a free key (`name (2)`, …), as [`AttachFile`]'s does.
///
/// An attachment the document doesn't hold is `unknown-attachment`; an empty name is
/// `invalid-op`.
///
/// On the wire: `{ "op": "updateAttachment", "args": { "attachment": "e12.0", "description":
/// "The invoice as XML", "relationship": "alternative" } }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct UpdateAttachment {
    /// The attachment.
    #[serde(with = "wire::attachment_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub attachment: AttachmentId,
    /// Its new name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub name: Option<String>,
    /// Its new description; `null` removes it.
    #[serde(
        default,
        deserialize_with = "wire::double_option::deserialize",
        skip_serializing_if = "Option::is_none"
    )]
    #[cfg_attr(feature = "tsify", tsify(optional, type = "string | null"))]
    pub description: Option<Option<String>>,
    /// Its new MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "tsify", tsify(optional))]
    pub mime: Option<String>,
    /// Its new relationship.
    #[serde(default, with = "wire::opt_af_relationship", skip_serializing_if = "Option::is_none")]
    #[cfg_attr(
        feature = "tsify",
        tsify(
            optional,
            type = "\"source\" | \"data\" | \"alternative\" | \"supplement\" | \"encryptedPayload\" | \"formData\" | \"schema\" | \"unspecified\""
        )
    )]
    pub relationship: Option<AfRelationship>,
}

impl UpdateAttachment {
    /// The op that changes nothing of `attachment` yet: set the fields to change.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(attachment: AttachmentId) -> Self {
        Self {
            attachment,
            name: None,
            description: None,
            mime: None,
            relationship: None,
        }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let (at, mut spec) = doc
            .file_spec_at(&self.attachment)?
            .ok_or_else(|| Error::UnknownAttachment(self.attachment.clone()))?;
        let text = |s: &str| Object::String(PdfString::literal(encode_text_string(s)));
        let name = self.name.as_deref().map(non_empty).transpose()?;
        if let Some(name) = name {
            spec.insert("F", text(name));
            spec.insert("UF", text(name));
        }
        match &self.description {
            Some(Some(desc)) => spec.insert("Desc", text(desc)),
            Some(None) => {
                spec.remove("Desc");
            }
            None => {}
        }
        if let Some(relationship) = self.relationship {
            spec.insert("AFRelationship", Object::name(relationship.as_str()));
        }
        if let Some(mime) = &self.mime {
            set_mime(doc, &spec, mime)?;
        }
        let mut rewritten = false;
        if let Some(name) = name {
            let mut tree = Tree::read(doc)?;
            if let Some(old) = tree.key_of(doc, &self.attachment)? {
                tree.entries.remove(&old);
                let key = tree.free_key(name);
                tree.entries.insert(key, spec_value(&at, &spec));
                tree.write(doc)?;
                rewritten = true;
            }
        }
        // A direct specification in the tree went into the new tree with its changes; the old
        // tree, where its path leads, is no longer the document's.
        if !(rewritten && matches!(at, FileSpecAt::Direct(_))) {
            write_spec(doc, &at, spec)?;
        }
        Ok(Effect::default())
    }
}

/// Deletes an attachment: takes its file specification out of the `/EmbeddedFiles` tree and
/// out of every `/AF` array that names it. A `FileAttachment` annotation that holds the same
/// file keeps it: removing the annotation is an annotation edit, so the op reports it in
/// [`Applied::attachments_held`](crate::Applied::attachments_held). An incremental save keeps
/// the file's bytes in the earlier revision ([`Applied::still_in_earlier_revisions`](crate::Applied::still_in_earlier_revisions)).
///
/// An attachment the document doesn't hold is `unknown-attachment`.
///
/// On the wire: `{ "op": "deleteAttachment", "args": { "attachment": "e12.0" } }`.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[non_exhaustive]
pub struct DeleteAttachment {
    /// The attachment.
    #[serde(with = "wire::attachment_id")]
    #[cfg_attr(feature = "tsify", tsify(type = "string"))]
    pub attachment: AttachmentId,
}

impl DeleteAttachment {
    /// The op that deletes `attachment`.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn new(attachment: AttachmentId) -> Self {
        Self { attachment }
    }

    pub(crate) fn apply_to(&self, doc: &mut laminapdf_model::Document) -> Result<Effect> {
        let attachment = doc
            .attachment(&self.attachment)?
            .ok_or_else(|| Error::UnknownAttachment(self.attachment.clone()))?;
        let (at, spec) = doc
            .file_spec_at(&self.attachment)?
            .ok_or_else(|| Error::UnknownAttachment(self.attachment.clone()))?;
        let value = spec_value(&at, &spec);
        let mut tree = Tree::read(doc)?;
        if let Some(key) = tree.key_of(doc, &self.attachment)? {
            tree.entries.remove(&key);
            tree.write(doc)?;
        }
        let mut held = Vec::new();
        let mut owners: Vec<Owner> = Vec::new();
        for holder in &attachment.holders {
            let owner = match holder {
                AttachmentHolder::Document => Owner::Catalog,
                AttachmentHolder::Page(page) => Owner::Object(page_ref(doc, page)?),
                AttachmentHolder::Annotation { annotation, .. } => match annotation.as_lamina_id().obj_ref() {
                    Some(r) => Owner::Object(r),
                    None => continue,
                },
                AttachmentHolder::FileAttachment { annotation, .. } => {
                    held.push(AttachmentHeld {
                        attachment: self.attachment.clone(),
                        annotation: annotation.clone(),
                    });
                    continue;
                }
                _ => continue,
            };
            if !owners.contains(&owner) {
                owners.push(owner);
            }
        }
        for owner in owners {
            owner.edit_af(doc, |items| items.retain(|item| !same_spec(item, &value)))?;
        }
        Ok(Effect {
            removed: true,
            attachments_held: held,
            ..Effect::default()
        })
    }
}

/// A file [`DeleteAttachment`] took out of the document's list and `/AF` arrays that a
/// `FileAttachment` annotation still holds (`attachment-still-held`): the file stays in the
/// document until the annotation is deleted too.
///
/// Added in 0.6.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct AttachmentHeld {
    /// The deleted attachment.
    pub attachment: AttachmentId,
    /// The annotation that still holds it.
    pub annotation: AnnotId,
}

/// The usage right an attachment op needs: the name a UR3 signature's `/EF` array must list for
/// the op to stay within the rights it grants (ISO 32000-2 §12.8.2.3, Table 260).
///
/// Added in 0.6.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum EmbeddedFileRight {
    /// `/Create`: attaching a file.
    Create,
    /// `/Modify`: changing one.
    Modify,
    /// `/Delete`: deleting one.
    Delete,
}

impl EmbeddedFileRight {
    /// The right that undoing a change of this kind needs: undoing an attachment deletes it,
    /// and undoing a deletion attaches the file again.
    ///
    /// Added in 0.6.0.
    #[must_use]
    pub const fn reverted(self) -> Self {
        match self {
            Self::Create => Self::Delete,
            Self::Modify => Self::Modify,
            Self::Delete => Self::Create,
        }
    }
}

/// What holds an `/AF` array the ops change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Catalog,
    Object(ObjRef),
}

impl Owner {
    /// Edits the owner's `/AF` array with `f`: in the array's own object when it has one, else
    /// in the owner; an array left empty is removed.
    fn edit_af(self, doc: &mut laminapdf_model::Document, f: impl FnOnce(&mut Vec<Object>)) -> Result<()> {
        let mut dict = match self {
            Self::Catalog => doc.catalog()?,
            Self::Object(r) => object_dict(doc, r)?.unwrap_or_default(),
        };
        let mut cos = doc.cos_mut().edit();
        if let Some(Object::Ref(array)) = dict.get("AF").cloned() {
            let mut items = cos
                .get(array)?
                .and_then(|o| o.as_array().map(<[Object]>::to_vec))
                .unwrap_or_default();
            f(&mut items);
            return Ok(cos.set(array, Object::Array(items))?);
        }
        let mut items = match dict.get("AF") {
            Some(Object::Array(items)) => items.clone(),
            _ => Vec::new(),
        };
        f(&mut items);
        let value = (!items.is_empty()).then_some(Object::Array(items));
        match self {
            Self::Catalog => Ok(set_catalog_entry(&mut cos, "AF", value)?),
            Self::Object(r) => {
                match value {
                    Some(v) => dict.insert("AF", v),
                    None => {
                        dict.remove("AF");
                    }
                }
                Ok(cos.set(r, Object::Dict(dict))?)
            }
        }
    }
}

/// The `/EmbeddedFiles` tree's entries, read whole and written whole (as the named destinations
/// are, through [`TreeWriter`]).
struct Tree {
    entries: BTreeMap<Vec<u8>, Object>,
}

impl Tree {
    fn read(doc: &laminapdf_model::Document) -> Result<Self> {
        let entries = match doc.name_tree("EmbeddedFiles")? {
            Some(tree) => tree.entries()?.into_iter().collect(),
            None => BTreeMap::new(),
        };
        Ok(Self { entries })
    }

    /// `name` as a key no entry has: itself, else `name (2)`, `name (3)`, ….
    fn free_key(&self, name: &str) -> Vec<u8> {
        let key = |s: &str| encode_text_string(s);
        let first = key(name);
        if !self.entries.contains_key(&first) {
            return first;
        }
        // Among one more name than there are entries, one is free.
        let last = u64::try_from(self.entries.len()).unwrap_or(u64::MAX).saturating_add(2);
        (2..=last)
            .map(|n| key(&format!("{name} ({n})")))
            .find(|k| !self.entries.contains_key(k))
            .unwrap_or(first)
    }

    /// The key the tree lists `id` under.
    fn key_of(&self, doc: &laminapdf_model::Document, id: &AttachmentId) -> Result<Option<Vec<u8>>> {
        if let Some(r) = id.as_lamina_id().obj_ref() {
            return Ok(self
                .entries
                .iter()
                .find(|(_, v)| v.as_ref() == Some(r))
                .map(|(k, _)| k.clone()));
        }
        // A direct specification: the tree lists it under the key the listing read.
        Ok(doc
            .attachment(id)?
            .and_then(|a| a.key)
            .map(|k| encode_text_string(&k))
            .filter(|k| self.entries.contains_key(k)))
    }

    fn write(&self, doc: &mut laminapdf_model::Document) -> Result<()> {
        let mut cos = doc.cos_mut().edit();
        let root = if self.entries.is_empty() {
            None
        } else {
            Some(Object::Ref(TreeWriter::new().write_names(&mut cos, &self.entries)?))
        };
        Ok(set_names_entry(&mut cos, "EmbeddedFiles", root)?)
    }
}

/// The value that names a file specification in a tree or an array: its reference, or the
/// dictionary itself when it is direct.
fn spec_value(at: &FileSpecAt, spec: &Dict) -> Object {
    match at {
        FileSpecAt::Object(r) => Object::Ref(*r),
        _ => Object::Dict(spec.clone()),
    }
}

/// Whether `item` names the same file specification as `value`.
fn same_spec(item: &Object, value: &Object) -> bool {
    match (item.as_ref(), value.as_ref()) {
        (Some(a), Some(b)) => a == b,
        (None, None) => item == value,
        _ => false,
    }
}

/// Writes `spec` back where it was read.
fn write_spec(doc: &mut laminapdf_model::Document, at: &FileSpecAt, spec: Dict) -> Result<()> {
    let mut cos = doc.cos_mut().edit();
    match at {
        FileSpecAt::Object(r) => Ok(cos.set(*r, Object::Dict(spec))?),
        FileSpecAt::Direct(path) => Ok(set_at_path(&mut cos, path, Object::Dict(spec))?),
        _ => Err(Error::InvalidOp("the attachment is stored where it can't be written")),
    }
}

/// Sets the MIME type (`/Subtype`) of `spec`'s embedded file stream: `/EF /UF` and `/EF /F`,
/// which name the same stream when Lamina PDF wrote them.
fn set_mime(doc: &mut laminapdf_model::Document, spec: &Dict, mime: &str) -> Result<()> {
    let ef = match spec.get("EF") {
        Some(Object::Ref(r)) => object_dict(doc, *r)?,
        Some(Object::Dict(d)) => Some(d.clone()),
        _ => None,
    }
    .unwrap_or_default();
    let mut streams: Vec<ObjRef> = ["UF", "F"]
        .iter()
        .filter_map(|k| ef.get(k).and_then(Object::as_ref))
        .collect();
    streams.dedup();
    let mut cos = doc.cos_mut().edit();
    for r in streams {
        let Some(current) = cos.get(r)? else { continue };
        let Object::Stream(mut stream) = (*current).clone() else {
            continue;
        };
        if mime.is_empty() {
            stream.dict.remove("Subtype");
        } else {
            stream.dict.insert("Subtype", Object::name(mime));
        }
        cos.set(r, Object::Stream(stream))?;
    }
    Ok(())
}

/// `name` when it isn't empty.
fn non_empty(name: &str) -> Result<&str> {
    if name.is_empty() {
        Err(Error::InvalidOp("an attachment needs a name"))
    } else {
        Ok(name)
    }
}
