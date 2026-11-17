//! The files a document carries, across the boundary: `WasmDocument.attachments()`,
//! `attachmentBytes(id)`, `collection()`, and `addBlob(bytes)`, which keeps the bytes the
//! `attachFile` op names. Every module has them: an attachment is part of the document, like its
//! metadata.

use js_sys::{Object, Reflect, Uint8Array};
use laminapdf::{
    AfRelationship, AttachedFile, AttachmentHolder, Checksum, Collection, CollectionFieldKind, CollectionValue,
    CollectionView,
};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// One attached file.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentJs {
    /// Its ID (`e12.0`), which the attachment edits take.
    pub id: String,
    /// Its key in the document's list, when the list has it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub key: Option<String>,
    /// The file's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub name: Option<String>,
    /// Its description.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub description: Option<String>,
    /// Its MIME type.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub mime: Option<String>,
    /// Its size in bytes, as the file states it.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub size: Option<u64>,
    /// When it was made, as a PDF date.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub created: Option<String>,
    /// When it last changed, as a PDF date.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub modified: Option<String>,
    /// Whether the file states an MD5 checksum, which `attachmentBytes` compares.
    pub has_checksum: bool,
    /// How it relates to what it is associated with.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(
        optional,
        type = "\"source\" | \"data\" | \"alternative\" | \"supplement\" | \"encryptedPayload\" | \"formData\" | \"schema\" | \"unspecified\""
    )]
    pub relationship: Option<&'static str>,
    /// Whether its bytes are in the document.
    pub embedded: bool,
    /// What holds it.
    pub holders: Vec<HolderJs>,
}

/// What holds an attached file: the document's list, the document, a page, an annotation
/// (`/AF`) or a file attachment annotation, with the page's 0-based index and the annotation's ID.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HolderJs {
    /// `embeddedFiles`, `document`, `page`, `annotation` or `fileAttachment`.
    #[tsify(type = "\"embeddedFiles\" | \"document\" | \"page\" | \"annotation\" | \"fileAttachment\"")]
    pub kind: &'static str,
    /// The page, for every kind but the list and the document.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub page: Option<u32>,
    /// The annotation, for `annotation` and `fileAttachment`.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub annotation: Option<String>,
}

/// A portfolio (`/Collection`).
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollectionJs {
    /// The columns, in order.
    pub schema: Vec<CollectionFieldJs>,
    /// The keys the files are sorted by, each with whether it is ascending.
    pub sort: Vec<(String, bool)>,
    /// How it asks to be shown.
    #[tsify(type = "\"details\" | \"tile\" | \"hidden\" | \"custom\"")]
    pub view: &'static str,
    /// The key of the file to show first.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub initial: Option<String>,
    /// The files, in the list's order.
    pub files: Vec<CollectionFileJs>,
}

/// One column of a portfolio's schema.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollectionFieldJs {
    /// Its key, which the files' values use.
    pub key: String,
    /// Its heading.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[tsify(optional)]
    pub name: Option<String>,
    /// What it shows.
    #[tsify(
        type = "\"text\" | \"date\" | \"number\" | \"fileName\" | \"description\" | \"modified\" | \"created\" | \"size\" | \"compressedSize\" | \"other\""
    )]
    pub kind: &'static str,
    /// Whether it is shown.
    pub visible: bool,
    /// Whether a viewer may let the user edit it.
    pub editable: bool,
}

/// One file of a portfolio, with its values.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollectionFileJs {
    /// The attachment's ID.
    pub attachment: String,
    /// Its key in the list.
    pub key: String,
    /// Its values by column key: text, or a number.
    #[tsify(type = "Record<string, string | number>")]
    pub values: serde_json::Map<String, serde_json::Value>,
}

/// The wire name of a relationship, as the attachment ops take it.
const fn relationship(r: AfRelationship) -> &'static str {
    match r {
        AfRelationship::Source => "source",
        AfRelationship::Data => "data",
        AfRelationship::Alternative => "alternative",
        AfRelationship::Supplement => "supplement",
        AfRelationship::EncryptedPayload => "encryptedPayload",
        AfRelationship::FormData => "formData",
        AfRelationship::Schema => "schema",
        _ => "unspecified",
    }
}

fn attachment(doc: &laminapdf::Document, a: AttachedFile) -> JsResult<AttachmentJs> {
    let page = |id: &laminapdf::PageId| -> JsResult<Option<u32>> {
        let pages = doc.page_infos().map_err(|e| lamina_error(&e))?;
        Ok(pages
            .iter()
            .position(|p| &p.id == id)
            .and_then(|i| u32::try_from(i).ok()))
    };
    let mut holders = Vec::with_capacity(a.holders.len());
    for holder in &a.holders {
        holders.push(match holder {
            AttachmentHolder::Document => HolderJs {
                kind: "document",
                page: None,
                annotation: None,
            },
            AttachmentHolder::Page(id) => HolderJs {
                kind: "page",
                page: page(id)?,
                annotation: None,
            },
            AttachmentHolder::Annotation { page: id, annotation } => HolderJs {
                kind: "annotation",
                page: page(id)?,
                annotation: Some(annotation.to_string()),
            },
            AttachmentHolder::FileAttachment { page: id, annotation } => HolderJs {
                kind: "fileAttachment",
                page: page(id)?,
                annotation: Some(annotation.to_string()),
            },
            _ => HolderJs {
                kind: "embeddedFiles",
                page: None,
                annotation: None,
            },
        });
    }
    Ok(AttachmentJs {
        id: a.id.to_string(),
        key: a.key,
        name: a.name,
        description: a.description,
        mime: a.mime,
        size: a.size,
        created: a.created.map(|d| d.to_string()),
        modified: a.modified.map(|d| d.to_string()),
        has_checksum: a.checksum.is_some(),
        relationship: a.relationship.map(relationship),
        embedded: a.embedded,
        holders,
    })
}

impl From<Collection> for CollectionJs {
    fn from(c: Collection) -> Self {
        Self {
            schema: c
                .schema
                .into_iter()
                .map(|f| CollectionFieldJs {
                    key: f.key,
                    name: f.name,
                    kind: match f.kind {
                        CollectionFieldKind::Text => "text",
                        CollectionFieldKind::Date => "date",
                        CollectionFieldKind::Number => "number",
                        CollectionFieldKind::FileName => "fileName",
                        CollectionFieldKind::Description => "description",
                        CollectionFieldKind::Modified => "modified",
                        CollectionFieldKind::Created => "created",
                        CollectionFieldKind::Size => "size",
                        CollectionFieldKind::CompressedSize => "compressedSize",
                        _ => "other",
                    },
                    visible: f.visible,
                    editable: f.editable,
                })
                .collect(),
            sort: c.sort,
            view: match c.view {
                CollectionView::Tile => "tile",
                CollectionView::Hidden => "hidden",
                CollectionView::Custom => "custom",
                _ => "details",
            },
            initial: c.initial,
            files: c
                .files
                .into_iter()
                .map(|f| CollectionFileJs {
                    attachment: f.attachment.to_string(),
                    key: f.key,
                    values: f
                        .values
                        .into_iter()
                        .map(|(k, v)| {
                            let v = match v {
                                CollectionValue::Number(n) => serde_json::json!(n),
                                CollectionValue::Text(t) => serde_json::json!(t),
                                _ => serde_json::Value::Null,
                            };
                            (k, v)
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// Every file the document carries, each once with what holds it.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "AttachmentJs[]")]
    pub fn attachments(&self) -> JsResult<JsValue> {
        let doc = self.shared().borrow();
        let list = doc.attachments().map_err(|e| lamina_error(&e))?;
        let out = list
            .into_iter()
            .map(|a| attachment(&doc, a))
            .collect::<JsResult<Vec<_>>>()?;
        Ok(to_js(&out))
    }

    /// The bytes of the attached file `id`, exactly as attached, and how they compare with the
    /// checksum the file states: `{ bytes, checksum }`.
    ///
    /// # Errors
    /// `invalid-argument` for an ID that isn't an attachment's; `unknown-attachment`;
    /// `limit-exceeded` for a file over the document's decode limit; decoding errors.
    #[wasm_bindgen(
        js_name = attachmentBytes,
        unchecked_return_type = "{ bytes: Uint8Array<ArrayBuffer>; checksum: \"absent\" | \"matches\" | \"mismatch\" }"
    )]
    pub fn attachment_bytes(&self, id: &str) -> JsResult<JsValue> {
        let id = id
            .parse()
            .map_err(|_| crate::error::invalid_argument("id", "not an attachment ID".to_owned(), Vec::new()))?;
        let file = self
            .shared()
            .borrow()
            .attachment_bytes(&id)
            .map_err(|e| lamina_error(&e))?;
        let out = Object::new();
        Reflect::set(&out, &"bytes".into(), &Uint8Array::from(file.bytes.as_slice()))?;
        let checksum = match file.checksum {
            Checksum::Matches => "matches",
            Checksum::Mismatch => "mismatch",
            _ => "absent",
        };
        Reflect::set(&out, &"checksum".into(), &checksum.into())?;
        Ok(out.into())
    }

    /// The portfolio the document is, or `undefined` when it isn't one.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "CollectionJs | undefined")]
    pub fn collection(&self) -> JsResult<JsValue> {
        let collection = self.shared().borrow().collection().map_err(|e| lamina_error(&e))?;
        Ok(collection.map_or(JsValue::UNDEFINED, |c| to_js(&CollectionJs::from(c))))
    }

    /// Keeps `bytes` for the session (an attached file, a replacement image's JPEG or PNG) and
    /// returns their ID, 64 hex digits, which the `attachFile` op and a `replaceImage` edit name
    /// them by.
    #[wasm_bindgen(js_name = addBlob)]
    pub fn add_blob(&mut self, bytes: Vec<u8>) -> String {
        self.shared().borrow_mut().add_blob(bytes).to_string()
    }
}
