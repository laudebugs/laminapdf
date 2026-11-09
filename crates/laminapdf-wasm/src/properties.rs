//! A document's properties across the boundary: `WasmDocument.properties()` (the extract and text
//! modules, so the base module stays in its size budget).

use laminapdf::{Cipher, DocumentProperties, FontEncoding, FontSummary};
use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::convert::to_js;
use crate::document::WasmDocument;
use crate::error::{JsResult, lamina_error};

/// A document's properties, for a properties view.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PropertiesJs {
    /// `/Title`.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// `/Author`.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// `/Subject`.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// `/Keywords`.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<String>,
    /// `/Creator`: the application the document was made with.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator: Option<String>,
    /// `/Producer`: the application that wrote the PDF.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub producer: Option<String>,
    /// `/CreationDate` as ISO 8601.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    /// `/ModDate` as ISO 8601.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified: Option<String>,
    /// The XMP packet as text.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xmp: Option<String>,
    /// The PDF version.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The number of pages.
    pub page_count: u32,
    /// The file's size in bytes.
    pub file_size: f64,
    /// The file is linearized ("fast web view").
    pub linearized: bool,
    /// The catalog says the document is tagged.
    pub tagged: bool,
    /// How the file is encrypted; absent when it isn't.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionJs>,
    /// The fonts the pages use.
    pub fonts: Vec<FontJs>,
    /// The signatures present, not verified.
    pub signatures: u32,
    /// The signature fields' full names.
    pub signature_fields: Vec<String>,
    /// What the document holds besides its pages.
    pub contains: ContainsJs,
}

/// What a document holds besides its pages.
#[derive(Debug, Serialize, Tsify)]
pub(crate) struct ContainsJs {
    /// Some action runs JavaScript.
    pub javascript: bool,
    /// The number of embedded files.
    pub attachments: u32,
    /// The document has a form with at least one field.
    pub forms: bool,
}

/// How a file is encrypted, and what its author permits.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EncryptionJs {
    /// `/V`.
    pub version: u8,
    /// `/R`.
    pub revision: u8,
    /// The key length in bits.
    pub key_bits: u16,
    /// The cipher streams use.
    pub cipher: CipherJs,
    /// What `/P` permits; absent when the handler gives no permissions.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionsJs>,
}

/// What a file's author permits: a request to readers, not a lock.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[expect(clippy::struct_excessive_bools, reason = "one flag per permission bit")]
pub(crate) struct PermissionsJs {
    /// Printing.
    pub print: bool,
    /// Printing at full quality.
    pub print_high_quality: bool,
    /// Changing the contents.
    pub modify: bool,
    /// Copying text and graphics.
    pub copy: bool,
    /// Adding or changing annotations.
    pub annotate: bool,
    /// Filling form fields.
    pub fill_forms: bool,
    /// Extracting text for accessibility.
    pub accessibility: bool,
    /// Inserting, rotating or deleting pages.
    pub assemble: bool,
}

/// One font the document uses.
#[derive(Debug, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FontJs {
    /// Its name without a subset tag.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `Type0`, `Type1`, `MMType1`, `Type3` or `TrueType`.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// The file holds the font.
    pub embedded: bool,
    /// The file holds only the glyphs used.
    pub subset: bool,
    /// Its encoding; absent when the font gives none.
    #[tsify(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<EncodingJs>,
}

/// The cipher a file's streams use (ISO 32000-2 §7.6): the one list, which the `.d.ts` gets as a union.
#[derive(Debug, Serialize, Tsify)]
pub(crate) enum CipherJs {
    /// RC4 (deprecated).
    #[serde(rename = "rc4")]
    Rc4,
    /// AES with a 128-bit key (deprecated in PDF 2.0).
    #[serde(rename = "aes-128")]
    Aes128,
    /// AES with a 256-bit key.
    #[serde(rename = "aes-256")]
    Aes256,
    /// No encryption of streams (`/Identity`).
    #[serde(rename = "identity")]
    Identity,
    /// A cipher this version doesn't name.
    #[serde(rename = "other")]
    Other,
}

impl From<Cipher> for CipherJs {
    fn from(cipher: Cipher) -> Self {
        match cipher {
            Cipher::Rc4 => Self::Rc4,
            Cipher::Aes128 => Self::Aes128,
            Cipher::Aes256 => Self::Aes256,
            Cipher::Identity => Self::Identity,
            // A cipher a later version adds still encrypts: it is never reported as Identity.
            _ => Self::Other,
        }
    }
}

/// A font's encoding: a named one (`WinAnsiEncoding`, `Identity-H`), or the font's own.
#[derive(Debug, Serialize, Tsify)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum EncodingJs {
    /// A named encoding or predefined CMap.
    Named {
        /// Its name.
        name: String,
    },
    /// An encoding dictionary or an embedded CMap.
    Custom,
}

impl From<&FontSummary> for FontJs {
    fn from(f: &FontSummary) -> Self {
        Self {
            name: f.name.clone(),
            kind: f.kind.clone(),
            embedded: f.embedded,
            subset: f.subset,
            encoding: f.encoding.as_ref().map(|e| match e {
                FontEncoding::Named(name) => EncodingJs::Named { name: name.clone() },
                // `Custom`, and any form a later version adds, is the font's own.
                _ => EncodingJs::Custom,
            }),
        }
    }
}

impl From<DocumentProperties> for PropertiesJs {
    fn from(p: DocumentProperties) -> Self {
        let encryption = p.encryption.map(|e| EncryptionJs {
            version: e.version,
            revision: e.revision,
            key_bits: e.key_bits,
            cipher: CipherJs::from(e.cipher),
            permissions: e.permissions.map(|p| PermissionsJs {
                print: p.print,
                print_high_quality: p.print_high_quality,
                modify: p.modify,
                copy: p.copy,
                annotate: p.annotate,
                fill_forms: p.fill_forms,
                accessibility: p.accessibility,
                assemble: p.assemble,
            }),
        });
        // A file past 2^53 bytes isn't one a reader opens; the size loses nothing as a number.
        #[expect(clippy::cast_precision_loss, reason = "file sizes stay below 2^53")]
        let file_size = p.file_size as f64;
        Self {
            title: p.info.title,
            author: p.info.author,
            subject: p.info.subject,
            keywords: p.info.keywords,
            creator: p.info.creator,
            producer: p.info.producer,
            created: p.created,
            modified: p.modified,
            xmp: p.xmp,
            version: p.version,
            page_count: p.page_count,
            file_size,
            linearized: p.linearized,
            tagged: p.tagged,
            encryption,
            fonts: p.fonts.iter().map(FontJs::from).collect(),
            signatures: p.signatures,
            signature_fields: p.signature_fields,
            contains: ContainsJs {
                javascript: p.contains.javascript,
                attachments: p.contains.attachments,
                forms: p.contains.forms,
            },
        }
    }
}

#[wasm_bindgen]
impl WasmDocument {
    /// The document's properties (the extract and text modules). Reads every object once.
    ///
    /// # Errors
    /// `data-pending` and the reading errors.
    #[wasm_bindgen(unchecked_return_type = "PropertiesJs")]
    pub fn properties(&self) -> JsResult<JsValue> {
        let properties = self.shared().borrow().properties().map_err(|e| lamina_error(&e))?;
        Ok(to_js(&PropertiesJs::from(properties)))
    }
}
